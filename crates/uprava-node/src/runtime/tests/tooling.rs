use super::*;

#[test]
fn linear_authorization_url_accepts_pinned_https_host_and_state() {
    let line = "Please open this URL in your browser: https://linear.app/oauth/authorize?client_id=uprava&state=opaque";

    let url = linear_authorization_url_from_line(line);

    assert_eq!(
        url.as_deref(),
        Some("https://linear.app/oauth/authorize?client_id=uprava&state=opaque")
    );
}

#[test]
fn linear_authorization_url_rejects_lookalike_host() {
    let line = "Please open this URL in your browser: https://linear.app.attacker.test/oauth/authorize?state=opaque";

    assert!(linear_authorization_url_from_line(line).is_none());
}

#[test]
fn durable_tooling_result_removes_ephemeral_authorization_url() {
    let payload = JsonValue(serde_json::json!({
        "authorization_url": "https://linear.app/oauth/authorize?state=secret-state",
        "authorization_expires_at": "2026-07-19T10:05:00Z",
        "status": null,
        "definitions": [],
        "event": null
    }));

    let durable = durable_tooling_result_payload(&payload);

    assert!(durable.0.get("authorization_url").is_none());
}
