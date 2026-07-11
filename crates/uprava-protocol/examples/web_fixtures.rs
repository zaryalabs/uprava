use chrono::{TimeZone, Utc};
use serde_json::{json, Map, Value};
use uprava_protocol::{
    ActorRef, CommandAcceptedResponse, CommandKind, CommandState, EventEnvelope, EventId,
    EventKind, ProjectPlacementId, ScopeRef, TerminalId, WorkspaceCommandHistoryItem,
    WorkspaceCommandHistoryResponse, WorkspaceCommandIntent, WorkspaceCommandRunResponse,
    WorkspaceTerminalListResponse, WorkspaceTerminalOpenResponse, WorkspaceTerminalOutputFrame,
    WorkspaceTerminalState, WorkspaceTerminalStreamFrame, WorkspaceTerminalSummary,
};

fn main() {
    let at = Utc
        .with_ymd_and_hms(2026, 7, 10, 12, 0, 0)
        .single()
        .expect("fixture timestamp is valid");
    let placement_id = ProjectPlacementId::from("placement-fixture");
    let terminal_id = TerminalId::from("terminal-fixture");
    let terminal = WorkspaceTerminalSummary {
        placement_id: placement_id.clone(),
        terminal_id: terminal_id.clone(),
        title: "Fixture shell".to_owned(),
        cwd: "/workspace".to_owned(),
        shell: "/bin/sh".to_owned(),
        cols: 120,
        rows: 40,
        state: WorkspaceTerminalState::Running,
        exit_code: None,
        created_at: at,
        updated_at: at,
    };
    let output = WorkspaceTerminalOutputFrame {
        terminal_id: terminal_id.clone(),
        seq: 1,
        data: "ready\n".to_owned(),
        sent_at: at,
    };

    let mut fixtures = Map::new();
    insert(
        &mut fixtures,
        "command_accepted",
        CommandAcceptedResponse {
            command_id: "command-fixture".into(),
            session: None,
        },
    );
    insert(
        &mut fixtures,
        "workspace_command_run",
        WorkspaceCommandRunResponse {
            placement_id: placement_id.clone(),
            terminal_command_id: "terminal-command-fixture".to_owned(),
            command: "make".to_owned(),
            args: vec!["l".to_owned()],
            intent: WorkspaceCommandIntent::Check,
            label: Some("Light checks".to_owned()),
            exit_code: Some(0),
            success: true,
            stdout: "ok\n".to_owned(),
            stderr: String::new(),
            stdout_truncated: false,
            stderr_truncated: false,
            duration_ms: 12,
            started_at: at,
            completed_at: at,
        },
    );
    insert(
        &mut fixtures,
        "workspace_terminal_open",
        WorkspaceTerminalOpenResponse {
            placement_id: placement_id.clone(),
            terminal: terminal.clone(),
            replay: vec![output.clone()],
        },
    );
    insert(
        &mut fixtures,
        "workspace_command_history",
        WorkspaceCommandHistoryResponse {
            placement_id: placement_id.clone(),
            commands: vec![WorkspaceCommandHistoryItem {
                command_id: "command-fixture".into(),
                kind: CommandKind::RunWorkspaceCommand,
                state: CommandState::Completed,
                created_at: at,
                completed_at: Some(at),
                payload: json!({ "command": "make", "args": ["l"] }).into(),
                result_payload: Some(json!({ "success": true }).into()),
            }],
            generated_at: at,
        },
    );
    insert(&mut fixtures, "workspace_terminal_output", output.clone());
    insert(
        &mut fixtures,
        "workspace_terminal_list",
        WorkspaceTerminalListResponse {
            placement_id,
            terminals: vec![terminal],
            generated_at: at,
        },
    );
    insert(
        &mut fixtures,
        "workspace_terminal_stream",
        WorkspaceTerminalStreamFrame::Output {
            terminal_id,
            seq: output.seq,
            data: output.data,
            sent_at: at,
        },
    );
    insert(
        &mut fixtures,
        "event_envelope",
        EventEnvelope {
            event_id: EventId::from("event-fixture"),
            command_id: None,
            correlation_id: None,
            actor_ref: ActorRef::System,
            scope_ref: ScopeRef::Unknown {
                scope: "fixture".to_owned(),
            },
            node_id: None,
            runtime_session_id: None,
            session_thread_id: None,
            turn_id: None,
            seq: 1,
            session_projection_seq: None,
            kind: EventKind::RuntimeReady,
            happened_at: at,
            source_refs: vec![],
            evidence_refs: vec![],
            cause_refs: vec![],
            result_refs: vec![],
            payload: uprava_protocol::EventPayload::from_json(
                EventKind::RuntimeReady,
                json!({ "provider": "fixture" }),
            ),
        },
    );

    println!(
        "{}",
        serde_json::to_string_pretty(&Value::Object(fixtures))
            .expect("web protocol fixtures serialize")
    );
}

fn insert<T: serde::Serialize>(fixtures: &mut Map<String, Value>, name: &str, fixture: T) {
    fixtures.insert(
        name.to_owned(),
        serde_json::to_value(fixture).expect("fixture serializes"),
    );
}
