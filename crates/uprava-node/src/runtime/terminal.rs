//! Workspace PTY ownership, replay and control-channel routing.

use super::*;

#[derive(Clone, Default)]
pub(crate) struct TerminalSupervisor {
    pub(crate) manager: Arc<Mutex<WorkspaceTerminalManager>>,
}

impl TerminalSupervisor {
    pub(crate) async fn rebind_sender(&self, sender: &ControlFrameSender) {
        self.manager.lock().await.rebind_sender(sender).await;
    }

    pub(crate) async fn detach_sender(&self) {
        self.manager.lock().await.detach_sender().await;
    }

    pub(crate) async fn open(
        &self,
        config: &NodeConfig,
        command: &CommandEnvelope,
        sender: &ControlFrameSender,
    ) -> Result<WorkspaceTerminalOpenResponse, WorkspaceInspectError> {
        self.manager
            .lock()
            .await
            .open(config, command, sender)
            .await
    }

    pub(crate) async fn attach(&self, sender: &ControlFrameSender, terminal_id: &TerminalId) {
        self.manager.lock().await.attach(sender, terminal_id).await;
    }

    pub(crate) async fn input(&self, terminal_id: &TerminalId, data: String) {
        self.manager.lock().await.input(terminal_id, data);
    }

    pub(crate) async fn resize(&self, terminal_id: &TerminalId, cols: u16, rows: u16) {
        self.manager.lock().await.resize(terminal_id, cols, rows);
    }

    pub(crate) async fn close(&self, terminal_id: &TerminalId) {
        self.manager.lock().await.close(terminal_id).await;
    }

    pub(crate) async fn shutdown(&self) {
        self.manager.lock().await.shutdown().await;
    }
}

#[derive(Default)]
pub(crate) struct WorkspaceTerminalManager {
    pub(crate) terminals: HashMap<String, WorkspaceTerminalHandle>,
}

pub(crate) struct WorkspaceTerminalHandle {
    pub(crate) replay: Arc<RwLock<VecDeque<WorkspaceTerminalOutputFrame>>>,
    pub(crate) control_tx: mpsc::UnboundedSender<WorkspaceTerminalControl>,
    pub(crate) sender_route: TerminalSenderRoute,
    pub(crate) task: tokio::task::JoinHandle<()>,
}

pub(crate) enum WorkspaceTerminalControl {
    Input(String),
    Resize { cols: u16, rows: u16 },
    Close,
}

impl WorkspaceTerminalManager {
    pub(crate) async fn rebind_sender(&self, sender: &ControlFrameSender) {
        for handle in self.terminals.values() {
            *handle.sender_route.write().await = Some(sender.clone());
        }
    }

    pub(crate) async fn detach_sender(&self) {
        for handle in self.terminals.values() {
            *handle.sender_route.write().await = None;
        }
    }

