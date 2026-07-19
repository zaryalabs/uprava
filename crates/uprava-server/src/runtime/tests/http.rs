use super::*;

#[tokio::test]
async fn health_endpoint_returns_ok() {
    let app = build_router(test_state().await);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .header(CORRELATION_ID_HEADER, "health-correlation")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(CORRELATION_ID_HEADER),
        Some(&HeaderValue::from_static("health-correlation"))
    );
}

#[tokio::test]
async fn public_rate_limit_does_not_starve_health() {
    let app = build_router(test_state().await);
    for _ in 0..30 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/auth/status")
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(response.status(), StatusCode::OK);
    }
    let limited = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/status")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);

    let health = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    assert_eq!(health.status(), StatusCode::OK);
}

#[tokio::test]
async fn general_local_traffic_does_not_consume_enrollment_budget() {
    let app = build_router(test_state().await);
    for _ in 0..31 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/inventory")
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(response.status(), StatusCode::OK);
    }

    let enrollment = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/node-enrollments")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");

    assert_eq!(enrollment.status(), StatusCode::OK);
}

#[tokio::test]
async fn response_header_and_error_body_share_correlation_id() {
    let response = build_router(test_state().await)
        .oneshot(
            Request::builder()
                .uri("/api/v1/nodes/missing")
                .header(CORRELATION_ID_HEADER, "correlation-response")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        response.headers().get(CORRELATION_ID_HEADER),
        Some(&HeaderValue::from_static("correlation-response"))
    );
    let body = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("error body loads");
    let error: ApiError = serde_json::from_slice(&body).expect("error body decodes");
    assert_eq!(error.correlation_id.as_str(), "correlation-response");
}

#[tokio::test]
async fn metrics_endpoint_exposes_bounded_core_counters() {
    let state = test_state().await;
    state
        .core_metrics
        .accepted_events
        .store(3, Ordering::Relaxed);
    let response = build_router(state)
        .oneshot(
            Request::builder()
                .uri("/api/v1/metrics")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    let body = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("metrics body loads");
    let body = String::from_utf8(body.to_vec()).expect("metrics are utf8");
    assert!(body.contains("uprava_core_events_accepted_total 3"));
    assert!(body.contains("uprava_core_command_results_total 0"));
    assert!(body.contains("uprava_core_auth_failures_total 0"));
    assert!(body.contains("uprava_core_log_records_dropped_total"));
}

#[tokio::test]
async fn router_rejects_oversized_request_body() {
    let app = build_router(test_state().await);
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/client/logs")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(vec![b'a'; MAX_HTTP_REQUEST_BODY_BYTES + 1]))
                .expect("request builds"),
        )
        .await
        .expect("router responds");

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn client_logs_endpoint_appends_local_jsonl_record() {
    let state = test_state().await;
    let log_path = state.config.client_log_file.clone();
    let app = build_router(state);
    let request = ClientLogRequest {
        level: ClientLogLevel::Error,
        source: "web.global_error".to_owned(),
        message: "render failed".to_owned(),
        route: Some("/nodes".to_owned()),
        user_agent: Some("vitest".to_owned()),
        occurred_at: Utc::now(),
        detail: JsonValue(json!({ "component": "NodesRoute" })),
    };
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/client/logs")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&request).expect("request serializes"),
                ))
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    let status = response.status();
    let body = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("response body loads");
    let log_content = std::fs::read_to_string(&log_path).expect("client log file reads");
    let first_line = log_content.lines().next().expect("client log line exists");
    let record: serde_json::Value =
        serde_json::from_str(first_line).expect("client log json parses");
    let _ = std::fs::remove_file(log_path);

    assert_eq!(status, StatusCode::OK);
    assert!(
        serde_json::from_slice::<ClientLogResponse>(&body)
            .expect("response parses")
            .accepted
    );
    assert_eq!(record["level"], "error");
    assert_eq!(record["source"], "web.global_error");
    assert_eq!(record["message"], "render failed");
    assert!(record["detail"]
        .as_str()
        .expect("detail is bounded string")
        .contains("NodesRoute"));
}

