//! Generic visual object and durable artifact contracts.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualRenderState {
    Ready,
    Loading,
    Error,
    Unavailable,
    Stale,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualObjectDescriptor {
    pub visual_object_id: Option<String>,
    pub visual_kind: String,
    pub parent_ref: UpravaRef,
    pub source_ref: UpravaRef,
    pub render_scope: VisualRenderScope,
    pub renderer_id: String,
    pub title: Option<String>,
    pub state: VisualRenderState,
    #[serde(default)]
    pub actions: Vec<String>,
    #[serde(default)]
    pub required_permissions: Vec<String>,
    pub fallback_text: String,
    #[serde(default)]
    pub cause_refs: Vec<UpravaRef>,
    pub artifact_ref: Option<UpravaRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactState {
    Active,
    Stale,
    Archived,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactSummary {
    pub artifact_id: ArtifactId,
    pub artifact_type: String,
    pub title: String,
    pub scope_ref: ScopeRef,
    pub owner_plugin_id: PluginId,
    pub current_version: u64,
    pub state: ArtifactState,
    pub created_by: ActorRef,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactVersion {
    pub artifact_id: ArtifactId,
    pub version: u64,
    pub schema_version: u16,
    pub payload: JsonValue,
    pub fallback_text: String,
    pub source_version: Option<String>,
    #[serde(default)]
    pub source_refs: Vec<UpravaRef>,
    #[serde(default)]
    pub evidence_refs: Vec<UpravaRef>,
    #[serde(default)]
    pub cause_refs: Vec<UpravaRef>,
    #[serde(default)]
    pub trace_refs: Vec<UpravaRef>,
    pub provenance: JsonValue,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactDetail {
    pub artifact: ArtifactSummary,
    pub version: ArtifactVersion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactListResponse {
    pub items: Vec<ArtifactSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateArtifactRequest {
    pub artifact_type: String,
    pub title: String,
    pub scope_ref: ScopeRef,
    pub schema_version: u16,
    pub payload: JsonValue,
    pub fallback_text: String,
    pub source_version: Option<String>,
    #[serde(default)]
    pub source_refs: Vec<UpravaRef>,
    #[serde(default)]
    pub evidence_refs: Vec<UpravaRef>,
    #[serde(default)]
    pub cause_refs: Vec<UpravaRef>,
    #[serde(default)]
    pub trace_refs: Vec<UpravaRef>,
    pub provenance: JsonValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateArtifactVersionRequest {
    pub expected_current_version: u64,
    pub schema_version: u16,
    pub payload: JsonValue,
    pub fallback_text: String,
    pub source_version: Option<String>,
    #[serde(default)]
    pub source_refs: Vec<UpravaRef>,
    #[serde(default)]
    pub evidence_refs: Vec<UpravaRef>,
    #[serde(default)]
    pub cause_refs: Vec<UpravaRef>,
    #[serde(default)]
    pub trace_refs: Vec<UpravaRef>,
    pub provenance: JsonValue,
}
