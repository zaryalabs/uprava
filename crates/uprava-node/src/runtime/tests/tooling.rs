use super::*;

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