    pub(crate) async fn open(
        &mut self,
        config: &NodeConfig,
        command: &CommandEnvelope,
        sender: &ControlFrameSender,
    ) -> Result<WorkspaceTerminalOpenResponse, WorkspaceInspectError> {
        let request = workspace_command_payload::<WorkspaceTerminalOpenRequest>(command)?;
        let placement_id = workspace_command_placement_id(command)?;
        let workspace_root = canonical_workspace_root(config, workspace_command_path(command)?)?;
        let shell = select_workspace_shell(request.shell_profile.as_deref())?;
        let cols = request
            .cols
            .clamp(MIN_WORKSPACE_TERMINAL_COLS, MAX_WORKSPACE_TERMINAL_COLS);
        let rows = request
            .rows
            .clamp(MIN_WORKSPACE_TERMINAL_ROWS, MAX_WORKSPACE_TERMINAL_ROWS);
        let terminal_id = TerminalId::from(format!("terminal-{}", command.command_id.as_str()));
        let (pty, pts) = pty_process::open()
            .map_err(|error| workspace_terminal_error("workspace_terminal.open_failed", error))?;
        pty.resize(PtySize::new(rows, cols))
            .map_err(|error| workspace_terminal_error("workspace_terminal.resize_failed", error))?;
        let child = PtyCommand::new(&shell)
            .current_dir(&workspace_root)
            .kill_on_drop(true)
            .spawn(pts)
            .map_err(|error| workspace_terminal_error("workspace_terminal.spawn_failed", error))?;
        let (control_tx, control_rx) = mpsc::unbounded_channel();
        let replay = Arc::new(RwLock::new(VecDeque::new()));
        let sender_route = Arc::new(RwLock::new(Some(sender.clone())));
        let now = Utc::now();
        let summary = WorkspaceTerminalSummary {
            placement_id: placement_id.clone(),
            terminal_id: terminal_id.clone(),
            title: workspace_root
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("workspace")
                .to_owned(),
            cwd: workspace_root.display().to_string(),
            shell: shell.clone(),
            cols,
            rows,
            state: WorkspaceTerminalState::Running,
            exit_code: None,
            created_at: now,
            updated_at: now,
        };
        let task = tokio::spawn(run_workspace_terminal(
            pty,
            child,
            control_rx,
            sender_route.clone(),
            terminal_id.clone(),
            replay.clone(),
        ));
        self.terminals.insert(
            terminal_id.to_string(),
            WorkspaceTerminalHandle {
                replay: replay.clone(),
                control_tx,
                sender_route: sender_route.clone(),
                task,
            },
        );
        send_terminal_status(
            sender,
            &terminal_id,
            WorkspaceTerminalState::Running,
            None,
            Some("terminal started".to_owned()),
        )
        .await;
        Ok(WorkspaceTerminalOpenResponse {
            placement_id,
            terminal: summary,
            replay: vec![],
        })
    }

    pub(crate) async fn attach(&self, sender: &ControlFrameSender, terminal_id: &TerminalId) {
        let Some(handle) = self.terminals.get(terminal_id.as_str()) else {
            send_terminal_status(
                sender,
                terminal_id,
                WorkspaceTerminalState::Error,
                None,
                Some("terminal not found".to_owned()),
            )
            .await;
            return;
        };
        let replay = handle.replay.read().await;
        if replay.front().is_some_and(|frame| frame.seq > 1) {
            send_terminal_status(
                sender,
                terminal_id,
                WorkspaceTerminalState::Detached,
                None,
                Some("terminal replay gap; older output is no longer retained".to_owned()),
            )
            .await;
        }
        for frame in replay.iter() {
            let _ = send_frame(
                sender,
                ControlFrame::WorkspaceTerminalOutput {
                    frame_id: Uuid::new_v4().to_string(),
                    protocol_version: API_VERSION.to_owned(),
                    sent_at: frame.sent_at,
                    terminal_id: frame.terminal_id.clone(),
                    seq: frame.seq,
                    data: frame.data.clone(),
                },
            )
            .await;
        }
    }

    pub(crate) fn input(&self, terminal_id: &TerminalId, data: String) {
        if data.chars().count() > MAX_WORKSPACE_TERMINAL_INPUT_CHARS {
            return;
        }
        if let Some(handle) = self.terminals.get(terminal_id.as_str()) {
            let _ = handle
                .control_tx
                .send(WorkspaceTerminalControl::Input(data));
        }
    }

    pub(crate) fn resize(&self, terminal_id: &TerminalId, cols: u16, rows: u16) {
        if let Some(handle) = self.terminals.get(terminal_id.as_str()) {
            let _ = handle.control_tx.send(WorkspaceTerminalControl::Resize {
                cols: cols.clamp(MIN_WORKSPACE_TERMINAL_COLS, MAX_WORKSPACE_TERMINAL_COLS),
                rows: rows.clamp(MIN_WORKSPACE_TERMINAL_ROWS, MAX_WORKSPACE_TERMINAL_ROWS),
            });
        }
    }

