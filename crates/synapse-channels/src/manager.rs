use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinHandle;

use crate::{ChannelEvent, Connector, ConnectorConfig, ConnectorStatus};

pub struct ChannelHandle {
    pub name: String,
    pub source_type: String,
    pub connector: Arc<RwLock<Box<dyn Connector>>>,
    task: Option<JoinHandle<()>>,
}

pub struct ChannelManager {
    channels: HashMap<String, ChannelHandle>,
    event_tx: mpsc::UnboundedSender<ChannelEvent>,
    event_rx: Option<mpsc::UnboundedReceiver<ChannelEvent>>,
}

impl ChannelManager {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            channels: HashMap::new(),
            event_tx: tx,
            event_rx: Some(rx),
        }
    }

    pub fn take_receiver(&mut self) -> Option<mpsc::UnboundedReceiver<ChannelEvent>> {
        self.event_rx.take()
    }

    pub async fn register(
        &mut self,
        name: String,
        mut connector: Box<dyn Connector>,
        config: &ConnectorConfig,
    ) -> anyhow::Result<()> {
        connector.connect(config).await?;
        let source_type = connector.source_type().to_string();
        tracing::info!(
            channel = %name,
            source = %source_type,
            "channel connector registered"
        );
        self.channels.insert(
            name.clone(),
            ChannelHandle {
                name,
                source_type,
                connector: Arc::new(RwLock::new(connector)),
                task: None,
            },
        );
        Ok(())
    }

    pub fn start_polling(&mut self, channel_name: &str, interval: Duration) {
        if let Some(handle) = self.channels.get_mut(channel_name) {
            let connector = handle.connector.clone();
            let tx = self.event_tx.clone();
            let name = channel_name.to_string();

            let task = tokio::spawn(async move {
                let mut tick = tokio::time::interval(interval);
                loop {
                    tick.tick().await;
                    let conn = connector.read().await;
                    match conn.poll().await {
                        Ok(events) => {
                            for event in events {
                                if tx.send(event).is_err() {
                                    tracing::warn!(channel = %name, "event receiver dropped");
                                    return;
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                channel = %name,
                                error = %e,
                                "polling failed"
                            );
                        }
                    }
                }
            });

            handle.task = Some(task);
            tracing::info!(
                channel = %channel_name,
                interval_secs = interval.as_secs(),
                "started polling"
            );
        }
    }

    pub fn start_all(&mut self, default_interval: Duration) {
        let names: Vec<String> = self.channels.keys().cloned().collect();
        for name in names {
            self.start_polling(&name, default_interval);
        }
    }

    pub async fn stop_all(&mut self) {
        for (name, handle) in &mut self.channels {
            if let Some(task) = handle.task.take() {
                task.abort();
                tracing::info!(channel = %name, "stopped polling");
            }
            let mut conn = handle.connector.write().await;
            let _ = conn.disconnect().await;
        }
    }

    pub fn channel_names(&self) -> Vec<&str> {
        self.channels.keys().map(|s| s.as_str()).collect()
    }

    pub async fn channel_status(&self) -> HashMap<String, ConnectorStatus> {
        let mut statuses = HashMap::new();
        for (name, handle) in &self.channels {
            let conn = handle.connector.read().await;
            statuses.insert(name.clone(), conn.status());
        }
        statuses
    }
}

impl Default for ChannelManager {
    fn default() -> Self {
        Self::new()
    }
}
