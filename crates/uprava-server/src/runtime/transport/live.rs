//! Process-local Core control connections, command waiters and terminal fan-out.

use super::super::*;

#[derive(Clone)]
pub(crate) struct NodeContext {
    pub(crate) node_id: NodeId,
    pub(crate) generation: u64,
    pub(crate) sender: mpsc::Sender<ControlFrame>,
}

#[derive(Clone)]
pub(crate) struct ControlConnection {
    pub(crate) generation: u64,
    pub(crate) sender: mpsc::Sender<ControlFrame>,
}

pub(crate) type CommandEventContextRow = (String, Option<String>, Option<String>, Option<String>);

pub(crate) struct ConnectionRegistry {
    pub(crate) entries: RwLock<HashMap<String, ControlConnection>>,
    pub(crate) next_generation: AtomicU64,
}

pub(crate) struct TerminalHub {
    pub(crate) channels: RwLock<HashMap<String, broadcast::Sender<TerminalFrameNotice>>>,
}

pub(crate) struct CommandWaiterGuard<'a> {
    pub(crate) state: &'a AppState,
    pub(crate) command_id: CommandId,
}

impl<'a> CommandWaiterGuard<'a> {
    pub(crate) fn new(state: &'a AppState, command_id: CommandId) -> Self {
        Self { state, command_id }
    }
}

impl Drop for CommandWaiterGuard<'_> {
    fn drop(&mut self) {
        match self.state.command_waiters.lock() {
            Ok(mut waiters) => {
                waiters.remove(self.command_id.as_str());
            }
            Err(error) => {
                tracing::error!(
                    error = %error,
                    "command waiter registry lock poisoned during cleanup"
                );
            }
        }
    }
}

pub(crate) fn lock_command_waiters(
    state: &AppState,
) -> Result<
    std::sync::MutexGuard<'_, HashMap<String, oneshot::Sender<CommandResultNotice>>>,
    AppError,
> {
    state.command_waiters.lock().map_err(|error| {
        AppError::internal(format!("command waiter registry lock poisoned: {error}"))
    })
}

pub(crate) fn command_waiter_exists(
    state: &AppState,
    command_id: &CommandId,
) -> Result<bool, AppError> {
    Ok(lock_command_waiters(state)?.contains_key(command_id.as_str()))
}

impl TerminalHub {
    pub(crate) fn new() -> Self {
        Self {
            channels: RwLock::new(HashMap::new()),
        }
    }

    pub(crate) async fn channel(
        &self,
        terminal_id: &TerminalId,
    ) -> broadcast::Sender<TerminalFrameNotice> {
        if let Some(sender) = self.channels.read().await.get(terminal_id.as_str()) {
            return sender.clone();
        }
        let mut channels = self.channels.write().await;
        channels
            .entry(terminal_id.to_string())
            .or_insert_with(|| broadcast::channel(1024).0)
            .clone()
    }

    pub(crate) async fn subscribe(
        &self,
        terminal_id: &TerminalId,
    ) -> broadcast::Receiver<TerminalFrameNotice> {
        self.channel(terminal_id).await.subscribe()
    }

    pub(crate) async fn publish(&self, terminal_id: &TerminalId, notice: TerminalFrameNotice) {
        let _ = self.channel(terminal_id).await.send(notice);
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(crate) enum ControlSendError {
    #[error("Node control connection is unavailable")]
    Unavailable,
    #[error("Node control queue is saturated")]
    Saturated,
    #[error("Node control queue is closed")]
    Closed,
}

impl ConnectionRegistry {
    pub(crate) fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            next_generation: AtomicU64::new(1),
        }
    }

    pub(crate) fn context(
        &self,
        node_id: NodeId,
        sender: mpsc::Sender<ControlFrame>,
    ) -> NodeContext {
        NodeContext {
            node_id,
            generation: self.next_generation.fetch_add(1, Ordering::Relaxed),
            sender,
        }
    }

    pub(crate) async fn activate(&self, context: &NodeContext) -> bool {
        let mut entries = self.entries.write().await;
        if entries
            .get(context.node_id.as_str())
            .is_some_and(|connection| connection.generation > context.generation)
        {
            return false;
        }
        entries.insert(
            context.node_id.to_string(),
            ControlConnection {
                generation: context.generation,
                sender: context.sender.clone(),
            },
        );
        true
    }

    pub(crate) async fn is_active(&self, context: &NodeContext) -> bool {
        self.entries
            .read()
            .await
            .get(context.node_id.as_str())
            .is_some_and(|connection| connection.generation == context.generation)
    }

    pub(crate) async fn sender(&self, node_id: &NodeId) -> Option<mpsc::Sender<ControlFrame>> {
        self.entries
            .read()
            .await
            .get(node_id.as_str())
            .map(|connection| connection.sender.clone())
    }

    pub(crate) async fn contains(&self, node_id: &NodeId) -> bool {
        self.entries.read().await.contains_key(node_id.as_str())
    }

    pub(crate) async fn remove_if_active(&self, context: &NodeContext) -> bool {
        let mut entries = self.entries.write().await;
        let is_active = entries
            .get(context.node_id.as_str())
            .is_some_and(|connection| connection.generation == context.generation);
        if is_active {
            entries.remove(context.node_id.as_str());
        }
        is_active
    }

    pub(crate) async fn remove_node(&self, node_id: &NodeId) {
        self.entries.write().await.remove(node_id.as_str());
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CommandResultNotice {
    pub(crate) command_id: CommandId,
    pub(crate) status: CommandState,
    pub(crate) payload: JsonValue,
}

#[derive(Debug, Clone)]
pub(crate) enum TerminalFrameNotice {
    Output {
        terminal_id: TerminalId,
        seq: u64,
        data: String,
        sent_at: DateTime<Utc>,
    },
    Status {
        terminal_id: TerminalId,
        state: WorkspaceTerminalState,
        exit_code: Option<i32>,
        message: Option<String>,
        sent_at: DateTime<Utc>,
    },
}