    pub(crate) async fn close(&mut self, terminal_id: &TerminalId) {
        if let Some(handle) = self.terminals.remove(terminal_id.as_str()) {
            stop_terminal_handle(handle).await;
        }
    }

    pub(crate) async fn shutdown(&mut self) {
        let handles = self
            .terminals
            .drain()
            .map(|(_, handle)| handle)
            .collect::<Vec<_>>();
        for handle in handles {
            stop_terminal_handle(handle).await;
        }
    }
}

pub(crate) async fn run_workspace_terminal(
    mut pty: pty_process::Pty,
    mut child: tokio::process::Child,
    mut control_rx: mpsc::UnboundedReceiver<WorkspaceTerminalControl>,
    sender_route: TerminalSenderRoute,
    terminal_id: TerminalId,
    replay: Arc<RwLock<VecDeque<WorkspaceTerminalOutputFrame>>>,
) {
    let mut seq = 0_u64;
    let mut read_buffer = vec![0_u8; WORKSPACE_TERMINAL_READ_BYTES];
    loop {
        tokio::select! {
            control = control_rx.recv() => {
                let Some(control) = control else {
                    break;
                };
                match control {
                    WorkspaceTerminalControl::Input(data) => {
                        if pty.write_all(data.as_bytes()).await.is_err() {
                            break;
                        }
                        let _ = pty.flush().await;
                    }
                    WorkspaceTerminalControl::Resize { cols, rows } => {
                        if let Err(error) = pty.resize(PtySize::new(rows, cols)) {
                            send_terminal_status_via_route(
                                &sender_route,
                                &terminal_id,
                                WorkspaceTerminalState::Error,
                                None,
                                Some(format!("resize failed: {error}")),
                            )
                            .await;
                        }
                    }
                    WorkspaceTerminalControl::Close => {
                        let _ = child.start_kill();
                        let _ = child.wait().await;
                        send_terminal_status_via_route(
                            &sender_route,
                            &terminal_id,
                            WorkspaceTerminalState::Closed,
                            None,
                            Some("terminal closed".to_owned()),
                        )
                        .await;
                        return;
                    }
                }
            }
            read_result = pty.read(&mut read_buffer) => {
                match read_result {
                    Ok(0) => break,
                    Ok(read) => {
                        seq = seq.saturating_add(1);
                        let data = String::from_utf8_lossy(&read_buffer[..read]).into_owned();
                        let sent_at = Utc::now();
                        record_terminal_replay(
                            &replay,
                            WorkspaceTerminalOutputFrame {
                                terminal_id: terminal_id.clone(),
                                seq,
                                data: data.clone(),
                                sent_at,
                            },
                        ).await;
                        let _ = send_terminal_frame(
                            &sender_route,
                            ControlFrame::WorkspaceTerminalOutput {
                                frame_id: Uuid::new_v4().to_string(),
                                protocol_version: API_VERSION.to_owned(),
                                sent_at,
                                terminal_id: terminal_id.clone(),
                                seq,
                                data,
                            },
                        ).await;
                    }
                    Err(error) if error.kind() == ErrorKind::Interrupted => {}
                    Err(_) => break,
                }
            }
        }
    }
    let exit_code = child.wait().await.ok().and_then(|status| status.code());
    send_terminal_status_via_route(
        &sender_route,
        &terminal_id,
        WorkspaceTerminalState::Exited,
        exit_code,
        Some("terminal exited".to_owned()),
    )
    .await;
}

pub(crate) async fn send_terminal_frame(
    route: &TerminalSenderRoute,
    frame: ControlFrame,
) -> anyhow::Result<()> {
    let sender = route.read().await.clone();
    let Some(sender) = sender else {
        return Ok(());
    };
    send_frame(&sender, frame).await
}

