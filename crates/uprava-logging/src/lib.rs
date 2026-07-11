use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc::{self, SyncSender, TrySendError},
        OnceLock,
    },
    thread,
};

use opentelemetry::trace::TracerProvider as _;
use opentelemetry_sdk::{
    error::OTelSdkResult,
    trace::{SpanData, SpanExporter},
    Resource,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Debug, thiserror::Error)]
pub enum LoggingError {
    #[error("failed to create log directory `{path}`")]
    CreateDirectory { path: PathBuf, source: io::Error },
    #[error("failed to open log file `{path}`")]
    OpenFile { path: PathBuf, source: io::Error },
    #[error("failed to initialize tracing subscriber")]
    SubscriberInit {
        #[source]
        source: tracing_subscriber::util::TryInitError,
    },
}

const DEFAULT_LOG_CHANNEL_CAPACITY: usize = 8_192;
const DEFAULT_LOG_MAX_BYTES: u64 = 10 * 1024 * 1024;
const DEFAULT_LOG_MAX_FILES: usize = 5;
static DROPPED_LOG_RECORDS: AtomicU64 = AtomicU64::new(0);
static OTLP_EXPORT_FAILURES: AtomicU64 = AtomicU64::new(0);
static OTLP_PROVIDER: OnceLock<opentelemetry_sdk::trace::SdkTracerProvider> = OnceLock::new();

#[derive(Debug)]
struct CountingOtlpExporter(opentelemetry_otlp::SpanExporter);

impl SpanExporter for CountingOtlpExporter {
    async fn export(&self, batch: Vec<SpanData>) -> OTelSdkResult {
        let result = self.0.export(batch).await;
        if result.is_err() {
            OTLP_EXPORT_FAILURES.fetch_add(1, Ordering::Relaxed);
        }
        result
    }

    fn shutdown_with_timeout(&self, timeout: std::time::Duration) -> OTelSdkResult {
        self.0.shutdown_with_timeout(timeout)
    }

    fn force_flush(&self) -> OTelSdkResult {
        self.0.force_flush()
    }

    fn set_resource(&mut self, resource: &Resource) {
        self.0.set_resource(resource);
    }
}

#[derive(Clone)]
struct NonBlockingLogWriter {
    sender: SyncSender<Vec<u8>>,
}

struct SharedLogFileWriter {
    sender: SyncSender<Vec<u8>>,
}

impl<'writer> tracing_subscriber::fmt::MakeWriter<'writer> for NonBlockingLogWriter {
    type Writer = SharedLogFileWriter;

    fn make_writer(&'writer self) -> Self::Writer {
        SharedLogFileWriter {
            sender: self.sender.clone(),
        }
    }
}

impl Write for SharedLogFileWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        match self.sender.try_send(buffer.to_vec()) {
            Ok(()) => Ok(buffer.len()),
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {
                DROPPED_LOG_RECORDS.fetch_add(1, Ordering::Relaxed);
                Ok(buffer.len())
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub fn dropped_log_records() -> u64 {
    DROPPED_LOG_RECORDS.load(Ordering::Relaxed)
}

pub fn otlp_export_failures() -> u64 {
    OTLP_EXPORT_FAILURES.load(Ordering::Relaxed)
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
    let max_bytes = env_u64("UPRAVA_LOG_MAX_BYTES", DEFAULT_LOG_MAX_BYTES);
    let max_files = env_usize("UPRAVA_LOG_MAX_FILES", DEFAULT_LOG_MAX_FILES).max(1);
    let capacity = env_usize("UPRAVA_LOG_CHANNEL_CAPACITY", DEFAULT_LOG_CHANNEL_CAPACITY).max(1);
    let writer = start_log_writer(log_path.clone(), capacity, max_bytes, max_files)?;

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let stderr_layer = tracing_subscriber::fmt::layer()
        .with_writer(io::stderr)
        .with_target(true);
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(writer)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true);
    let otlp_layer = optional_otlp_provider()
        .map(|provider| tracing_opentelemetry::layer().with_tracer(provider.tracer(service_name)));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(stderr_layer)
        .with(file_layer)
        .with(otlp_layer)
        .try_init()
        .map_err(|source| LoggingError::SubscriberInit { source })?;
    tracing::info!(service = service_name, "file logging initialized");
    Ok(log_path)
}

fn optional_otlp_provider() -> Option<&'static opentelemetry_sdk::trace::SdkTracerProvider> {
    let enabled = std::env::var("UPRAVA_OTLP_ENABLED")
        .ok()
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on"))
        || std::env::var_os("OTEL_EXPORTER_OTLP_ENDPOINT").is_some();
    if !enabled {
        return None;
    }
    if let Some(provider) = OTLP_PROVIDER.get() {
        return Some(provider);
    }
    let exporter = match opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .build()
    {
        Ok(exporter) => exporter,
        Err(error) => {
            OTLP_EXPORT_FAILURES.fetch_add(1, Ordering::Relaxed);
            eprintln!("OTLP exporter disabled after initialization failure: {error}");
            return None;
        }
    };
    let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_batch_exporter(CountingOtlpExporter(exporter))
        .build();
    let _ = OTLP_PROVIDER.set(provider);
    OTLP_PROVIDER.get()
}

