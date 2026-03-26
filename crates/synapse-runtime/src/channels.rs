use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use synapse_channels::ChannelEvent;
use synapse_channels::manager::ChannelManager;
use synapse_channels::registry;
use synapse_dsl::ast::ChannelDef;
use tokio::sync::RwLock;

use crate::config::ChannelRuntimeConfig;
use crate::interpreter::Runtime;
use crate::value::Value;

pub async fn setup_channels(
    channel_configs: &HashMap<String, ChannelRuntimeConfig>,
) -> anyhow::Result<ChannelManager> {
    let mut manager = ChannelManager::new();

    for (name, ch_cfg) in channel_configs {
        let connector = registry::create_connector(&ch_cfg.source)
            .ok_or_else(|| anyhow::anyhow!(
                "unknown channel source '{}' for channel '{}'",
                ch_cfg.source,
                name
            ))?;

        manager
            .register(name.clone(), connector, &ch_cfg.config)
            .await?;

        println!("  \u{2713} Channel [{name}] connected via {}", ch_cfg.source);
    }

    Ok(manager)
}

pub fn start_channel_polling(
    manager: &mut ChannelManager,
    channel_configs: &HashMap<String, ChannelRuntimeConfig>,
) {
    for (name, ch_cfg) in channel_configs {
        let interval = Duration::from_secs(ch_cfg.poll_interval_secs);
        manager.start_polling(name, interval);
    }
}

pub fn spawn_event_dispatcher(
    runtime: Arc<RwLock<Runtime>>,
    mut rx: tokio::sync::mpsc::UnboundedReceiver<ChannelEvent>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let rt = runtime.read().await;

            let channel_defs: Vec<ChannelDef> = {
                fn collect_channels(items: &[synapse_dsl::ast::Item]) -> Vec<ChannelDef> {
                    let mut defs = Vec::new();
                    for item in items {
                        match item {
                            synapse_dsl::ast::Item::Channel(ch) => defs.push(ch.clone()),
                            synapse_dsl::ast::Item::Namespace(ns) => {
                                defs.extend(collect_channels(&ns.items));
                            }
                            _ => {}
                        }
                    }
                    defs
                }
                collect_channels(&rt.program.items)
            };

            for ch_def in &channel_defs {
                if ch_def.name != event.channel_name {
                    continue;
                }
                for handler in &ch_def.events {
                    if handler.event != event.event_type.as_str() {
                        continue;
                    }

                    let mut payload = serde_json::Map::new();
                    payload.insert(
                        "content".to_string(),
                        serde_json::Value::String(event.content.clone()),
                    );
                    if let Some(ref author) = event.author {
                        payload.insert(
                            "author".to_string(),
                            serde_json::Value::String(author.clone()),
                        );
                    }
                    if let Some(ref msg_id) = event.message_id {
                        payload.insert(
                            "message_id".to_string(),
                            serde_json::Value::String(msg_id.clone()),
                        );
                    }
                    payload.insert(
                        "source".to_string(),
                        serde_json::Value::String(event.source.clone()),
                    );
                    payload.insert(
                        "timestamp".to_string(),
                        serde_json::Value::String(event.timestamp.to_rfc3339()),
                    );
                    for (k, v) in &event.metadata {
                        payload.insert(k.clone(), v.clone());
                    }

                    let target_event = handler.target.as_deref();

                    match target_event {
                        Some("ingest") | None => {
                            if let Some(_ingest_handler) = rt.handlers.get("ingest") {
                                let json_payload =
                                    serde_json::Value::Object(payload.clone());
                                if let Err(e) = rt.emit("ingest", json_payload).await {
                                    tracing::error!(
                                        channel = %ch_def.name,
                                        event = %handler.event,
                                        error = %e,
                                        "channel event dispatch to ingest failed"
                                    );
                                } else {
                                    tracing::info!(
                                        channel = %ch_def.name,
                                        event = %handler.event,
                                        "dispatched to ingest"
                                    );
                                }
                            } else {
                                tracing::warn!(
                                    channel = %ch_def.name,
                                    "no ingest handler defined, executing channel handler body directly"
                                );
                                let mut env = crate::interpreter::ExecEnv::new(
                                    rt.storage.clone(),
                                    rt.llm.clone(),
                                    rt.embedder.clone(),
                                    rt.handlers.clone(),
                                    rt.extern_fns.clone(),
                                )
                                .with_queries(rt.queries.clone())
                                .with_updates(rt.updates.clone())
                                .with_memories(rt.memories.clone());

                                for param in &handler.params {
                                    if let Some(val) = payload.get(&param.name) {
                                        env.set(
                                            &param.name,
                                            Value::from(
                                                serde_json::Value::from(val.clone()),
                                            ),
                                        );
                                    }
                                }

                                if let Err(e) = crate::interpreter::handler::exec_stmts(
                                    &mut env,
                                    &handler.body,
                                )
                                .await
                                {
                                    tracing::error!(
                                        channel = %ch_def.name,
                                        event = %handler.event,
                                        error = %e,
                                        "channel handler execution failed"
                                    );
                                }
                            }
                        }
                        Some("update") => {
                            let json_payload =
                                serde_json::Value::Object(payload.clone());
                            if let Err(e) = rt.emit("ingest", json_payload).await {
                                tracing::error!(
                                    channel = %ch_def.name,
                                    event = %handler.event,
                                    error = %e,
                                    "channel event dispatch to update failed"
                                );
                            } else {
                                tracing::info!(
                                    channel = %ch_def.name,
                                    event = %handler.event,
                                    "dispatched to ingest (triggers update via on_conflict)"
                                );
                            }
                        }
                        Some(other) => {
                            if let Err(e) = rt
                                .emit(other, serde_json::Value::Object(payload.clone()))
                                .await
                            {
                                tracing::error!(
                                    channel = %ch_def.name,
                                    event = %handler.event,
                                    target = other,
                                    error = %e,
                                    "channel event dispatch failed"
                                );
                            }
                        }
                    }
                }
            }
        }
    })
}
