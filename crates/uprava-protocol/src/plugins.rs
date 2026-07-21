//! Shared Plugin Registry and declarative contribution contracts.
//!
//! Plugin packages extend Uprava through versioned data contracts. Core owns
//! installation, compatibility, permissions and effective state; clients only
//! mount contributions included in an effective snapshot.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use super::*;

/// First supported plugin manifest major version.
pub const PLUGIN_MANIFEST_VERSION_V1: u16 = 1;
/// First supported `ui.theme` contribution major version.
pub const THEME_CONTRIBUTION_VERSION_V1: u16 = 1;
/// First supported `visual.renderer` contribution major version.
pub const VISUAL_RENDERER_CONTRIBUTION_VERSION_V1: u16 = 1;
/// Visual renderer version that adds inline, block and artifact scopes.
pub const VISUAL_RENDERER_CONTRIBUTION_VERSION_V2: u16 = 2;
/// First supported `artifact.type` contribution major version.
pub const ARTIFACT_TYPE_CONTRIBUTION_VERSION_V1: u16 = 1;
/// First supported `generated_ui.runtime` contribution major version.
pub const GENERATED_UI_RUNTIME_CONTRIBUTION_VERSION_V1: u16 = 1;
/// First supported `generated_ui.sdk` contribution major version.
pub const GENERATED_UI_SDK_CONTRIBUTION_VERSION_V1: u16 = 1;
/// First supported `generated_ui.action_bridge` contribution major version.
pub const GENERATED_UI_ACTION_BRIDGE_CONTRIBUTION_VERSION_V1: u16 = 1;
/// Permission required by a package that contributes a theme.
pub const THEME_CONTRIBUTION_PERMISSION: &str = "ui.theme.contribute";
/// Permission required by a package that contributes a visual renderer.
pub const VISUAL_RENDERER_CONTRIBUTION_PERMISSION: &str = "visual.renderer.contribute";
/// Permission required by a package that contributes an artifact type.
pub const ARTIFACT_TYPE_CONTRIBUTION_PERMISSION: &str = "artifact.type.contribute";
/// Permission required by a package that contributes a generated UI runtime.
pub const GENERATED_UI_RUNTIME_CONTRIBUTION_PERMISSION: &str = "generated_ui.runtime.contribute";
/// Permission required by a package that contributes a generated UI SDK.
pub const GENERATED_UI_SDK_CONTRIBUTION_PERMISSION: &str = "generated_ui.sdk.contribute";
/// Permission required by a package that contributes an action bridge.
pub const GENERATED_UI_ACTION_BRIDGE_CONTRIBUTION_PERMISSION: &str =
    "generated_ui.action_bridge.contribute";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginTrustLevel {
    DataOnly,
    TrustedBundled,
    SandboxedWeb,
    SandboxedNode,
    ExternalService,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginInstallSource {
    Bundled,
    Local,
    TeamCatalog,
    CommunityCatalog,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginDesiredState {
    Disabled,
    Enabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginEffectiveState {
    Disabled,
    Active,
    Incompatible,
    Degraded,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginCompatibilityState {
    Compatible,
    Incompatible,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginCompatibility {
    pub state: PluginCompatibilityState,
    #[serde(default)]
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginVersionRange {
    pub minimum: Option<String>,
    pub maximum_exclusive: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginCompatibilityRequirements {
    pub core: PluginVersionRange,
    pub web: PluginVersionRange,
    pub node: Option<PluginVersionRange>,
    #[serde(default)]
    pub protocol_versions: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeKind {
    Light,
    Dark,
    HighContrast,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeColorScheme {
    Light,
    Dark,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonacoThemeV1 {
    pub base: String,
    #[serde(default)]
    pub colors: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalThemeV1 {
    #[serde(default)]
    pub colors: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeContributionV1 {
    pub theme_id: String,
    pub label: String,
    pub kind: ThemeKind,
    pub color_scheme: ThemeColorScheme,
    #[serde(default)]
    pub semantic_tokens: BTreeMap<String, String>,
    pub monaco: MonacoThemeV1,
    pub terminal: TerminalThemeV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualRendererKind {
    Content,
    InlineFragment,
    Block,
    ArtifactViewer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualRenderScope {
    ContentEnhancement,
    InlineFragment,
    Block,
    ArtifactViewer,
    DetailView,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualRendererFallback {
    PlainText,
    Source,
    Metadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VisualSourceMatcherV1 {
    FencedLanguage {
        #[serde(default)]
        language_ids: Vec<String>,
    },
    StrictColorLiteral {
        #[serde(default)]
        formats: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualRendererContributionV1 {
    pub renderer_id: String,
    pub implementation_id: String,
    pub renderer_kind: VisualRendererKind,
    #[serde(default)]
    pub accepted_source_kinds: Vec<String>,
    #[serde(default)]
    pub render_scopes: Vec<VisualRenderScope>,
    #[serde(default)]
    pub allowed_surfaces: Vec<String>,
    pub fallback_strategy: VisualRendererFallback,
    #[serde(default)]
    pub source_matcher: Option<VisualSourceMatcherV1>,
    #[serde(default)]
    pub visual_kinds: Vec<String>,
    #[serde(default)]
    pub actions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactTypeContributionV1 {
    pub artifact_type_id: String,
    pub display_name: String,
    pub description: String,
    pub schema_version: u16,
    pub fallback_strategy: VisualRendererFallback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedUiRuntimeContributionV1 {
    pub runtime_id: String,
    pub implementation_id: String,
    pub runtime_version: String,
    pub sdk_id: String,
    pub action_bridge_id: String,
    #[serde(default)]
    pub supported_sdk_versions: Vec<String>,
    #[serde(default)]
    pub supported_layouts: Vec<GeneratedUiLayoutIntent>,
    #[serde(default)]
    pub sandbox_capabilities: Vec<GeneratedUiCapability>,
    #[serde(default)]
    pub allowed_imports: Vec<String>,
    pub max_source_bytes: u64,
    pub max_bundle_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedUiSdkContributionV1 {
    pub sdk_id: String,
    pub package_name: String,
    pub api_version: String,
    pub design_token_version: String,
    pub api_schema: JsonValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedUiActionBridgeContributionV1 {
    pub bridge_id: String,
    #[serde(default)]
    pub supported_actions: Vec<GeneratedUiActionKind>,
}

/// A manifest-declared extension point.
///
/// Contributions remain declarative; executable implementations are activated
/// only by a bounded host that recognizes the declared contract version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PluginContribution {
    UiTheme {
        contribution_id: String,
        contract_version: u16,
        contribution: ThemeContributionV1,
    },
    VisualRenderer {
        contribution_id: String,
        contract_version: u16,
        contribution: VisualRendererContributionV1,
    },
    AgentTool {
        contribution_id: String,
        contract_version: u16,
        tool_id: ToolId,
    },
    ArtifactType {
        contribution_id: String,
        contract_version: u16,
        contribution: ArtifactTypeContributionV1,
    },
    GeneratedUiRuntime {
        contribution_id: String,
        contract_version: u16,
        contribution: GeneratedUiRuntimeContributionV1,
    },
    GeneratedUiSdk {
        contribution_id: String,
        contract_version: u16,
        contribution: GeneratedUiSdkContributionV1,
    },
    GeneratedUiActionBridge {
        contribution_id: String,
        contract_version: u16,
        contribution: GeneratedUiActionBridgeContributionV1,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginManifest {
    pub manifest_version: u16,
    pub plugin_id: PluginId,
    pub version: String,
    pub display_name: String,
    pub description: String,
    pub publisher: String,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub install_source: PluginInstallSource,
    pub trust_level: PluginTrustLevel,
    pub compatibility: PluginCompatibilityRequirements,
    #[serde(default)]
    pub activation_conditions: Vec<String>,
    #[serde(default)]
    pub requested_permissions: Vec<String>,
    pub configuration_schema: Option<JsonValue>,
    #[serde(default)]
    pub contributions: Vec<PluginContribution>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginPackageSummary {
    pub plugin_id: PluginId,
    pub version: String,
    pub manifest_hash: String,
    pub manifest_version: u16,
    pub display_name: String,
    pub description: String,
    pub publisher: String,
    pub install_source: PluginInstallSource,
    pub trust_level: PluginTrustLevel,
    pub requested_permissions: Vec<String>,
    pub contributions: Vec<PluginContribution>,
    pub discovered_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginInstallationSummary {
    pub package: PluginPackageSummary,
    pub desired_state: PluginDesiredState,
    pub effective_state: PluginEffectiveState,
    pub compatibility: PluginCompatibility,
    pub configuration_revision: u64,
    pub granted_permissions: Vec<String>,
    pub installed_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_error_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginConfiguration {
    pub plugin_id: PluginId,
    pub revision: u64,
    pub values: JsonValue,
    pub values_hash: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginPermissionDecision {
    Granted,
    Denied,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginPermissionGrant {
    pub plugin_id: PluginId,
    pub permission_id: String,
    pub decision: PluginPermissionDecision,
    pub granted_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginListResponse {
    pub items: Vec<PluginInstallationSummary>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContributionResolutionMode {
    Exclusive,
    Ordered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectiveContributionState {
    Available,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ContributionTarget {
    UiTheme {
        theme_id: String,
    },
    VisualRenderer {
        source_kind: String,
        surface: String,
        render_scope: VisualRenderScope,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        selector: Option<String>,
    },
    ArtifactType {
        artifact_type: String,
    },
    GeneratedUiRuntime {
        runtime_id: String,
    },
    GeneratedUiSdk {
        sdk_id: String,
    },
    GeneratedUiActionBridge {
        bridge_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContributionRef {
    pub plugin_id: PluginId,
    pub contribution_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectiveContribution {
    pub plugin_id: PluginId,
    pub plugin_version: String,
    pub contribution_id: String,
    pub extension_point: String,
    pub contract_version: u16,
    pub target: ContributionTarget,
    pub effective_state: EffectiveContributionState,
    pub contribution: PluginContribution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContributionTargetResolution {
    pub target_id: String,
    pub extension_point: String,
    pub mode: ContributionResolutionMode,
    pub target: ContributionTarget,
    pub revision: u64,
    pub conflict: bool,
    pub contributions: Vec<EffectiveContribution>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateContributionTargetPreferencesRequest {
    pub expected_revision: u64,
    #[serde(default)]
    pub ordered_contributions: Vec<ContributionRef>,
    #[serde(default)]
    pub disabled_contributions: Vec<ContributionRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectivePluginSnapshot {
    pub contributions: Vec<EffectiveContribution>,
    #[serde(default)]
    pub resolutions: Vec<ContributionTargetResolution>,
    pub generated_at: DateTime<Utc>,
}

/// Computes the stable content hash used for immutable package identity.
pub fn compute_plugin_manifest_hash(
    manifest: &PluginManifest,
) -> Result<String, serde_json::Error> {
    let encoded = serde_json::to_vec(manifest)?;
    Ok(format!("sha256:{:x}", Sha256::digest(encoded)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_manifest() -> PluginManifest {
        PluginManifest {
            manifest_version: PLUGIN_MANIFEST_VERSION_V1,
            plugin_id: PluginId::from("uprava.theme-dark"),
            version: "1.0.0".to_owned(),
            display_name: "Dark Theme".to_owned(),
            description: "Fixture".to_owned(),
            publisher: "Uprava".to_owned(),
            license: None,
            homepage: None,
            install_source: PluginInstallSource::Bundled,
            trust_level: PluginTrustLevel::DataOnly,
            compatibility: PluginCompatibilityRequirements {
                core: PluginVersionRange {
                    minimum: Some("0.2.12".to_owned()),
                    maximum_exclusive: Some("0.3.0".to_owned()),
                },
                web: PluginVersionRange {
                    minimum: Some("0.2.12".to_owned()),
                    maximum_exclusive: Some("0.3.0".to_owned()),
                },
                node: None,
                protocol_versions: vec![CURRENT_PROTOCOL_VERSION.to_owned()],
            },
            activation_conditions: Vec::new(),
            requested_permissions: vec![THEME_CONTRIBUTION_PERMISSION.to_owned()],
            configuration_schema: None,
            contributions: vec![PluginContribution::UiTheme {
                contribution_id: "uprava.theme-dark.theme".to_owned(),
                contract_version: THEME_CONTRIBUTION_VERSION_V1,
                contribution: ThemeContributionV1 {
                    theme_id: "uprava.dark".to_owned(),
                    label: "Dark".to_owned(),
                    kind: ThemeKind::Dark,
                    color_scheme: ThemeColorScheme::Dark,
                    semantic_tokens: BTreeMap::from([
                        ("content.primary".to_owned(), "#f4f4ef".to_owned()),
                        ("surface.background".to_owned(), "#111310".to_owned()),
                    ]),
                    monaco: MonacoThemeV1 {
                        base: "vs-dark".to_owned(),
                        colors: BTreeMap::new(),
                    },
                    terminal: TerminalThemeV1 {
                        colors: BTreeMap::new(),
                    },
                },
            }],
        }
    }

    #[test]
    fn manifest_hash_should_be_stable_for_same_manifest() {
        let manifest = fixture_manifest();
        let first = compute_plugin_manifest_hash(&manifest).expect("manifest hashes");
        let second = compute_plugin_manifest_hash(&manifest).expect("manifest hashes");

        assert_eq!(first, second);
    }

    #[test]
    fn contribution_should_round_trip_with_discriminator() {
        let contribution = fixture_manifest().contributions.remove(0);
        let encoded = serde_json::to_value(&contribution).expect("contribution serializes");
        let decoded: PluginContribution =
            serde_json::from_value(encoded.clone()).expect("contribution deserializes");

        assert_eq!(encoded["kind"], "ui_theme");
        assert_eq!(decoded, contribution);
    }
}
