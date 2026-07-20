//! Private ToolHive bridge for a bare-metal Uprava Node.
//!
//! The bridge is intentionally narrow: it owns the pinned ToolHive CLI and
//! local MCP proxy inside Compose, while Node retains reconciliation and Core
//! remains the policy and trace authority.

use std::{net::SocketAddr, process::Stdio, sync::Arc, time::Duration};

use anyhow::Context;
use axum::{
    extract::{DefaultBodyLimit, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::{Duration as ChronoDuration, Utc};
use serde::Serialize;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, BufReader, Lines},
    process::{ChildStderr, Command},
    sync::Mutex,
    time::timeout,
};
use tower_http::trace::TraceLayer;
use uprava_logging::init_tracing;
use uprava_protocol::{
    JsonValue, ToolhiveBridgeAuthorizationResponse, ToolhiveBridgeMcpRequest,
    ToolhiveBridgeMcpResponse, ToolhiveBridgeVersionResponse, ToolhiveBridgeWorkloadRequest,
    ToolhiveBridgeWorkloadStatusResponse, TOOLHIVE_BRIDGE_CONTRACT_VERSION_V1,
    TOOLING_MCP_REVISION, TOOL_RESULT_MAX_BYTES,
};

const LINEAR_MCP_URL: &str = "https://mcp.linear.app/mcp";
const LINEAR_WORKLOAD_NAME: &str = "uprava-linear";
const LINEAR_PROXY_PORT: u16 = 18_766;
const LINEAR_CALLBACK_PORT: u16 = 18_765;
const TOOLHIVE_COMMAND_TIMEOUT: Duration = Duration::from_secs(30);
const TOOL_PROBE_TIMEOUT: Duration = Duration::from_secs(3);
const TOOLHIVE_START_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const AUTHORIZATION_URL_CAPTURE_TIMEOUT: Duration = Duration::from_secs(45);
const AUTHORIZATION_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const MCP_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_PROCESS_OUTPUT_BYTES: usize = 256 * 1024;
const MAX_REQUEST_BODY_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone)]
struct BridgeConfig {
    bind: SocketAddr,
    toolhive_binary: String,
}

impl BridgeConfig {
    fn from_env() -> anyhow::Result<Self> {
        let bind = std::env::var("UPRAVA_TOOLHIVE_BIND")
            .unwrap_or_else(|_| "0.0.0.0:18081".to_owned())
            .parse()
            .context("UPRAVA_TOOLHIVE_BIND must be a socket address")?;
        let toolhive_binary =
            std::env::var("UPRAVA_TOOLHIVE_BINARY").unwrap_or_else(|_| "thv".to_owned());
        Ok(Self {
            bind,
            toolhive_binary,
        })
    }
}

struct AppState {
    config: BridgeConfig,
    client: reqwest::Client,
    workload_lock: Mutex<()>,
}

#[derive(Debug)]
struct BridgeError {
    status: StatusCode,
    code: &'static str,
    message: &'static str,
}

impl BridgeError {
    fn bad_request(message: &'static str) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "toolhive.invalid_request",
            message,
        }
    }

    fn unavailable(code: &'static str, message: &'static str) -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code,
            message,
        }
    }
}

#[derive(Serialize)]
struct ErrorEnvelope {
    code: &'static str,
    message: &'static str,
}

impl IntoResponse for BridgeError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorEnvelope {
                code: self.code,
                message: self.message,
            }),
        )
            .into_response()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::args().nth(1).as_deref() == Some("healthcheck") {
        reqwest::get("http://127.0.0.1:18081/health")
            .await
            .context("ToolHive bridge healthcheck request failed")?
            .error_for_status()
            .context("ToolHive bridge healthcheck returned an error")?;
        return Ok(());
    }
    let _log_path = init_tracing(
        "toolhive-bridge",
        "UPRAVA_TOOLHIVE_LOG_FILE",
        ".local/logs/toolhive-bridge.log",
    )?;
    let config = BridgeConfig::from_env()?;
    let client = reqwest::Client::builder()
        .timeout(MCP_REQUEST_TIMEOUT)
        .build()
        .context("failed to build ToolHive bridge HTTP client")?;
    let state = Arc::new(AppState {
        config: config.clone(),
        client,
        workload_lock: Mutex::new(()),
    });
    let app = Router::new()
        .route("/health", get(health))
        .route("/api/v1/version", get(version))
        .route(
            "/api/v1/workloads/{workload_name}",
            get(workload_status).delete(delete_workload),
        )
        .route(
            "/api/v1/workloads/{workload_name}/authorize",
            post(begin_authorization),
        )
        .route(
            "/api/v1/workloads/{workload_name}/start",
            post(start_workload),
        )
        .route("/api/v1/workloads/{workload_name}/mcp", post(mcp_request))
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BODY_BYTES))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(config.bind)
        .await
        .context("failed to bind ToolHive bridge")?;
    tracing::info!(bind = %config.bind, "starting ToolHive bridge");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("ToolHive bridge stopped unexpectedly")
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("SIGTERM handler must install");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = terminate.recv() => {}
        }
    }
    #[cfg(not(unix))]
    let _ = tokio::signal::ctrl_c().await;
}

