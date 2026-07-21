//! Core-owned Plugin Registry lifecycle and effective contribution projection.

use std::collections::{BTreeMap, HashSet};

use semver::Version;
use uprava_protocol::{
    compute_plugin_manifest_hash, ArtifactTypeContributionV1, ContributionRef,
    ContributionResolutionMode, ContributionTarget, ContributionTargetResolution,
    EffectiveContribution, EffectiveContributionState, EffectivePluginSnapshot,
    PluginCompatibility, PluginCompatibilityState, PluginContribution, PluginDesiredState,
    PluginEffectiveState, PluginId, PluginInstallSource, PluginInstallationSummary,
    PluginListResponse, PluginManifest, PluginPackageSummary, PluginTrustLevel, PluginVersionRange,
    ThemeContributionV1, UpdateContributionTargetPreferencesRequest, VisualRenderScope,
    VisualRendererContributionV1, VisualRendererKind, VisualSourceMatcherV1,
    ARTIFACT_TYPE_CONTRIBUTION_PERMISSION, ARTIFACT_TYPE_CONTRIBUTION_VERSION_V1,
    CURRENT_PROTOCOL_VERSION, PLUGIN_MANIFEST_VERSION_V1, THEME_CONTRIBUTION_PERMISSION,
    THEME_CONTRIBUTION_VERSION_V1, VISUAL_RENDERER_CONTRIBUTION_PERMISSION,
    VISUAL_RENDERER_CONTRIBUTION_VERSION_V1, VISUAL_RENDERER_CONTRIBUTION_VERSION_V2,
};

use super::super::*;

const DARK_THEME_MANIFEST: &str =
    include_str!("../../../bundled-plugins/uprava.theme-dark/manifest.json");
const MARKDOWN_MANIFEST: &str =
    include_str!("../../../bundled-plugins/uprava.markdown/manifest.json");
const PLAIN_TEXT_MANIFEST: &str =
    include_str!("../../../bundled-plugins/uprava.plain-text/manifest.json");
const CONTENT_ENHANCEMENTS_MANIFEST: &str =
    include_str!("../../../bundled-plugins/uprava.content-enhancements/manifest.json");
const DIAGRAMS_MANIFEST: &str =
    include_str!("../../../bundled-plugins/uprava.diagrams/manifest.json");
const REVIEW_ARTIFACTS_MANIFEST: &str =
    include_str!("../../../bundled-plugins/uprava.review-artifacts/manifest.json");
const TRACE_ARTIFACTS_MANIFEST: &str =
    include_str!("../../../bundled-plugins/uprava.trace-artifacts/manifest.json");
const MAX_PLUGIN_ID_CHARS: usize = 128;
const MAX_PLUGIN_TEXT_CHARS: usize = 2_000;
const MAX_PLUGIN_PERMISSIONS: usize = 64;
const MAX_PLUGIN_CONTRIBUTIONS: usize = 128;
const MAX_THEME_TOKENS: usize = 128;
const MAX_THEME_ADAPTER_COLORS: usize = 128;
const MAX_RENDERER_TARGETS: usize = 32;
const MAX_NORMALIZED_RENDERER_TARGETS: usize = 128;

const REQUIRED_THEME_TOKENS: &[&str] = &[
    "surface.background",
    "surface.muted",
    "surface.raised",
    "content.primary",
    "content.muted",
    "content.inverse",
    "border.default",
    "border.strong",
    "status.risk",
    "status.notice",
    "focus",
    "selection",
    "editor.background",
    "editor.foreground",
    "terminal.background",
    "terminal.foreground",
];

#[derive(Debug, Deserialize)]
pub(crate) struct PluginContributionQuery {
    kind: Option<String>,
}

pub(crate) async fn bootstrap_bundled_plugins(state: &AppState) -> Result<(), AppError> {
    for (source, default_state) in [
        (DARK_THEME_MANIFEST, PluginDesiredState::Disabled),
        (MARKDOWN_MANIFEST, PluginDesiredState::Enabled),
        (PLAIN_TEXT_MANIFEST, PluginDesiredState::Enabled),
        (CONTENT_ENHANCEMENTS_MANIFEST, PluginDesiredState::Enabled),
        (DIAGRAMS_MANIFEST, PluginDesiredState::Enabled),
        (REVIEW_ARTIFACTS_MANIFEST, PluginDesiredState::Enabled),
        (TRACE_ARTIFACTS_MANIFEST, PluginDesiredState::Enabled),
    ] {
        let manifest: PluginManifest = serde_json::from_str(source)?;
        validate_plugin_manifest(&manifest)?;
        register_bundled_plugin(state, &manifest, default_state).await?;
    }
    Ok(())
}

pub(crate) async fn plugin_list_route(
    State(state): State<Arc<AppState>>,
) -> Result<Json<PluginListResponse>, AppError> {
    Ok(Json(list_plugins(&state).await?))
}

pub(crate) async fn plugin_detail_route(
    State(state): State<Arc<AppState>>,
    Path(plugin_id): Path<String>,
) -> Result<Json<PluginInstallationSummary>, AppError> {
    Ok(Json(
        load_plugin_installation(&state, &PluginId::from(plugin_id)).await?,
    ))
}

pub(crate) async fn enable_plugin_route(
    State(state): State<Arc<AppState>>,
    Path(plugin_id): Path<String>,
) -> Result<Json<PluginInstallationSummary>, AppError> {
    let plugin_id = PluginId::from(plugin_id);
    set_plugin_desired_state(&state, &plugin_id, PluginDesiredState::Enabled).await?;
    Ok(Json(load_plugin_installation(&state, &plugin_id).await?))
}

pub(crate) async fn disable_plugin_route(
    State(state): State<Arc<AppState>>,
    Path(plugin_id): Path<String>,
) -> Result<Json<PluginInstallationSummary>, AppError> {
    let plugin_id = PluginId::from(plugin_id);
    set_plugin_desired_state(&state, &plugin_id, PluginDesiredState::Disabled).await?;
    Ok(Json(load_plugin_installation(&state, &plugin_id).await?))
}

pub(crate) async fn plugin_contributions_route(
    State(state): State<Arc<AppState>>,
    Query(query): Query<PluginContributionQuery>,
) -> Result<Json<EffectivePluginSnapshot>, AppError> {
    if query
        .kind
        .as_deref()
        .is_some_and(|kind| !matches!(kind, "ui.theme" | "visual.renderer" | "artifact.type"))
    {
        return Err(AppError::bad_request(
            "plugin.contribution_kind_unsupported",
            "Unsupported plugin contribution kind",
        ));
    }
    let mut snapshot = effective_plugin_snapshot(&state).await?;
    if let Some(kind) = query.kind.as_deref() {
        snapshot
            .contributions
            .retain(|contribution| contribution.extension_point == kind);
        snapshot
            .resolutions
            .retain(|resolution| resolution.extension_point == kind);
    }
    Ok(Json(snapshot))
}

