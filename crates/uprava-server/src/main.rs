use std::{
    env,
    io::{Read, Write},
    net::{SocketAddr, TcpStream, ToSocketAddrs},
    path::Path,
    time::Duration,
};

use chrono::{DateTime, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use uprava_logging::init_tracing;
use uprava_server::{build_router, shutdown_signal, AppConfig, AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("healthcheck") => {
            let address = args.next().unwrap_or_else(|| "127.0.0.1:8080".to_owned());
            return run_healthcheck(&address);
        }
        Some("deployment-status") => {
            let node_name = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("deployment-status requires a Node name"))?;
            let node_version = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("deployment-status requires a Node version"))?;
            let max_age_seconds = args
                .next()
                .unwrap_or_else(|| "45".to_owned())
                .parse::<i64>()?;
            return run_deployment_status(&node_name, &node_version, max_age_seconds).await;
        }
        Some(command) => anyhow::bail!("unknown command: {command}"),
        None => {}
    }

    let _log_path = init_tracing("core", "UPRAVA_CORE_LOG_FILE", ".local/logs/core.log")?;

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
        profile = ?config.profile,
        "starting uprava core"
    );

    let listener = tokio::net::TcpListener::bind(address).await?;
    let shutdown_timeout = Duration::from_secs(config.core_shutdown_timeout_seconds.max(0) as u64);
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let shutdown_task = tokio::spawn(async move {
        shutdown_signal().await;
        let _ = shutdown_tx.send(true);
    });
    let server = std::future::IntoFuture::into_future(
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(wait_for_shutdown(shutdown_rx.clone())),
    );
    tokio::pin!(server);
    let mut shutdown_rx = shutdown_rx;

    tokio::select! {
        result = &mut server => {
            shutdown_task.abort();
            result?;
        }
        changed = shutdown_rx.changed() => {
            if changed.is_ok() {
                tracing::info!(
                    timeout_seconds = shutdown_timeout.as_secs(),
                    "shutdown signal received; waiting for graceful core shutdown"
                );
            }
            match tokio::time::timeout(shutdown_timeout, &mut server).await {
                Ok(result) => result?,
                Err(_) => {
                    tracing::warn!(
                        timeout_seconds = shutdown_timeout.as_secs(),
                        "core graceful shutdown timed out; forcing exit"
                    );
                }
            }
        }
    }

    Ok(())
}

async fn run_deployment_status(
    expected_name: &str,
    expected_version: &str,
    max_age_seconds: i64,
) -> anyhow::Result<()> {
    if max_age_seconds < 0 {
        anyhow::bail!("maximum heartbeat age must be non-negative");
    }

    let config = AppConfig::from_env()?;
    let options = config
        .database_url
        .parse::<SqliteConnectOptions>()?
        .read_only(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    let node = sqlx::query_as::<_, (String, Option<DateTime<Utc>>, String)>(
        "select presence, last_heartbeat_at, daemon_version from nodes where display_name = ?1 order by updated_at desc limit 1",
    )
    .bind(expected_name)
    .fetch_optional(&pool)
    .await?;

    validate_deployment_node(node, expected_version, max_age_seconds, Utc::now())?;
    println!("Node {expected_name} is ready at version {expected_version}");
    Ok(())
}

fn validate_deployment_node(
    node: Option<(String, Option<DateTime<Utc>>, String)>,
    expected_version: &str,
    max_age_seconds: i64,
    now: DateTime<Utc>,
) -> anyhow::Result<()> {
    let Some((presence, heartbeat, version)) = node else {
        anyhow::bail!("expected production Node is not enrolled");
    };
    if presence != "reachable" {
        anyhow::bail!("production Node is not reachable");
    }
    if version != expected_version {
        anyhow::bail!("Node version mismatch: expected {expected_version}, got {version}");
    }
    let heartbeat = heartbeat.ok_or_else(|| anyhow::anyhow!("Node heartbeat is missing"))?;
    let age = now.signed_duration_since(heartbeat).num_seconds();
    if age < 0 || age > max_age_seconds {
        anyhow::bail!("Node heartbeat age {age}s is outside 0..={max_age_seconds}s");
    }
    Ok(())
}

async fn wait_for_shutdown(mut shutdown_rx: tokio::sync::watch::Receiver<bool>) {
    loop {
        if *shutdown_rx.borrow_and_update() {
            return;
        }
        if shutdown_rx.changed().await.is_err() {
            return;
        }
    }
}

fn run_healthcheck(address: &str) -> anyhow::Result<()> {
    let addrs: Vec<_> = address.to_socket_addrs()?.collect();
    if addrs.is_empty() {
        anyhow::bail!("healthcheck address did not resolve");
    }
    run_healthcheck_addrs(addrs)
}

fn run_healthcheck_addrs(addrs: impl IntoIterator<Item = SocketAddr>) -> anyhow::Result<()> {
    let mut errors = Vec::new();
    for address in addrs {
        match run_healthcheck_addr(address) {
            Ok(()) => return Ok(()),
            Err(error) => errors.push(format!("{address}: {error}")),
        }
    }
    anyhow::bail!(
        "healthcheck failed for all resolved addresses: {}",
        errors.join("; ")
    );
}

fn run_healthcheck_addr(address: SocketAddr) -> anyhow::Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn healthcheck_tries_later_address_after_failed_first_address() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listener binds");
        let good_address = listener.local_addr().expect("local address reads");
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("healthcheck connects");
            let mut request = [0_u8; 256];
            let _ = stream.read(&mut request).expect("request reads");
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                .expect("response writes");
        });

        run_healthcheck_addrs([SocketAddr::from(([127, 0, 0, 1], 9)), good_address])
            .expect("second address succeeds");
        server.join().expect("server thread joins");
    }

    #[test]
    fn deployment_node_accepts_fresh_matching_heartbeat() {
        let now = Utc::now();
        let result = validate_deployment_node(
            Some(("reachable".to_owned(), Some(now), "0.2.3".to_owned())),
            "0.2.3",
            45,
            now,
        );

        assert!(result.is_ok(), "matching Node should be ready: {result:?}");
    }

    #[test]
    fn deployment_node_rejects_stale_heartbeat() {
        let now = Utc::now();
        let heartbeat = now - chrono::Duration::seconds(46);
        let error = validate_deployment_node(
            Some(("reachable".to_owned(), Some(heartbeat), "0.2.3".to_owned())),
            "0.2.3",
            45,
            now,
        )
        .expect_err("stale Node must fail readiness");

        assert!(error.to_string().contains("heartbeat age"));
    }
}
