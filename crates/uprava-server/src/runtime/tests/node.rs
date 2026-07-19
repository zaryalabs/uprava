use super::*;

#[tokio::test]
async fn delete_node_removes_inventory_dependents() {
    let state = test_state().await;
    let (node_id, detail, _workspace_path) = create_test_session(&state).await;
    let app = build_router(state.clone());
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!("/api/v1/nodes/{node_id}"))
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    let status = response.status();
    let body = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("response body loads");
    let deletion: NodeDeletionResponse =
        serde_json::from_slice(&body).expect("node deletion response parses");
    let inventory = load_inventory(&state).await.expect("inventory loads");

    assert_eq!(status, StatusCode::OK);
    assert_eq!(deletion.node_id, node_id);
    assert!(deletion.deleted);
    assert!(!inventory.nodes.iter().any(|node| node.node_id == node_id));
    assert!(!inventory
        .placements
        .iter()
        .any(|placement| placement.node_id == node_id));
    assert!(!inventory
        .sessions
        .iter()
        .any(|session| { session.session_thread_id == detail.session.session_thread_id }));
}

#[tokio::test]
async fn delete_node_removes_deleted_workspace_tombstones() {
    let state = test_state().await;
    let (node_id, detail, _workspace_path) = create_test_session(&state).await;

    let placement_deletion = delete_placement(
        State(state.clone()),
        Path(detail.placement.project_placement_id.to_string()),
    )
    .await
    .expect("placement delete succeeds")
    .0;
    let tombstone_count: i64 =
        sqlx::query_scalar("select count(*) from deleted_workspace_bindings where node_id = ?1")
            .bind(node_id.as_str())
            .fetch_one(&state.pool)
            .await
            .expect("tombstone count loads");

    let node_deletion = delete_node(State(state.clone()), Path(node_id.to_string()))
        .await
        .expect("node delete succeeds")
        .0;
    let remaining_tombstones: i64 =
        sqlx::query_scalar("select count(*) from deleted_workspace_bindings where node_id = ?1")
            .bind(node_id.as_str())
            .fetch_one(&state.pool)
            .await
            .expect("remaining tombstone count loads");

    assert!(placement_deletion.deleted);
    assert_eq!(tombstone_count, 1);
    assert!(node_deletion.deleted);
    assert_eq!(remaining_tombstones, 0);
}

#[tokio::test]
async fn delete_placement_removes_inventory_dependents_but_keeps_node() {
    let state = test_state().await;
    let (node_id, detail, _workspace_path) = create_test_session(&state).await;
    let placement_id = detail.placement.project_placement_id.clone();
    let app = build_router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!("/api/v1/placements/{placement_id}"))
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    let status = response.status();
    let body = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("response body loads");
    let deletion: PlacementDeletionResponse =
        serde_json::from_slice(&body).expect("placement deletion response parses");
    let inventory = load_inventory(&state).await.expect("inventory loads");

    assert_eq!(status, StatusCode::OK);
    assert_eq!(deletion.project_placement_id, placement_id);
    assert!(deletion.deleted);
    assert!(inventory.nodes.iter().any(|node| node.node_id == node_id));
    assert!(!inventory
        .placements
        .iter()
        .any(|placement| { placement.project_placement_id == deletion.project_placement_id }));
    assert!(!inventory
        .sessions
        .iter()
        .any(|session| { session.session_thread_id == detail.session.session_thread_id }));
}

#[tokio::test]
async fn heartbeat_appears_in_inventory() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let credential = claim.credential.clone();
    let request = NodeHeartbeatRequest {
        node_id: claim.node_id.clone(),
        display_name: "Test node".to_owned(),
        daemon_version: "0.1.0".to_owned(),
        capabilities: vec![CapabilitySummary {
            key: "provider.codex".to_owned(),
            value: CapabilityValue::provider(true),
        }],
        observed_capabilities: vec![],
        diagnostics: Some("daemon_installation_id=daemon-test".to_owned()),
        active_runtime_count: 0,
        sleep_hint: SleepHint::Awake,
        workspace_summaries: vec![],
    };

    let response = node_heartbeat(State(state.clone()), credential.as_deref(), Json(request))
        .await
        .expect("heartbeat accepted");
    let inventory = load_inventory(&state).await.expect("inventory loads");

    assert_eq!(response.0.accepted, !inventory.nodes.is_empty());
    assert!(inventory.nodes.iter().any(|node| node
        .diagnostics
        .contains("daemon_installation_id=daemon-test")));
}