pub(crate) async fn update_plugin_contribution_preferences_route(
    State(state): State<Arc<AppState>>,
    Path(target_id): Path<String>,
    Json(request): Json<UpdateContributionTargetPreferencesRequest>,
) -> Result<Json<ContributionTargetResolution>, AppError> {
    update_contribution_target_preferences(&state, &target_id, request).await?;
    let snapshot = effective_plugin_snapshot(&state).await?;
    let resolution = snapshot
        .resolutions
        .into_iter()
        .find(|resolution| resolution.target_id == target_id)
        .ok_or_else(|| {
            AppError::not_found(
                "plugin.contribution_target_not_found",
                "Plugin contribution target not found",
            )
        })?;
    Ok(Json(resolution))
}

pub(crate) async fn register_bundled_plugin(
    state: &AppState,
    manifest: &PluginManifest,
    default_state: PluginDesiredState,
) -> Result<(), AppError> {
    let manifest_hash = compute_plugin_manifest_hash(manifest)?;
    let manifest_json = serde_json::to_string(manifest)?;
    let compatibility = evaluate_plugin_compatibility(manifest);
    let compatibility_json = serde_json::to_string(&compatibility)?;
    let now = Utc::now();
    let mut transaction = state.pool.begin().await?;

    let existing_hash: Option<String> = sqlx::query_scalar(
        "select manifest_hash from plugin_packages where plugin_id = ?1 and version = ?2",
    )
    .bind(manifest.plugin_id.as_str())
    .bind(&manifest.version)
    .fetch_optional(&mut *transaction)
    .await?;
    if existing_hash
        .as_deref()
        .is_some_and(|hash| hash != manifest_hash)
    {
        return Err(AppError::internal(format!(
            "bundled plugin {}@{} changed without a version bump",
            manifest.plugin_id, manifest.version
        )));
    }

    let package_insert = sqlx::query(
        r#"
        insert into plugin_packages (
            plugin_id, version, manifest_hash, manifest_version, display_name,
            description, publisher, install_source, trust_level, manifest_json,
            discovered_at
        ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        on conflict(plugin_id, version) do nothing
        "#,
    )
    .bind(manifest.plugin_id.as_str())
    .bind(&manifest.version)
    .bind(&manifest_hash)
    .bind(i64::from(manifest.manifest_version))
    .bind(&manifest.display_name)
    .bind(&manifest.description)
    .bind(&manifest.publisher)
    .bind(format_install_source(manifest.install_source))
    .bind(format_trust_level(manifest.trust_level))
    .bind(&manifest_json)
    .bind(now)
    .execute(&mut *transaction)
    .await?;

    let initial_effective_state = match (compatibility.state, default_state) {
        (PluginCompatibilityState::Compatible, PluginDesiredState::Enabled) => {
            PluginEffectiveState::Active
        }
        (PluginCompatibilityState::Compatible, PluginDesiredState::Disabled) => {
            PluginEffectiveState::Disabled
        }
        (PluginCompatibilityState::Incompatible, _) => PluginEffectiveState::Incompatible,
    };
    sqlx::query(
        r#"
        insert into plugin_installations (
            plugin_id, active_version, desired_state, effective_state,
            compatibility_json, configuration_revision, installed_at, updated_at,
            last_error_code
        ) values (?1, ?2, ?3, ?4, ?5, 0, ?6, ?6, null)
        on conflict(plugin_id) do nothing
        "#,
    )
    .bind(manifest.plugin_id.as_str())
    .bind(&manifest.version)
    .bind(format_desired_state(default_state))
    .bind(format_effective_state(initial_effective_state))
    .bind(&compatibility_json)
    .bind(now)
    .execute(&mut *transaction)
    .await?;

    let active_version: String =
        sqlx::query_scalar("select active_version from plugin_installations where plugin_id = ?1")
            .bind(manifest.plugin_id.as_str())
            .fetch_one(&mut *transaction)
            .await?;
    let active_semver = Version::parse(&active_version)
        .map_err(|_| AppError::internal("installed plugin version is not SemVer"))?;
    let discovered_semver = Version::parse(&manifest.version)
        .map_err(|_| AppError::internal("bundled plugin version is not SemVer"))?;
    if discovered_semver > active_semver {
        sqlx::query(
            r#"
            update plugin_installations
            set active_version = ?1, updated_at = ?2
            where plugin_id = ?3
            "#,
        )
        .bind(&manifest.version)
        .bind(now)
        .bind(manifest.plugin_id.as_str())
        .execute(&mut *transaction)
        .await?;
    }

    let empty_configuration = JsonValue(json!({}));
    let configuration_json = serde_json::to_string(&empty_configuration.0)?;
    let configuration_hash = format!("sha256:{:x}", Sha256::digest(configuration_json.as_bytes()));
    sqlx::query(
        r#"
        insert into plugin_configurations (
            plugin_id, revision, values_json, values_hash, updated_at
        ) values (?1, 0, ?2, ?3, ?4)
        on conflict(plugin_id) do nothing
        "#,
    )
    .bind(manifest.plugin_id.as_str())
    .bind(configuration_json)
    .bind(configuration_hash)
    .bind(now)
    .execute(&mut *transaction)
    .await?;

    for permission in &manifest.requested_permissions {
        sqlx::query(
            r#"
            insert into plugin_permission_grants (
                plugin_id, permission_id, decision, granted_at, updated_at
            ) values (?1, ?2, 'granted', ?3, ?3)
            on conflict(plugin_id, permission_id) do nothing
            "#,
        )
        .bind(manifest.plugin_id.as_str())
        .bind(permission)
        .bind(now)
        .execute(&mut *transaction)
        .await?;
    }
    let desired_state: String =
        sqlx::query_scalar("select desired_state from plugin_installations where plugin_id = ?1")
            .bind(manifest.plugin_id.as_str())
            .fetch_one(&mut *transaction)
            .await?;
    let reconciled_effective_state = match compatibility.state {
        PluginCompatibilityState::Incompatible => PluginEffectiveState::Incompatible,
        PluginCompatibilityState::Compatible if desired_state == "enabled" => {
            PluginEffectiveState::Active
        }
        PluginCompatibilityState::Compatible => PluginEffectiveState::Disabled,
    };
    sqlx::query(
        r#"
        update plugin_installations
        set effective_state = ?1, compatibility_json = ?2, updated_at = ?3,
            last_error_code = case when ?1 = 'incompatible'
                then 'plugin.incompatible' else null end
        where plugin_id = ?4
          and (effective_state != ?1 or compatibility_json != ?2)
        "#,
    )
    .bind(format_effective_state(reconciled_effective_state))
    .bind(&compatibility_json)
    .bind(now)
    .bind(manifest.plugin_id.as_str())
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;

    if package_insert.rows_affected() > 0 {
        audit_security_event(
            state,
            "plugin.discovered",
            None,
            Some("core.bootstrap".to_owned()),
            "registered",
            JsonValue(json!({
                "plugin_id": manifest.plugin_id,
                "version": manifest.version,
                "manifest_hash": manifest_hash,
                "install_source": "bundled"
            })),
        )
        .await?;
    }
    Ok(())
}

pub(crate) async fn list_plugins(state: &AppState) -> Result<PluginListResponse, AppError> {
    let plugin_ids: Vec<String> =
        sqlx::query_scalar("select plugin_id from plugin_installations order by plugin_id asc")
            .fetch_all(&state.pool)
            .await?;
    let mut items = Vec::with_capacity(plugin_ids.len());
    for plugin_id in plugin_ids {
        items.push(load_plugin_installation(state, &PluginId::from(plugin_id)).await?);
    }
    Ok(PluginListResponse { items })
}

pub(crate) async fn load_plugin_installation(
    state: &AppState,
    plugin_id: &PluginId,
) -> Result<PluginInstallationSummary, AppError> {
    let row = sqlx::query(
        r#"
        select p.version, p.manifest_hash, p.manifest_version, p.display_name,
               p.description, p.publisher, p.install_source, p.trust_level,
               p.manifest_json, p.discovered_at, i.desired_state,
               i.effective_state, i.compatibility_json,
               i.configuration_revision, i.installed_at, i.updated_at,
               i.last_error_code
        from plugin_installations i
        join plugin_packages p
          on p.plugin_id = i.plugin_id and p.version = i.active_version
        where i.plugin_id = ?1
        "#,
    )
    .bind(plugin_id.as_str())
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::not_found("plugin.not_found", "Plugin not found"))?;

    let manifest: PluginManifest = serde_json::from_str(row.get("manifest_json"))?;
    let compatibility: PluginCompatibility = serde_json::from_str(row.get("compatibility_json"))?;
    let grants: Vec<(String, String)> = sqlx::query_as(
        "select permission_id, decision from plugin_permission_grants where plugin_id = ?1 order by permission_id",
    )
    .bind(plugin_id.as_str())
    .fetch_all(&state.pool)
    .await?;
    let granted_permissions = grants
        .into_iter()
        .filter_map(|(permission, decision)| (decision == "granted").then_some(permission))
        .collect();

    Ok(PluginInstallationSummary {
        package: PluginPackageSummary {
            plugin_id: plugin_id.clone(),
            version: row.get("version"),
            manifest_hash: row.get("manifest_hash"),
            manifest_version: u16::try_from(row.get::<i64, _>("manifest_version"))
                .map_err(|_| AppError::internal("plugin manifest version exceeds u16"))?,
            display_name: row.get("display_name"),
            description: row.get("description"),
            publisher: row.get("publisher"),
            install_source: parse_install_source(row.get("install_source"))?,
            trust_level: parse_trust_level(row.get("trust_level"))?,
            requested_permissions: manifest.requested_permissions,
            contributions: manifest.contributions,
            discovered_at: row.get("discovered_at"),
        },
        desired_state: parse_desired_state(row.get("desired_state"))?,
        effective_state: parse_effective_state(row.get("effective_state"))?,
        compatibility,
        configuration_revision: u64::try_from(row.get::<i64, _>("configuration_revision"))
            .map_err(|_| AppError::internal("plugin configuration revision is invalid"))?,
        granted_permissions,
        installed_at: row.get("installed_at"),
        updated_at: row.get("updated_at"),
        last_error_code: row.get("last_error_code"),
    })
}

