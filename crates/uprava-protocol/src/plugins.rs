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
/// Permission required by a package that contributes a theme.
pub const THEME_CONTRIBUTION_PERMISSION: &str = "ui.theme.contribute";

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

/// A manifest-declared extension point.
///
/// `AgentTool` and `ArtifactType` reserve typed links for later slices. Plugin
/// Registry v1 activates only `UiTheme` contributions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PluginContribution {
    UiTheme {
        contract_version: u16,
        contribution: ThemeContributionV1,
    },
    AgentTool {
        contract_version: u16,
        tool_id: ToolId,
    },
    ArtifactType {
        contract_version: u16,
        artifact_type_id: String,
        display_name: String,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectivePluginSnapshot {
    pub contributions: Vec<PluginContribution>,
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
