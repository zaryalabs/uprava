use std::path::PathBuf;

use axum::http::{header::InvalidHeaderValue, HeaderValue};
use uprava_protocol::DeploymentProfile;

use super::persistence::DEFAULT_CORE_DATABASE_URL;

/// Validated Core process configuration.
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub bind_address: String,
    pub database_url: String,
    pub profile: DeploymentProfile,
    pub allowed_origins: Vec<HeaderValue>,
    pub stale_after_seconds: i64,
    pub offline_after_seconds: i64,
    pub enrollment_ttl_seconds: i64,
    pub max_pending_enrollments: i64,
    pub runtime_expiry_seconds: i64,
    pub auto_approve_enrollments: bool,
    pub auto_approve_node_name: Option<String>,
    pub client_log_file: PathBuf,
    pub web_auth_required: bool,
    pub web_session_ttl_seconds: i64,
    pub cookie_secure: bool,
    pub core_shutdown_timeout_seconds: i64,
}

impl AppConfig {
    /// Load and validate Core configuration from the process environment.
    pub fn from_env() -> Result<Self, ConfigError> {
        let profile = parse_profile(std::env::var("UPRAVA_DEPLOYMENT_PROFILE").ok())?;
        Ok(Self {
            bind_address: std::env::var("UPRAVA_CORE_BIND")
                .unwrap_or_else(|_| "127.0.0.1:8080".to_owned()),
            database_url: std::env::var("UPRAVA_DATABASE_URL")
                .unwrap_or_else(|_| DEFAULT_CORE_DATABASE_URL.to_owned()),
            profile,
            allowed_origins: parse_allowed_origins(std::env::var("UPRAVA_ALLOWED_ORIGINS").ok())?,
            stale_after_seconds: parse_env_i64("UPRAVA_HEARTBEAT_STALE_SECONDS", 15)?,
            offline_after_seconds: parse_env_i64("UPRAVA_HEARTBEAT_OFFLINE_SECONDS", 45)?,
            enrollment_ttl_seconds: parse_env_i64("UPRAVA_ENROLLMENT_TTL_SECONDS", 600)?,
            max_pending_enrollments: parse_env_i64("UPRAVA_MAX_PENDING_ENROLLMENTS", 100)?,
            runtime_expiry_seconds: parse_env_i64("UPRAVA_RUNTIME_EXPIRY_SECONDS", 86_400)?,
            auto_approve_enrollments: parse_auto_approve_enrollments()?,
            auto_approve_node_name: parse_optional_non_empty("UPRAVA_AUTO_APPROVE_NODE_NAME"),
            client_log_file: std::env::var("UPRAVA_CLIENT_LOG_FILE")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from(".local/logs/client.log")),
            web_auth_required: parse_web_auth_required(
                std::env::var("UPRAVA_WEB_AUTH").ok(),
                profile,
            )?,
            web_session_ttl_seconds: parse_env_i64("UPRAVA_WEB_SESSION_TTL_SECONDS", 86_400)?,
            cookie_secure: parse_env_bool("UPRAVA_COOKIE_SECURE", false),
            core_shutdown_timeout_seconds: parse_env_i64(
                "UPRAVA_CORE_SHUTDOWN_TIMEOUT_SECONDS",
                5,
            )?,
        })
    }
}

/// Configuration validation failure reported before the server binds a port.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("invalid deployment profile `{0}`")]
    InvalidProfile(String),
    #[error("invalid CORS origin `{origin}`")]
    InvalidOrigin {
        origin: String,
        source: InvalidHeaderValue,
    },
    #[error("wildcard CORS origin is not allowed in trusted development profile")]
    WildcardOrigin,
    #[error("invalid web auth mode `{0}`")]
    InvalidWebAuthMode(String),
    #[error("node enrollment auto-approval is not supported")]
    AutoApproveEnrollments,
    #[error("invalid integer environment variable `{name}`")]
    InvalidInteger {
        name: String,
        source: std::num::ParseIntError,
    },
}

fn parse_profile(value: Option<String>) -> Result<DeploymentProfile, ConfigError> {
    match value.as_deref() {
        Some("controlled_dev") | None => Ok(DeploymentProfile::ControlledDev),
        Some(other) => Err(ConfigError::InvalidProfile(other.to_owned())),
    }
}

fn parse_allowed_origins(value: Option<String>) -> Result<Vec<HeaderValue>, ConfigError> {
    let Some(value) = value else {
        return Ok(default_allowed_origins());
    };
    let origins = value
        .split(',')
        .map(str::trim)
        .filter(|origin| !origin.is_empty())
        .map(|origin| {
            if origin == "*" {
                return Err(ConfigError::WildcardOrigin);
            }
            HeaderValue::from_str(origin).map_err(|source| ConfigError::InvalidOrigin {
                origin: origin.to_owned(),
                source,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(if origins.is_empty() {
        default_allowed_origins()
    } else {
        origins
    })
}

fn parse_web_auth_required(
    value: Option<String>,
    _profile: DeploymentProfile,
) -> Result<bool, ConfigError> {
    match value.as_deref() {
        Some("local") | Some("required") | Some("1") | Some("true") => Ok(true),
        Some("auto") | None => Ok(true),
        Some(other) => Err(ConfigError::InvalidWebAuthMode(other.to_owned())),
    }
}

fn parse_auto_approve_enrollments() -> Result<bool, ConfigError> {
    if parse_env_bool("UPRAVA_AUTO_APPROVE_ENROLLMENTS", false) {
        return Err(ConfigError::AutoApproveEnrollments);
    }
    Ok(false)
}

fn parse_optional_non_empty(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

pub(crate) fn default_allowed_origins() -> Vec<HeaderValue> {
    vec![
        HeaderValue::from_static("http://127.0.0.1:5173"),
        HeaderValue::from_static("http://localhost:5173"),
    ]
}

fn parse_env_i64(name: &str, fallback: i64) -> Result<i64, ConfigError> {
    match std::env::var(name) {
        Ok(value) => value
            .parse::<i64>()
            .map_err(|source| ConfigError::InvalidInteger {
                name: name.to_owned(),
                source,
            }),
        Err(_) => Ok(fallback),
    }
}

fn parse_env_bool(name: &str, fallback: bool) -> bool {
    std::env::var(name)
        .ok()
        .map(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(fallback)
}
