//! Generated UI proposal, runtime, state and action contracts.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeneratedUiLayoutIntent {
    Inline,
    Panel,
    Canvas,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeneratedUiCapability {
    PersistState,
    SendAgentInput,
    OpenReference,
    RequestLayoutChange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeneratedUiActionKind {
    UpdateArtifactState,
    SendAgentInput,
    OpenReference,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedUiActionDefinition {
    pub action_id: String,
    pub kind: GeneratedUiActionKind,
    pub label: String,
    pub input_schema: JsonValue,
    #[serde(default)]
    pub required_capabilities: Vec<GeneratedUiCapability>,
    #[serde(default)]
    pub confirmation_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateDynamicUiProposalRequest {
    pub title: String,
    pub description: Option<String>,
    pub scope_ref: ScopeRef,
    pub runtime_id: String,
    pub sdk_version: String,
    pub layout_intent: GeneratedUiLayoutIntent,
    pub source: String,
    pub data_model: JsonValue,
    #[serde(default)]
    pub actions: Vec<GeneratedUiActionDefinition>,
    #[serde(default)]
    pub requested_capabilities: Vec<GeneratedUiCapability>,
    pub fallback_markdown: String,
    pub fallback_snapshot: Option<String>,
    #[serde(default)]
    pub source_refs: Vec<UpravaRef>,
    #[serde(default)]
    pub evidence_refs: Vec<UpravaRef>,
    #[serde(default)]
    pub cause_refs: Vec<UpravaRef>,
    #[serde(default)]
    pub trace_refs: Vec<UpravaRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedUiArtifactPayload {
    pub description: Option<String>,
    pub runtime_id: String,
    pub sdk_version: String,
    pub layout_intent: GeneratedUiLayoutIntent,
    pub source_blob_hash: String,
    pub data_model: JsonValue,
    #[serde(default)]
    pub actions: Vec<GeneratedUiActionDefinition>,
    #[serde(default)]
    pub granted_capabilities: Vec<GeneratedUiCapability>,
    pub fallback_snapshot: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeneratedUiBuildState {
    Pending,
    Ready,
    Failed,
    FallbackOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeneratedUiDiagnosticSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedUiBuildDiagnostic {
    pub severity: GeneratedUiDiagnosticSeverity,
    pub message: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedUiBuild {
    pub build_id: String,
    pub artifact_id: ArtifactId,
    pub artifact_version: u64,
    pub state: GeneratedUiBuildState,
    pub runtime_id: String,
    pub runtime_version: String,
    pub sdk_version: String,
    pub source_blob_hash: String,
    pub bundle_blob_hash: Option<String>,
    pub dependency_lock: JsonValue,
    #[serde(default)]
    pub diagnostics: Vec<GeneratedUiBuildDiagnostic>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedUiState {
    pub artifact_id: ArtifactId,
    pub revision: u64,
    pub values: JsonValue,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedUiRuntimeDetail {
    pub artifact: ArtifactDetail,
    pub build: GeneratedUiBuild,
    pub state: GeneratedUiState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateGeneratedUiStateRequest {
    pub expected_revision: u64,
    pub values: JsonValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvokeGeneratedUiActionRequest {
    pub artifact_version: u64,
    pub idempotency_key: String,
    pub input: JsonValue,
    #[serde(default)]
    pub confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedUiActionResult {
    pub action_request_id: String,
    pub artifact_id: ArtifactId,
    pub action_id: String,
    pub kind: GeneratedUiActionKind,
    pub result: JsonValue,
    pub state: Option<GeneratedUiState>,
    pub command_id: Option<CommandId>,
    pub completed_at: DateTime<Utc>,
}
