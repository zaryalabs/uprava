use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Debug, thiserror::Error)]
pub enum LoggingError {
    #[error("failed to create log directory `{path}`")]
    CreateDirectory { path: PathBuf, source: io::Error },
    #[error("failed to open log file `{path}`")]
    OpenFile { path: PathBuf, source: io::Error },
}

#[derive(Clone)]
struct SharedLogFile {
    file: Arc<Mutex<File>>,
}

impl SharedLogFile {
    fn new(file: File) -> Self {
        Self {
            file: Arc::new(Mutex::new(file)),
        }
    }
}

struct SharedLogFileWriter {
    file: Arc<Mutex<File>>,
}

impl<'writer> tracing_subscriber::fmt::MakeWriter<'writer> for SharedLogFile {
    type Writer = SharedLogFileWriter;

    fn make_writer(&'writer self) -> Self::Writer {
        SharedLogFileWriter {
            file: Arc::clone(&self.file),
        }
    }
}

impl Write for SharedLogFileWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let mut file = self
            .file
            .lock()
            .map_err(|_| io::Error::other("log file lock poisoned"))?;
        let written = file.write(buffer)?;
        file.flush()?;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file
            .lock()
            .map_err(|_| io::Error::other("log file lock poisoned"))?
            .flush()
    }
}

pub fn init_tracing(
    service_name: &'static str,
    log_file_env: &'static str,
    default_log_path: impl AsRef<Path>,
) -> Result<PathBuf, LoggingError> {
    let log_path = std::env::var(log_file_env)
        .map(PathBuf::from)
        .unwrap_or_else(|_| default_log_path.as_ref().to_path_buf());
    if let Some(parent) = log_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).map_err(|source| LoggingError::CreateDirectory {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|source| LoggingError::OpenFile {
            path: log_path.clone(),
            source,
        })?;

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let stderr_layer = tracing_subscriber::fmt::layer()
        .with_writer(io::stderr)
        .with_target(true);
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(SharedLogFile::new(file))
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(stderr_layer)
        .with(file_layer)
        .init();
    tracing::info!(
        service = service_name,
        log_file = %log_path.display(),
        "file logging initialized"
    );
    Ok(log_path)
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn init_tracing_writes_events_to_configured_file() {
        let path = std::env::temp_dir().join(format!(
            "cortex-logging-test-{}-{}.log",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock is after unix epoch")
                .as_nanos()
        ));

        std::env::set_var("RUST_LOG", "info");
        init_tracing("test", "CORTEX_LOGGING_TEST_FILE", &path).expect("tracing initializes");
        tracing::info!("logging helper test event");
        let content = std::fs::read_to_string(&path).expect("log file reads");
        let _ = std::fs::remove_file(path);

        assert!(
            content.contains("logging helper test event"),
            "log file content was: {content}"
        );
    }
}