async fn health(State(state): State<Arc<AppState>>) -> Result<StatusCode, BridgeError> {
    toolhive_version(&state).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn version(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ToolhiveBridgeVersionResponse>, BridgeError> {
    Ok(Json(ToolhiveBridgeVersionResponse {
        version: toolhive_version(&state).await?,
    }))
}

async fn workload_status(
    State(state): State<Arc<AppState>>,
    Path(workload_name): Path<String>,
) -> Result<Json<ToolhiveBridgeWorkloadStatusResponse>, BridgeError> {
    validate_workload_name(&workload_name)?;
    Ok(Json(ToolhiveBridgeWorkloadStatusResponse {
        running: workload_is_running(&state, &workload_name).await,
    }))
}

async fn begin_authorization(
    State(state): State<Arc<AppState>>,
    Path(workload_name): Path<String>,
    Json(request): Json<ToolhiveBridgeWorkloadRequest>,
) -> Result<Json<ToolhiveBridgeAuthorizationResponse>, BridgeError> {
    validate_workload_request(&workload_name, &request)?;
    let _workload_guard = state.workload_lock.lock().await;
    cleanup_workload(&state, &workload_name).await;
    let response = capture_authorization_url(&state, &request).await?;
    Ok(Json(response))
}

async fn start_workload(
    State(state): State<Arc<AppState>>,
    Path(workload_name): Path<String>,
    Json(request): Json<ToolhiveBridgeWorkloadRequest>,
) -> Result<StatusCode, BridgeError> {
    validate_workload_request(&workload_name, &request)?;
    let _workload_guard = state.workload_lock.lock().await;
    if workload_is_running(&state, &workload_name).await {
        return Ok(StatusCode::NO_CONTENT);
    }
    cleanup_workload(&state, &workload_name).await;
    let proxy_port = request.proxy_port.to_string();
    let callback_port = request.callback_port.to_string();
    run_toolhive(
        &state,
        &[
            "run",
            request.upstream_url.as_str(),
            "--name",
            request.workload_name.as_str(),
            "--remote-auth",
            "--remote-auth-skip-browser",
            "--remote-auth-timeout",
            "5m",
            "--remote-auth-callback-port",
            callback_port.as_str(),
            "--proxy-port",
            proxy_port.as_str(),
        ],
        TOOLHIVE_START_TIMEOUT,
    )
    .await
    .map_err(|error| {
        tracing::warn!(error = %error, "ToolHive workload start failed");
        BridgeError::unavailable("toolhive.start_failed", "ToolHive workload could not start")
    })?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_workload(
    State(state): State<Arc<AppState>>,
    Path(workload_name): Path<String>,
) -> Result<StatusCode, BridgeError> {
    validate_workload_name(&workload_name)?;
    let _workload_guard = state.workload_lock.lock().await;
    cleanup_workload(&state, &workload_name).await;
    Ok(StatusCode::NO_CONTENT)
}

async fn mcp_request(
    State(state): State<Arc<AppState>>,
    Path(workload_name): Path<String>,
    Json(request): Json<ToolhiveBridgeMcpRequest>,
) -> Result<Json<ToolhiveBridgeMcpResponse>, BridgeError> {
    validate_workload_name(&workload_name)?;
    if request.contract_version != TOOLHIVE_BRIDGE_CONTRACT_VERSION_V1
        || request.proxy_port != LINEAR_PROXY_PORT
        || !matches!(request.method.as_str(), "tools/list" | "tools/call")
    {
        return Err(BridgeError::bad_request("Unsupported ToolHive MCP request"));
    }
    if !workload_is_running(&state, &workload_name).await {
        return Err(BridgeError::unavailable(
            "toolhive.workload_unavailable",
            "ToolHive workload is not running",
        ));
    }
    let operation = async {
        let mut client = McpProxyClient::connect(state.client.clone(), request.proxy_port).await?;
        client.request(&request.method, request.params.0).await
    };
    let result = timeout(MCP_REQUEST_TIMEOUT, operation)
        .await
        .map_err(|_| {
            BridgeError::unavailable("toolhive.mcp_timeout", "ToolHive MCP request timed out")
        })?
        .map_err(|error| {
            tracing::warn!(error = %error, "ToolHive MCP request failed");
            BridgeError::unavailable("toolhive.mcp_failed", "ToolHive MCP request failed")
        })?;
    Ok(Json(ToolhiveBridgeMcpResponse {
        result: JsonValue(result),
    }))
}

fn validate_workload_name(workload_name: &str) -> Result<(), BridgeError> {
    if workload_name != LINEAR_WORKLOAD_NAME {
        return Err(BridgeError::bad_request(
            "Only the pinned Linear workload is supported",
        ));
    }
    Ok(())
}

fn validate_workload_request(
    workload_name: &str,
    request: &ToolhiveBridgeWorkloadRequest,
) -> Result<(), BridgeError> {
    validate_workload_name(workload_name)?;
    if request.contract_version != TOOLHIVE_BRIDGE_CONTRACT_VERSION_V1
        || request.workload_name != workload_name
        || request.upstream_url != LINEAR_MCP_URL
        || request.proxy_port != LINEAR_PROXY_PORT
        || request.callback_port != LINEAR_CALLBACK_PORT
    {
        return Err(BridgeError::bad_request(
            "ToolHive workload request does not match the pinned contract",
        ));
    }
    Ok(())
}

async fn toolhive_version(state: &AppState) -> Result<String, BridgeError> {
    run_toolhive(state, &["version"], TOOLHIVE_COMMAND_TIMEOUT)
        .await
        .map(|value| value.lines().next().unwrap_or_default().trim().to_owned())
        .map_err(|error| {
            tracing::warn!(error = %error, "ToolHive version probe failed");
            BridgeError::unavailable("toolhive.unavailable", "ToolHive is unavailable")
        })
}

async fn workload_is_running(state: &AppState, workload_name: &str) -> bool {
    let listed = run_toolhive(
        state,
        &["list", "--format", "json"],
        TOOLHIVE_COMMAND_TIMEOUT,
    )
    .await
    .ok()
    .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
    .is_some_and(|value| json_contains_workload(&value, workload_name));
    listed
        && timeout(
            TOOL_PROBE_TIMEOUT,
            tokio::net::TcpStream::connect(("127.0.0.1", LINEAR_PROXY_PORT)),
        )
        .await
        .is_ok_and(|connection| connection.is_ok())
}

fn json_contains_workload(value: &serde_json::Value, workload_name: &str) -> bool {
    match value {
        serde_json::Value::Object(object) => object.values().any(|value| {
            value.as_str() == Some(workload_name) || json_contains_workload(value, workload_name)
        }),
        serde_json::Value::Array(values) => values
            .iter()
            .any(|value| json_contains_workload(value, workload_name)),
        _ => false,
    }
}

async fn cleanup_workload(state: &AppState, workload_name: &str) {
    let _ = run_toolhive(state, &["stop", workload_name], TOOLHIVE_COMMAND_TIMEOUT).await;
    let _ = run_toolhive(state, &["rm", workload_name], TOOLHIVE_COMMAND_TIMEOUT).await;
}

async fn capture_authorization_url(
    state: &AppState,
    request: &ToolhiveBridgeWorkloadRequest,
) -> Result<ToolhiveBridgeAuthorizationResponse, BridgeError> {
    let proxy_port = request.proxy_port.to_string();
    let callback_port = request.callback_port.to_string();
    let mut command = Command::new(&state.config.toolhive_binary);
    command
        .args([
            "run",
            request.upstream_url.as_str(),
            "--name",
            request.workload_name.as_str(),
            "--foreground",
            "--remote-auth",
            "--remote-auth-skip-browser",
            "--remote-auth-timeout",
            "5m",
            "--remote-auth-callback-port",
            callback_port.as_str(),
            "--proxy-port",
            proxy_port.as_str(),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = command.spawn().map_err(|error| {
        tracing::warn!(error = %error, "ToolHive authorization process failed to start");
        BridgeError::unavailable(
            "toolhive.authorization_failed",
            "ToolHive authorization could not start",
        )
    })?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take().ok_or_else(|| {
        BridgeError::unavailable(
            "toolhive.authorization_failed",
            "ToolHive authorization output was unavailable",
        )
    })?;
    let stdout_task = tokio::spawn(read_capped_output(stdout));
    let mut stderr_lines = BufReader::new(stderr).lines();
    let captured = timeout(
        AUTHORIZATION_URL_CAPTURE_TIMEOUT,
        capture_authorization_url_from_stderr(&mut stderr_lines),
    )
    .await;
    let authorization_url = match captured {
        Ok(Ok(url)) => url,
        Ok(Err(error)) => {
            let _ = child.kill().await;
            tracing::warn!(error = %error, "ToolHive did not produce an authorization URL");
            return Err(BridgeError::unavailable(
                "toolhive.authorization_failed",
                "ToolHive did not produce an authorization URL",
            ));
        }
        Err(_) => {
            let _ = child.kill().await;
            return Err(BridgeError::unavailable(
                "toolhive.authorization_timeout",
                "ToolHive authorization URL timed out",
            ));
        }
    };
    tokio::spawn(async move {
        while stderr_lines.next_line().await.ok().flatten().is_some() {}
        let _ = stdout_task.await;
        let _ = child.wait().await;
    });
    let expires_at = Utc::now()
        + ChronoDuration::from_std(AUTHORIZATION_TIMEOUT)
            .unwrap_or_else(|_| ChronoDuration::minutes(5));
    Ok(ToolhiveBridgeAuthorizationResponse {
        authorization_url,
        expires_at,
    })
}

async fn capture_authorization_url_from_stderr(
    lines: &mut Lines<BufReader<ChildStderr>>,
) -> anyhow::Result<String> {
    let mut observed_bytes = 0_usize;
    while let Some(line) = lines.next_line().await? {
        observed_bytes = observed_bytes.saturating_add(line.len());
        anyhow::ensure!(
            observed_bytes <= MAX_PROCESS_OUTPUT_BYTES,
            "ToolHive authorization output exceeded its limit"
        );
        if let Some(url) = linear_authorization_url_from_line(&line) {
            return Ok(url);
        }
    }
    anyhow::bail!("ToolHive authorization exited before producing a URL")
}

fn linear_authorization_url_from_line(line: &str) -> Option<String> {
    let (_, raw_url) = line.split_once("Please open this URL in your browser: ")?;
    let url = reqwest::Url::parse(raw_url.trim()).ok()?;
    let host = url.host_str()?;
    if url.scheme() != "https"
        || (host != "linear.app" && !host.ends_with(".linear.app"))
        || url.query_pairs().all(|(key, _)| key != "state")
    {
        return None;
    }
    Some(url.into())
}

async fn run_toolhive(
    state: &AppState,
    args: &[&str],
    deadline: Duration,
) -> anyhow::Result<String> {
    let mut child = Command::new(&state.config.toolhive_binary)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .context("failed to launch ToolHive")?;
    let stdout_task = tokio::spawn(read_capped_output(child.stdout.take()));
    let stderr_task = tokio::spawn(read_capped_output(child.stderr.take()));
    let status = match timeout(deadline, child.wait()).await {
        Ok(status) => status.context("failed to wait for ToolHive")?,
        Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            anyhow::bail!("ToolHive command timed out")
        }
    };
    let stdout = stdout_task.await.context("ToolHive stdout task failed")??;
    let stderr = stderr_task.await.context("ToolHive stderr task failed")??;
    anyhow::ensure!(status.success(), "ToolHive command failed");
    anyhow::ensure!(
        stdout.len().saturating_add(stderr.len()) <= MAX_PROCESS_OUTPUT_BYTES,
        "ToolHive command output exceeded its limit"
    );
    Ok(String::from_utf8_lossy(&stdout).trim().to_owned())
}

async fn read_capped_output(reader: Option<impl AsyncRead + Unpin>) -> anyhow::Result<Vec<u8>> {
    let Some(reader) = reader else {
        return Ok(Vec::new());
    };
    let mut bytes = Vec::new();
    reader
        .take((MAX_PROCESS_OUTPUT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .await?;
    anyhow::ensure!(
        bytes.len() <= MAX_PROCESS_OUTPUT_BYTES,
        "ToolHive process output exceeded its limit"
    );
    Ok(bytes)
}

struct McpProxyClient {
    client: reqwest::Client,
    endpoint: reqwest::Url,
    session_id: Option<String>,
    next_id: u64,
}

impl McpProxyClient {
    async fn connect(client: reqwest::Client, proxy_port: u16) -> anyhow::Result<Self> {
        let endpoint = format!("http://127.0.0.1:{proxy_port}/mcp").parse()?;
        let mut client = Self {
            client,
            endpoint,
            session_id: None,
            next_id: 1,
        };
        client
            .request(
                "initialize",
                serde_json::json!({
                    "protocolVersion": TOOLING_MCP_REVISION,
                    "capabilities": {},
                    "clientInfo": {"name": "uprava-toolhive", "version": env!("CARGO_PKG_VERSION")},
                }),
            )
            .await?;
        client
            .notify("notifications/initialized", serde_json::json!({}))
            .await?;
        Ok(client)
    }

    async fn request(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let id = self.next_id;
        self.next_id += 1;
        let body =
            serde_json::json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        let mut request = self.client.post(self.endpoint.clone()).json(&body);
        if let Some(session_id) = &self.session_id {
            request = request.header("mcp-session-id", session_id);
        }
        let response = request.send().await?.error_for_status()?;
        if let Some(session_id) = response
            .headers()
            .get("mcp-session-id")
            .and_then(|value| value.to_str().ok())
        {
            self.session_id = Some(session_id.to_owned());
        }
        if response
            .content_length()
            .is_some_and(|size| size > TOOL_RESULT_MAX_BYTES)
        {
            anyhow::bail!("MCP response exceeded the transport limit");
        }
        let bytes = response.bytes().await?;
        anyhow::ensure!(
            bytes.len() as u64 <= TOOL_RESULT_MAX_BYTES,
            "MCP response exceeded the transport limit"
        );
        let envelope: serde_json::Value = serde_json::from_slice(&bytes)?;
        anyhow::ensure!(
            envelope.get("error").is_none(),
            "MCP upstream returned an error"
        );
        envelope
            .get("result")
            .cloned()
            .context("MCP response did not contain a result")
    }

    async fn notify(&self, method: &str, params: serde_json::Value) -> anyhow::Result<()> {
        let body = serde_json::json!({"jsonrpc": "2.0", "method": method, "params": params});
        let mut request = self.client.post(self.endpoint.clone()).json(&body);
        if let Some(session_id) = &self.session_id {
            request = request.header("mcp-session-id", session_id);
        }
        request.send().await?.error_for_status()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorization_url_accepts_pinned_https_host_and_state() {
        let line = "Please open this URL in your browser: https://linear.app/oauth/authorize?client_id=uprava&state=opaque";
        assert_eq!(
            linear_authorization_url_from_line(line).as_deref(),
            Some("https://linear.app/oauth/authorize?client_id=uprava&state=opaque")
        );
    }

    #[test]
    fn authorization_url_rejects_lookalike_hosts() {
        assert!(linear_authorization_url_from_line(
            "Please open this URL in your browser: https://linear.app.attacker.test/oauth?state=x"
        )
        .is_none());
    }

    #[test]
    fn workload_request_is_fixed_to_the_compose_boundary() {
        let request = ToolhiveBridgeWorkloadRequest {
            contract_version: TOOLHIVE_BRIDGE_CONTRACT_VERSION_V1,
            upstream_url: LINEAR_MCP_URL.to_owned(),
            workload_name: LINEAR_WORKLOAD_NAME.to_owned(),
            proxy_port: LINEAR_PROXY_PORT,
            callback_port: LINEAR_CALLBACK_PORT,
        };

        assert!(validate_workload_request(LINEAR_WORKLOAD_NAME, &request).is_ok());
    }
}
