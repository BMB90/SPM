//! End-to-end test of the capture pipeline (`SessionManager::capture` ->
//! `EventProcessor` -> `DependencyGraphBuilder` -> `TimelineBuilder`)
//! using synthetic collectors instead of real OS instrumentation. This is
//! the "mock collectors" / "synthetic boot event generator" the spec asks
//! for: it lets the whole engine be exercised — and CI-tested on any OS —
//! without touching a real machine's registry, SCM, or ETW session.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use spm_core::{
    BootStage, Collector, CollectorCategory, CollectorContext, Event, EventPayload, Platform, ProcessInfo,
    ProcessRole, ServiceInfo, ServiceState, SpmError, SpmResult, StreamingCollector,
};
use spm_engine::{DependencyGraphBuilder, SessionManager, TimelineBuilder};
use tokio::sync::mpsc;

/// Emits a small, deterministic synthetic boot: kernel -> init -> two
/// services -> a user application, plus matching `BootStageReached`
/// markers. Mirrors real collector output shape exactly (same `Event`
/// envelope), so it's a faithful stand-in for `spm-collector-windows`/
/// `spm-collector-linux` in tests.
struct SyntheticBootCollector;

#[async_trait]
impl Collector for SyntheticBootCollector {
    fn id(&self) -> &'static str {
        "test.synthetic_boot"
    }
    fn platform(&self) -> Platform {
        Platform::Linux
    }
    fn category(&self) -> CollectorCategory {
        CollectorCategory::Process
    }
    fn description(&self) -> &'static str {
        "synthetic boot sequence for tests"
    }

    async fn collect(&self, ctx: &CollectorContext) -> SpmResult<Vec<Event>> {
        let session_id = ctx.session.id;
        let now = chrono::Utc::now();
        let mut events = Vec::new();

        let mut kernel = ProcessInfo::new(session_id, 0, "kernel");
        kernel.role = ProcessRole::KernelProcess;
        kernel.start_time = Some(now);
        events.push(Event::new(session_id, self.id(), EventPayload::ProcessStarted(Box::new(kernel))));

        let mut init = ProcessInfo::new(session_id, 1, "init");
        init.ppid = Some(0);
        init.role = ProcessRole::System;
        init.start_time = Some(now + chrono::Duration::milliseconds(50));
        init.exit_time = Some(now + chrono::Duration::milliseconds(5000));
        events.push(Event::new(session_id, self.id(), EventPayload::ProcessStarted(Box::new(init))));

        let mut svc_a = ProcessInfo::new(session_id, 100, "network-manager");
        svc_a.ppid = Some(1);
        svc_a.owning_service = Some("NetworkManager".to_string());
        svc_a.role = ProcessRole::Service;
        svc_a.start_time = Some(now + chrono::Duration::milliseconds(400));
        svc_a.exit_time = Some(now + chrono::Duration::milliseconds(900));
        events.push(Event::new(session_id, self.id(), EventPayload::ProcessStarted(Box::new(svc_a))));

        let mut svc_b = ProcessInfo::new(session_id, 101, "docker");
        svc_b.ppid = Some(100);
        svc_b.owning_service = Some("docker".to_string());
        svc_b.role = ProcessRole::Service;
        svc_b.start_time = Some(now + chrono::Duration::milliseconds(900));
        svc_b.exit_time = Some(now + chrono::Duration::milliseconds(2200));
        events.push(Event::new(session_id, self.id(), EventPayload::ProcessStarted(Box::new(svc_b))));

        let mut app = ProcessInfo::new(session_id, 500, "chrome");
        app.ppid = Some(1);
        app.role = ProcessRole::UserApplication;
        app.start_time = Some(now + chrono::Duration::milliseconds(3000));
        events.push(Event::new(session_id, self.id(), EventPayload::ProcessStarted(Box::new(app))));

        let mut docker_service = ServiceInfo::new(session_id, "docker");
        docker_service.state = ServiceState::Running;
        docker_service.pid = Some(101);
        docker_service.depends_on = vec!["NetworkManager".to_string()];
        events.push(Event::new(session_id, self.id(), EventPayload::ServiceObserved(Box::new(docker_service))));

        for (stage, offset_ms) in [
            (BootStage::Kernel, 0),
            (BootStage::ServiceStartup, 400),
            (BootStage::DesktopReady, 3000),
        ] {
            events.push(Event::new(
                session_id,
                self.id(),
                EventPayload::BootStageReached {
                    stage,
                    timestamp: now + chrono::Duration::milliseconds(offset_ms),
                    detail: None,
                },
            ));
        }

        Ok(events)
    }
}