pub(crate) async fn set_plugin_desired_state(
    state: &AppState,
    plugin_id: &PluginId,
    desired_state: PluginDesiredState,
) -> Result<(), AppError> {
    let current = load_plugin_installation(state, plugin_id).await?;
    let manifest = load_plugin_manifest(state, plugin_id, &current.package.version).await?;
    validate_plugin_manifest(&manifest)?;
    let compatibility = evaluate_plugin_compatibility(&manifest);
    let compatibility_json = serde_json::to_string(&compatibility)?;
    let permission_granted = manifest
        .requested_permissions
        .iter()
        .all(|permission| current.granted_permissions.contains(permission));
    let (effective_state, last_error_code) = match desired_state {
        PluginDesiredState::Disabled => (PluginEffectiveState::Disabled, None),
        PluginDesiredState::Enabled
            if compatibility.state == PluginCompatibilityState::Incompatible =>
        {
            (
                PluginEffectiveState::Incompatible,
                Some("plugin.incompatible"),
            )
        }
        PluginDesiredState::Enabled if !permission_granted => (
            PluginEffectiveState::Error,
            Some("plugin.permission_denied"),
        ),
        PluginDesiredState::Enabled => (PluginEffectiveState::Active, None),
    };
    let now = Utc::now();
    sqlx::query(
        r#"
        update plugin_installations
        set desired_state = ?1, effective_state = ?2, compatibility_json = ?3,
            updated_at = ?4, last_error_code = ?5
        where plugin_id = ?6
        "#,
    )
    .bind(format_desired_state(desired_state))
    .bind(format_effective_state(effective_state))
    .bind(compatibility_json)
    .bind(now)
    .bind(last_error_code)
    .bind(plugin_id.as_str())
    .execute(&state.pool)
    .await?;

    let (event_kind, outcome) = match desired_state {
        PluginDesiredState::Disabled => ("plugin.disabled", "disabled"),
        PluginDesiredState::Enabled if effective_state == PluginEffectiveState::Active => {
            ("plugin.enabled", "active")
        }
        PluginDesiredState::Enabled if effective_state == PluginEffectiveState::Incompatible => {
            ("plugin.compatibility_failed", "incompatible")
        }
        PluginDesiredState::Enabled => ("plugin.activation_failed", "denied"),
    };
    audit_security_event(
        state,
        event_kind,
        None,
        Some("web.local_user".to_owned()),
        outcome,
        JsonValue(json!({
            "plugin_id": plugin_id,
            "version": manifest.version,
            "manifest_hash": current.package.manifest_hash,
            "effective_state": format_effective_state(effective_state),
            "error_code": last_error_code
        })),
    )
    .await?;

    if let Some(error_code) = last_error_code {
        return Err(AppError::bad_request(
            error_code,
            "Plugin could not be activated",
        ));
    }
    Ok(())
}

async fn load_plugin_manifest(
    state: &AppState,
    plugin_id: &PluginId,
    version: &str,
) -> Result<PluginManifest, AppError> {
    let manifest_json: String = sqlx::query_scalar(
        "select manifest_json from plugin_packages where plugin_id = ?1 and version = ?2",
    )
    .bind(plugin_id.as_str())
    .bind(version)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::not_found("plugin.package_not_found", "Plugin package not found"))?;
    serde_json::from_str(&manifest_json).map_err(AppError::from)
}

