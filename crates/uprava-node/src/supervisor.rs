use super::{
    config::NodeConfig, control_channel_loop, heartbeat_auth_rejected, ManagedRuntimeSupervisor,
    NodeStateStore, TerminalSupervisor,
};

/// Owns long-lived Node tasks and joins their shutdown boundary.
pub(crate) struct NodeSupervisor {
    config: NodeConfig,
    client: reqwest::Client,
    state_store: NodeStateStore,
    terminal_supervisor: TerminalSupervisor,
    managed_supervisor: ManagedRuntimeSupervisor,
    control_task: Option<tokio::task::JoinHandle<()>>,
}

impl NodeSupervisor {
    pub(crate) fn new(
        config: NodeConfig,
        client: reqwest::Client,
        state_store: NodeStateStore,
    ) -> Self {
        Self {
            config,
            client,
            state_store,
            terminal_supervisor: TerminalSupervisor::default(),
            managed_supervisor: ManagedRuntimeSupervisor::default(),
            control_task: None,
        }
    }

    pub(crate) async fn run(mut self) -> anyhow::Result<()> {
        if let Err(error) =
            super::reconcile_task_runtime_mappings(&self.config, &self.client, &self.state_store)
                .await
        {
            tracing::warn!(error = %error, "task runtime recovery reconciliation failed");
        }
        loop {
            let enrolled = match self.state_store.is_enrolled().await {
                Ok(enrolled) => enrolled,
                Err(error) => {
                    tracing::warn!(error = %error, "state store enrollment check failed");
                    if self.sleep_or_shutdown().await? {
                        break;
                    }
                    continue;
                }
            };
            if !enrolled {
                match self
                    .state_store
                    .ensure_enrollment(&self.client, &self.config)
                    .await
                {
                    Ok(true) => {}
                    Ok(false) => {
                        if self.sleep_or_shutdown().await? {
                            break;
                        }
                        continue;
                    }
                    Err(error) => {
                        tracing::warn!(error = %error, "enrollment step failed");
                        if self.sleep_or_shutdown().await? {
                            break;
                        }
                        continue;
                    }
                }
            }

            if self
                .control_task
                .as_ref()
                .is_some_and(tokio::task::JoinHandle::is_finished)
            {
                self.control_task = None;
            }

            match self
                .state_store
                .send_heartbeat(&self.client, &self.config)
                .await
            {
                Ok(response) => {
                    tracing::info!(
                        accepted = response.accepted,
                        open_control_channel = response.open_control_channel,
                        "heartbeat accepted"
                    );
                    if response.open_control_channel && self.control_task.is_none() {
                        self.control_task = Some(tokio::spawn(control_channel_loop(
                            self.config.clone(),
                            self.client.clone(),
                            self.state_store.clone(),
                            self.terminal_supervisor.clone(),
                            self.managed_supervisor.clone(),
                        )));
                    }
                }
                Err(error) => {
                    if let Err(metric_error) = self.state_store.persist_heartbeat_failure().await {
                        tracing::warn!(
                            error = %metric_error,
                            "failed to persist heartbeat failure metric"
                        );
                    }
                    if heartbeat_auth_rejected(&error) {
                        tracing::warn!(
                            error = %error,
                            "heartbeat auth rejected; clearing local node identity and re-enrolling"
                        );
                        if let Some(task) = self.control_task.take() {
                            task.abort();
                        }
                        if let Err(save_error) = self.state_store.clear_core_registration().await {
                            tracing::warn!(
                                error = %save_error,
                                "failed to persist cleared node identity"
                            );
                        }
                    } else {
                        tracing::warn!(error = %error, "heartbeat failed");
                    }
                }
            }
            if self.sleep_or_shutdown().await? {
                break;
            }
        }
        Ok(())
    }

    async fn sleep_or_shutdown(&mut self) -> anyhow::Result<bool> {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                self.shutdown().await?;
                Ok(true)
            }
            _ = tokio::time::sleep(self.config.heartbeat_interval) => Ok(false),
        }
    }

    async fn shutdown(&mut self) -> anyhow::Result<()> {
        tracing::info!("shutdown signal received; stopping uprava node");
        if let Some(task) = self.control_task.take() {
            task.abort();
            let _ = task.await;
        }
        self.terminal_supervisor.shutdown().await;
        self.managed_supervisor.shutdown().await;
        self.state_store.shutdown().await
    }
}