pub(crate) async fn send_terminal_status_via_route(
    route: &TerminalSenderRoute,
    terminal_id: &TerminalId,
    state: WorkspaceTerminalState,
    exit_code: Option<i32>,
    message: Option<String>,
) {
    let _ = send_terminal_frame(
        route,
        ControlFrame::WorkspaceTerminalStatus {
            frame_id: Uuid::new_v4().to_string(),
            protocol_version: API_VERSION.to_owned(),
            sent_at: Utc::now(),
            terminal_id: terminal_id.clone(),
            state,
            exit_code,
            message,
        },
    )
    .await;
}

pub(crate) async fn record_terminal_replay(
    replay: &Arc<RwLock<VecDeque<WorkspaceTerminalOutputFrame>>>,
    frame: WorkspaceTerminalOutputFrame,
) {
    let mut replay = replay.write().await;
    replay.push_back(frame);
    while replay.len() > MAX_WORKSPACE_TERMINAL_REPLAY_FRAMES
        || terminal_replay_bytes(&replay) > MAX_WORKSPACE_TERMINAL_REPLAY_BYTES
    {
        replay.pop_front();
    }
}

pub(crate) fn terminal_replay_bytes(replay: &VecDeque<WorkspaceTerminalOutputFrame>) -> usize {
    replay.iter().map(|frame| frame.data.len()).sum()
}

pub(crate) async fn stop_terminal_handle(handle: WorkspaceTerminalHandle) {
    let _ = handle.control_tx.send(WorkspaceTerminalControl::Close);
    join_terminal_task(handle.task).await;
}

pub(crate) async fn join_terminal_task(mut task: tokio::task::JoinHandle<()>) {
    tokio::select! {
        result = &mut task => {
            if let Err(error) = result {
                tracing::warn!(error = %error, "workspace terminal task failed");
            }
        }
        _ = tokio::time::sleep(WORKSPACE_TERMINAL_SHUTDOWN_TIMEOUT) => {
            task.abort();
            let _ = task.await;
            tracing::warn!("workspace terminal task aborted after shutdown timeout");
        }
    }
}

pub(crate) async fn send_terminal_status(
    sender: &ControlFrameSender,
    terminal_id: &TerminalId,
    state: WorkspaceTerminalState,
    exit_code: Option<i32>,
    message: Option<String>,
) {
    let _ = send_frame(
        sender,
        ControlFrame::WorkspaceTerminalStatus {
            frame_id: Uuid::new_v4().to_string(),
            protocol_version: API_VERSION.to_owned(),
            sent_at: Utc::now(),
            terminal_id: terminal_id.clone(),
            state,
            exit_code,
            message,
        },
    )
    .await;
}

pub(crate) fn workspace_terminal_error(
    code: &'static str,
    error: impl std::fmt::Display,
) -> WorkspaceInspectError {
    WorkspaceInspectError::new(code, format!("Workspace terminal failed: {error}"))
}

pub(crate) fn select_workspace_shell(
    profile: Option<&str>,
) -> Result<String, WorkspaceInspectError> {
    match profile.unwrap_or("default").trim() {
        "" | "default" => Ok(default_workspace_shell()),
        "sh" => Ok(shell_path("sh")),
        "bash" => Ok(shell_path("bash")),
        "zsh" => Ok(shell_path("zsh")),
        _ => Err(WorkspaceInspectError::new(
            "workspace_terminal.shell_denied",
            "Workspace terminal shell profile is not allowed by node policy",
        )),
    }
}

pub(crate) fn default_workspace_shell() -> String {
    std::env::var("SHELL")
        .ok()
        .and_then(|shell| {
            let name = Path::new(&shell).file_name()?.to_str()?;
            matches!(name, "sh" | "bash" | "zsh").then_some(shell)
        })
        .unwrap_or_else(|| shell_path("sh"))
}

pub(crate) fn shell_path(name: &str) -> String {
    ["/bin", "/usr/bin"]
        .iter()
        .map(|prefix| Path::new(prefix).join(name))
        .find(|path| path.exists())
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| name.to_owned())
}