pub(crate) async fn effective_plugin_snapshot(
    state: &AppState,
) -> Result<EffectivePluginSnapshot, AppError> {
    let manifest_rows: Vec<String> = sqlx::query_scalar(
        r#"
        select p.manifest_json
        from plugin_installations i
        join plugin_packages p
          on p.plugin_id = i.plugin_id and p.version = i.active_version
        where i.effective_state = 'active'
        order by i.plugin_id
        "#,
    )
    .fetch_all(&state.pool)
    .await?;
    let preference_rows = sqlx::query(
        r#"
        select target_id, revision, ordered_contributions_json, disabled_contributions_json
        from plugin_contribution_target_preferences
        "#,
    )
    .fetch_all(&state.pool)
    .await?;
    let mut preferences = BTreeMap::new();
    for row in preference_rows {
        preferences.insert(
            row.get::<String, _>("target_id"),
            StoredContributionPreference {
                revision: u64::try_from(row.get::<i64, _>("revision"))
                    .map_err(|_| AppError::internal("plugin preference revision is invalid"))?,
                ordered: serde_json::from_str(row.get("ordered_contributions_json"))?,
                disabled: serde_json::from_str(row.get("disabled_contributions_json"))?,
            },
        );
    }

    let mut direct_contributions = Vec::new();
    let mut target_groups: BTreeMap<String, TargetGroupDraft> = BTreeMap::new();
    for manifest_json in manifest_rows {
        let manifest: PluginManifest = serde_json::from_str(&manifest_json)?;
        for contribution in manifest.contributions {
            match &contribution {
                PluginContribution::UiTheme {
                    contribution_id,
                    contract_version: THEME_CONTRIBUTION_VERSION_V1,
                    contribution: theme,
                } => direct_contributions.push(EffectiveContribution {
                    plugin_id: manifest.plugin_id.clone(),
                    plugin_version: manifest.version.clone(),
                    contribution_id: contribution_id.clone(),
                    extension_point: "ui.theme".to_owned(),
                    contract_version: THEME_CONTRIBUTION_VERSION_V1,
                    target: ContributionTarget::UiTheme {
                        theme_id: theme.theme_id.clone(),
                    },
                    effective_state: EffectiveContributionState::Available,
                    contribution,
                }),
                PluginContribution::VisualRenderer {
                    contribution_id,
                    contract_version,
                    contribution: renderer,
                } if is_supported_visual_renderer_version(*contract_version) => {
                    for source_kind in &renderer.accepted_source_kinds {
                        for surface in &renderer.allowed_surfaces {
                            for render_scope in &renderer.render_scopes {
                                for selector in renderer_selectors(renderer) {
                                    let target = ContributionTarget::VisualRenderer {
                                        source_kind: source_kind.clone(),
                                        surface: surface.clone(),
                                        render_scope: *render_scope,
                                        selector,
                                    };
                                    let target_id = contribution_target_id(&target)?;
                                    let effective = EffectiveContribution {
                                        plugin_id: manifest.plugin_id.clone(),
                                        plugin_version: manifest.version.clone(),
                                        contribution_id: contribution_id.clone(),
                                        extension_point: "visual.renderer".to_owned(),
                                        contract_version: *contract_version,
                                        target: target.clone(),
                                        effective_state: EffectiveContributionState::Available,
                                        contribution: contribution.clone(),
                                    };
                                    target_groups
                                        .entry(target_id)
                                        .or_insert_with(|| TargetGroupDraft {
                                            extension_point: "visual.renderer",
                                            target,
                                            contributions: Vec::new(),
                                        })
                                        .contributions
                                        .push(effective);
                                }
                            }
                        }
                    }
                }
                PluginContribution::ArtifactType {
                    contribution_id,
                    contract_version: ARTIFACT_TYPE_CONTRIBUTION_VERSION_V1,
                    contribution: artifact_type,
                } => {
                    let target = ContributionTarget::ArtifactType {
                        artifact_type: artifact_type.artifact_type_id.clone(),
                    };
                    let target_id = contribution_target_id(&target)?;
                    let effective = EffectiveContribution {
                        plugin_id: manifest.plugin_id.clone(),
                        plugin_version: manifest.version.clone(),
                        contribution_id: contribution_id.clone(),
                        extension_point: "artifact.type".to_owned(),
                        contract_version: ARTIFACT_TYPE_CONTRIBUTION_VERSION_V1,
                        target: target.clone(),
                        effective_state: EffectiveContributionState::Available,
                        contribution,
                    };
                    target_groups
                        .entry(target_id)
                        .or_insert_with(|| TargetGroupDraft {
                            extension_point: "artifact.type",
                            target,
                            contributions: Vec::new(),
                        })
                        .contributions
                        .push(effective);
                }
                _ => {}
            }
        }
    }

    let mut resolutions = Vec::with_capacity(target_groups.len());
    for (target_id, mut draft) in target_groups {
        draft.contributions.sort_by(|left, right| {
            contribution_ref_sort_key(left).cmp(&contribution_ref_sort_key(right))
        });
        let preference = preferences.remove(&target_id).unwrap_or_default();
        let mut ordered = Vec::with_capacity(draft.contributions.len());
        for preferred in &preference.ordered {
            if let Some(index) = draft
                .contributions
                .iter()
                .position(|candidate| contribution_matches(candidate, preferred))
            {
                ordered.push(draft.contributions.remove(index));
            }
        }
        ordered.append(&mut draft.contributions);
        let disabled: HashSet<String> = preference
            .disabled
            .iter()
            .map(contribution_ref_key)
            .collect();
        for candidate in &mut ordered {
            if disabled.contains(&effective_contribution_key(candidate)) {
                candidate.effective_state = EffectiveContributionState::Disabled;
            }
        }
        let conflict = ordered
            .iter()
            .filter(|candidate| candidate.effective_state == EffectiveContributionState::Available)
            .count()
            > 1;
        resolutions.push(ContributionTargetResolution {
            target_id,
            extension_point: draft.extension_point.to_owned(),
            mode: ContributionResolutionMode::Exclusive,
            target: draft.target,
            revision: preference.revision,
            conflict,
            contributions: ordered,
        });
    }
    let mut contributions = direct_contributions;
    contributions.extend(
        resolutions
            .iter()
            .flat_map(|resolution| resolution.contributions.iter().cloned()),
    );
    Ok(EffectivePluginSnapshot {
        contributions,
        resolutions,
        generated_at: Utc::now(),
    })
}

fn renderer_selectors(renderer: &VisualRendererContributionV1) -> Vec<Option<String>> {
    match &renderer.source_matcher {
        Some(VisualSourceMatcherV1::FencedLanguage { language_ids }) => {
            language_ids.iter().cloned().map(Some).collect()
        }
        Some(VisualSourceMatcherV1::StrictColorLiteral { formats }) => {
            formats.iter().cloned().map(Some).collect()
        }
        None => vec![None],
    }
}

