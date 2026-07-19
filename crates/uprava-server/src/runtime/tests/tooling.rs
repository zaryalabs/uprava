use uprava_protocol::{
    compute_tool_schema_hash, ExecuteToolRequest, InspectToolRequest, SearchToolsRequest,
    ToolCallState, ToolExecutionErrorCode, ToolId, ToolScope, ToolSearchFilters, ToolSourceKind,
};

use super::*;

fn scope_for(node_id: &NodeId, detail: &SessionDetail, actor_ref: ActorRef) -> ToolScope {
    ToolScope {
        actor_ref,
        node_id: Some(node_id.clone()),
        project_id: detail.placement.project_id.clone(),
        project_placement_id: Some(detail.placement.project_placement_id.clone()),
        session_thread_id: Some(detail.session.session_thread_id.clone()),
    }
}

#[tokio::test]
async fn registry_keeps_version_for_same_schema_and_increments_for_schema_change() {
    let state = test_state().await;
    let tool_id = ToolId::from("uprava.node.inspect");
    let original = get_tool_definition(&state, &tool_id)
        .await
        .expect("seeded tool loads");
    register_tool_definitions(
        &state,
        "uprava-native",
        ToolSourceKind::UpravaNative,
        "Uprava",
        std::slice::from_ref(&original),
    )
    .await
    .expect("unchanged definition registers");
    let unchanged = get_tool_definition(&state, &tool_id)
        .await
        .expect("unchanged tool loads");
    assert_eq!(unchanged.version, original.version);

    let mut changed = original.clone();
    changed.input_schema = JsonValue(serde_json::json!({
        "type": "object",
        "properties": {
            "node_id": { "type": "string" },
            "include_diagnostics": { "type": "boolean" }
        },
        "required": ["node_id"],
        "additionalProperties": false
    }));
    changed.schema_hash = compute_tool_schema_hash(&changed.input_schema, None)
        .expect("changed schema hash computes");
    register_tool_definitions(
        &state,
        "uprava-native",
        ToolSourceKind::UpravaNative,
        "Uprava",
        &[changed],
    )
    .await
    .expect("changed definition registers");
    let incremented = get_tool_definition(&state, &tool_id)
        .await
        .expect("changed tool loads");
    assert_eq!(incremented.version, original.version + 1);
}

#[tokio::test]
async fn progressive_discovery_executes_native_tool_with_durable_trace() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    let scope = scope_for(&node_id, &detail, ActorRef::local_user());

    let search = search_tools(
        &state,
        &SearchToolsRequest {
            scope: scope.clone(),
            query: "session inspect".to_owned(),
            filters: ToolSearchFilters::default(),
            cursor: None,
            limit: Some(10),
        },
    )
    .await
    .expect("tool search succeeds");
    assert!(search
        .items
        .iter()
        .any(|item| item.tool_id.as_str() == "uprava.session.inspect"));

    let inspected = inspect_tool(
        &state,
        &InspectToolRequest {
            scope: scope.clone(),
            tool_id: ToolId::from("uprava.session.inspect"),
        },
    )
    .await
    .expect("tool inspect succeeds");
    assert_eq!(
        inspected.definition.tool_id.as_str(),
        "uprava.session.inspect"
    );

    let executed = execute_tool(
        &state,
        &ExecuteToolRequest {
            scope,
            tool_id: ToolId::from("uprava.session.inspect"),
            arguments: JsonValue(serde_json::json!({
                "session_thread_id": detail.session.session_thread_id
            })),
        },
    )
    .await
    .expect("native tool execution succeeds");
    assert_eq!(executed.state, ToolCallState::Completed);

    let detail = load_tool_call_detail(&state, &executed.tool_call_id)
        .await
        .expect("tool call detail loads");
    assert_eq!(detail.summary.route, "core_native");

    let event_states: Vec<String> = sqlx::query_scalar(
        "select state from tool_call_events where tool_call_id = ?1 order by sequence",
    )
    .bind(executed.tool_call_id.as_str())
    .fetch_all(&state.pool)
    .await
    .expect("tool call events load");
    assert_eq!(
        event_states,
        vec!["requested", "authorized", "started", "completed"]
    );

    let snapshot_count: i64 = sqlx::query_scalar(
        "select count(*) from session_tool_snapshots where session_thread_id = ?1 and tool_id = 'uprava.session.inspect'",
    )
    .bind(detail.summary.scope.session_thread_id.expect("session scope").as_str())
    .fetch_one(&state.pool)
    .await
    .expect("session tool snapshot count loads");
    assert_eq!(snapshot_count, 1);
    let _ = std::fs::remove_dir_all(workspace_path);
}

