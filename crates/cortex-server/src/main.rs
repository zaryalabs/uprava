use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream, ToSocketAddrs},
    path::Path,
    time::Duration,
};

use cortex_logging::init_tracing;
use cortex_server::{build_router, shutdown_signal, AppConfig, AppState};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::args().nth(1).as_deref() == Some("healthcheck") {
        let address = std::env::args()
            .nth(2)
            .unwrap_or_else(|| "127.0.0.1:8080".to_owned());
        return run_healthcheck(&address);
    }

    let log_path = init_tracing("core", "CORTEX_CORE_LOG_FILE", ".local/logs/core.log")?;

    let config = AppConfig::from_env()?;
    ensure_sqlite_parent_dir(&config.database_url)?;
    let options = config
        .database_url
        .parse::<SqliteConnectOptions>()?
        .create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    let state = AppState::new(config.clone(), pool).await?;
    let app = build_router(state);
    let address: SocketAddr = config.bind_address.parse()?;

    tracing::info!(
        bind_address = %address,
        database_url = %config.database_url,
        log_file = %log_path.display(),
        client_log_file = %config.client_log_file.display(),
        profile = ?config.profile,
        "starting cortex core"
    );

    let listener = tokio::net::TcpListener::bind(address).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

fn run_healthcheck(address: &str) -> anyhow::Result<()> {
    let mut addrs = address.to_socket_addrs()?;
    let Some(address) = addrs.next() else {
        anyhow::bail!("healthcheck address did not resolve");
    };
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(2))?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    stream.write_all(
        b"GET /api/v1/health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    )?;

    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    if response.starts_with("HTTP/1.1 200") || response.starts_with("HTTP/1.0 200") {
        return Ok(());
    }
    anyhow::bail!("healthcheck returned non-200 response");
}

fn ensure_sqlite_parent_dir(database_url: &str) -> anyhow::Result<()> {
    let Some(path) = database_url.strip_prefix("sqlite://") else {
        return Ok(());
    };
    if path == ":memory:" || path.is_empty() {
        return Ok(());
    }
    let Some(parent) = Path::new(path).parent() else {
        return Ok(());
    };
    if !parent.as_os_str().is_empty() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}