#[tokio::test]
async fn heartbeat_route_rejects_credential_in_body_without_bearer() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let node_id = claim.node_id.expect("node id returned");
    let app = build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/node/heartbeat")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({
                        "node_id": node_id,
                        "credential": claim.credential,
                        "display_name": "Test node",
                        "daemon_version": "0.1.0",
                        "capabilities": [],
                        "diagnostics": null,
                        "active_runtime_count": 0,
                        "sleep_hint": "awake",
                        "workspace_summaries": []
                    }))
                    .expect("heartbeat serializes"),
                ))
                .expect("request builds"),
        )
        .await
        .expect("router responds");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn heartbeat_replaces_normalized_node_capabilities() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let node_id = claim.node_id.clone().expect("node id returned");
    let credential = claim.credential.clone();
    let first = NodeHeartbeatRequest {
        node_id: Some(node_id.clone()),
        display_name: "Test node".to_owned(),
        daemon_version: "0.1.0".to_owned(),
        capabilities: vec![CapabilitySummary {
            key: "provider.codex".to_owned(),
            value: CapabilityValue::provider(true),
        }],
        observed_capabilities: vec![],
        diagnostics: None,
        active_runtime_count: 0,
        sleep_hint: SleepHint::Awake,
        workspace_summaries: vec![],
    };
    let _ = node_heartbeat(State(state.clone()), credential.as_deref(), Json(first))
        .await
        .expect("first heartbeat accepted");
    let second = NodeHeartbeatRequest {
        node_id: Some(node_id.clone()),
        display_name: "Test node".to_owned(),
        daemon_version: "0.1.0".to_owned(),
        capabilities: vec![CapabilitySummary {
            key: "provider.codex".to_owned(),
            value: CapabilityValue::provider(false),
        }],
        observed_capabilities: vec![],
        diagnostics: None,
        active_runtime_count: 0,
        sleep_hint: SleepHint::Awake,
        workspace_summaries: vec![],
    };

    let _ = node_heartbeat(State(state.clone()), credential.as_deref(), Json(second))
        .await
        .expect("second heartbeat accepted");
    let rows: Vec<(String, String)> = sqlx::query_as(
        r#"
            select capability_key, value_json
            from node_capabilities
            where node_id = ?1
            order by capability_key
            "#,
    )
    .bind(node_id.as_str())
    .fetch_all(&state.pool)
    .await
    .expect("capability rows load");

    assert_eq!(rows.len(), 1);
    let codex_capability =
        serde_json::from_str::<JsonValue>(&rows[0].1).expect("capability value json decodes");

    assert_eq!(rows[0].0, "provider.codex");
    assert_eq!(
        codex_capability
            .0
            .get("available")
            .and_then(serde_json::Value::as_bool),
        Some(false)
    );
    assert!(!node_supports_provider(&state, &node_id, "codex")
        .await
        .expect("provider support checks"));
}

#[tokio::test]
async fn inventory_lists_multiple_heartbeat_nodes_with_activity_counts() {
    let state = test_state().await;
    let first = enroll_test_node(&state).await;
    let second = enroll_test_node(&state).await;
    let first_node_id = first.node_id.expect("first node id returned");
    let second_node_id = second.node_id.expect("second node id returned");
    heartbeat_node(
        &state,
        first_node_id.clone(),
        first.credential,
        "Node one",
        SleepHint::Awake,
        2,
    )
    .await;
    heartbeat_node(
        &state,
        second_node_id.clone(),
        second.credential,
        "Node two",
        SleepHint::Sleeping,
        0,
    )
    .await;

    let inventory = load_inventory(&state).await.expect("inventory loads");
    let first_summary = inventory
        .nodes
        .iter()
        .find(|node| node.node_id == first_node_id)
        .expect("first node visible");
    let second_summary = inventory
        .nodes
        .iter()
        .find(|node| node.node_id == second_node_id)
        .expect("second node visible");

    assert_eq!(first_summary.active_runtime_count, 2);
    assert_eq!(second_summary.sleep_hint, SleepHint::Sleeping);
}