#[tokio::test]
async fn search_hides_all_definitions_from_denied_actor() {
    let state = test_state().await;
    let response = search_tools(
        &state,
        &SearchToolsRequest {
            scope: ToolScope {
                actor_ref: ActorRef::Unknown,
                node_id: None,
                project_id: None,
                project_placement_id: None,
                session_thread_id: None,
            },
            query: "inspect".to_owned(),
            filters: ToolSearchFilters::default(),
            cursor: None,
            limit: None,
        },
    )
    .await
    .expect("denied search returns an empty page");

    assert!(response.items.is_empty());
}

#[tokio::test]
async fn search_cursor_is_bounded_signed_and_bound_to_the_original_query() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    let scope = scope_for(&node_id, &detail, ActorRef::local_user());
    let first_page = search_tools(
        &state,
        &SearchToolsRequest {
            scope: scope.clone(),
            query: "inspect".to_owned(),
            filters: ToolSearchFilters::default(),
            cursor: None,
            limit: Some(1),
        },
    )
    .await
    .expect("first search page succeeds");
    assert_eq!(first_page.items.len(), 1);
    let cursor = first_page.next_cursor.expect("next cursor exists");

    let second_page = search_tools(
        &state,
        &SearchToolsRequest {
            scope: scope.clone(),
            query: "inspect".to_owned(),
            filters: ToolSearchFilters::default(),
            cursor: Some(cursor.clone()),
            limit: Some(1),
        },
    )
    .await
    .expect("bound cursor continues the search");
    assert_eq!(second_page.items.len(), 1);
    assert_ne!(first_page.items[0].tool_id, second_page.items[0].tool_id);

    let rebound = search_tools(
        &state,
        &SearchToolsRequest {
            scope,
            query: "trace".to_owned(),
            filters: ToolSearchFilters::default(),
            cursor: Some(cursor),
            limit: Some(1),
        },
    )
    .await
    .expect_err("cursor cannot be reused for another query");
    assert!(matches!(
        rebound,
        AppError::BadRequest { code, .. } if code == "tool_search.invalid_cursor"
    ));

    let invalid_limit = search_tools(
        &state,
        &SearchToolsRequest {
            scope: scope_for(&node_id, &detail, ActorRef::local_user()),
            query: String::new(),
            filters: ToolSearchFilters::default(),
            cursor: None,
            limit: Some(0),
        },
    )
    .await
    .expect_err("zero search limit is rejected");
    assert!(matches!(
        invalid_limit,
        AppError::BadRequest { code, .. } if code == "tool_search.invalid_limit"
    ));
    let _ = std::fs::remove_dir_all(workspace_path);
}

#[tokio::test]
async fn execute_rechecks_schema_and_records_terminal_failure() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    let response = execute_tool(
        &state,
        &ExecuteToolRequest {
            scope: scope_for(&node_id, &detail, ActorRef::local_user()),
            tool_id: ToolId::from("uprava.node.inspect"),
            arguments: JsonValue(serde_json::json!({"unexpected": true})),
        },
    )
    .await
    .expect("invalid execution returns a typed response");

    assert_eq!(response.state, ToolCallState::Failed);
    assert_eq!(
        response.error.expect("typed error").code,
        ToolExecutionErrorCode::InvalidArguments
    );
    let persisted_state: String =
        sqlx::query_scalar("select state from tool_calls where tool_call_id = ?1")
            .bind(response.tool_call_id.as_str())
            .fetch_one(&state.pool)
            .await
            .expect("failed tool call state loads");
    assert_eq!(persisted_state, "failed");
    let _ = std::fs::remove_dir_all(workspace_path);
}

