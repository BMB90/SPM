use std::sync::Arc;
use std::time::Duration;

use spm_core::{BootSession, Collector, CollectorContext, Event, Platform, StreamingCollector};
use tokio::sync::{mpsc, watch};
use tracing::{error, info, warn};

use crate::normalize::{EventProcessor, NormalizedSession};

/// Orchestrates one full capture: runs every applicable snapshot collector
/// concurrently, drains every streaming collector for the configured
/// capture window, and normalizes the combined event stream.
///
/// `spm-engine` never constructs collectors itself — callers (`spm-cli`,
/// `spm-api`) inject the platform-appropriate `Collector`/`StreamingCollector`
/// trait objects, keeping this crate free of any OS-specific dependency.
pub struct SessionManager {
    platform: Platform,
}

impl SessionManager {
    pub fn new(platform: Platform) -> Self {
        Self { platform }
    }

    pub fn new_session(&self, hostname: impl Into<String>, os_version: impl Into<String>) -> BootSession {
        BootSession::new(hostname, self.platform, os_version)
    }

    /// Run every collector, merge their output, and return the normalized
    /// view. Individual collector failures are logged and skipped rather
    /// than aborting the whole capture.
    pub async fn capture(
        &self,
        session: &BootSession,
        snapshot_collectors: Vec<Arc<dyn Collector>>,
        streaming_collectors: Vec<Arc<dyn StreamingCollector>>,
        capture_window: Duration,
    ) -> NormalizedSession {
        let (cancel_tx, cancel_rx) = watch::channel(false);
        let ctx = CollectorContext {
            session: session.clone(),
            capture_window,
            cancel: cancel_rx,
        };

        let mut all_events = Vec::new();

        let mut snapshot_set = tokio::task::JoinSet::new();
        for collector in snapshot_collectors {
            let ctx = ctx.clone();
            snapshot_set.spawn(async move {
                if let Err(e) = collector.is_available() {
                    warn!(collector = collector.id(), error = %e, "collector unavailable, skipping");
                    return Vec::new();
                }
                info!(collector = collector.id(), "running snapshot collector");
                match collector.collect(&ctx).await {
                    Ok(events) => events,
                    Err(e) => {
                        error!(collector = collector.id(), error = %e, "collector failed");
                        Vec::new()
                    }
                }
            });
        }
        while let Some(result) = snapshot_set.join_next().await {
            match result {
                Ok(events) => all_events.extend(events),
                Err(e) => error!(error = %e, "snapshot collector task panicked"),
            }
        }

        if !streaming_collectors.is_empty() {
            let (tx, mut rx) = mpsc::channel::<Event>(1024);
            let mut handles = Vec::new();
            for collector in streaming_collectors {
                if let Err(e) = collector.is_available() {
                    warn!(collector = collector.id(), error = %e, "streaming collector unavailable, skipping");
                    continue;
                }
                let ctx = ctx.clone();
                let tx = tx.clone();
                info!(collector = collector.id(), "starting streaming collector");
                handles.push(tokio::spawn(async move {
                    if let Err(e) = collector.stream(&ctx, tx).await {
                        error!(collector = collector.id(), error = %e, "streaming collector failed");
                    }
                }));
            }
            drop(tx);

            let window = tokio::time::sleep(capture_window);
            tokio::pin!(window);
            loop {
                tokio::select! {
                    _ = &mut window => {
                        let _ = cancel_tx.send(true);
                        break;
                    }
                    maybe_event = rx.recv() => {
                        match maybe_event {
                            Some(event) => all_events.push(event),
                            None => break,
                        }
                    }
                }
            }
            // Drain anything already queued after cancellation was signaled.
            while let Ok(event) = rx.try_recv() {
                all_events.push(event);
            }
            for handle in handles {
                let _ = handle.await;
            }
        }

        EventProcessor::new(session.id).process(all_events)
    }
}
