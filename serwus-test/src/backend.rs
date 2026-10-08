//! The service under test, run as a separate process, with its output in a log file.

use std::{
    path::PathBuf,
    process::Stdio,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use tokio::{
    process::{Child, Command},
    sync::Mutex,
};

/// How long the service may take to answer its healthcheck. Generous, because some services do
/// slow work before they open their port, like compiling WASM modules in a debug build.
pub const STARTUP_TIMEOUT: Duration = Duration::from_secs(180);

/// What serwus reads besides `DATABASE_URL`, as tests want it, and a backtrace for a panic.
const DEFAULT_ENV: [(&str, &str); 4] = [
    // Keeps the worker threads and the connection pool at 2
    ("TEST", "1"),
    ("ENV", "dev"),
    ("LOGGER_LEVEL", "info"),
    ("RUST_BACKTRACE", "1"),
];

pub struct Backend {
    child: Mutex<Option<Child>>,
    log_path: PathBuf,
}

impl Backend {
    /// Runs a serwus service and waits until it answers its healthcheck,
    /// `<base_url>/_healthcheck`.
    ///
    /// `command` runs the service's binary with the settings the app reads itself: its port,
    /// URLs, secrets. This adds the ones serwus reads: `DATABASE_URL`, and `TEST=1` (the worker
    /// threads and the connection pool at 2), `ENV=dev`, `LOGGER_LEVEL=info` and
    /// `RUST_BACKTRACE=1` unless `command` sets those already. Its stdout and stderr go to
    /// `log_path`. The process is killed when this is dropped, or by [`Backend::stop`].
    pub async fn start(
        mut command: Command,
        database_url: &str,
        base_url: &str,
        log_path: PathBuf,
    ) -> Result<Self> {
        command.env("DATABASE_URL", database_url);
        for (name, value) in DEFAULT_ENV {
            if !command.as_std().get_envs().any(|(set, _)| set == name) {
                command.env(name, value);
            }
        }

        let backend = Self::spawn(command, log_path)?;
        backend
            .wait_ready(&format!("{base_url}/_healthcheck"))
            .await?;
        Ok(backend)
    }

    fn spawn(mut command: Command, log_path: PathBuf) -> Result<Self> {
        if let Some(dir) = log_path.parent() {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("can't create {}", dir.display()))?;
        }
        let log = std::fs::File::create(&log_path)
            .with_context(|| format!("can't create {}", log_path.display()))?;

        let program = command.as_std().get_program().to_owned();
        println!(
            "Starting {}, log in {}",
            program.to_string_lossy(),
            log_path.display()
        );
        let child = command
            .stdin(Stdio::null())
            .stdout(log.try_clone()?)
            .stderr(log)
            .kill_on_drop(true)
            .spawn()
            .with_context(|| format!("can't run {}", program.to_string_lossy()))?;

        Ok(Self {
            child: Mutex::new(Some(child)),
            log_path,
        })
    }

    /// Waits until `url` answers with a success. Fails, with the end of the log, if the
    /// process exits or doesn't answer within [`STARTUP_TIMEOUT`].
    async fn wait_ready(&self, url: &str) -> Result<()> {
        let deadline = Instant::now() + STARTUP_TIMEOUT;
        let http = reqwest::Client::new();

        loop {
            if let Some(child) = self.child.lock().await.as_mut()
                && let Some(status) = child.try_wait()?
            {
                bail!("the backend exited ({status}):\n{}", self.log_tail());
            }

            if let Ok(response) = http.get(url).send().await
                && response.status().is_success()
            {
                return Ok(());
            }

            if Instant::now() > deadline {
                bail!("the backend didn't start in time:\n{}", self.log_tail());
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }

    pub async fn stop(&self) {
        if let Some(mut child) = self.child.lock().await.take() {
            let _ = child.kill().await;
        }
    }

    fn log_tail(&self) -> String {
        let log = std::fs::read_to_string(&self.log_path).unwrap_or_default();
        let lines: Vec<&str> = log.lines().collect();
        lines[lines.len().saturating_sub(40)..].join("\n")
    }
}
