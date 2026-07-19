mod capture;
mod platform_collectors;

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use spm_analysis::{HistoricalComparator, ReportGenerator};
use spm_storage::{Pagination, Storage};
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "spm", version, about = "SPM Startup Intelligence Platform — headless capture / query / report CLI")]
struct Cli {
    /// Path to the SQLite database (created if absent).
    #[arg(long, global = true, default_value = "./data/spm.db")]
    db: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Capture the current boot/startup state into a new session.
    Capture {
        /// How long to run streaming collectors (e.g. ETW) before closing the capture window.
        #[arg(long, default_value_t = 5)]
        capture_window_secs: u64,
        /// Skip per-process SHA-256 hashing and Authenticode signature checks (faster).
        #[arg(long)]
        no_enrich: bool,
        #[arg(long)]
        notes: Option<String>,
    },
    /// List captured sessions, newest first.
    Sessions {
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    /// Export a report for a session (defaults to the most recent).
    Report {
        #[arg(long)]
        session: Option<Uuid>,
        #[arg(long, value_enum, default_value_t = ReportFormat::Json)]
        format: ReportFormat,
        /// Output file path; defaults to stdout (ignored for `sqlite`, which always needs a path).
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Diff two sessions: added/removed processes, startup-item drift, boot duration delta.
    Compare {
        #[arg(long)]
        baseline: Uuid,
        #[arg(long)]
        target: Uuid,
    },
}

#[derive(Clone, ValueEnum)]
enum ReportFormat {
    Json,
    Csv,
    Markdown,
    Html,
    Sqlite,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).init();

    let cli = Cli::parse();
    if let Some(parent) = cli.db.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).context("creating database directory")?;
        }
    }
    let storage = Storage::open(&cli.db).context("opening database")?;

    match cli.command {
        Command::Capture { capture_window_secs, no_enrich, notes } => {
            let session_id =
                capture::run_capture(&storage, Duration::from_secs(capture_window_secs), !no_enrich, notes).await?;
            println!("Capture complete. Session: {session_id}");
        }
        Command::Sessions { limit } => {
            let page = storage.list_sessions(Pagination::new(limit, 0))?;
            let shown = page.items.len();
            let total = page.total;
            println!("{:<38} {:<20} {:<10} {:<25} STATUS", "SESSION ID", "HOSTNAME", "PLATFORM", "STARTED");
            for s in page.items {
                let status = if s.capture_completed_at.is_some() { "complete" } else { "incomplete" };
                println!(
                    "{:<38} {:<20} {:<10} {:<25} {}",
                    s.id,
                    s.hostname,
                    s.platform.to_string(),
                    s.capture_started_at.to_rfc3339(),
                    status
                );
            }
            println!("\n{shown} of {total} sessions shown");
        }
        Command::Report { session, format, out } => {
            let session_id = resolve_session(&storage, session)?;
            let generator = ReportGenerator::new(&storage);

            if matches!(format, ReportFormat::Sqlite) {
                let path = out.context("`--out <path>` is required for --format sqlite")?;
                generator.export_sqlite(session_id, &path)?;
                println!("Wrote SQLite export to {}", path.display());
                return Ok(());
            }

            let content = match format {
                ReportFormat::Json => generator.to_json(session_id)?,
                ReportFormat::Csv => generator.to_csv_processes(session_id)?,
                ReportFormat::Markdown => generator.to_markdown(session_id)?,
                ReportFormat::Html => generator.to_html(session_id)?,
                ReportFormat::Sqlite => unreachable!(),
            };

            match out {
                Some(path) => {
                    std::fs::write(&path, content)?;
                    println!("Wrote report to {}", path.display());
                }
                None => println!("{content}"),
            }
        }
        Command::Compare { baseline, target } => {
            let comparison = HistoricalComparator::new(&storage).compare(baseline, target)?;
            println!("{}", serde_json::to_string_pretty(&comparison)?);
        }
    }

    Ok(())
}

fn resolve_session(storage: &Storage, session: Option<Uuid>) -> Result<Uuid> {
    match session {
        Some(id) => Ok(id),
        None => storage
            .latest_session()?
            .map(|s| s.id)
            .context("no sessions found — run `spm capture` first"),
    }
}
