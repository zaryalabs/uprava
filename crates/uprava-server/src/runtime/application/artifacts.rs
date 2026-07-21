//! Generic artifact identity, versioning and fallback read models.

use uprava_protocol::{
    ArtifactDetail, ArtifactId, ArtifactListResponse, ArtifactState, ArtifactSummary,
    ArtifactVersion, CreateArtifactRequest, CreateArtifactVersionRequest,
    EffectiveContributionState, PluginContribution, PluginId, ProjectPlacementId, ScopeRef,
    SessionThreadId,
};

use super::super::*;

const MAX_ARTIFACT_TITLE_CHARS: usize = 240;
const MAX_ARTIFACT_FALLBACK_CHARS: usize = 65_536;
const MAX_ARTIFACT_PAYLOAD_BYTES: usize = 512 * 1024;
const MAX_ARTIFACT_PROVENANCE_BYTES: usize = 64 * 1024;
const MAX_ARTIFACT_REFS_BYTES: usize = 256 * 1024;
const MAX_ARTIFACT_REFS: usize = 256;
const MAX_ARTIFACT_SOURCE_VERSION_CHARS: usize = 1_024;
const MAX_ARTIFACT_JSON_DEPTH: usize = 32;
const MAX_ARTIFACT_JSON_STRING_CHARS: usize = 131_072;

#[derive(Debug, Deserialize)]
pub(crate) struct ArtifactListQuery {
    pub(crate) session_thread_id: Option<String>,
    pub(crate) project_placement_id: Option<String>,
    pub(crate) artifact_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ArtifactDetailQuery {
    version: Option<u64>,
}

pub(crate) async fn artifact_list_route(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ArtifactListQuery>,
) -> Result<Json<ArtifactListResponse>, AppError> {
    Ok(Json(list_artifacts(&state, query).await?))
}

pub(crate) async fn artifact_detail_route(
    State(state): State<Arc<AppState>>,
    Path(artifact_id): Path<String>,
    Query(query): Query<ArtifactDetailQuery>,
) -> Result<Json<ArtifactDetail>, AppError> {
    Ok(Json(
        load_artifact_detail(&state, &ArtifactId::from(artifact_id), query.version).await?,
    ))
}

pub(crate) async fn create_artifact_route(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateArtifactRequest>,
) -> Result<Json<ArtifactDetail>, AppError> {
    Ok(Json(create_artifact(&state, request).await?))
}

pub(crate) async fn create_artifact_version_route(
    State(state): State<Arc<AppState>>,
    Path(artifact_id): Path<String>,
    Json(request): Json<CreateArtifactVersionRequest>,
) -> Result<Json<ArtifactDetail>, AppError> {
    Ok(Json(
        create_artifact_version(&state, &ArtifactId::from(artifact_id), request).await?,
    ))
}

pub(crate) async fn list_artifacts(
    state: &AppState,
    query: ArtifactListQuery,
) -> Result<ArtifactListResponse, AppError> {
    if query.session_thread_id.is_some() && query.project_placement_id.is_some() {
        return Err(AppError::bad_request(
            "artifact.scope_ambiguous",
            "Filter artifacts by a session or a placement, not both",
        ));
    }
    let scope_json = match (query.session_thread_id, query.project_placement_id) {
        (Some(session_thread_id), None) => Some(serde_json::to_string(&ScopeRef::Session {
            session_thread_id: SessionThreadId::from(session_thread_id),
        })?),
        (None, Some(project_placement_id)) => Some(serde_json::to_string(&ScopeRef::Placement {
            project_placement_id: ProjectPlacementId::from(project_placement_id),
        })?),
        (None, None) => None,
        (Some(_), Some(_)) => {
            return Err(AppError::bad_request(
                "artifact.scope_ambiguous",
                "Filter artifacts by a session or a placement, not both",
            ));
        }
    };
    let rows = sqlx::query(
        r#"
        select artifact_id, artifact_type, title, scope_ref_json, owner_plugin_id,
               current_version, state, created_by_json, created_at, updated_at
        from artifacts
        where (?1 is null or scope_ref_json = ?1)
          and (?2 is null or artifact_type = ?2)
        order by updated_at desc, artifact_id
        limit 500
        "#,
    )
    .bind(scope_json)
    .bind(query.artifact_type)
    .fetch_all(&state.pool)
    .await?;
    let items = rows
        .iter()
        .map(artifact_summary_from_row)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ArtifactListResponse { items })
}