#[tokio::test]
async fn stale_node_keeps_sleep_hint_and_accepts_commands() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let node_id = claim.node_id.expect("node id returned");
    heartbeat_node(
        &state,
        node_id.clone(),
        claim.credential,
        "Sleepy node",
        SleepHint::Sleeping,
        0,
    )
    .await;
    age_node_heartbeat(&state, &node_id, state.config.stale_after_seconds + 1).await;

    let placement = validate_placement(
        State(state.clone()),
        Json(CreatePlacementRequest {
            node_id: node_id.clone(),
            display_name: "workspace".to_owned(),
            workspace_path: "/tmp/uprava-stale-node-workspace".to_owned(),
        }),
    )
    .await
    .expect("stale node can still accept commands")
    .0;
    let inventory = load_inventory(&state).await.expect("inventory loads");
    let node = inventory
        .nodes
        .iter()
        .find(|node| node.node_id == node_id)
        .expect("node visible");

    assert_eq!(node.presence, NodePresence::Stale);
    assert_eq!(node.sleep_hint, SleepHint::Sleeping);
    assert_eq!(placement.state, PlacementState::Pending);
}

#[tokio::test]
async fn revoked_node_cannot_heartbeat() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let node_id = claim.node_id.clone().expect("node id returned");
    let _ = revoke_node(State(state.clone()), Path(node_id.to_string()))
        .await
        .expect("node revokes");

    let result = node_heartbeat(
        State(state),
        claim.credential.as_deref(),
        Json(NodeHeartbeatRequest {
            node_id: Some(node_id),
            display_name: "Test node".to_owned(),
            daemon_version: "0.1.0".to_owned(),
            capabilities: vec![],
            observed_capabilities: vec![],
            diagnostics: None,
            active_runtime_count: 0,
            sleep_hint: SleepHint::Awake,
            workspace_summaries: vec![],
        }),
    )
    .await;

    assert!(matches!(result, Err(AppError::Auth { .. })));
}

#[tokio::test]
async fn rotated_node_credential_replaces_previous_credential() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let node_id = claim.node_id.clone().expect("node id returned");
    let old_credential = claim.credential.clone();

    let rotation = rotate_node_credential(State(state.clone()), Path(node_id.to_string()))
        .await
        .expect("credential rotates")
        .0;
    let old_result = node_heartbeat(
        State(state.clone()),
        old_credential.as_deref(),
        Json(NodeHeartbeatRequest {
            node_id: Some(node_id.clone()),
            display_name: "Test node".to_owned(),
            daemon_version: "0.1.0".to_owned(),
            capabilities: vec![],
            observed_capabilities: vec![],
            diagnostics: None,
            active_runtime_count: 0,
            sleep_hint: SleepHint::Awake,
            workspace_summaries: vec![],
        }),
    )
    .await;
    let new_result = node_heartbeat(
        State(state),
        Some(rotation.credential.as_str()),
        Json(NodeHeartbeatRequest {
            node_id: Some(node_id),
            display_name: "Test node".to_owned(),
            daemon_version: "0.1.0".to_owned(),
            capabilities: vec![],
            observed_capabilities: vec![],
            diagnostics: None,
            active_runtime_count: 0,
            sleep_hint: SleepHint::Awake,
            workspace_summaries: vec![],
        }),
    )
    .await;

    assert!(matches!(old_result, Err(AppError::Auth { .. })));
    assert!(new_result.is_ok());
}