fn start_log_writer(
    path: PathBuf,
    capacity: usize,
    max_bytes: u64,
    max_files: usize,
) -> Result<NonBlockingLogWriter, LoggingError> {
    let file = open_log_file(&path).map_err(|source| LoggingError::OpenFile {
        path: path.clone(),
        source,
    })?;
    let (sender, receiver) = mpsc::sync_channel::<Vec<u8>>(capacity);
    thread::Builder::new()
        .name("uprava-log-writer".to_owned())
        .spawn(move || {
            let mut file = file;
            let mut size = file.metadata().map(|metadata| metadata.len()).unwrap_or(0);
            while let Ok(record) = receiver.recv() {
                if max_bytes > 0 && size.saturating_add(record.len() as u64) > max_bytes {
                    match rotate_logs(&path, max_files).and_then(|()| open_log_file(&path)) {
                        Ok(next) => {
                            file = next;
                            size = 0;
                        }
                        Err(_) => {
                            DROPPED_LOG_RECORDS.fetch_add(1, Ordering::Relaxed);
                            continue;
                        }
                    }
                }
                if file.write_all(&record).is_err() {
                    DROPPED_LOG_RECORDS.fetch_add(1, Ordering::Relaxed);
                } else {
                    size = size.saturating_add(record.len() as u64);
                }
            }
            let _ = file.flush();
        })
        .map_err(|source| LoggingError::OpenFile {
            path: log_path_for_thread_error(),
            source,
        })?;
    Ok(NonBlockingLogWriter { sender })
}

fn open_log_file(path: &Path) -> io::Result<File> {
    OpenOptions::new().create(true).append(true).open(path)
}

fn rotate_logs(path: &Path, max_files: usize) -> io::Result<()> {
    if max_files <= 1 {
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        return Ok(());
    }
    let oldest = rotated_path(path, max_files - 1);
    if oldest.exists() {
        std::fs::remove_file(oldest)?;
    }
    for index in (1..max_files - 1).rev() {
        let current = rotated_path(path, index);
        if current.exists() {
            std::fs::rename(current, rotated_path(path, index + 1))?;
        }
    }
    if path.exists() {
        std::fs::rename(path, rotated_path(path, 1))?;
    }
    Ok(())
}

fn rotated_path(path: &Path, index: usize) -> PathBuf {
    PathBuf::from(format!("{}.{}", path.display(), index))
}

fn env_u64(name: &str, fallback: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(fallback)
}

fn env_usize(name: &str, fallback: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(fallback)
}

fn log_path_for_thread_error() -> PathBuf {
    PathBuf::from("<log-writer-thread>")
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn init_tracing_writes_events_to_configured_file() {
        let path = std::env::temp_dir().join(format!(
            "uprava-logging-test-{}-{}.log",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock is after unix epoch")
                .as_nanos()
        ));

        std::env::set_var("RUST_LOG", "info");
        init_tracing("test", "UPRAVA_LOGGING_TEST_FILE", &path).expect("tracing initializes");
        tracing::info!("logging helper test event");
        let content = (0..100)
            .find_map(|_| {
                let content = std::fs::read_to_string(&path).ok()?;
                content
                    .contains("logging helper test event")
                    .then_some(content)
                    .or_else(|| {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                        None
                    })
            })
            .expect("log writer flushes the event");
        let second_path = std::env::temp_dir().join(format!(
            "uprava-logging-double-init-test-{}-{}.log",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock is after unix epoch")
                .as_nanos()
        ));
        let second = init_tracing("test", "UPRAVA_LOGGING_DOUBLE_INIT_TEST_FILE", &second_path);
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(second_path);

        assert!(
            content.contains("logging helper test event"),
            "log file content was: {content}"
        );
        assert!(matches!(second, Err(LoggingError::SubscriberInit { .. })));
    }

    #[test]
    fn rotating_writer_bounds_retained_files() {
        let path = std::env::temp_dir().join(format!(
            "uprava-logging-rotation-{}-{}.log",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock is after unix epoch")
                .as_nanos()
        ));
        std::fs::write(&path, "current").expect("seed current log");
        std::fs::write(rotated_path(&path, 1), "previous").expect("seed previous log");
        rotate_logs(&path, 2).expect("logs rotate");

        assert!(!path.exists());
        assert!(rotated_path(&path, 1).exists());
        assert!(!rotated_path(&path, 2).exists());
        let _ = std::fs::remove_file(rotated_path(&path, 1));
    }
}
