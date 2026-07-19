use super::*;

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
