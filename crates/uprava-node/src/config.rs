use std::{path::PathBuf, time::Duration};

use anyhow::Context;
use reqwest::Url;

use super::default_state_path;

/// Validated Node daemon process configuration.
#[derive(Debug, Clone)]
pub(crate) struct NodeConfig {
    pub(crate) core_url: Url,
    pub(crate) display_name: String,
    pub(crate) heartbeat_interval: Duration,
    pub(crate) state_path: PathBuf,
    pub(crate) workspace_paths: Vec<PathBuf>,
    pub(crate) codex_binary: String,
    pub(crate) codex_ignore_user_config: bool,
    pub(crate) codex_timeout: Duration,
    pub(crate) toolhive_binary: String,
    pub(crate) toolhive_start_timeout: Duration,
}

impl NodeConfig {
    /// Load configuration and reject an empty workspace allow-list.
    pub(crate) fn from_env() -> anyhow::Result<Self> {
        let core_url = std::env::var("UPRAVA_CORE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8080".to_owned())
            .parse::<Url>()
            .context("UPRAVA_CORE_URL must be a valid URL")?;
        let display_name =
            std::env::var("UPRAVA_NODE_DISPLAY_NAME").unwrap_or_else(|_| "Local Node".to_owned());
        let heartbeat_interval = parse_env_duration_seconds("UPRAVA_NODE_HEARTBEAT_SECONDS", 5)?;
        let state_path = std::env::var("UPRAVA_NODE_STATE_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| default_state_path());
        let workspace_paths = parse_workspace_paths()?;
        let codex_binary =
            std::env::var("UPRAVA_CODEX_BINARY").unwrap_or_else(|_| "codex".to_owned());
        let codex_ignore_user_config = parse_env_bool("UPRAVA_CODEX_IGNORE_USER_CONFIG", true)?;
        let codex_timeout =
            parse_env_duration_seconds("UPRAVA_CODEX_TIMEOUT_SECONDS", 24 * 60 * 60)?;
        let toolhive_binary =
            std::env::var("UPRAVA_TOOLHIVE_BINARY").unwrap_or_else(|_| "thv".to_owned());
        let toolhive_start_timeout =
            parse_env_duration_seconds("UPRAVA_TOOLHIVE_START_TIMEOUT_SECONDS", 5 * 60)?;

        Ok(Self {
            core_url,
            display_name,
            heartbeat_interval,
            state_path,
            workspace_paths,
            codex_binary,
            codex_ignore_user_config,
            codex_timeout,
            toolhive_binary,
            toolhive_start_timeout,
        })
    }
}

fn parse_env_bool(name: &str, fallback: bool) -> anyhow::Result<bool> {
    match std::env::var(name) {
        Ok(value) if value.eq_ignore_ascii_case("true") => Ok(true),
        Ok(value) if value.eq_ignore_ascii_case("false") => Ok(false),
        Ok(_) => anyhow::bail!("{name} must be true or false"),
        Err(_) => Ok(fallback),
    }
}

fn parse_env_duration_seconds(name: &str, fallback_seconds: u64) -> anyhow::Result<Duration> {
    match std::env::var(name) {
        Ok(value) => value
            .parse::<u64>()
            .map(Duration::from_secs)
            .with_context(|| format!("{name} must be an unsigned integer number of seconds")),
        Err(_) => Ok(Duration::from_secs(fallback_seconds)),
    }
}

fn parse_workspace_paths() -> anyhow::Result<Vec<PathBuf>> {
    let value = std::env::var("UPRAVA_NODE_WORKSPACES")
        .context("UPRAVA_NODE_WORKSPACES must list one or more allowed workspace roots")?;
    let paths = value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    if paths.is_empty() {
        anyhow::bail!("UPRAVA_NODE_WORKSPACES must list one or more allowed workspace roots");
    }
    Ok(paths)
}