/// A collector that always fails — verifies the engine skips it without
/// aborting the whole capture.
struct FlakyCollector;

#[async_trait]
impl Collector for FlakyCollector {
    fn id(&self) -> &'static str {
        "test.flaky"
    }
    fn platform(&self) -> Platform {
        Platform::Linux
    }
    fn category(&self) -> CollectorCategory {
        CollectorCategory::Process
    }
    fn description(&self) -> &'static str {
        "always fails"
    }
    async fn collect(&self, _ctx: &CollectorContext) -> SpmResult<Vec<Event>> {
        Err(SpmError::Collector { collector: self.id().to_string(), message: "synthetic failure".to_string() })
    }
}

/// A streaming collector that emits one extra process shortly after
/// starting, then exits before the capture window closes.
struct SyntheticStreamingCollector;

#[async_trait]
impl StreamingCollector for SyntheticStreamingCollector {
    fn id(&self) -> &'static str {
        "test.synthetic_stream"
    }
    fn platform(&self) -> Platform {
        Platform::Linux
    }
    fn category(&self) -> CollectorCategory {
        CollectorCategory::Process
    }
    fn description(&self) -> &'static str {
        "synthetic streaming source for tests"
    }
    async fn stream(&self, ctx: &CollectorContext, tx: mpsc::Sender<Event>) -> SpmResult<()> {
        let mut late = ProcessInfo::new(ctx.session.id, 600, "late-daemon");
        late.ppid = Some(1);
        late.role = ProcessRole::Daemon;
        late.start_time = Some(chrono::Utc::now());
        let _ = tx.send(Event::new(ctx.session.id, self.id(), EventPayload::ProcessStarted(Box::new(late)))).await;
        Ok(())
    }
}

#[tokio::test]
async fn capture_merges_snapshot_and_streaming_collectors_and_skips_failures() {
    let manager = SessionManager::new(Platform::Linux);
    let session = manager.new_session("test-host", "Test OS 1.0");

    let snapshot: Vec<Arc<dyn Collector>> = vec![Arc::new(SyntheticBootCollector), Arc::new(FlakyCollector)];
    let streaming: Vec<Arc<dyn StreamingCollector>> = vec![Arc::new(SyntheticStreamingCollector)];

    let normalized = manager.capture(&session, snapshot, streaming, Duration::from_millis(300)).await;

    // 5 from the synthetic snapshot + 1 from the streaming collector.
    assert_eq!(normalized.processes.len(), 6);
    assert!(normalized.processes.contains_key(&0));
    assert!(normalized.processes.contains_key(&600), "streaming collector's process should be merged in");
    assert_eq!(normalized.services.len(), 1);
    assert_eq!(normalized.boot_stage_events.len(), 3);

    let docker = &normalized.processes[&101];
    assert_eq!(docker.owning_service.as_deref(), Some("docker"));
}

#[tokio::test]
async fn full_pipeline_builds_graph_and_timeline_with_critical_path() {
    let manager = SessionManager::new(Platform::Linux);
    let session = manager.new_session("test-host", "Test OS 1.0");

    let snapshot: Vec<Arc<dyn Collector>> = vec![Arc::new(SyntheticBootCollector)];
    let normalized = manager.capture(&session, snapshot, vec![], Duration::from_millis(50)).await;

    let graph = DependencyGraphBuilder::build(session.id, &normalized);
    assert!(graph.nodes.iter().any(|n| n.id == "kernel"));
    // process(0) -> process(1) -> process(100) -> process(101) -> nothing further;
    // process(1) -> process(500) is a separate, shorter branch.
    assert!(graph.edges.iter().any(|e| e.from == "kernel" && e.to == "process:0"));

    let critical_path = DependencyGraphBuilder::critical_path(&graph, &normalized);
    assert!(critical_path.contains("process:101"), "the longest-lifetime chain should reach the docker process");

    let timeline = TimelineBuilder::build(&session, &normalized, &critical_path);
    assert!(!timeline.is_empty());
    assert!(timeline.iter().any(|t| t.on_critical_path), "timeline should mark at least one entry as on the critical path");
    // Entries must be sorted by timestamp (== non-decreasing offset).
    for pair in timeline.windows(2) {
        assert!(pair[0].offset_seconds <= pair[1].offset_seconds);
    }
}
