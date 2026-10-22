use std::path::Path;
use std::sync::OnceLock;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{InitError, RollingFileAppender, Rotation};
use tracing_subscriber::EnvFilter;

const RELEASE_FILTER: &str = "warn,wallscape_win_lib=info";
const DEBUG_FILTER: &str = "warn,wallscape_win_lib=debug";
const MAX_LOG_FILES: usize = 7;
const LOG_PREFIX: &str = "wallscape.log";
static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

pub fn init() {
    let log_dir = std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Wallscape")
        .join("logs");

    let (writer, guard) = match file_appender(&log_dir) {
        Ok(appender) => tracing_appender::non_blocking(appender),
        Err(error) => {
            eprintln!("Failed to initialize Wallscape log files: {error}");
            tracing_appender::non_blocking(std::io::stderr())
        }
    };
    let _ = LOG_GUARD.set(guard);

    let directives = std::env::var("RUST_LOG").ok();
    tracing_subscriber::fmt()
        .with_writer(writer)
        .with_ansi(false)
        .with_env_filter(log_filter(directives.as_deref(), cfg!(debug_assertions)))
        .init();
}

fn file_appender(log_dir: &Path) -> Result<RollingFileAppender, InitError> {
    RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix(LOG_PREFIX)
        .max_log_files(MAX_LOG_FILES)
        .build(log_dir)
}

fn log_filter(directives: Option<&str>, debug: bool) -> EnvFilter {
    if let Some(directives) = directives.filter(|value| !value.trim().is_empty()) {
        match EnvFilter::builder().with_regex(false).parse(directives) {
            Ok(filter) => return filter,
            Err(error) => eprintln!("Invalid RUST_LOG filter; using Wallscape defaults: {error}"),
        }
    }

    EnvFilter::new(if debug { DEBUG_FILTER } else { RELEASE_FILTER })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::sync::Mutex;

    fn captured_logs(directives: Option<&str>, debug: bool) -> String {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("captured.log");
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .with_writer(Mutex::new(fs::File::create(&path).unwrap()))
            .with_env_filter(log_filter(directives, debug))
            .finish();
        tracing::subscriber::with_default(subscriber, || {
            tracing::debug!(target: "wallscape_win_lib::test", "app-debug");
            tracing::info!(target: "wallscape_win_lib::test", "app-info");
            tracing::warn!(target: "wallscape_win_lib::test", "app-warning");
            tracing::error!(target: "wallscape_win_lib::test", "app-error");
            tracing::debug!(target: "dependency::test", "dependency-debug");
            tracing::info!(target: "dependency::test", "dependency-info");
            tracing::warn!(target: "dependency::test", "dependency-warning");
        });
        fs::read_to_string(path).unwrap()
    }

    #[test]
    fn release_logs_summaries_and_warnings_without_debug_noise() {
        let logs = captured_logs(None, false);
        for expected in ["app-info", "app-warning", "app-error", "dependency-warning"] {
            assert!(logs.contains(expected), "missing {expected}: {logs}");
        }
        for excluded in ["app-debug", "dependency-info", "dependency-debug"] {
            assert!(!logs.contains(excluded), "unexpected {excluded}: {logs}");
        }
    }

    #[test]
    fn development_enables_app_debug_without_dependency_noise() {
        let logs = captured_logs(None, true);
        assert!(logs.contains("app-debug"));
        assert!(!logs.contains("dependency-debug"));
        assert!(!logs.contains("dependency-info"));
    }

    #[test]
    fn diagnostic_filter_enables_app_debug_in_release() {
        let logs = captured_logs(Some("warn,wallscape_win_lib=debug"), false);
        assert!(logs.contains("app-debug"));
        assert!(logs.contains("dependency-warning"));
        assert!(!logs.contains("dependency-debug"));
    }

    #[test]
    fn explicit_filter_can_reduce_verbosity_further() {
        let logs = captured_logs(Some("error"), false);
        assert!(logs.contains("app-error"));
        assert!(!logs.contains("app-info"));
        assert!(!logs.contains("app-warning"));
        assert!(!logs.contains("dependency-warning"));
    }

    #[test]
    fn invalid_or_empty_filter_preserves_release_defaults() {
        for directives in ["", "   ", "wallscape_win_lib=invalid"] {
            let logs = captured_logs(Some(directives), false);
            assert!(logs.contains("app-warning"));
            assert!(logs.contains("app-info"));
            assert!(!logs.contains("app-debug"));
        }
    }

    #[test]
    fn daily_retention_prunes_old_logs_and_preserves_unrelated_files() {
        let dir = tempfile::tempdir().unwrap();
        for day in 1..=10 {
            fs::write(
                dir.path().join(format!("{LOG_PREFIX}.2000-01-{day:02}")),
                "old",
            )
            .unwrap();
        }
        // The previous unrotated file is also eligible for retention cleanup.
        fs::write(dir.path().join(LOG_PREFIX), "legacy").unwrap();
        let unrelated = dir.path().join("other.log");
        fs::write(&unrelated, "keep").unwrap();

        let mut appender = file_appender(dir.path()).unwrap();
        appender.write_all(b"current-log").unwrap();
        appender.flush().unwrap();
        drop(appender);

        let logs: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap())
            .filter(|entry| entry.file_name().to_string_lossy().starts_with(LOG_PREFIX))
            .collect();
        assert_eq!(logs.len(), MAX_LOG_FILES);
        assert!(logs
            .iter()
            .any(|entry| { fs::read_to_string(entry.path()).unwrap() == "current-log" }));
        assert_eq!(fs::read_to_string(unrelated).unwrap(), "keep");
    }

    #[test]
    fn unavailable_log_directory_returns_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("file-not-directory");
        fs::write(&path, "keep").unwrap();
        assert!(file_appender(&path).is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), "keep");
    }
}
