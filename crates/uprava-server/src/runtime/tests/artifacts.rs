use uprava_protocol::{CreateArtifactRequest, CreateArtifactVersionRequest, PluginId};

use super::*;

fn diagram_request(session_thread_id: SessionThreadId) -> CreateArtifactRequest {
    CreateArtifactRequest {
        artifact_type: "uprava.diagram".to_owned(),
        title: "Runtime diagram".to_owned(),
        scope_ref: ScopeRef::Session {
            session_thread_id: session_thread_id.clone(),
        },
        schema_version: 1,
        payload: json!({
            "language": "mermaid",
            "source": "flowchart LR\nCore --> Node",
        })
        .into(),
        fallback_text: "flowchart LR\nCore --> Node".to_owned(),
        source_version: Some("message:v1".to_owned()),
        source_refs: vec![UpravaRef::Session { session_thread_id }],
        evidence_refs: Vec::new(),
        cause_refs: Vec::new(),
        trace_refs: Vec::new(),
        provenance: json!({ "kind": "test_visual_promotion" }).into(),
    }
}

#[tokio::test]
async fn generic_artifacts_are_versioned_listable_and_keep_old_versions() {
    let state = test_state().await;
    let (_node_id, session, workspace_path) = create_test_session(&state).await;
    let created = create_artifact(
        &state,
        diagram_request(session.session.session_thread_id.clone()),
    )
    .await
    .expect("artifact creates");

    assert_eq!(
        created.artifact.owner_plugin_id,
        PluginId::from("uprava.diagrams")
    );
    assert_eq!(created.artifact.current_version, 1);
    assert_eq!(created.version.source_refs.len(), 1);

    let updated = create_artifact_version(
        &state,
        &created.artifact.artifact_id,
        CreateArtifactVersionRequest {
            expected_current_version: 1,
            schema_version: 1,
            payload: json!({
                "language": "mermaid",
                "source": "flowchart LR\nCore --> Node --> Tool",
            })
            .into(),
            fallback_text: "flowchart LR\nCore --> Node --> Tool".to_owned(),
            source_version: Some("message:v2".to_owned()),
            source_refs: created.version.source_refs.clone(),
            evidence_refs: Vec::new(),
            cause_refs: Vec::new(),
            trace_refs: Vec::new(),
            provenance: json!({ "kind": "test_visual_update" }).into(),
        },
    )
    .await
    .expect("artifact version creates");
    let conflict = create_artifact_version(
        &state,
        &created.artifact.artifact_id,
        CreateArtifactVersionRequest {
            expected_current_version: 1,
            schema_version: 1,
            payload: created.version.payload.clone(),
            fallback_text: created.version.fallback_text.clone(),
            source_version: None,
            source_refs: Vec::new(),
            evidence_refs: Vec::new(),
            cause_refs: Vec::new(),
            trace_refs: Vec::new(),
            provenance: json!({ "kind": "stale_update" }).into(),
        },
    )
    .await
    .expect_err("stale artifact update conflicts");
    let original = load_artifact_detail(&state, &created.artifact.artifact_id, Some(1))
        .await
        .expect("old artifact version remains readable");
    let listed = list_artifacts(
        &state,
        ArtifactListQuery {
            session_thread_id: Some(session.session.session_thread_id.to_string()),
            project_placement_id: None,
            artifact_type: Some("uprava.diagram".to_owned()),
        },
    )
    .await
    .expect("artifacts list");

    std::fs::remove_dir_all(workspace_path).expect("workspace dir removes");
    assert_eq!(updated.artifact.current_version, 2);
    assert!(matches!(
        conflict,
        AppError::Conflict {
            code: "artifact.version_conflict",
            ..
        }
    ));
    assert_eq!(updated.version.version, 2);
    assert_eq!(original.version.version, 1);
    assert_eq!(listed.items.len(), 1);
}

#[tokio::test]
async fn disabled_type_blocks_mutation_but_preserves_fallback_reads() {
    let state = test_state().await;
    let (_node_id, session, workspace_path) = create_test_session(&state).await;
    let created = create_artifact(
        &state,
        diagram_request(session.session.session_thread_id.clone()),
    )
    .await
    .expect("artifact creates");
    set_plugin_desired_state(
        &state,
        &PluginId::from("uprava.diagrams"),
        uprava_protocol::PluginDesiredState::Disabled,
    )
    .await
    .expect("diagram plugin disables");

    let readable = load_artifact_detail(&state, &created.artifact.artifact_id, None)
        .await
        .expect("fallback remains readable");
    let error = create_artifact_version(
        &state,
        &created.artifact.artifact_id,
        CreateArtifactVersionRequest {
            expected_current_version: 1,
            schema_version: 1,
            payload: created.version.payload.clone(),
            fallback_text: created.version.fallback_text.clone(),
            source_version: None,
            source_refs: Vec::new(),
            evidence_refs: Vec::new(),
            cause_refs: Vec::new(),
            trace_refs: Vec::new(),
            provenance: json!({ "kind": "disabled_update" }).into(),
        },
    )
    .await
    .expect_err("disabled artifact type rejects mutation");

    std::fs::remove_dir_all(workspace_path).expect("workspace dir removes");
    assert_eq!(
        readable.version.fallback_text,
        created.version.fallback_text
    );
    assert!(matches!(
        error,
        AppError::BadRequest {
            code: "artifact.type_unavailable",
            ..
        }
    ));
}

#[tokio::test]
async fn artifact_creation_rejects_types_without_active_contributions() {
    let state = test_state().await;
    let (_node_id, session, workspace_path) = create_test_session(&state).await;
    let mut request = diagram_request(session.session.session_thread_id);
    request.artifact_type = "example.unknown".to_owned();

    let error = create_artifact(&state, request)
        .await
        .expect_err("unknown artifact type rejects");

    std::fs::remove_dir_all(workspace_path).expect("workspace dir removes");
    assert!(matches!(
        error,
        AppError::BadRequest {
            code: "artifact.type_unavailable",
            ..
        }
    ));
}
