//! Core enrollment and heartbeat HTTP transport.

use super::super::*;

impl NodeStateStore {
    pub(crate) async fn ensure_enrollment(
        &self,
        client: &reqwest::Client,
        config: &NodeConfig,
    ) -> anyhow::Result<bool> {
        ensure_enrollment(client, config, self).await
    }

    pub(crate) async fn send_heartbeat(
        &self,
        client: &reqwest::Client,
        config: &NodeConfig,
    ) -> anyhow::Result<NodeHeartbeatResponse> {
        let state = self.snapshot().await?;
        send_heartbeat(client, config, &state).await
    }
}

pub(crate) fn http_error_status(error: &anyhow::Error) -> Option<reqwest::StatusCode> {
    error.chain().find_map(|cause| {
        cause
            .downcast_ref::<reqwest::Error>()
            .and_then(reqwest::Error::status)
    })
}

pub(crate) fn heartbeat_auth_rejected(error: &anyhow::Error) -> bool {
    http_error_status(error) == Some(reqwest::StatusCode::UNAUTHORIZED)
}

pub(crate) fn enrollment_claim_status_invalidates_attempt(
    status: Option<reqwest::StatusCode>,
) -> bool {
    matches!(
        status,
        Some(reqwest::StatusCode::NOT_FOUND | reqwest::StatusCode::UNAUTHORIZED)
    )
}

pub(crate) fn enrollment_claim_invalidates_attempt(error: &anyhow::Error) -> bool {
    enrollment_claim_status_invalidates_attempt(http_error_status(error))
}

pub(crate) async fn ensure_enrollment(
    client: &reqwest::Client,
    config: &NodeConfig,
    store: &NodeStateStore,
) -> anyhow::Result<bool> {
    let mut local_state = store.snapshot().await?;
    if local_state.enrollment_id.is_none() || local_state.pairing_code.is_none() {
        let response = request_enrollment(client, config).await?;
        tracing::info!(
            expires_at = %response.expires_at,
            "enrollment requested; approve this enrollment in Core"
        );
        store
            .persist_enrollment_attempt(
                response.enrollment_id.clone(),
                response.pairing_code.clone(),
            )
            .await?;
        local_state.enrollment_id = Some(response.enrollment_id);
        local_state.pairing_code = Some(response.pairing_code);
    }

    let claim = match claim_enrollment(client, config, &local_state).await {
        Ok(claim) => claim,
        Err(error) if enrollment_claim_invalidates_attempt(&error) => {
            tracing::warn!(
                error = %error,
                "enrollment claim was rejected; clearing stale local enrollment"
            );
            store.clear_enrollment_attempt().await?;
            return Ok(false);
        }
        Err(error) => return Err(error),
    };
    if claim.pending {
        tracing::info!("waiting for enrollment approval");
        return Ok(false);
    }
    if !claim.accepted {
        tracing::warn!(message = %claim.message, "enrollment was not accepted");
        store.clear_enrollment_attempt().await?;
        return Ok(false);
    }
    if let (Some(node_id), Some(credential)) = (claim.node_id, claim.credential) {
        store
            .persist_enrollment_identity(node_id, credential)
            .await?;
        tracing::info!("enrollment claimed and credential stored");
        return Ok(true);
    }
    store.is_enrolled().await
}

pub(crate) async fn request_enrollment(
    client: &reqwest::Client,
    config: &NodeConfig,
) -> anyhow::Result<NodeEnrollmentRequestedResponse> {
    let endpoint = config
        .core_url
        .join("/api/v1/node/enrollment-requests")
        .context("enrollment request URL should be valid")?;
    let request = NodeEnrollmentRequest {
        display_name: config.display_name.clone(),
        daemon_version: daemon_version(),
        capabilities: capabilities(config),
    };
    client
        .post(endpoint)
        .json(&request)
        .send()
        .await
        .context("enrollment request failed")?
        .error_for_status()
        .context("enrollment request returned an error status")?
        .json::<NodeEnrollmentRequestedResponse>()
        .await
        .context("enrollment response was not valid JSON")
}