#[tokio::test]
async fn repeated_invalid_node_credentials_are_rate_limited() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let node_id = claim.node_id.expect("node id returned");

    for _ in 0..AUTH_FAILURE_LIMIT {
        let _ = node_heartbeat(
            State(state.clone()),
            Some("wrong-credential"),
            Json(NodeHeartbeatRequest {
                node_id: Some(node_id.clone()),
                display_name: "Test node".to_owned(),
                daemon_version: "0.1.0".to_owned(),
                capabilities: vec![],
                observed_capabilities: vec![],
                diagnostics: None,
                active_runtime_count: 0,
                sleep_hint: SleepHint::Awake,
                workspace_summaries: vec![],
            }),
        )
        .await;
    }
    let error = node_heartbeat(
        State(state),
        claim.credential.as_deref(),
        Json(NodeHeartbeatRequest {
            node_id: Some(node_id),
            display_name: "Test node".to_owned(),
            daemon_version: "0.1.0".to_owned(),
            capabilities: vec![],
            observed_capabilities: vec![],
            diagnostics: None,
            active_runtime_count: 0,
            sleep_hint: SleepHint::Awake,
            workspace_summaries: vec![],
        }),
    )
    .await
    .expect_err("valid credential is temporarily rate limited");

    assert!(matches!(
        error,
        AppError::RateLimited {
            code: "auth.rate_limited",
            ..
        }
    ));
}

#[tokio::test]
async fn pending_enrollment_count_is_bounded() {
    let mut config = test_config(86_400);
    config.max_pending_enrollments = 1;
    let state = AppState::new(config, memory_pool().await)
        .await
        .expect("state migrates");
    create_enrollment(&state, "first", None, vec![])
        .await
        .expect("first enrollment is created");

    let error = create_enrollment(&state, "second", None, vec![])
        .await
        .expect_err("pending enrollment limit is enforced");
    assert!(matches!(
        error,
        AppError::RateLimited {
            code: "node_enrollment.pending_limit",
            ..
        }
    ));
}

#[tokio::test]
async fn unapproved_enrollment_claim_remains_pending() {
    let state = test_state().await;
    let requested = create_enrollment(&state, "Pending node", Some("0.1.0"), vec![])
        .await
        .expect("enrollment creates");

    let claim = claim_enrollment(
        &state,
        &NodeEnrollmentClaimRequest {
            enrollment_id: requested.enrollment_id,
            pairing_code: requested.pairing_code,
        },
    )
    .await
    .expect("claim returns pending");

    assert!(claim.pending);
}

#[tokio::test]
async fn matching_production_node_name_is_auto_approved() {
    let mut config = test_config(86_400);
    config.auto_approve_node_name = Some("Zarya Server".to_owned());
    let state = AppState::new(config, memory_pool().await)
        .await
        .expect("state migrates");

    let requested = create_enrollment(&state, " Zarya Server ", Some("0.2.2"), vec![])
        .await
        .expect("matching enrollment creates");

    assert_eq!(requested.status, EnrollmentState::Approved);
}

#[tokio::test]
async fn non_matching_node_name_still_requires_approval() {
    let mut config = test_config(86_400);
    config.auto_approve_node_name = Some("Zarya Server".to_owned());
    let state = AppState::new(config, memory_pool().await)
        .await
        .expect("state migrates");

    let requested = create_enrollment(&state, "Unexpected Node", Some("0.2.2"), vec![])
        .await
        .expect("non-matching enrollment creates");

    assert_eq!(requested.status, EnrollmentState::PendingUserApproval);
}

#[tokio::test]
async fn duplicate_production_node_name_is_not_auto_approved() {
    let mut config = test_config(86_400);
    config.auto_approve_node_name = Some("Zarya Server".to_owned());
    let state = AppState::new(config, memory_pool().await)
        .await
        .expect("state migrates");
    create_enrollment(&state, "Zarya Server", Some("0.2.2"), vec![])
        .await
        .expect("first enrollment creates");

    let duplicate = create_enrollment(&state, "Zarya Server", Some("0.2.2"), vec![])
        .await
        .expect("duplicate enrollment creates");

    assert_eq!(duplicate.status, EnrollmentState::PendingUserApproval);
}