pub(crate) async fn create_artifact(
    state: &AppState,
    request: CreateArtifactRequest,
) -> Result<ArtifactDetail, AppError> {
    validate_create_artifact_request(&request)?;
    validate_artifact_scope(state, &request.scope_ref).await?;
    let (owner_plugin_id, declared_schema_version) =
        resolve_active_artifact_type(state, &request.artifact_type).await?;
    if request.schema_version != declared_schema_version {
        return Err(AppError::bad_request(
            "artifact.schema_version_unsupported",
            "Artifact payload does not match the active artifact type schema version",
        ));
    }
    let artifact_id = ArtifactId::new();
    let now = Utc::now();
    let actor = ActorRef::local_user();
    let mut transaction = state.pool.begin().await?;
    sqlx::query(
        r#"
        insert into artifacts (
            artifact_id, artifact_type, title, scope_ref_json, owner_plugin_id,
            current_version, state, created_by_json, created_at, updated_at
        ) values (?1, ?2, ?3, ?4, ?5, 1, 'active', ?6, ?7, ?7)
        "#,
    )
    .bind(artifact_id.as_str())
    .bind(&request.artifact_type)
    .bind(&request.title)
    .bind(serde_json::to_string(&request.scope_ref)?)
    .bind(owner_plugin_id.as_str())
    .bind(serde_json::to_string(&actor)?)
    .bind(now)
    .execute(&mut *transaction)
    .await?;
    insert_artifact_version(
        &mut transaction,
        &artifact_id,
        1,
        request.schema_version,
        &request.payload,
        &request.fallback_text,
        request.source_version.as_deref(),
        &request.source_refs,
        &request.evidence_refs,
        &request.cause_refs,
        &request.trace_refs,
        &request.provenance,
        now,
    )
    .await?;
    transaction.commit().await?;
    load_artifact_detail(state, &artifact_id, Some(1)).await
}

pub(crate) async fn create_artifact_version(
    state: &AppState,
    artifact_id: &ArtifactId,
    request: CreateArtifactVersionRequest,
) -> Result<ArtifactDetail, AppError> {
    validate_artifact_version_request(&request)?;
    let current = load_artifact_detail(state, artifact_id, None).await?;
    if current.artifact.current_version != request.expected_current_version {
        return Err(AppError::conflict(
            "artifact.version_conflict",
            "Artifact changed; reload it before adding a version",
        ));
    }
    let (active_owner_plugin_id, declared_schema_version) =
        resolve_active_artifact_type(state, &current.artifact.artifact_type).await?;
    if active_owner_plugin_id != current.artifact.owner_plugin_id {
        return Err(AppError::conflict(
            "artifact.type_owner_changed",
            "The active artifact type provider changed; preserve the stored version and create a new artifact",
        ));
    }
    if request.schema_version != declared_schema_version {
        return Err(AppError::bad_request(
            "artifact.schema_version_unsupported",
            "Artifact payload does not match the active artifact type schema version",
        ));
    }
    let version = request
        .expected_current_version
        .checked_add(1)
        .ok_or_else(|| {
            AppError::bad_request("artifact.version_invalid", "Artifact version is exhausted")
        })?;
    let now = Utc::now();
    let mut transaction = state.pool.begin().await?;
    let updated = sqlx::query(
        r#"
        update artifacts
        set current_version = ?1, state = 'active', updated_at = ?2
        where artifact_id = ?3 and current_version = ?4
        "#,
    )
    .bind(i64::try_from(version).map_err(|_| artifact_version_invalid())?)
    .bind(now)
    .bind(artifact_id.as_str())
    .bind(i64::try_from(request.expected_current_version).map_err(|_| artifact_version_invalid())?)
    .execute(&mut *transaction)
    .await?;
    if updated.rows_affected() != 1 {
        return Err(AppError::conflict(
            "artifact.version_conflict",
            "Artifact changed; reload it before adding a version",
        ));
    }
    insert_artifact_version(
        &mut transaction,
        artifact_id,
        version,
        request.schema_version,
        &request.payload,
        &request.fallback_text,
        request.source_version.as_deref(),
        &request.source_refs,
        &request.evidence_refs,
        &request.cause_refs,
        &request.trace_refs,
        &request.provenance,
        now,
    )
    .await?;
    transaction.commit().await?;
    load_artifact_detail(state, artifact_id, Some(version)).await
}