fn is_supported_visual_renderer_version(version: u16) -> bool {
    matches!(
        version,
        VISUAL_RENDERER_CONTRIBUTION_VERSION_V1 | VISUAL_RENDERER_CONTRIBUTION_VERSION_V2
    )
}

#[derive(Default)]
struct StoredContributionPreference {
    revision: u64,
    ordered: Vec<ContributionRef>,
    disabled: Vec<ContributionRef>,
}

struct TargetGroupDraft {
    extension_point: &'static str,
    target: ContributionTarget,
    contributions: Vec<EffectiveContribution>,
}

fn contribution_target_id(target: &ContributionTarget) -> Result<String, AppError> {
    let encoded = serde_json::to_vec(target)?;
    Ok(format!("{:x}", Sha256::digest(encoded)))
}

fn contribution_ref_sort_key(contribution: &EffectiveContribution) -> (&str, &str) {
    (
        contribution.plugin_id.as_str(),
        contribution.contribution_id.as_str(),
    )
}

fn contribution_ref_key(reference: &ContributionRef) -> String {
    format!("{}\u{1f}{}", reference.plugin_id, reference.contribution_id)
}

fn effective_contribution_key(contribution: &EffectiveContribution) -> String {
    format!(
        "{}\u{1f}{}",
        contribution.plugin_id, contribution.contribution_id
    )
}

fn contribution_matches(contribution: &EffectiveContribution, reference: &ContributionRef) -> bool {
    contribution.plugin_id == reference.plugin_id
        && contribution.contribution_id == reference.contribution_id
}

pub(crate) async fn update_contribution_target_preferences(
    state: &AppState,
    target_id: &str,
    request: UpdateContributionTargetPreferencesRequest,
) -> Result<(), AppError> {
    let snapshot = effective_plugin_snapshot(state).await?;
    let resolution = snapshot
        .resolutions
        .iter()
        .find(|resolution| resolution.target_id == target_id)
        .ok_or_else(|| {
            AppError::not_found(
                "plugin.contribution_target_not_found",
                "Plugin contribution target not found",
            )
        })?;
    if resolution.revision != request.expected_revision {
        return Err(AppError::conflict(
            "plugin.contribution_preference_conflict",
            "Plugin contribution preferences changed; reload and retry",
        ));
    }
    validate_contribution_references(resolution, &request.ordered_contributions)?;
    validate_contribution_references(resolution, &request.disabled_contributions)?;

    let next_revision = request.expected_revision.checked_add(1).ok_or_else(|| {
        AppError::bad_request(
            "plugin.contribution_revision_invalid",
            "Plugin contribution preference revision is exhausted",
        )
    })?;
    let target_json = serde_json::to_string(&resolution.target)?;
    let ordered_json = serde_json::to_string(&request.ordered_contributions)?;
    let disabled_json = serde_json::to_string(&request.disabled_contributions)?;
    let updated = sqlx::query(
        r#"
        insert into plugin_contribution_target_preferences (
            target_id, target_json, revision, ordered_contributions_json,
            disabled_contributions_json, updated_at
        ) values (?1, ?2, ?3, ?4, ?5, ?6)
        on conflict(target_id) do update set
            target_json = excluded.target_json,
            revision = excluded.revision,
            ordered_contributions_json = excluded.ordered_contributions_json,
            disabled_contributions_json = excluded.disabled_contributions_json,
            updated_at = excluded.updated_at
        where plugin_contribution_target_preferences.revision = ?7
        "#,
    )
    .bind(target_id)
    .bind(target_json)
    .bind(i64::try_from(next_revision).map_err(|_| {
        AppError::bad_request(
            "plugin.contribution_revision_invalid",
            "Plugin contribution preference revision is invalid",
        )
    })?)
    .bind(ordered_json)
    .bind(disabled_json)
    .bind(Utc::now())
    .bind(i64::try_from(request.expected_revision).map_err(|_| {
        AppError::bad_request(
            "plugin.contribution_revision_invalid",
            "Plugin contribution preference revision is invalid",
        )
    })?)
    .execute(&state.pool)
    .await?;
    if updated.rows_affected() != 1 {
        return Err(AppError::conflict(
            "plugin.contribution_preference_conflict",
            "Plugin contribution preferences changed; reload and retry",
        ));
    }
    audit_security_event(
        state,
        "plugin.contribution_preferences_changed",
        None,
        Some("web.local_user".to_owned()),
        "updated",
        JsonValue(json!({
            "target_id": target_id,
            "revision": next_revision,
            "ordered_count": request.ordered_contributions.len(),
            "disabled_count": request.disabled_contributions.len()
        })),
    )
    .await?;
    Ok(())
}

fn validate_contribution_references(
    resolution: &ContributionTargetResolution,
    references: &[ContributionRef],
) -> Result<(), AppError> {
    let mut seen = HashSet::new();
    for reference in references {
        let key = contribution_ref_key(reference);
        if !seen.insert(key)
            || !resolution
                .contributions
                .iter()
                .any(|candidate| contribution_matches(candidate, reference))
        {
            return Err(AppError::bad_request(
                "plugin.contribution_preference_invalid",
                "Plugin contribution preference contains an unknown or duplicate entry",
            ));
        }
    }
    Ok(())
}