#[tokio::test]
async fn approval_moves_enrollment_to_approved_state() {
    let state = test_state().await;
    let requested = create_enrollment(&state, "Approved node", Some("0.1.0"), vec![])
        .await
        .expect("enrollment creates");

    let response = approve_node_enrollment(State(state), Path(requested.enrollment_id.to_string()))
        .await
        .expect("enrollment approves")
        .0;

    assert_eq!(response.enrollment.status, EnrollmentState::Approved);
    assert!(response.enrollment.approved_at.is_some());
}

#[tokio::test]
async fn approved_enrollment_claim_registers_node() {
    let state = test_state().await;
    let requested = create_enrollment(&state, "Approved node", Some("0.1.0"), vec![])
        .await
        .expect("enrollment creates");
    let _ = approve_node_enrollment(
        State(state.clone()),
        Path(requested.enrollment_id.to_string()),
    )
    .await
    .expect("enrollment approves");

    let claim = claim_enrollment(
        &state,
        &NodeEnrollmentClaimRequest {
            enrollment_id: requested.enrollment_id.clone(),
            pairing_code: requested.pairing_code,
        },
    )
    .await
    .expect("approved claim registers");
    let enrollment = load_enrollment(&state, &requested.enrollment_id)
        .await
        .expect("enrollment loads");

    assert!(claim.accepted);
    assert!(!claim.pending);
    assert!(claim.node_id.is_some());
    assert!(claim.credential.is_some());
    assert_eq!(enrollment.status, EnrollmentState::Registered);
    let audit_count: i64 = sqlx::query_scalar(
        "select count(*) from security_audit_events where kind = 'node.enrollment.claimed'",
    )
    .fetch_one(&state.pool)
    .await
    .expect("claim audit loads");
    assert_eq!(audit_count, 1);
}

#[tokio::test]
async fn legacy_approved_pending_enrollment_claim_registers_node() {
    let state = test_state().await;
    let requested = create_enrollment(&state, "Legacy node", Some("0.1.0"), vec![])
        .await
        .expect("enrollment creates");
    sqlx::query(
        r#"
            update node_enrollments
            set approved_at = ?1
            where enrollment_id = ?2
            "#,
    )
    .bind(Utc::now())
    .bind(requested.enrollment_id.as_str())
    .execute(&state.pool)
    .await
    .expect("legacy approval stores");

    let claim = claim_enrollment(
        &state,
        &NodeEnrollmentClaimRequest {
            enrollment_id: requested.enrollment_id.clone(),
            pairing_code: requested.pairing_code,
        },
    )
    .await
    .expect("legacy approved claim registers");

    assert!(claim.accepted);
    assert!(claim.credential.is_some());
}

#[tokio::test]
async fn expired_enrollment_claim_marks_enrollment_expired_without_credential() {
    let state = test_state().await;
    let requested = create_enrollment(&state, "Expired node", Some("0.1.0"), vec![])
        .await
        .expect("enrollment creates");
    let enrollment_id = requested.enrollment_id.clone();
    sqlx::query(
        r#"
            update node_enrollments
            set expires_at = ?1
            where enrollment_id = ?2
            "#,
    )
    .bind(Utc::now() - chrono::Duration::seconds(1))
    .bind(enrollment_id.as_str())
    .execute(&state.pool)
    .await
    .expect("enrollment expiry rewinds");

    let claim = claim_enrollment(
        &state,
        &NodeEnrollmentClaimRequest {
            enrollment_id: enrollment_id.clone(),
            pairing_code: requested.pairing_code,
        },
    )
    .await
    .expect("expired claim returns safe response");
    let enrollment = load_enrollment(&state, &enrollment_id)
        .await
        .expect("enrollment loads");

    assert!(!claim.accepted);
    assert!(!claim.pending);
    assert_eq!(claim.node_id, None);
    assert_eq!(claim.credential, None);
    assert_eq!(claim.message, "Enrollment expired");
    assert_eq!(enrollment.status, EnrollmentState::Expired);
}

