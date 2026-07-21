use uprava_protocol::{
    CreateDynamicUiProposalRequest, ExecuteToolRequest, GeneratedUiActionDefinition,
    GeneratedUiActionKind, GeneratedUiBuildState, GeneratedUiCapability, GeneratedUiLayoutIntent,
    InvokeGeneratedUiActionRequest, PluginDesiredState, PluginId, ToolCallState, ToolId,
    ToolRiskLevel, ToolScope,
};

use super::*;

async fn test_state_without_builder() -> Arc<AppState> {
    let mut config = test_config(86_400);
    config.generated_ui_builder_url = "http://127.0.0.1:1".to_owned();
    config.generated_ui_builder_timeout_seconds = 1;
    AppState::new(config, memory_pool().await)
        .await
        .expect("state migrates")
}

async fn test_state_with_builder(url: String) -> Arc<AppState> {
    let mut config = test_config(86_400);
    config.generated_ui_builder_url = url;
    config.generated_ui_builder_timeout_seconds = 2;
    AppState::new(config, memory_pool().await)
        .await
        .expect("state migrates")
}

fn proposal(session_thread_id: SessionThreadId) -> CreateDynamicUiProposalRequest {
    CreateDynamicUiProposalRequest {
        title: "Interactive counter".to_owned(),
        description: Some("A generated counter fixture".to_owned()),
        scope_ref: ScopeRef::Session {
            session_thread_id: session_thread_id.clone(),
        },
        runtime_id: "uprava.generated-react".to_owned(),
        sdk_version: "1.0.0".to_owned(),
        layout_intent: GeneratedUiLayoutIntent::Inline,
        source: "import React from 'react'; import { Card, Text } from '@uprava/ui-sdk'; export default function Counter(){ return <Card><Text>Counter</Text></Card>; }".to_owned(),
        data_model: json!({ "count": 0 }).into(),
        actions: vec![GeneratedUiActionDefinition {
            action_id: "counter.save".to_owned(),
            kind: GeneratedUiActionKind::UpdateArtifactState,
            label: "Save counter".to_owned(),
            input_schema: json!({ "type": "object" }).into(),
            required_capabilities: vec![GeneratedUiCapability::PersistState],
            confirmation_required: false,
        }],
        requested_capabilities: vec![GeneratedUiCapability::PersistState],
        fallback_markdown: "Counter: 0".to_owned(),
        fallback_snapshot: None,
        source_refs: vec![UpravaRef::Session { session_thread_id }],
        evidence_refs: vec![],
        cause_refs: vec![],
        trace_refs: vec![],
    }
}

async fn enable_generated_react(state: &AppState) {
    set_plugin_desired_state(
        state,
        &PluginId::from("uprava.generated-react"),
        PluginDesiredState::Enabled,
    )
    .await
    .expect("generated React plugin enables");
}

#[tokio::test]
async fn generated_ui_is_opt_in_and_rejects_unsafe_source() {
    let state = test_state_without_builder().await;
    let (_node_id, session, workspace_path) = create_test_session(&state).await;
    let request = proposal(session.session.session_thread_id.clone());

    let disabled = create_dynamic_ui_proposal(&state, request.clone())
        .await
        .expect_err("disabled runtime rejects proposal");
    enable_generated_react(&state).await;
    let mut unsafe_request = request;
    unsafe_request.source = "export default function Bad(){ eval('1'); return null; }".to_owned();
    let unsafe_error = create_dynamic_ui_proposal(&state, unsafe_request)
        .await
        .expect_err("unsafe source rejects");

    let _ = std::fs::remove_dir_all(workspace_path);
    assert!(matches!(
        disabled,
        AppError::BadRequest {
            code: "generated_ui.runtime_unavailable",
            ..
        }
    ));
    assert!(matches!(
        unsafe_error,
        AppError::BadRequest {
            code: "generated_ui.source_unsafe",
            ..
        }
    ));
}

#[tokio::test]
async fn build_failure_keeps_fallback_and_actions_are_idempotent() {
    let state = test_state_without_builder().await;
    let (_node_id, session, workspace_path) = create_test_session(&state).await;
    enable_generated_react(&state).await;

    let detail =
        create_dynamic_ui_proposal(&state, proposal(session.session.session_thread_id.clone()))
            .await
            .expect("proposal remains readable when builder is unavailable");
    assert_eq!(detail.build.state, GeneratedUiBuildState::Failed);
    assert_eq!(detail.artifact.version.fallback_text, "Counter: 0");

    let request = InvokeGeneratedUiActionRequest {
        artifact_version: 1,
        idempotency_key: "counter-save-1".to_owned(),
        input: json!({
            "expected_revision": 0,
            "values": { "count": 1 }
        })
        .into(),
        confirmed: false,
    };
    let first = invoke_generated_ui_action(
        &state,
        &detail.artifact.artifact.artifact_id,
        "counter.save",
        request.clone(),
    )
    .await
    .expect("state action completes");
    let replay = invoke_generated_ui_action(
        &state,
        &detail.artifact.artifact.artifact_id,
        "counter.save",
        request,
    )
    .await
    .expect("idempotent replay returns stored result");

    let _ = std::fs::remove_dir_all(workspace_path);
    assert_eq!(first.action_request_id, replay.action_request_id);
    assert_eq!(first.state.expect("state returns").revision, 1);
}