#[tokio::test]
async fn deterministic_mock_backend_proves_success_and_failure_routes() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    let mut definition = mock_external_definition();
    definition.redaction.result_json_pointers = vec!["/backend".to_owned()];
    register_tool_definitions(
        &state,
        "mock-external",
        ToolSourceKind::ExternalMcp,
        "Mock external backend",
        &[definition],
    )
    .await
    .expect("mock definition registers");
    let scope = scope_for(&node_id, &detail, ActorRef::local_user());
    let successful = execute_tool(
        &state,
        &ExecuteToolRequest {
            scope: scope.clone(),
            tool_id: ToolId::from("mock.external.echo"),
            arguments: JsonValue(serde_json::json!({"value": "fixture"})),
        },
    )
    .await
    .expect("mock success returns");
    assert_eq!(successful.state, ToolCallState::Completed);
    assert_eq!(
        successful
            .result
            .as_ref()
            .expect("mock result exists")
            .content
            .0["backend"],
        "[redacted]"
    );
    let persisted_summary: String = sqlx::query_scalar(
        "select redacted_result_summary from tool_calls where tool_call_id = ?1",
    )
    .bind(successful.tool_call_id.as_str())
    .fetch_one(&state.pool)
    .await
    .expect("redacted mock result summary loads");
    assert!(persisted_summary.contains("[redacted]"));
    assert!(!persisted_summary.contains("\"backend\":\"mock\""));

    let failed = execute_tool(
        &state,
        &ExecuteToolRequest {
            scope,
            tool_id: ToolId::from("mock.external.echo"),
            arguments: JsonValue(serde_json::json!({"fail": true})),
        },
    )
    .await
    .expect("mock failure returns");
    assert_eq!(failed.state, ToolCallState::Failed);
    assert_eq!(
        failed.error.expect("mock error").code,
        ToolExecutionErrorCode::BackendFailed
    );
    let _ = std::fs::remove_dir_all(workspace_path);
}

#[tokio::test]
async fn leases_reject_rotation_expiry_revocation_and_foreign_scope() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    let actor = ActorRef::Provider {
        provider: "codex".to_owned(),
    };
    let (first_token, first_claims) = state
        .create_mcp_access_lease(&detail.session.session_thread_id, actor.clone())
        .await
        .expect("first lease issues");
    validate_mcp_access_lease(&state, &first_token)
        .await
        .expect("first lease validates");

    let (second_token, second_claims) = state
        .create_mcp_access_lease(&detail.session.session_thread_id, actor.clone())
        .await
        .expect("rotated lease issues");
    let rotated = validate_mcp_access_lease(&state, &first_token)
        .await
        .expect_err("rotated lease is revoked");
    assert_eq!(rotated.code, ToolExecutionErrorCode::LeaseRevoked);

    let mut foreign_scope = scope_for(&node_id, &detail, actor);
    foreign_scope.node_id = Some(NodeId::from("foreign-node"));
    assert!(!lease_scope_matches(&second_claims, &foreign_scope));

    sqlx::query("update mcp_access_leases set expires_at = ?1 where lease_id = ?2")
        .bind(Utc::now() - chrono::Duration::seconds(1))
        .bind(second_claims.lease_id.as_str())
        .execute(&state.pool)
        .await
        .expect("lease expiry updates");
    let expired = validate_mcp_access_lease(&state, &second_token)
        .await
        .expect_err("expired lease is rejected");
    assert_eq!(expired.code, ToolExecutionErrorCode::LeaseExpired);

    let (third_token, _) = state
        .create_mcp_access_lease(
            &detail.session.session_thread_id,
            ActorRef::Provider {
                provider: "codex".to_owned(),
            },
        )
        .await
        .expect("third lease issues");
    revoke_session_mcp_leases(&state, &detail.session.session_thread_id, "test_revocation")
        .await
        .expect("lease revokes");
    let revoked = validate_mcp_access_lease(&state, &third_token)
        .await
        .expect_err("revoked lease is rejected");
    assert_eq!(revoked.code, ToolExecutionErrorCode::LeaseRevoked);
    assert_eq!(
        first_claims.session_thread_id,
        detail.session.session_thread_id
    );
    let _ = std::fs::remove_dir_all(workspace_path);
}