#[tokio::test]
async fn invalid_pairing_code_rejects_claim_with_safe_error() {
    let state = test_state().await;
    let requested = create_enrollment(&state, "Invalid code node", Some("0.1.0"), vec![])
        .await
        .expect("enrollment creates");
    let enrollment_id = requested.enrollment_id.clone();

    let error = claim_enrollment(
        &state,
        &NodeEnrollmentClaimRequest {
            enrollment_id: enrollment_id.clone(),
            pairing_code: "wrong-pairing-code".to_owned(),
        },
    )
    .await
    .expect_err("invalid pairing code rejects");
    let enrollment = load_enrollment(&state, &enrollment_id)
        .await
        .expect("enrollment loads");

    assert!(matches!(
        error,
        AppError::Auth {
            code: "auth_dev.invalid_pairing_code",
            message
        } if message == "Pairing code is invalid"
    ));
    assert_eq!(enrollment.status, EnrollmentState::PendingUserApproval);
}

#[tokio::test]
async fn heartbeat_upserts_node_reported_workspace() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let credential = claim.credential.clone();
    let workspace_path = std::env::temp_dir().join(format!("uprava-test-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&workspace_path).expect("workspace dir creates");

    let _ = node_heartbeat(
        State(state.clone()),
        credential.as_deref(),
        Json(NodeHeartbeatRequest {
            node_id: claim.node_id,
            display_name: "Test node".to_owned(),
            daemon_version: "0.1.0".to_owned(),
            capabilities: vec![],
            observed_capabilities: vec![],
            diagnostics: None,
            active_runtime_count: 0,
            sleep_hint: SleepHint::Awake,
            workspace_summaries: vec![workspace_snapshot_from_request(
                "workspace",
                &workspace_path.display().to_string(),
                PlacementState::Pending,
            )],
        }),
    )
    .await
    .expect("heartbeat accepted");
    let inventory = load_inventory(&state).await.expect("inventory loads");

    let placement = inventory
        .placements
        .iter()
        .find(|placement| placement.workspace_path == workspace_path.display().to_string())
        .expect("heartbeat workspace placement appears in inventory");
    let project_id = placement
        .project_id
        .clone()
        .expect("heartbeat workspace placement has project id");
    let project_display_name: String =
        sqlx::query_scalar("select display_name from projects where project_id = ?1")
            .bind(project_id.as_str())
            .fetch_one(&state.pool)
            .await
            .expect("heartbeat project row loads");
    std::fs::remove_dir_all(&workspace_path).expect("workspace dir removes");

    assert_eq!(project_display_name, "workspace");
}

#[tokio::test]
async fn heartbeat_updates_a_manually_created_workspace_binding() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let node_id = claim.node_id.clone().expect("node id returned");
    let credential = claim.credential.clone();
    let workspace_path_buf = std::env::temp_dir().join(format!("uprava-test-{}", Uuid::new_v4()));
    let workspace_path = workspace_path_buf.display().to_string();
    let existing_placement_id = ProjectPlacementId::new();
    let existing_project_id = ProjectId::new();
    let now = Utc::now();
    std::fs::create_dir_all(&workspace_path_buf).expect("workspace dir creates");

    sqlx::query(
        r#"
        insert into projects (project_id, display_name, repo_id, created_at, updated_at)
        values (?1, 'Manual workspace', null, ?2, ?2)
        "#,
    )
    .bind(existing_project_id.as_str())
    .bind(now)
    .execute(&state.pool)
    .await
    .expect("manual project inserts");
    sqlx::query(
        r#"
        insert into project_placements (
            project_placement_id, project_id, node_id, display_name, workspace_path,
            state, resource_badges_json, last_validated_at, created_at, updated_at
        ) values (?1, ?2, ?3, 'Manual workspace', ?4, 'pending', '[]', ?5, ?5, ?5)
        "#,
    )
    .bind(existing_placement_id.as_str())
    .bind(existing_project_id.as_str())
    .bind(node_id.as_str())
    .bind(&workspace_path)
    .bind(now)
    .execute(&state.pool)
    .await
    .expect("manual workspace binding inserts");

    let _ = node_heartbeat(
        State(state.clone()),
        credential.as_deref(),
        Json(NodeHeartbeatRequest {
            node_id: Some(node_id.clone()),
            display_name: "Test node".to_owned(),
            daemon_version: "0.1.0".to_owned(),
            capabilities: vec![],
            observed_capabilities: vec![],
            diagnostics: None,
            active_runtime_count: 0,
            sleep_hint: SleepHint::Awake,
            workspace_summaries: vec![workspace_snapshot_from_request(
                "Node workspace",
                &workspace_path,
                PlacementState::Validated,
            )],
        }),
    )
    .await
    .expect("heartbeat accepts an existing workspace binding");

    let placement = load_placement(&state, &existing_placement_id)
        .await
        .expect("manual workspace binding remains available");
    std::fs::remove_dir_all(&workspace_path_buf).expect("workspace dir removes");

    assert_eq!(placement.display_name, "Node workspace");
    assert_eq!(placement.state, PlacementState::Validated);
}