pub(crate) async fn claim_enrollment(
    client: &reqwest::Client,
    config: &NodeConfig,
    local_state: &NodeLocalState,
) -> anyhow::Result<NodeEnrollmentClaimResponse> {
    let endpoint = config
        .core_url
        .join("/api/v1/node/enrollment-claims")
        .context("enrollment claim URL should be valid")?;
    let enrollment_id = local_state
        .enrollment_id
        .clone()
        .context("local enrollment id missing")?;
    let pairing_code = local_state
        .pairing_code
        .clone()
        .context("local pairing code missing")?;
    client
        .post(endpoint)
        .json(&NodeEnrollmentClaimRequest {
            enrollment_id,
            pairing_code,
        })
        .send()
        .await
        .context("enrollment claim failed")?
        .error_for_status()
        .context("enrollment claim returned an error status")?
        .json::<NodeEnrollmentClaimResponse>()
        .await
        .context("enrollment claim response was not valid JSON")
}

pub(crate) async fn send_heartbeat(
    client: &reqwest::Client,
    config: &NodeConfig,
    local_state: &NodeLocalState,
) -> anyhow::Result<NodeHeartbeatResponse> {
    let endpoint = config
        .core_url
        .join("/api/v1/node/heartbeat")
        .context("heartbeat URL should be valid")?;
    let node_id = local_state
        .node_id
        .clone()
        .context("local node id missing")?;
    let request = NodeHeartbeatRequest {
        node_id: Some(node_id.clone()),
        display_name: config.display_name.clone(),
        daemon_version: daemon_version(),
        capabilities: capabilities(config),
        observed_capabilities: observed_capabilities(config, &node_id).await,
        diagnostics: Some(node_diagnostics(local_state)),
        active_runtime_count: active_runtime_count(local_state),
        sleep_hint: SleepHint::Awake,
        workspace_summaries: config
            .workspace_paths
            .iter()
            .map(|path| validate_workspace(path))
            .collect(),
    };

    client
        .post(endpoint)
        .bearer_auth(
            local_state
                .credential
                .as_deref()
                .context("local node credential missing")?,
        )
        .json(&request)
        .send()
        .await
        .context("heartbeat request failed")?
        .error_for_status()
        .context("heartbeat returned an error status")?
        .json::<NodeHeartbeatResponse>()
        .await
        .context("heartbeat response was not valid JSON")
}

pub(crate) async fn request_provider_mcp_access(
    client: &reqwest::Client,
    config: &NodeConfig,
    local_state: &NodeLocalState,
    command_id: &CommandId,
) -> anyhow::Result<ProviderMcpAccess> {
    let endpoint = config
        .core_url
        .join("/api/v1/node/provider-mcp-access")
        .context("provider MCP access URL should be valid")?;
    let node_id = local_state
        .node_id
        .as_ref()
        .context("local node id missing")?;
    let credential = local_state
        .credential
        .as_deref()
        .context("local node credential missing")?;
    let mut access = client
        .post(endpoint)
        .header("x-uprava-node-id", node_id.as_str())
        .bearer_auth(credential)
        .json(&ProviderMcpAccessRequest {
            command_id: command_id.clone(),
        })
        .send()
        .await
        .context("provider MCP access request failed")?
        .error_for_status()
        .context("provider MCP access request returned an error status")?
        .json::<ProviderMcpAccess>()
        .await
        .context("provider MCP access response was not valid JSON")?;
    let mcp_endpoint = config
        .core_url
        .join(&access.endpoint_url)
        .context("provider MCP endpoint URL should be valid")?;
    if mcp_endpoint.origin() != config.core_url.origin() {
        anyhow::bail!("provider MCP endpoint must use the configured Core origin");
    }
    if access.expires_at <= Utc::now() {
        anyhow::bail!("provider MCP access expired before delivery");
    }
    access.endpoint_url = mcp_endpoint.to_string();
    Ok(access)
}

pub(crate) fn node_diagnostics(local_state: &NodeLocalState) -> String {
    format!(
        "outbox_events={}; cached_commands={}; reconnect_attempts={}; dropped_events={}; heartbeat_failures={}; dropped_log_records={}; otlp_export_failures={}",
        local_state.event_outbox.len(),
        local_state.command_status.len(),
        local_state.reconnect_attempts,
        local_state.dropped_event_count,
        local_state.heartbeat_failures,
        uprava_logging::dropped_log_records(),
        uprava_logging::otlp_export_failures(),
    )
}
