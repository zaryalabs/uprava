use super::*;

#[tokio::test]
async fn terminal_manager_rebinds_routes_without_dropping_pty_handles() {
    let (old_sender, _old_receiver) = mpsc::channel(4);
    let (new_sender, _new_receiver) = mpsc::channel(4);
    let route = Arc::new(RwLock::new(Some(old_sender)));
    let control_tx = mpsc::unbounded_channel().0;
    let mut manager = WorkspaceTerminalManager::default();
    manager.terminals.insert(
        "terminal-1".to_owned(),
        WorkspaceTerminalHandle {
            replay: Arc::new(RwLock::new(VecDeque::new())),
            control_tx,
            sender_route: route.clone(),
            task: tokio::spawn(async {}),
        },
    );

    manager.rebind_sender(&new_sender).await;
    assert!(route.read().await.is_some());
    assert_eq!(manager.terminals.len(), 1);

    manager.detach_sender().await;
    assert!(route.read().await.is_none());
    assert_eq!(manager.terminals.len(), 1);
}

#[tokio::test]
async fn terminal_replay_is_bounded_by_bytes() {
    let replay = Arc::new(RwLock::new(VecDeque::new()));
    let terminal_id = TerminalId::from("terminal-replay");
    for seq in 1..=4 {
        record_terminal_replay(
            &replay,
            WorkspaceTerminalOutputFrame {
                terminal_id: terminal_id.clone(),
                seq,
                data: "x".repeat(MAX_WORKSPACE_TERMINAL_REPLAY_BYTES / 2),
                sent_at: Utc::now(),
            },
        )
        .await;
    }

    let replay = replay.read().await;
    assert!(terminal_replay_bytes(&replay) <= MAX_WORKSPACE_TERMINAL_REPLAY_BYTES);
    assert!(replay.front().is_some_and(|frame| frame.seq > 1));
}

#[tokio::test]
async fn terminal_manager_shutdown_sends_close_and_joins_tasks() {
    let (control_tx, mut control_rx) = mpsc::unbounded_channel();
    let (closed_tx, closed_rx) = oneshot::channel();
    let task = tokio::spawn(async move {
        while let Some(control) = control_rx.recv().await {
            if matches!(control, WorkspaceTerminalControl::Close) {
                let _ = closed_tx.send(());
                break;
            }
        }
    });
    let route = Arc::new(RwLock::new(None));
    let mut manager = WorkspaceTerminalManager::default();
    manager.terminals.insert(
        "terminal-shutdown".to_owned(),
        WorkspaceTerminalHandle {
            replay: Arc::new(RwLock::new(VecDeque::new())),
            control_tx,
            sender_route: route,
            task,
        },
    );

    manager.shutdown().await;

    closed_rx.await.expect("terminal task observed close");
    assert!(manager.terminals.is_empty());
}