#[tokio::test]
async fn successful_build_is_content_addressed_and_ready() {
    let builder = axum::Router::new().route(
        "/build",
        axum::routing::post(|| async {
            Json(json!({
                "bundle": "globalThis.__upravaFixture = true;",
                "dependency_lock": {
                    "runtime_id": "uprava.generated-react",
                    "runtime_version": "1.0.0",
                    "sdk_version": "1.0.0"
                },
                "diagnostics": []
            }))
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("builder listener binds");
    let address = listener.local_addr().expect("builder address loads");
    let server = tokio::spawn(async move {
        axum::serve(listener, builder)
            .await
            .expect("builder server runs");
    });
    let state = test_state_with_builder(format!("http://{address}")).await;
    let (_node_id, session, workspace_path) = create_test_session(&state).await;
    enable_generated_react(&state).await;

    let detail =
        create_dynamic_ui_proposal(&state, proposal(session.session.session_thread_id.clone()))
            .await
            .expect("proposal builds");
    let bundle_hash = detail
        .build
        .bundle_blob_hash
        .clone()
        .expect("bundle hash is recorded");
    let stored: Vec<u8> =
        sqlx::query_scalar("select content from artifact_blobs where blob_hash = ?1")
            .bind(&bundle_hash)
            .fetch_one(&state.pool)
            .await
            .expect("bundle blob loads");

    server.abort();
    let _ = std::fs::remove_dir_all(workspace_path);
    assert_eq!(detail.build.state, GeneratedUiBuildState::Ready);
    assert!(bundle_hash.starts_with("sha256:"));
    assert_eq!(stored, b"globalThis.__upravaFixture = true;");
}

#[tokio::test]
async fn native_tool_creates_only_a_session_scoped_generated_ui_artifact() {
    let state = test_state_without_builder().await;
    let (node_id, session, workspace_path) = create_test_session(&state).await;
    let scope = ToolScope {
        actor_ref: ActorRef::Provider {
            provider: "codex".to_owned(),
        },
        node_id: Some(node_id),
        project_id: session.placement.project_id.clone(),
        project_placement_id: Some(session.placement.project_placement_id.clone()),
        session_thread_id: Some(session.session.session_thread_id.clone()),
    };
    let capabilities = execute_tool(
        &state,
        &ExecuteToolRequest {
            scope: scope.clone(),
            tool_id: ToolId::from("uprava.dynamic_ui.inspect"),
            arguments: json!({}).into(),
        },
    )
    .await
    .expect("generated UI capabilities inspect");
    assert_eq!(capabilities.state, ToolCallState::Completed);
    let capability_content = &capabilities.result.expect("capability result").content.0;
    assert_eq!(capability_content.get("enabled"), Some(&json!(false)));
    assert!(capability_content
        .pointer("/sdk/api_schema/hooks/usePersistedState")
        .is_some());

    enable_generated_react(&state).await;
    let inspected = inspect_tool(
        &state,
        &uprava_protocol::InspectToolRequest {
            scope: scope.clone(),
            tool_id: ToolId::from("uprava.dynamic_ui.create"),
        },
    )
    .await
    .expect("generated UI tool inspects");
    assert_eq!(
        inspected.definition.risk_level,
        ToolRiskLevel::WorkspaceWrite
    );

    let executed = execute_tool(
        &state,
        &ExecuteToolRequest {
            scope,
            tool_id: ToolId::from("uprava.dynamic_ui.create"),
            arguments: json!({
                "title": "Agent widget",
                "runtime_id": "uprava.generated-react",
                "sdk_version": "1.0.0",
                "layout_intent": "inline",
                "source": "export default function Widget(){ return null; }",
                "data_model": {},
                "fallback_markdown": "Agent widget fallback"
            })
            .into(),
        },
    )
    .await
    .expect("generated UI tool executes");

    let _ = std::fs::remove_dir_all(workspace_path);
    assert_eq!(executed.state, ToolCallState::Completed);
    let content = &executed.result.expect("tool result").content.0;
    assert_eq!(
        content.pointer("/artifact/artifact/scope_ref/kind"),
        Some(&json!("session"))
    );
    assert_eq!(
        content.pointer("/artifact/artifact/scope_ref/session_thread_id"),
        Some(&json!(session.session.session_thread_id))
    );
    assert_eq!(
        content.pointer("/artifact/artifact/created_by"),
        Some(&json!({ "kind": "provider", "provider": "codex" }))
    );
}