#[tokio::test]
async fn client_log_retention_rotates_a_full_file() {
    let path =
        std::env::temp_dir().join(format!("uprava-client-rotation-{}.jsonl", Uuid::new_v4()));
    let file = std::fs::File::create(&path).expect("client log creates");
    file.set_len(MAX_CLIENT_LOG_BYTES)
        .expect("client log fills");

    append_jsonl_log(path.clone(), "{\"message\":\"next\"}".to_owned())
        .await
        .expect("client log rotates");

    assert!(path.exists());
    assert!(PathBuf::from(format!("{}.1", path.display())).exists());
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(PathBuf::from(format!("{}.1", path.display())));
}

#[tokio::test]
async fn cors_preflight_allows_configured_loopback_origin() {
    let app = build_router(test_state().await);
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/api/v1/health")
                .header("origin", "http://127.0.0.1:5173")
                .header("access-control-request-method", "GET")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("access-control-allow-origin")
            .and_then(|value| value.to_str().ok()),
        Some("http://127.0.0.1:5173")
    );
}

#[tokio::test]
async fn cors_preflight_rejects_unknown_origin() {
    let app = build_router(test_state().await);
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/api/v1/health")
                .header("origin", "https://example.com")
                .header("access-control-request-method", "GET")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");

    assert_eq!(response.status(), StatusCode::OK);
    assert!(response
        .headers()
        .get("access-control-allow-origin")
        .is_none());
}

#[tokio::test]
async fn hardened_web_auth_requires_setup_before_client_routes() {
    let app = build_router(test_state_with_web_auth().await);

    let protected = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/inventory")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    let status = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/status")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    let body = to_bytes(status.into_body(), 64 * 1024)
        .await
        .expect("status body loads");
    let auth_status: WebAuthStatusResponse =
        serde_json::from_slice(&body).expect("auth status parses");

    assert_eq!(protected.status(), StatusCode::UNAUTHORIZED);
    assert!(auth_status.auth_required);
    assert!(auth_status.setup_required);
    assert!(!auth_status.authenticated);
}

#[test]
fn password_hash_uses_argon2id_and_rejects_legacy_sha256_records() {
    let password = "very-secure-local-password";
    let hash = hash_password(password).expect("Argon2id hash creates");
    assert!(hash.starts_with("$argon2id$"));
    assert!(verify_password(&hash, password));
    assert!(!verify_password(&hash, "wrong-password"));
    assert!(!verify_password("pwd-sha256:salt:digest", password));
}

#[tokio::test]
async fn hardened_web_auth_sets_session_and_enforces_csrf_for_mutations() {
    let app = build_router(test_state_with_web_auth().await);
    let setup = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/auth/setup")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&WebAuthSetupRequest {
                        password: "very-secure-local-password".to_owned(),
                    })
                    .expect("setup serializes"),
                ))
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    let cookie_header = set_cookie_header(&setup);
    let csrf_token = csrf_from_cookie_header(&cookie_header);

    let read = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/inventory")
                .header(COOKIE, cookie_header.as_str())
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    let rejected_mutation = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/node-enrollments")
                .header(COOKIE, cookie_header.as_str())
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&ClientCreateNodeEnrollmentRequest {
                        display_name: "Secure node".to_owned(),
                    })
                    .expect("enrollment serializes"),
                ))
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    let accepted_mutation = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/node-enrollments")
                .header(COOKIE, cookie_header.as_str())
                .header(CSRF_HEADER, csrf_token)
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&ClientCreateNodeEnrollmentRequest {
                        display_name: "Secure node".to_owned(),
                    })
                    .expect("enrollment serializes"),
                ))
                .expect("request builds"),
        )
        .await
        .expect("router responds");

    assert_eq!(setup.status(), StatusCode::OK);
    assert!(!cookie_header.is_empty());
    assert_eq!(read.status(), StatusCode::OK);
    assert_eq!(rejected_mutation.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(accepted_mutation.status(), StatusCode::OK);
}

#[tokio::test]
async fn missing_resource_api_error_uses_safe_envelope_with_correlation_id() {
    let app = build_router(test_state().await);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/nodes/missing-node")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    let status = response.status();
    let body = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("response body loads");
    let envelope: ApiError = serde_json::from_slice(&body).expect("api error envelope parses");

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(envelope.error_code, "node.not_found");
    assert_eq!(envelope.message, "Node not found");
    assert!(!envelope.retryable);
    assert!(!envelope.correlation_id.as_str().is_empty());
}
