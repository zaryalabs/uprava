use super::*;

#[tokio::test]
async fn create_session_defaults_to_exec_compatibility_with_hashed_policy() {
    let state = test_state().await;
    let (_node_id, detail, workspace_path) = create_test_session(&state).await;

    let policy = detail
        .session
        .runtime
        .effective_policy
        .as_ref()
        .expect("effective policy is projected");
    let stored_hash = detail
        .session
        .runtime
        .effective_policy_hash
        .as_ref()
        .expect("effective policy hash is projected");

    assert_eq!(
        (
            detail.session.runtime.execution_profile,
            policy.sandbox_mode,
            policy.approval_mode,
            policy.policy_hash().expect("policy hashes"),
        ),
        (
            AgentExecutionProfile::ExecCompatibility,
            ProviderSandboxMode::DangerFullAccess,
            ProviderApprovalMode::Never,
            stored_hash.clone(),
        )
    );
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");
}

#[tokio::test]
async fn runtime_session_effective_policy_is_database_immutable() {
    let state = test_state().await;
    let (_node_id, detail, workspace_path) = create_test_session(&state).await;

    let error = sqlx::query(
        "update runtime_sessions set effective_policy_hash = 'sha256:tampered' where runtime_session_id = ?1",
    )
    .bind(detail.session.runtime.runtime_session_id.as_str())
    .execute(&state.pool)
    .await
    .expect_err("effective policy mutation must fail");

    assert!(error.to_string().contains("effective policy are immutable"));
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");
}

#[tokio::test]
async fn create_managed_session_rejects_missing_capabilities_without_fallback() {
    let state = test_state().await;
    let (_node_id, detail, workspace_path) = create_test_session(&state).await;
    let before = command_count(&state).await;

    let result = create_session(
        State(state.clone()),
        Json(CreateSessionRequest {
            project_placement_id: detail.placement.project_placement_id,
            title: Some("Managed session".to_owned()),
            provider: "codex".to_owned(),
            execution_profile: Some(AgentExecutionProfile::Managed),
            force: false,
        }),
    )
    .await;

    assert!(matches!(
        result,
        Err(AppError::BadRequest {
            code: "runtime.profile_capability_unavailable",
            ..
        })
    ));
    assert_eq!(command_count(&state).await, before);
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");
}

#[tokio::test]
async fn create_managed_session_persists_safe_policy_and_typed_start_command() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    let capabilities = std::iter::once(CapabilitySummary {
        key: "provider.codex".to_owned(),
        value: CapabilityValue::provider(true),
    })
    .chain(
        ProviderRuntimeCapability::required_for_managed_codex()
            .iter()
            .map(|capability| CapabilitySummary {
                key: capability.as_str().to_owned(),
                value: CapabilityValue::Provider {
                    available: true,
                    configured: true,
                    mode: "managed".to_owned(),
                    timeout_seconds: None,
                    unavailable_reason: None,
                },
            }),
    )
    .collect();
    set_node_capabilities(&state, &node_id, capabilities).await;

    let managed = create_session(
        State(state.clone()),
        Json(CreateSessionRequest {
            project_placement_id: detail.placement.project_placement_id,
            title: Some("Managed session".to_owned()),
            provider: "codex".to_owned(),
            execution_profile: Some(AgentExecutionProfile::Managed),
            force: false,
        }),
    )
    .await
    .expect("managed session creates when capabilities are proven")
    .0;
    let command_json: String = sqlx::query_scalar(
        "select command_json from commands where runtime_session_id = ?1 and kind = 'StartRuntime'",
    )
    .bind(managed.session.runtime.runtime_session_id.as_str())
    .fetch_one(&state.pool)
    .await
    .expect("managed start command loads");
    let command: CommandEnvelope =
        serde_json::from_str(&command_json).expect("managed start command parses");
    let CommandPayload::StartRuntime {
        execution_profile,
        effective_policy: Some(policy),
        effective_policy_hash: Some(policy_hash),
        ..
    } = command.payload
    else {
        panic!("managed start command must carry its policy snapshot");
    };

    assert_eq!(
        (
            managed.session.runtime.execution_profile,
            execution_profile,
            policy.sandbox_mode,
            policy.approval_mode,
            policy.policy_hash().expect("policy hashes"),
        ),
        (
            AgentExecutionProfile::Managed,
            AgentExecutionProfile::Managed,
            ProviderSandboxMode::WorkspaceWrite,
            ProviderApprovalMode::Untrusted,
            policy_hash,
        )
    );
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");
}

#[tokio::test]
async fn send_turn_persists_durable_turn_and_user_message() {
    let state = test_state().await;
    let (_node_id, detail, workspace_path) = create_test_session(&state).await;
    set_session_runtime_state(&state, &detail, RuntimeSessionState::Ready).await;

    let response = send_turn(
        State(state.clone()),
        Path(detail.session.session_thread_id.to_string()),
        Json(SendTurnRequest {
            content: "persist this turn".to_owned(),
        }),
    )
    .await
    .expect("turn sends")
    .0;
    let (turn_state, content, user_message_count): (String, String, i64) = sqlx::query_as(
        r#"
            select t.state, t.content, count(m.message_id)
            from turns t
            left join messages m on m.turn_id = t.turn_id and m.role = 'user'
            where t.command_id = ?1
            group by t.turn_id, t.state, t.content
            "#,
    )
    .bind(response.command_id.as_str())
    .fetch_one(&state.pool)
    .await
    .expect("turn row loads");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_eq!(turn_state, "created");
    assert_eq!(content, "persist this turn");
    assert_eq!(user_message_count, 1);
}
