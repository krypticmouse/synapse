use synapse_dsl::ast::*;

use super::handler::exec_stmts;
use super::ExecEnv;
use crate::storage::QueryFilter;
use crate::value::Value;

/// Execute update rules for a given memory type.
/// Called periodically by the policy scheduler for `every` rules,
/// and reactively for `on_access` / `on_conflict` rules.
pub async fn exec_on_access(
    env: &mut ExecEnv,
    update: &UpdateDef,
    record_id: &str,
) -> anyhow::Result<()> {
    for rule in &update.rules {
        if let UpdateRule::OnAccess { body } = rule {
            if let Some(record) = env.storage.get(&update.target, record_id).await? {
                env.push_scope();
                env.set("_update_target", Value::String(update.target.clone()));
                for (name, value) in &record.fields {
                    env.set(name, value.clone());
                }
                env.set("id", Value::String(record.id.clone()));

                exec_stmts(env, body).await?;

                let mut updated_record = record;
                for (name, _) in &updated_record.fields.clone() {
                    let new_val = env.get(name);
                    if new_val != Value::Null {
                        updated_record.set(name, new_val);
                    }
                }
                env.storage.store(&updated_record).await?;

                env.pop_scope();
            }
        }
    }
    Ok(())
}

/// Execute on_conflict rule when a conflicting record is detected.
pub async fn exec_on_conflict(
    env: &mut ExecEnv,
    update: &UpdateDef,
    old_id: &str,
    new_record: &crate::value::Record,
) -> anyhow::Result<()> {
    for rule in &update.rules {
        if let UpdateRule::OnConflict {
            old_name,
            new_name,
            body,
        } = rule
        {
            if let Some(old_record) = env.storage.get(&update.target, old_id).await? {
                env.push_scope();
                env.set("_update_target", Value::String(update.target.clone()));
                env.set(old_name, Value::Record(old_record));
                env.set(new_name, Value::Record(new_record.clone()));

                exec_stmts(env, body).await?;

                env.pop_scope();
            }
        }
    }
    Ok(())
}

/// Execute periodic `every` rules for a memory type.
pub async fn exec_every(env: &mut ExecEnv, update: &UpdateDef) -> anyhow::Result<()> {
    for rule in &update.rules {
        if let UpdateRule::Every { body, .. } = rule {
            tracing::info!(target = %update.target, "exec_every: querying records");
            let records = env
                .storage
                .query(&update.target, &QueryFilter::default())
                .await?;

            tracing::info!(target = %update.target, count = records.len(), "exec_every: found {} records", records.len());

            for record in records {
                env.push_scope();
                env.set("_update_target", Value::String(update.target.clone()));
                for (name, value) in &record.fields {
                    env.set(name, value.clone());
                }
                env.set("id", Value::String(record.id.clone()));

                exec_stmts(env, body).await?;

                let mut updated = record;
                for (name, _) in &updated.fields.clone() {
                    let new_val = env.get(name);
                    if new_val != Value::Null {
                        updated.set(name, new_val);
                    }
                }
                env.storage.store(&updated).await?;

                env.pop_scope();
            }
        }
    }
    Ok(())
}
