use std::collections::HashMap;
use std::sync::Arc;

use synapse_core::ast::*;
use tokio::sync::RwLock;

use super::update;
use super::{ExecEnv, Runtime};
use crate::storage::StorageManager;

/// Policy scheduler — runs periodic `every` rules on a timer.
pub struct PolicyScheduler {
    /// Update definitions with `every` rules: (target, interval_secs, UpdateDef)
    periodic_rules: Vec<(String, u64, UpdateDef)>,
    storage: Arc<StorageManager>,
    running: Arc<RwLock<bool>>,
}

impl PolicyScheduler {
    pub fn from_program(
        program: &synapse_core::ast::Program,
        storage: Arc<StorageManager>,
    ) -> Self {
        let mut periodic_rules = Vec::new();

        collect_periodic_rules(&program.items, &mut periodic_rules);

        Self {
            periodic_rules,
            storage,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start all periodic rules as background tasks.
    /// Returns a JoinHandle that can be used to stop the scheduler.
    pub fn start(&self) -> Vec<tokio::task::JoinHandle<()>> {
        let mut handles = Vec::new();

        for (target, interval_secs, update_def) in &self.periodic_rules {
            let storage = self.storage.clone();
            let running = self.running.clone();
            let interval = std::time::Duration::from_secs(*interval_secs);
            let update_def = update_def.clone();

            let handle = tokio::spawn(async move {
                let mut ticker = tokio::time::interval(interval);
                loop {
                    ticker.tick().await;

                    if !*running.read().await {
                        break;
                    }

                    let mut env = ExecEnv::new(storage.clone());
                    if let Err(e) = update::exec_every(&mut env, &update_def).await {
                        tracing::error!(
                            error = %e,
                            target = %update_def.target,
                            "periodic update rule failed"
                        );
                    }
                }
            });

            handles.push(handle);
            tracing::info!(
                target = %target,
                interval_secs = %interval_secs,
                "scheduled periodic update rule"
            );
        }

        handles
    }

    pub async fn stop(&self) {
        *self.running.write().await = false;
    }
}

fn collect_periodic_rules(items: &[Item], rules: &mut Vec<(String, u64, UpdateDef)>) {
    for item in items {
        match item {
            Item::Update(u) => {
                for rule in &u.rules {
                    if let UpdateRule::Every { interval, .. } = rule {
                        rules.push((u.target.clone(), interval.to_secs(), u.clone()));
                    }
                }
            }
            Item::Policy(p) => {
                // Policies can contain `every` rules too
                let pseudo_update = UpdateDef {
                    target: p.name.clone(),
                    rules: p.rules.clone(),
                };
                for rule in &p.rules {
                    if let UpdateRule::Every { interval, .. } = rule {
                        rules.push((p.name.clone(), interval.to_secs(), pseudo_update.clone()));
                    }
                }
            }
            Item::Namespace(ns) => {
                collect_periodic_rules(&ns.items, rules);
            }
            _ => {}
        }
    }
}