#[tokio::test]
async fn delete_placement_tombstones_node_reported_workspace_until_explicit_validate() {
    let state = test_state().await;
    let claim = enroll_test_node(&state).await;
    let node_id = claim.node_id.clone().expect("node id returned");
    let credential = claim.credential.clone();
    let workspace_path_buf = std::env::temp_dir().join(format!("uprava-test-{}", Uuid::new_v4()));
    let workspace_path = workspace_path_buf.display().to_string();
    std::fs::create_dir_all(&workspace_path_buf).expect("workspace dir creates");

    let _ = node_heartbeat(
        State(state.clone()),
        credential.as_deref(),
        Json(NodeHeartbeatRequest {
            node_id: claim.node_id.clone(),
            display_name: "Test node".to_owned(),
            daemon_version: "0.1.0".to_owned(),
            capabilities: vec![],
            observed_capabilities: vec![],
            diagnostics: None,
            active_runtime_count: 0,
            sleep_hint: SleepHint::Awake,
            workspace_summaries: vec![workspace_snapshot_from_request(
                "workspace",
                &workspace_path,
                PlacementState::Validated,
            )],
        }),
    )
    .await
    .expect("heartbeat accepted");
    let placement = load_inventory(&state)
        .await
        .expect("inventory loads")
        .placements
        .into_iter()
        .find(|placement| placement.workspace_path == workspace_path)
        .expect("heartbeat placement appears");
    let app = build_router(state.clone());

    let delete_response = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!(
                    "/api/v1/placements/{}",
                    placement.project_placement_id
                ))
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");

    let _ = node_heartbeat(
        State(state.clone()),
        credential.as_deref(),
        Json(NodeHeartbeatRequest {
            node_id: claim.node_id,
            display_name: "Test node".to_owned(),
            daemon_version: "0.1.0".to_owned(),
            capabilities: vec![],
            observed_capabilities: vec![],
            diagnostics: None,
            active_runtime_count: 0,
            sleep_hint: SleepHint::Awake,
            workspace_summaries: vec![workspace_snapshot_from_request(
                "workspace",
                &workspace_path,
                PlacementState::Validated,
            )],
        }),
    )
    .await
    .expect("heartbeat accepted");
    let inventory_after_heartbeat = load_inventory(&state).await.expect("inventory loads");

    let explicit_placement = validate_placement(
        State(state.clone()),
        Json(CreatePlacementRequest {
            node_id,
            display_name: "workspace".to_owned(),
            workspace_path: workspace_path.clone(),
        }),
    )
    .await
    .expect("explicit validation recreates placement")
    .0;
    std::fs::remove_dir_all(&workspace_path_buf).expect("workspace dir removes");

    assert_eq!(delete_response.status(), StatusCode::OK);
    assert!(!inventory_after_heartbeat
        .placements
        .iter()
        .any(|placement| placement.workspace_path == workspace_path));
    assert_eq!(explicit_placement.workspace_path, workspace_path);
}