#[tokio::test]
async fn mcp_streamable_http_lists_only_meta_tools_and_searches_registry() {
    let state = test_state().await;
    let (node_id, detail, workspace_path) = create_test_session(&state).await;
    let actor = ActorRef::Provider {
        provider: "codex".to_owned(),
    };
    let scope = scope_for(&node_id, &detail, actor.clone());
    let (access_token, _) = state
        .create_mcp_access_lease(&detail.session.session_thread_id, actor)
        .await
        .expect("MCP lease issues");
    let authorization = format!("Bearer {access_token}");
    let app = build_router(state.clone());
    let initialize = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": {"name": "uprava-test", "version": "1"}
        }
    });
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp")
                .header("host", "localhost")
                .header(AUTHORIZATION, &authorization)
                .header(CONTENT_TYPE, "application/json")
                .header("accept", "application/json, text/event-stream")
                .body(Body::from(initialize.to_string()))
                .expect("initialize request builds"),
        )
        .await
        .expect("initialize response returns");
    assert_eq!(response.status(), StatusCode::OK);
    let session_id = response
        .headers()
        .get("mcp-session-id")
        .expect("MCP session id returned")
        .to_str()
        .expect("MCP session id is utf8")
        .to_owned();
    let initialize_body = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("initialize body reads");
    assert!(!initialize_body.is_empty());

    let initialized = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp")
                .header("host", "localhost")
                .header(AUTHORIZATION, &authorization)
                .header(CONTENT_TYPE, "application/json")
                .header("accept", "application/json, text/event-stream")
                .header("mcp-session-id", &session_id)
                .header("mcp-protocol-version", "2025-11-25")
                .body(Body::from(
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "method": "notifications/initialized"
                    })
                    .to_string(),
                ))
                .expect("initialized request builds"),
        )
        .await
        .expect("initialized response returns");
    assert_eq!(initialized.status(), StatusCode::ACCEPTED);

    let list_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp")
                .header("host", "localhost")
                .header(AUTHORIZATION, &authorization)
                .header(CONTENT_TYPE, "application/json")
                .header("accept", "application/json, text/event-stream")
                .header("mcp-session-id", &session_id)
                .header("mcp-protocol-version", "2025-11-25")
                .body(Body::from(
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": 2,
                        "method": "tools/list",
                        "params": {}
                    })
                    .to_string(),
                ))
                .expect("list request builds"),
        )
        .await
        .expect("list response returns");
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_headers = list_response.headers().clone();
    let list_bytes = to_bytes(list_response.into_body(), 64 * 1024)
        .await
        .expect("list body reads");
    assert!(
        !list_bytes.is_empty(),
        "list body is empty; headers: {list_headers:?}"
    );
    let list_body = parse_mcp_http_body(&list_bytes);
    let names = list_body["result"]["tools"]
        .as_array()
        .expect("tools list exists")
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["search_tools", "inspect_tool", "execute_tool"]);

    let search_request = SearchToolsRequest {
        scope,
        query: "session inspect".to_owned(),
        filters: ToolSearchFilters::default(),
        cursor: None,
        limit: Some(10),
    };
    let search_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp")
                .header("host", "localhost")
                .header(AUTHORIZATION, authorization)
                .header(CONTENT_TYPE, "application/json")
                .header("accept", "application/json, text/event-stream")
                .header("mcp-session-id", session_id)
                .header("mcp-protocol-version", "2025-11-25")
                .body(Body::from(
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": 3,
                        "method": "tools/call",
                        "params": {
                            "name": "search_tools",
                            "arguments": search_request
                        }
                    })
                    .to_string(),
                ))
                .expect("search request builds"),
        )
        .await
        .expect("search response returns");
    assert_eq!(search_response.status(), StatusCode::OK);
    let search_bytes = to_bytes(search_response.into_body(), 128 * 1024)
        .await
        .expect("search body reads");
    let search_body = parse_mcp_http_body(&search_bytes);
    assert!(search_body["result"]["structuredContent"]["items"]
        .as_array()
        .expect("search items exist")
        .iter()
        .any(|item| item["tool_id"] == "uprava.session.inspect"));
    let _ = std::fs::remove_dir_all(workspace_path);
}

fn parse_mcp_http_body(body: &[u8]) -> serde_json::Value {
    let text = std::str::from_utf8(body).expect("MCP response is utf8");
    if let Ok(value) = serde_json::from_str(text) {
        return value;
    }
    text.lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .find_map(|data| serde_json::from_str(data).ok())
        .unwrap_or_else(|| panic!("MCP response contains no JSON-RPC payload: {text:?}"))
}