fn validate_plugin_manifest(manifest: &PluginManifest) -> Result<(), AppError> {
    if manifest.manifest_version != PLUGIN_MANIFEST_VERSION_V1 {
        return Err(plugin_manifest_error("Unsupported plugin manifest version"));
    }
    validate_namespaced_id(manifest.plugin_id.as_str(), "plugin_id")?;
    Version::parse(&manifest.version)
        .map_err(|_| plugin_manifest_error("Plugin version must be SemVer"))?;
    for value in [
        manifest.display_name.as_str(),
        manifest.description.as_str(),
        manifest.publisher.as_str(),
    ] {
        if value.is_empty() || value.chars().count() > MAX_PLUGIN_TEXT_CHARS {
            return Err(plugin_manifest_error(
                "Plugin metadata is empty or oversized",
            ));
        }
    }
    if manifest.requested_permissions.len() > MAX_PLUGIN_PERMISSIONS
        || manifest.contributions.len() > MAX_PLUGIN_CONTRIBUTIONS
    {
        return Err(plugin_manifest_error(
            "Plugin manifest has too many entries",
        ));
    }
    if manifest.install_source != PluginInstallSource::Bundled
        || !matches!(
            manifest.trust_level,
            PluginTrustLevel::DataOnly | PluginTrustLevel::TrustedBundled
        )
    {
        return Err(plugin_manifest_error(
            "Plugin Registry accepts bundled data-only or trusted packages",
        ));
    }
    if manifest.requested_permissions.iter().any(|permission| {
        !matches!(
            permission.as_str(),
            THEME_CONTRIBUTION_PERMISSION
                | VISUAL_RENDERER_CONTRIBUTION_PERMISSION
                | ARTIFACT_TYPE_CONTRIBUTION_PERMISSION
        )
    }) {
        return Err(plugin_manifest_error(
            "Plugin Registry v1 contains an unsupported permission",
        ));
    }
    if manifest.plugin_id.as_str().starts_with("uprava.")
        && manifest.install_source != PluginInstallSource::Bundled
    {
        return Err(plugin_manifest_error("Reserved plugin namespace"));
    }
    let mut contribution_ids = HashSet::new();
    for contribution in &manifest.contributions {
        let contribution_id = contribution_id(contribution);
        validate_namespaced_id(contribution_id, "contribution_id")?;
        if !contribution_ids.insert(contribution_id) {
            return Err(plugin_manifest_error(
                "Plugin contribution identifiers must be unique",
            ));
        }
        match contribution {
            PluginContribution::UiTheme {
                contract_version,
                contribution,
                ..
            } => {
                if *contract_version != THEME_CONTRIBUTION_VERSION_V1 {
                    return Err(plugin_manifest_error(
                        "Unsupported ui.theme contribution version",
                    ));
                }
                if !manifest
                    .requested_permissions
                    .iter()
                    .any(|permission| permission == THEME_CONTRIBUTION_PERMISSION)
                {
                    return Err(plugin_manifest_error(
                        "Theme contribution requires ui.theme.contribute",
                    ));
                }
                validate_theme_contribution(contribution)?;
            }
            PluginContribution::VisualRenderer {
                contract_version,
                contribution,
                ..
            } => {
                if !is_supported_visual_renderer_version(*contract_version) {
                    return Err(plugin_manifest_error(
                        "Unsupported visual.renderer contribution version",
                    ));
                }
                if manifest.trust_level != PluginTrustLevel::TrustedBundled {
                    return Err(plugin_manifest_error(
                        "Visual renderer contribution requires trusted_bundled",
                    ));
                }
                if !manifest
                    .requested_permissions
                    .iter()
                    .any(|permission| permission == VISUAL_RENDERER_CONTRIBUTION_PERMISSION)
                {
                    return Err(plugin_manifest_error(
                        "Visual renderer contribution requires visual.renderer.contribute",
                    ));
                }
                validate_visual_renderer_contribution(*contract_version, contribution)?;
            }
            PluginContribution::ArtifactType {
                contract_version,
                contribution,
                ..
            } => {
                if *contract_version != ARTIFACT_TYPE_CONTRIBUTION_VERSION_V1 {
                    return Err(plugin_manifest_error(
                        "Unsupported artifact.type contribution version",
                    ));
                }
                if !manifest
                    .requested_permissions
                    .iter()
                    .any(|permission| permission == ARTIFACT_TYPE_CONTRIBUTION_PERMISSION)
                {
                    return Err(plugin_manifest_error(
                        "Artifact type contribution requires artifact.type.contribute",
                    ));
                }
                validate_artifact_type_contribution(contribution)?;
            }
            PluginContribution::AgentTool { .. } => {}
        }
    }
    Ok(())
}

fn validate_visual_renderer_contribution(
    contract_version: u16,
    renderer: &VisualRendererContributionV1,
) -> Result<(), AppError> {
    validate_namespaced_id(&renderer.renderer_id, "renderer_id")?;
    validate_namespaced_id(&renderer.implementation_id, "implementation_id")?;
    if renderer.accepted_source_kinds.is_empty()
        || renderer.render_scopes.is_empty()
        || renderer.allowed_surfaces.is_empty()
        || renderer.accepted_source_kinds.len() > MAX_RENDERER_TARGETS
        || renderer.render_scopes.len() > MAX_RENDERER_TARGETS
        || renderer.allowed_surfaces.len() > MAX_RENDERER_TARGETS
    {
        return Err(plugin_manifest_error(
            "Visual renderer targets are empty or oversized",
        ));
    }
    let selectors = renderer_selectors(renderer);
    let normalized_target_count = renderer
        .accepted_source_kinds
        .len()
        .checked_mul(renderer.render_scopes.len())
        .and_then(|count| count.checked_mul(renderer.allowed_surfaces.len()))
        .and_then(|count| count.checked_mul(selectors.len()))
        .filter(|count| *count <= MAX_NORMALIZED_RENDERER_TARGETS)
        .ok_or_else(|| plugin_manifest_error("Visual renderer expands to too many targets"))?;
    if normalized_target_count == 0
        || has_duplicates(&renderer.accepted_source_kinds)
        || has_duplicates(&renderer.render_scopes)
        || has_duplicates(&renderer.allowed_surfaces)
    {
        return Err(plugin_manifest_error(
            "Visual renderer targets must be unique",
        ));
    }
    for source_kind in &renderer.accepted_source_kinds {
        validate_namespaced_id(source_kind, "accepted_source_kind")?;
    }
    for surface in &renderer.allowed_surfaces {
        validate_namespaced_id(surface, "allowed_surface")?;
    }
    let scopes_match_renderer_kind = match renderer.renderer_kind {
        VisualRendererKind::Content => renderer
            .render_scopes
            .iter()
            .all(|scope| *scope == VisualRenderScope::ContentEnhancement),
        VisualRendererKind::InlineFragment => renderer.render_scopes.iter().all(|scope| {
            matches!(
                scope,
                VisualRenderScope::InlineFragment | VisualRenderScope::DetailView
            )
        }),
        VisualRendererKind::Block => renderer
            .render_scopes
            .iter()
            .all(|scope| *scope == VisualRenderScope::Block),
        VisualRendererKind::ArtifactViewer => renderer.render_scopes.iter().all(|scope| {
            matches!(
                scope,
                VisualRenderScope::ArtifactViewer | VisualRenderScope::DetailView
            )
        }),
    };
    if !scopes_match_renderer_kind {
        return Err(plugin_manifest_error(
            "Visual renderer kind does not match its render scopes",
        ));
    }
    if (renderer.renderer_kind == VisualRendererKind::InlineFragment)
        != renderer.source_matcher.is_some()
    {
        return Err(plugin_manifest_error(
            "Inline fragment renderers require a declarative source matcher",
        ));
    }
    if contract_version == VISUAL_RENDERER_CONTRIBUTION_VERSION_V1
        && (renderer.renderer_kind != VisualRendererKind::Content
            || renderer.render_scopes != [VisualRenderScope::ContentEnhancement]
            || renderer.source_matcher.is_some()
            || !renderer.visual_kinds.is_empty()
            || !renderer.actions.is_empty())
    {
        return Err(plugin_manifest_error(
            "visual.renderer v1 only supports content enhancement",
        ));
    }
    if renderer.source_matcher.is_some()
        && (selectors.is_empty()
            || selectors.len() > MAX_RENDERER_TARGETS
            || selectors.iter().any(|selector| {
                selector
                    .as_deref()
                    .is_none_or(|selector| !valid_selector(selector))
            }))
    {
        return Err(plugin_manifest_error(
            "Visual renderer source matcher is empty or invalid",
        ));
    }
    for visual_kind in &renderer.visual_kinds {
        validate_namespaced_id(visual_kind, "visual_kind")?;
    }
    for action in &renderer.actions {
        validate_namespaced_id(action, "visual_action")?;
    }
    Ok(())
}