#[expect(
    clippy::too_many_arguments,
    reason = "the immutable artifact version mirrors its wire contract"
)]
async fn insert_artifact_version(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    artifact_id: &ArtifactId,
    version: u64,
    schema_version: u16,
    payload: &JsonValue,
    fallback_text: &str,
    source_version: Option<&str>,
    source_refs: &[UpravaRef],
    evidence_refs: &[UpravaRef],
    cause_refs: &[UpravaRef],
    trace_refs: &[UpravaRef],
    provenance: &JsonValue,
    created_at: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        insert into artifact_versions (
            artifact_id, version, schema_version, payload_json, fallback_text,
            source_version, source_refs_json, evidence_refs_json, cause_refs_json,
            trace_refs_json, provenance_json, created_at
        ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
        "#,
    )
    .bind(artifact_id.as_str())
    .bind(i64::try_from(version).map_err(|_| artifact_version_invalid())?)
    .bind(i64::from(schema_version))
    .bind(serde_json::to_string(payload)?)
    .bind(fallback_text)
    .bind(source_version)
    .bind(serde_json::to_string(source_refs)?)
    .bind(serde_json::to_string(evidence_refs)?)
    .bind(serde_json::to_string(cause_refs)?)
    .bind(serde_json::to_string(trace_refs)?)
    .bind(serde_json::to_string(provenance)?)
    .bind(created_at)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

pub(crate) async fn load_artifact_detail(
    state: &AppState,
    artifact_id: &ArtifactId,
    version: Option<u64>,
) -> Result<ArtifactDetail, AppError> {
    let artifact_row = sqlx::query(
        r#"
        select artifact_id, artifact_type, title, scope_ref_json, owner_plugin_id,
               current_version, state, created_by_json, created_at, updated_at
        from artifacts where artifact_id = ?1
        "#,
    )
    .bind(artifact_id.as_str())
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::not_found("artifact.not_found", "Artifact not found"))?;
    let artifact = artifact_summary_from_row(&artifact_row)?;
    let selected_version = version.unwrap_or(artifact.current_version);
    let version_row = sqlx::query(
        r#"
        select artifact_id, version, schema_version, payload_json, fallback_text,
               source_version, source_refs_json, evidence_refs_json, cause_refs_json,
               trace_refs_json, provenance_json, created_at
        from artifact_versions where artifact_id = ?1 and version = ?2
        "#,
    )
    .bind(artifact_id.as_str())
    .bind(i64::try_from(selected_version).map_err(|_| artifact_version_invalid())?)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| {
        AppError::not_found("artifact.version_not_found", "Artifact version not found")
    })?;
    Ok(ArtifactDetail {
        artifact,
        version: artifact_version_from_row(&version_row)?,
    })
}