fn valid_selector(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_PLUGIN_ID_CHARS
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-' | b'_')
        })
}

fn validate_artifact_type_contribution(
    contribution: &ArtifactTypeContributionV1,
) -> Result<(), AppError> {
    validate_namespaced_id(&contribution.artifact_type_id, "artifact_type_id")?;
    if contribution.display_name.is_empty()
        || contribution.description.is_empty()
        || contribution.display_name.chars().count() > MAX_PLUGIN_TEXT_CHARS
        || contribution.description.chars().count() > MAX_PLUGIN_TEXT_CHARS
        || contribution.schema_version == 0
    {
        return Err(plugin_manifest_error(
            "Artifact type metadata is empty or invalid",
        ));
    }
    Ok(())
}

fn has_duplicates<T: PartialEq>(values: &[T]) -> bool {
    values
        .iter()
        .enumerate()
        .any(|(index, value)| values[index + 1..].contains(value))
}

fn contribution_id(contribution: &PluginContribution) -> &str {
    match contribution {
        PluginContribution::UiTheme {
            contribution_id, ..
        }
        | PluginContribution::VisualRenderer {
            contribution_id, ..
        }
        | PluginContribution::AgentTool {
            contribution_id, ..
        }
        | PluginContribution::ArtifactType {
            contribution_id, ..
        } => contribution_id,
    }
}

fn validate_theme_contribution(theme: &ThemeContributionV1) -> Result<(), AppError> {
    validate_namespaced_id(&theme.theme_id, "theme_id")?;
    if theme.semantic_tokens.len() > MAX_THEME_TOKENS
        || theme.monaco.colors.len() > MAX_THEME_ADAPTER_COLORS
        || theme.terminal.colors.len() > MAX_THEME_ADAPTER_COLORS
    {
        return Err(plugin_manifest_error("Theme palette is oversized"));
    }
    if REQUIRED_THEME_TOKENS
        .iter()
        .any(|token| !theme.semantic_tokens.contains_key(*token))
    {
        return Err(plugin_manifest_error(
            "Theme is missing required semantic tokens",
        ));
    }
    if theme
        .semantic_tokens
        .values()
        .chain(theme.monaco.colors.values())
        .chain(theme.terminal.colors.values())
        .any(|color| !valid_hex_color(color))
    {
        return Err(plugin_manifest_error("Theme contains an invalid color"));
    }
    for (foreground, background, minimum) in [
        ("content.primary", "surface.background", 4.5),
        ("content.muted", "surface.background", 3.0),
        ("focus", "surface.background", 3.0),
        ("terminal.foreground", "terminal.background", 4.5),
    ] {
        let foreground = theme
            .semantic_tokens
            .get(foreground)
            .and_then(|color| parse_hex_rgb(color));
        let background = theme
            .semantic_tokens
            .get(background)
            .and_then(|color| parse_hex_rgb(color));
        if foreground
            .zip(background)
            .is_none_or(|(foreground, background)| contrast_ratio(foreground, background) < minimum)
        {
            return Err(plugin_manifest_error(
                "Theme does not meet minimum critical contrast",
            ));
        }
    }
    if !matches!(theme.monaco.base.as_str(), "vs" | "vs-dark" | "hc-black") {
        return Err(plugin_manifest_error(
            "Theme contains an invalid Monaco base",
        ));
    }
    Ok(())
}

fn evaluate_plugin_compatibility(manifest: &PluginManifest) -> PluginCompatibility {
    let mut diagnostics = Vec::new();
    if !version_in_range(APP_VERSION, &manifest.compatibility.core) {
        diagnostics.push(format!("Core {APP_VERSION} is outside the supported range"));
    }
    if !version_in_range(APP_VERSION, &manifest.compatibility.web) {
        diagnostics.push(format!("Web {APP_VERSION} is outside the supported range"));
    }
    if !manifest
        .compatibility
        .protocol_versions
        .iter()
        .any(|version| version == CURRENT_PROTOCOL_VERSION)
    {
        diagnostics.push(format!(
            "Protocol {CURRENT_PROTOCOL_VERSION} is not supported by the plugin"
        ));
    }
    PluginCompatibility {
        state: if diagnostics.is_empty() {
            PluginCompatibilityState::Compatible
        } else {
            PluginCompatibilityState::Incompatible
        },
        diagnostics,
    }
}

fn version_in_range(version: &str, range: &PluginVersionRange) -> bool {
    let Ok(version) = Version::parse(version) else {
        return false;
    };
    if let Some(minimum) = &range.minimum {
        let Ok(minimum) = Version::parse(minimum) else {
            return false;
        };
        if version < minimum {
            return false;
        }
    }
    if let Some(maximum) = &range.maximum_exclusive {
        let Ok(maximum) = Version::parse(maximum) else {
            return false;
        };
        if version >= maximum {
            return false;
        }
    }
    true
}

fn validate_namespaced_id(value: &str, field: &str) -> Result<(), AppError> {
    let valid = value.len() <= MAX_PLUGIN_ID_CHARS
        && value.contains('.')
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-' | b'_')
        });
    if valid {
        Ok(())
    } else {
        Err(plugin_manifest_error(format!(
            "Plugin {field} must be a bounded namespaced identifier"
        )))
    }
}

fn valid_hex_color(value: &str) -> bool {
    matches!(value.len(), 4 | 7 | 9)
        && value.starts_with('#')
        && value[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn parse_hex_rgb(value: &str) -> Option<[u8; 3]> {
    let hex = value.strip_prefix('#')?;
    match hex.len() {
        3 => {
            let mut bytes = [0u8; 3];
            for (index, digit) in hex.bytes().enumerate() {
                let value = (digit as char).to_digit(16)? as u8;
                bytes[index] = value * 17;
            }
            Some(bytes)
        }
        6 => Some([
            u8::from_str_radix(&hex[0..2], 16).ok()?,
            u8::from_str_radix(&hex[2..4], 16).ok()?,
            u8::from_str_radix(&hex[4..6], 16).ok()?,
        ]),
        _ => None,
    }
}

fn contrast_ratio(first: [u8; 3], second: [u8; 3]) -> f64 {
    let first = relative_luminance(first);
    let second = relative_luminance(second);
    (first.max(second) + 0.05) / (first.min(second) + 0.05)
}

fn relative_luminance(color: [u8; 3]) -> f64 {
    let channel = |value: u8| {
        let value = f64::from(value) / 255.0;
        if value <= 0.040_45 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(color[0]) + 0.7152 * channel(color[1]) + 0.0722 * channel(color[2])
}

fn plugin_manifest_error(message: impl Into<String>) -> AppError {
    AppError::bad_request("plugin.manifest_invalid", message)
}

fn format_install_source(value: PluginInstallSource) -> &'static str {
    match value {
        PluginInstallSource::Bundled => "bundled",
        PluginInstallSource::Local => "local",
        PluginInstallSource::TeamCatalog => "team_catalog",
        PluginInstallSource::CommunityCatalog => "community_catalog",
    }
}

fn parse_install_source(value: &str) -> Result<PluginInstallSource, AppError> {
    match value {
        "bundled" => Ok(PluginInstallSource::Bundled),
        "local" => Ok(PluginInstallSource::Local),
        "team_catalog" => Ok(PluginInstallSource::TeamCatalog),
        "community_catalog" => Ok(PluginInstallSource::CommunityCatalog),
        _ => Err(AppError::internal("plugin install source is invalid")),
    }
}

fn format_trust_level(value: PluginTrustLevel) -> &'static str {
    match value {
        PluginTrustLevel::DataOnly => "data_only",
        PluginTrustLevel::TrustedBundled => "trusted_bundled",
        PluginTrustLevel::SandboxedWeb => "sandboxed_web",
        PluginTrustLevel::SandboxedNode => "sandboxed_node",
        PluginTrustLevel::ExternalService => "external_service",
    }
}

fn parse_trust_level(value: &str) -> Result<PluginTrustLevel, AppError> {
    match value {
        "data_only" => Ok(PluginTrustLevel::DataOnly),
        "trusted_bundled" => Ok(PluginTrustLevel::TrustedBundled),
        "sandboxed_web" => Ok(PluginTrustLevel::SandboxedWeb),
        "sandboxed_node" => Ok(PluginTrustLevel::SandboxedNode),
        "external_service" => Ok(PluginTrustLevel::ExternalService),
        _ => Err(AppError::internal("plugin trust level is invalid")),
    }
}

fn format_desired_state(value: PluginDesiredState) -> &'static str {
    match value {
        PluginDesiredState::Disabled => "disabled",
        PluginDesiredState::Enabled => "enabled",
    }
}

fn parse_desired_state(value: &str) -> Result<PluginDesiredState, AppError> {
    match value {
        "disabled" => Ok(PluginDesiredState::Disabled),
        "enabled" => Ok(PluginDesiredState::Enabled),
        _ => Err(AppError::internal("plugin desired state is invalid")),
    }
}

fn format_effective_state(value: PluginEffectiveState) -> &'static str {
    match value {
        PluginEffectiveState::Disabled => "disabled",
        PluginEffectiveState::Active => "active",
        PluginEffectiveState::Incompatible => "incompatible",
        PluginEffectiveState::Degraded => "degraded",
        PluginEffectiveState::Error => "error",
    }
}

fn parse_effective_state(value: &str) -> Result<PluginEffectiveState, AppError> {
    match value {
        "disabled" => Ok(PluginEffectiveState::Disabled),
        "active" => Ok(PluginEffectiveState::Active),
        "incompatible" => Ok(PluginEffectiveState::Incompatible),
        "degraded" => Ok(PluginEffectiveState::Degraded),
        "error" => Ok(PluginEffectiveState::Error),
        _ => Err(AppError::internal("plugin effective state is invalid")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_theme_manifest_should_validate() {
        let manifest: PluginManifest =
            serde_json::from_str(DARK_THEME_MANIFEST).expect("manifest parses");

        validate_plugin_manifest(&manifest).expect("manifest validates");
    }

    #[test]
    fn markdown_manifest_should_validate() {
        let manifest: PluginManifest =
            serde_json::from_str(MARKDOWN_MANIFEST).expect("manifest parses");

        validate_plugin_manifest(&manifest).expect("manifest validates");
    }

    #[test]
    fn plain_text_manifest_should_validate() {
        let manifest: PluginManifest =
            serde_json::from_str(PLAIN_TEXT_MANIFEST).expect("manifest parses");

        validate_plugin_manifest(&manifest).expect("manifest validates");
    }

    #[test]
    fn visual_renderer_should_reject_duplicate_targets() {
        let mut manifest: PluginManifest =
            serde_json::from_str(MARKDOWN_MANIFEST).expect("manifest parses");
        let PluginContribution::VisualRenderer { contribution, .. } =
            &mut manifest.contributions[0]
        else {
            panic!("fixture contains a visual renderer");
        };
        contribution
            .accepted_source_kinds
            .push("chat.assistant_message".to_owned());

        let error =
            validate_plugin_manifest(&manifest).expect_err("duplicate targets are rejected");

        assert!(matches!(error, AppError::BadRequest { .. }));
    }

    #[test]
    fn visual_renderer_should_require_non_empty_targets() {
        let mut manifest: PluginManifest =
            serde_json::from_str(MARKDOWN_MANIFEST).expect("manifest parses");
        let PluginContribution::VisualRenderer { contribution, .. } =
            &mut manifest.contributions[0]
        else {
            panic!("fixture contains a visual renderer");
        };
        contribution.accepted_source_kinds.clear();

        let error = validate_plugin_manifest(&manifest).expect_err("empty targets are rejected");

        assert!(matches!(error, AppError::BadRequest { .. }));
    }

    #[test]
    fn namespaced_id_should_reject_reserved_characters() {
        let error = validate_namespaced_id("uprava.theme/<script>", "plugin_id")
            .expect_err("identifier is rejected");

        assert!(matches!(error, AppError::BadRequest { .. }));
    }

    #[test]
    fn critical_theme_pairs_should_reject_low_contrast() {
        let mut manifest: PluginManifest =
            serde_json::from_str(DARK_THEME_MANIFEST).expect("manifest parses");
        let PluginContribution::UiTheme { contribution, .. } = &mut manifest.contributions[0]
        else {
            panic!("fixture contains a theme");
        };
        contribution
            .semantic_tokens
            .insert("content.primary".to_owned(), "#111310".to_owned());

        let error = validate_plugin_manifest(&manifest).expect_err("contrast is rejected");

        assert!(matches!(error, AppError::BadRequest { .. }));
    }

    #[test]
    fn v1_manifest_should_reject_non_bundled_or_executable_trust() {
        let mut manifest: PluginManifest =
            serde_json::from_str(DARK_THEME_MANIFEST).expect("manifest parses");
        manifest.trust_level = PluginTrustLevel::SandboxedWeb;

        let error = validate_plugin_manifest(&manifest).expect_err("executable trust is rejected");

        assert!(matches!(error, AppError::BadRequest { .. }));
    }

    #[test]
    fn v1_manifest_should_reject_unknown_permissions() {
        let mut manifest: PluginManifest =
            serde_json::from_str(DARK_THEME_MANIFEST).expect("manifest parses");
        manifest
            .requested_permissions
            .push("workspace.write".to_owned());

        let error = validate_plugin_manifest(&manifest).expect_err("permission is rejected");

        assert!(matches!(error, AppError::BadRequest { .. }));
    }
}