fn artifact_summary_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<ArtifactSummary, AppError> {
    Ok(ArtifactSummary {
        artifact_id: ArtifactId::from(row.try_get::<String, _>("artifact_id")?),
        artifact_type: row.try_get("artifact_type")?,
        title: row.try_get("title")?,
        scope_ref: serde_json::from_str(&row.try_get::<String, _>("scope_ref_json")?)?,
        owner_plugin_id: PluginId::from(row.try_get::<String, _>("owner_plugin_id")?),
        current_version: u64::try_from(row.try_get::<i64, _>("current_version")?)
            .map_err(|_| artifact_version_invalid())?,
        state: parse_artifact_state(&row.try_get::<String, _>("state")?)?,
        created_by: serde_json::from_str(&row.try_get::<String, _>("created_by_json")?)?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn artifact_version_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<ArtifactVersion, AppError> {
    Ok(ArtifactVersion {
        artifact_id: ArtifactId::from(row.try_get::<String, _>("artifact_id")?),
        version: u64::try_from(row.try_get::<i64, _>("version")?)
            .map_err(|_| artifact_version_invalid())?,
        schema_version: u16::try_from(row.try_get::<i64, _>("schema_version")?)
            .map_err(|_| AppError::internal("artifact schema version is invalid"))?,
        payload: serde_json::from_str(&row.try_get::<String, _>("payload_json")?)?,
        fallback_text: row.try_get("fallback_text")?,
        source_version: row.try_get("source_version")?,
        source_refs: serde_json::from_str(&row.try_get::<String, _>("source_refs_json")?)?,
        evidence_refs: serde_json::from_str(&row.try_get::<String, _>("evidence_refs_json")?)?,
        cause_refs: serde_json::from_str(&row.try_get::<String, _>("cause_refs_json")?)?,
        trace_refs: serde_json::from_str(&row.try_get::<String, _>("trace_refs_json")?)?,
        provenance: serde_json::from_str(&row.try_get::<String, _>("provenance_json")?)?,
        created_at: row.try_get("created_at")?,
    })
}

pub(crate) async fn resolve_active_artifact_type(
    state: &AppState,
    artifact_type: &str,
) -> Result<(PluginId, u16), AppError> {
    let snapshot = effective_plugin_snapshot(state).await?;
    snapshot
        .contributions
        .into_iter()
        .find_map(|effective| {
            let PluginContribution::ArtifactType { contribution, .. } = effective.contribution
            else {
                return None;
            };
            (effective.effective_state == EffectiveContributionState::Available
                && contribution.artifact_type_id == artifact_type)
                .then_some((effective.plugin_id, contribution.schema_version))
        })
        .ok_or_else(|| {
            AppError::bad_request(
                "artifact.type_unavailable",
                "Artifact type is not provided by an active compatible plugin",
            )
        })
}

async fn validate_artifact_scope(state: &AppState, scope: &ScopeRef) -> Result<(), AppError> {
    let exists = match scope {
        ScopeRef::Session { session_thread_id } => {
            sqlx::query_scalar::<_, i64>(
                "select count(*) from session_threads where session_thread_id = ?1",
            )
            .bind(session_thread_id.as_str())
            .fetch_one(&state.pool)
            .await?
                > 0
        }
        ScopeRef::Placement {
            project_placement_id,
        } => {
            sqlx::query_scalar::<_, i64>(
                "select count(*) from project_placements where project_placement_id = ?1",
            )
            .bind(project_placement_id.as_str())
            .fetch_one(&state.pool)
            .await?
                > 0
        }
        _ => {
            return Err(AppError::bad_request(
                "artifact.scope_unsupported",
                "Artifacts currently require a session or placement scope",
            ));
        }
    };
    if !exists {
        return Err(AppError::not_found(
            "artifact.scope_not_found",
            "Artifact scope not found",
        ));
    }
    Ok(())
}

fn validate_create_artifact_request(request: &CreateArtifactRequest) -> Result<(), AppError> {
    validate_artifact_metadata(
        &request.artifact_type,
        &request.title,
        request.schema_version,
        &request.payload,
        &request.fallback_text,
        [
            request.source_refs.len(),
            request.evidence_refs.len(),
            request.cause_refs.len(),
            request.trace_refs.len(),
        ],
    )?;
    validate_artifact_auxiliary(
        request.source_version.as_deref(),
        &request.provenance,
        [
            &request.source_refs,
            &request.evidence_refs,
            &request.cause_refs,
            &request.trace_refs,
        ],
    )
}

fn validate_artifact_version_request(
    request: &CreateArtifactVersionRequest,
) -> Result<(), AppError> {
    validate_artifact_metadata(
        "artifact.version",
        "artifact version",
        request.schema_version,
        &request.payload,
        &request.fallback_text,
        [
            request.source_refs.len(),
            request.evidence_refs.len(),
            request.cause_refs.len(),
            request.trace_refs.len(),
        ],
    )?;
    validate_artifact_auxiliary(
        request.source_version.as_deref(),
        &request.provenance,
        [
            &request.source_refs,
            &request.evidence_refs,
            &request.cause_refs,
            &request.trace_refs,
        ],
    )
}

fn validate_artifact_auxiliary(
    source_version: Option<&str>,
    provenance: &JsonValue,
    refs: [&[UpravaRef]; 4],
) -> Result<(), AppError> {
    if source_version.is_some_and(|value| {
        value.is_empty() || value.chars().count() > MAX_ARTIFACT_SOURCE_VERSION_CHARS
    }) {
        return Err(AppError::bad_request(
            "artifact.source_version_invalid",
            "Artifact source version is empty or oversized",
        ));
    }
    if serde_json::to_vec(provenance)?.len() > MAX_ARTIFACT_PROVENANCE_BYTES {
        return Err(AppError::bad_request(
            "artifact.provenance_too_large",
            "Artifact provenance exceeds the bounded storage limit",
        ));
    }
    validate_artifact_json(&provenance.0, 0)?;
    if serde_json::to_vec(&refs)?.len() > MAX_ARTIFACT_REFS_BYTES {
        return Err(AppError::bad_request(
            "artifact.refs_too_large",
            "Artifact references exceed the bounded storage limit",
        ));
    }
    Ok(())
}

fn validate_artifact_metadata(
    artifact_type: &str,
    title: &str,
    schema_version: u16,
    payload: &JsonValue,
    fallback_text: &str,
    ref_counts: [usize; 4],
) -> Result<(), AppError> {
    if artifact_type.is_empty()
        || artifact_type.len() > 128
        || !artifact_type.contains('.')
        || title.is_empty()
        || title.chars().count() > MAX_ARTIFACT_TITLE_CHARS
        || schema_version == 0
        || fallback_text.chars().count() > MAX_ARTIFACT_FALLBACK_CHARS
        || ref_counts.into_iter().sum::<usize>() > MAX_ARTIFACT_REFS
    {
        return Err(AppError::bad_request(
            "artifact.request_invalid",
            "Artifact metadata is empty or exceeds bounded limits",
        ));
    }
    let payload_json = serde_json::to_vec(payload)?;
    if payload_json.len() > MAX_ARTIFACT_PAYLOAD_BYTES {
        return Err(AppError::bad_request(
            "artifact.payload_too_large",
            "Artifact payload exceeds the bounded storage limit",
        ));
    }
    validate_artifact_json(&payload.0, 0)
}

fn validate_artifact_json(value: &serde_json::Value, depth: usize) -> Result<(), AppError> {
    if depth > MAX_ARTIFACT_JSON_DEPTH {
        return Err(AppError::bad_request(
            "artifact.payload_too_deep",
            "Artifact payload exceeds the bounded nesting limit",
        ));
    }
    match value {
        serde_json::Value::String(value)
            if value.chars().count() > MAX_ARTIFACT_JSON_STRING_CHARS =>
        {
            Err(AppError::bad_request(
                "artifact.payload_string_too_large",
                "Artifact payload contains an oversized string",
            ))
        }
        serde_json::Value::Array(values) => {
            for value in values {
                validate_artifact_json(value, depth + 1)?;
            }
            Ok(())
        }
        serde_json::Value::Object(values) => {
            for value in values.values() {
                validate_artifact_json(value, depth + 1)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn parse_artifact_state(value: &str) -> Result<ArtifactState, AppError> {
    match value {
        "active" => Ok(ArtifactState::Active),
        "stale" => Ok(ArtifactState::Stale),
        "archived" => Ok(ArtifactState::Archived),
        _ => Err(AppError::internal("artifact state is invalid")),
    }
}

fn artifact_version_invalid() -> AppError {
    AppError::internal("artifact version is invalid")
}
