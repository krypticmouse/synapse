use chrono::Utc;
use synapse_core::ast::*;

use super::ExecEnv;
use crate::value::{Record, Value};

/// Execute a list of statements in the given environment.
pub fn exec_stmts<'a>(
    env: &'a mut ExecEnv,
    stmts: &'a [Stmt],
) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<Option<Value>>> + Send + 'a>>
{
    Box::pin(async move {
        for stmt in stmts {
            if let Some(val) = exec_stmt(env, stmt).await? {
                return Ok(Some(val));
            }
        }
        Ok(None)
    })
}

async fn exec_stmt(env: &mut ExecEnv, stmt: &Stmt) -> anyhow::Result<Option<Value>> {
    match stmt {
        Stmt::Let { name, value } => {
            let val = eval_expr(env, value).await?;
            env.set(name, val);
            Ok(None)
        }
        Stmt::Assign { target, value } => {
            let val = eval_expr(env, value).await?;
            match target {
                Expr::Ident(name) => {
                    env.set(name, val);
                }
                Expr::FieldAccess { object, field } => {
                    if let Expr::Ident(obj_name) = object.as_ref() {
                        if let Value::Record(mut record) = env.get(obj_name) {
                            record.set(field, val);
                            env.set(obj_name, Value::Record(record));
                        }
                    }
                }
                _ => {}
            }
            Ok(None)
        }
        Stmt::If {
            condition,
            then_body,
            else_body,
        } => {
            let cond = eval_expr(env, condition).await?;
            if cond.is_truthy() {
                env.push_scope();
                let r = exec_stmts(env, then_body).await?;
                env.pop_scope();
                Ok(r)
            } else if let Some(else_body) = else_body {
                env.push_scope();
                let r = exec_stmts(env, else_body).await?;
                env.pop_scope();
                Ok(r)
            } else {
                Ok(None)
            }
        }
        Stmt::For { var, iter, body } => {
            let iter_val = eval_expr(env, iter).await?;
            if let Value::Array(items) = iter_val {
                for item in items {
                    env.push_scope();
                    env.set(var, item);
                    if let Some(val) = exec_stmts(env, body).await? {
                        env.pop_scope();
                        return Ok(Some(val));
                    }
                    env.pop_scope();
                }
            }
            Ok(None)
        }
        Stmt::Return(expr) => {
            let val = match expr {
                Some(e) => eval_expr(env, e).await?,
                None => Value::Null,
            };
            Ok(Some(val))
        }
        Stmt::Expr(expr) => {
            eval_expr(env, expr).await?;
            Ok(None)
        }
    }
}

/// Evaluate an expression, returning a runtime Value.
pub fn eval_expr<'a>(
    env: &'a mut ExecEnv,
    expr: &'a Expr,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<Value>> + Send + 'a>> {
    Box::pin(eval_expr_inner(env, expr))
}

async fn eval_expr_inner(env: &mut ExecEnv, expr: &Expr) -> anyhow::Result<Value> {
    match expr {
        Expr::Int(n) => Ok(Value::Int(*n)),
        Expr::Float(f) => Ok(Value::Float(*f)),
        Expr::Str(s) => Ok(Value::String(s.clone())),
        Expr::Bool(b) => Ok(Value::Bool(*b)),
        Expr::Null => Ok(Value::Null),
        Expr::Duration(d) => Ok(Value::Int(d.to_secs() as i64)),

        Expr::Ident(name) => Ok(env.get(name)),

        Expr::FieldAccess { object, field } => {
            let obj = eval_expr(env, object).await?;
            match obj {
                Value::Record(r) => Ok(r.get(field).cloned().unwrap_or(Value::Null)),
                _ => Ok(Value::Null),
            }
        }

        Expr::OptionalChain { object, field } => {
            let obj = eval_expr(env, object).await?;
            match obj {
                Value::Null => Ok(Value::Null),
                Value::Record(r) => Ok(r.get(field).cloned().unwrap_or(Value::Null)),
                _ => Ok(Value::Null),
            }
        }

        Expr::IndexAccess { object, index } => {
            let obj = eval_expr(env, object).await?;
            let idx = eval_expr(env, index).await?;
            match (obj, idx) {
                (Value::Array(a), Value::Int(i)) => {
                    Ok(a.get(i as usize).cloned().unwrap_or(Value::Null))
                }
                _ => Ok(Value::Null),
            }
        }

        Expr::Binary { left, op, right } => {
            let l = eval_expr(env, left).await?;
            let r = eval_expr(env, right).await?;
            Ok(eval_binop(&l, op, &r))
        }

        Expr::Unary { op, operand } => {
            let val = eval_expr(env, operand).await?;
            match op {
                UnaryOp::Neg => match val {
                    Value::Int(n) => Ok(Value::Int(-n)),
                    Value::Float(f) => Ok(Value::Float(-f)),
                    _ => Ok(Value::Null),
                },
                UnaryOp::Not => Ok(Value::Bool(!val.is_truthy())),
            }
        }

        Expr::Call { func, args } => eval_call(env, func, args).await,

        Expr::Pipe { left, right } => {
            let left_val = eval_expr(env, left).await?;
            // Pipe: left |> right(args) → right(left, args)
            match right.as_ref() {
                Expr::Call { func, args } => {
                    let mut new_args = vec![CallArg {
                        name: None,
                        value: Expr::Null, // placeholder
                    }];
                    new_args.extend(args.iter().cloned());
                    // Evaluate the function call with left_val prepended
                    eval_piped_call(env, func, left_val, args).await
                }
                _ => {
                    // right is a bare function name: right(left)
                    eval_piped_call(env, right, left_val, &[]).await
                }
            }
        }

        Expr::StructInit { name, fields } => {
            let mut record = Record::new(name.as_str());
            for fi in fields {
                let val = eval_expr(env, &fi.value).await?;
                record.set(&fi.name, val);
            }
            Ok(Value::Record(record))
        }

        Expr::Lambda { .. } => {
            // Lambdas are evaluated lazily when called (e.g., in map/filter)
            Ok(Value::Null)
        }

        Expr::Array(elems) => {
            let mut arr = Vec::with_capacity(elems.len());
            for e in elems {
                arr.push(eval_expr(env, e).await?);
            }
            Ok(Value::Array(arr))
        }

        Expr::InlineQuery(qb) => {
            let results = super::query::exec_query_body(env, qb).await?;
            Ok(Value::Array(results))
        }
    }
}

/// Evaluate a function call
async fn eval_call(env: &mut ExecEnv, func: &Expr, args: &[CallArg]) -> anyhow::Result<Value> {
    // Get function name
    let func_name = match func {
        Expr::Ident(name) => name.as_str(),
        _ => return Ok(Value::Null),
    };

    // Evaluate arguments
    let mut arg_values = Vec::new();
    for arg in args {
        arg_values.push(eval_expr(env, &arg.value).await?);
    }

    // Built-in functions
    match func_name {
        "now" => Ok(Value::Timestamp(Utc::now())),

        "store" => {
            if let Some(Value::Record(record)) = arg_values.first() {
                let invariants = env
                    .memories
                    .get(&record.type_name)
                    .map(|m| m.invariants.clone())
                    .unwrap_or_default();
                let type_name = record.type_name.clone();
                for invariant in &invariants {
                    env.push_scope();
                    for (field_name, field_val) in &record.fields {
                        env.set(field_name, field_val.clone());
                    }
                    let result = eval_expr(env, invariant).await?;
                    env.pop_scope();
                    if !result.is_truthy() {
                        anyhow::bail!(
                            "invariant violation in '{}': condition evaluated to false",
                            type_name
                        );
                    }
                }
                let conflict_handled = try_on_conflict(env, record).await?;
                if !conflict_handled {
                    env.storage.store(record).await?;
                    env.stored_count += 1;
                }
            }
            Ok(Value::Null)
        }

        "delete" => {
            if let Some(Value::Record(record)) = arg_values.first() {
                env.storage.delete(&record.type_name, &record.id).await?;
            } else {
                let type_name = env.get("_update_target");
                let id = env.get("id");
                if let (Value::String(t), Value::String(i)) = (type_name, id) {
                    env.storage.delete(&t, &i).await?;
                }
            }
            Ok(Value::Null)
        }

        "archive" => {
            let type_name = env.get("_update_target");
            let id = env.get("id");
            if let (Value::String(t), Value::String(i)) = (type_name, id) {
                if let Some(mut record) = env.storage.get(&t, &i).await? {
                    record.set("_archived", Value::Bool(true));
                    env.storage.store(&record).await?;
                }
            }
            Ok(Value::Null)
        }

        "discard" => Ok(Value::Null),

        "supersede" => {
            if let (Some(Value::Record(mut old)), Some(Value::Record(new_rec))) =
                (arg_values.first().cloned(), arg_values.get(1).cloned())
            {
                old.set("superseded_by", Value::String(new_rec.id.clone()));
                old.set("valid_until", Value::Timestamp(Utc::now()));
                env.storage.store(&old).await?;
                env.storage.store(&new_rec).await?;
                env.stored_count += 1;
            }
            Ok(Value::Null)
        }

        "len" => match arg_values.first() {
            Some(Value::Array(a)) => Ok(Value::Int(a.len() as i64)),
            Some(Value::String(s)) => Ok(Value::Int(s.len() as i64)),
            _ => Ok(Value::Int(0)),
        },

        "min" => match (arg_values.first(), arg_values.get(1)) {
            (Some(Value::Float(a)), Some(Value::Float(b))) => Ok(Value::Float(a.min(*b))),
            (Some(Value::Int(a)), Some(Value::Int(b))) => Ok(Value::Int(*a.min(b))),
            _ => Ok(Value::Null),
        },

        "max" => match (arg_values.first(), arg_values.get(1)) {
            (Some(Value::Float(a)), Some(Value::Float(b))) => Ok(Value::Float(a.max(*b))),
            (Some(Value::Int(a)), Some(Value::Int(b))) => Ok(Value::Int(*a.max(b))),
            _ => Ok(Value::Null),
        },

        "extract" => {
            let text = arg_values.first().map(value_to_text).unwrap_or_default();
            if let Some(ref llm) = env.llm {
                match llm.extract(&text).await {
                    Ok(facts) => Ok(Value::Array(facts)),
                    Err(e) => {
                        tracing::error!(error = %e, "extract() LLM call failed");
                        Ok(Value::Array(vec![]))
                    }
                }
            } else {
                tracing::warn!("extract() called but no extractor configured");
                Ok(Value::Array(vec![]))
            }
        }

        "summarize" => {
            let text = arg_values.first().map(value_to_text).unwrap_or_default();
            if let Some(ref llm) = env.llm {
                match llm.summarize(&text).await {
                    Ok(summary) => Ok(Value::String(summary)),
                    Err(e) => {
                        tracing::error!(error = %e, "summarize() LLM call failed");
                        Ok(Value::String(String::new()))
                    }
                }
            } else {
                tracing::warn!("summarize() called but no extractor configured");
                Ok(Value::String(String::new()))
            }
        }

        "semantic_match" => {
            if let (Some(text_a), Some(text_b)) = (
                arg_values.first().and_then(|v| v.as_str()),
                arg_values.get(1).and_then(|v| v.as_str()),
            ) {
                let threshold = args
                    .iter()
                    .find_map(|a| {
                        if a.name.as_deref() == Some("threshold") {
                            if let Expr::Float(f) = &a.value {
                                return Some(*f);
                            }
                        }
                        None
                    })
                    .unwrap_or(0.7);

                if let Some(ref _llm) = env.llm {
                    if let Some(ref embedder) = env.embedder {
                        match embedder.similarity(text_a, text_b).await {
                            Ok(sim) => Ok(Value::Bool(sim >= threshold)),
                            Err(e) => {
                                tracing::error!(error = %e, "semantic_match embedding failed");
                                Ok(Value::Bool(false))
                            }
                        }
                    } else {
                        tracing::warn!("semantic_match() called but no embedding model configured");
                        Ok(Value::Bool(false))
                    }
                } else {
                    tracing::warn!("semantic_match() called but no LLM configured");
                    Ok(Value::Bool(false))
                }
            } else {
                Ok(Value::Bool(true))
            }
        }

        "regex" => {
            let text = arg_values
                .first()
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let pattern = arg_values
                .get(1)
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            match regex::Regex::new(pattern) {
                Ok(re) => Ok(Value::Bool(re.is_match(text))),
                Err(e) => {
                    tracing::error!(error = %e, pattern = %pattern, "invalid regex pattern");
                    Ok(Value::Bool(false))
                }
            }
        }

        "sql" => {
            let query_str = arg_values
                .first()
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            if query_str.is_empty() {
                return Ok(Value::Array(vec![]));
            }
            match env.storage.raw_sql(&query_str) {
                Ok(records) => {
                    let results: Vec<Value> = records.into_iter().map(Value::Record).collect();
                    Ok(Value::Array(results))
                }
                Err(e) => {
                    tracing::error!(error = %e, "sql() execution failed");
                    Ok(Value::Array(vec![]))
                }
            }
        }

        "graph_match" => {
            let input = arg_values
                .first()
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let hops = arg_values
                .get(1)
                .and_then(|v| match v {
                    Value::Int(n) => Some(*n as usize),
                    _ => None,
                })
                .unwrap_or(2);
            let type_name = env.get("_update_target");
            let tn = type_name.as_str().unwrap_or("Entity");
            if let Some(crate::storage::StorageBackend::Neo4j(ref neo)) = *&env.storage.graph {
                match neo.graph_match_ids(tn, &input, hops).await {
                    Ok(ids) => {
                        let results: Vec<Value> = ids.into_iter().map(Value::String).collect();
                        Ok(Value::Array(results))
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "graph_match() failed");
                        Ok(Value::Array(vec![]))
                    }
                }
            } else {
                tracing::warn!("graph_match() called but no graph backend configured");
                Ok(Value::Array(vec![]))
            }
        }

        "cypher" => {
            let query_str = arg_values
                .first()
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            if query_str.is_empty() {
                return Ok(Value::Array(vec![]));
            }
            let mut params = std::collections::HashMap::new();
            for (i, arg) in args.iter().enumerate().skip(1) {
                if let Some(ref name) = arg.name {
                    if let Some(val) = arg_values.get(i) {
                        if let Some(s) = val.as_str() {
                            params.insert(name.clone(), s.to_string());
                        }
                    }
                }
            }
            if let Some(crate::storage::StorageBackend::Neo4j(ref neo)) = *&env.storage.graph {
                match neo.cypher_query_ids(&query_str, &params).await {
                    Ok(ids) => {
                        let results: Vec<Value> = ids.into_iter().map(Value::String).collect();
                        Ok(Value::Array(results))
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "cypher() failed");
                        Ok(Value::Array(vec![]))
                    }
                }
            } else {
                tracing::warn!("cypher() called but no graph backend configured");
                Ok(Value::Array(vec![]))
            }
        }

        "map" | "filter" | "each" => {
            tracing::debug!("{func_name}() called outside pipe context");
            Ok(Value::Null)
        }

        "emit" => {
            if let Some(Value::String(event_name)) = arg_values.first() {
                if let Some(handler) = env.handlers.get(event_name.as_str()) {
                    let handler = handler.clone();
                    env.push_scope();
                    for (i, param) in handler.params.iter().enumerate() {
                        if let Some(val) = arg_values.get(i + 1) {
                            env.set(&param.name, val.clone());
                        }
                    }
                    exec_stmts(env, &handler.body).await?;
                    env.pop_scope();
                } else {
                    tracing::warn!("emit(): unknown event '{event_name}'");
                }
            }
            Ok(Value::Null)
        }

        _ => {
            // Try invoking a named query defined in the DSL
            if let Some(query_def) = env.queries.get(func_name).cloned() {
                let mut child_env = ExecEnv::new(
                    env.storage.clone(),
                    env.llm.clone(),
                    env.embedder.clone(),
                    env.handlers.clone(),
                    env.extern_fns.clone(),
                )
                .with_queries(env.queries.clone())
                .with_memories(env.memories.clone());

                for (param, val) in query_def.params.iter().zip(arg_values.iter()) {
                    child_env.set(&param.name, val.clone());
                }

                match super::query::exec_query(&mut child_env, &query_def).await {
                    Ok(results) => return Ok(Value::Array(results)),
                    Err(e) => {
                        tracing::error!(error = %e, query = func_name, "query call failed");
                        return Ok(Value::Array(vec![]));
                    }
                }
            }

            // Try extern fn via LLM
            if let Some(ref llm) = env.llm {
                if let Some(ext_fn) = env.extern_fns.get(func_name) {
                    let params: Vec<(String, String)> = ext_fn
                        .params
                        .iter()
                        .map(|p| (p.name.clone(), format!("{:?}", p.ty)))
                        .collect();
                    let return_type = ext_fn
                        .return_ty
                        .as_ref()
                        .map(|t| format!("{t:?}"))
                        .unwrap_or_else(|| "any".into());
                    match llm
                        .call_extern(func_name, &params, &return_type, &arg_values)
                        .await
                    {
                        Ok(val) => return Ok(val),
                        Err(e) => {
                            tracing::error!(error = %e, "extern fn {func_name} LLM call failed");
                        }
                    }
                }
            }
            tracing::warn!("unknown function: {func_name}");
            Ok(Value::Null)
        }
    }
}

/// Check if a record conflicts with an existing one and run on_conflict if so.
/// Returns true if on_conflict handled the record (caller should NOT store).
async fn try_on_conflict(env: &mut ExecEnv, new_record: &Record) -> anyhow::Result<bool> {
    let update_def = match env.updates.get(&new_record.type_name) {
        Some(u) => u.clone(),
        None => return Ok(false),
    };

    let has_on_conflict = update_def
        .rules
        .iter()
        .any(|r| matches!(r, synapse_core::ast::UpdateRule::OnConflict { .. }));
    if !has_on_conflict {
        return Ok(false);
    }

    // Build conflict key from the record's fields.
    // SPO triples: subject + predicate is the natural conflict key.
    // Generic fallback: all @index fields (future), or no conflict detection.
    let subject = new_record.fields.get("subject").and_then(|v| v.as_str());
    let predicate = new_record.fields.get("predicate").and_then(|v| v.as_str());

    let (subj, pred) = match (subject, predicate) {
        (Some(s), Some(p)) => (s.to_string(), p.to_string()),
        _ => return Ok(false), // no conflict key — store normally
    };

    use crate::storage::{Condition, ConditionOp, QueryFilter};
    let filter = QueryFilter {
        conditions: vec![
            Condition {
                field: "subject".into(),
                op: ConditionOp::Eq,
                value: Value::String(subj),
            },
            Condition {
                field: "predicate".into(),
                op: ConditionOp::Eq,
                value: Value::String(pred),
            },
        ],
        ..Default::default()
    };

    let existing = env
        .storage
        .query(&new_record.type_name, &filter)
        .await
        .unwrap_or_default();

    if existing.is_empty() {
        return Ok(false);
    }

    for old in &existing {
        if old.id == new_record.id {
            continue; // same record, not a conflict
        }
        tracing::info!(
            type_name = %new_record.type_name,
            old_id = %old.id,
            new_id = %new_record.id,
            "conflict detected, running on_conflict"
        );
        super::update::exec_on_conflict(env, &update_def, &old.id, new_record).await?;
    }

    Ok(true)
}

/// Convert a Value to a text string for LLM consumption.
/// Handles strings directly, Records by pulling their `content` field
/// (or joining all string fields), and arrays by recursing on each element.
fn value_to_text(val: &Value) -> String {
    match val {
        Value::String(s) => s.clone(),
        Value::Record(r) => {
            if let Some(Value::String(s)) = r.fields.get("content") {
                s.clone()
            } else {
                r.fields
                    .values()
                    .filter_map(|v| match v {
                        Value::String(s) => Some(s.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            }
        }
        Value::Array(arr) => arr.iter().map(value_to_text).collect::<Vec<_>>().join("\n"),
        Value::Int(n) => n.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

/// Apply a lambda expression: bind params, evaluate body, return result.
async fn apply_lambda(
    env: &mut ExecEnv,
    params: &[String],
    body: &Expr,
    arg: Value,
) -> anyhow::Result<Value> {
    env.push_scope();
    if let Some(param) = params.first() {
        env.set(param, arg);
    }
    let result = eval_expr(env, body).await?;
    env.pop_scope();
    Ok(result)
}

/// Evaluate a piped function call: left |> func(args)
async fn eval_piped_call(
    env: &mut ExecEnv,
    func: &Expr,
    left_val: Value,
    extra_args: &[CallArg],
) -> anyhow::Result<Value> {
    let (func_name, lambda_args): (&str, &[CallArg]) = match func {
        Expr::Ident(name) => (name.as_str(), extra_args),
        Expr::Call { func: inner, args } => {
            let name = match inner.as_ref() {
                Expr::Ident(n) => n.as_str(),
                _ => return Ok(Value::Null),
            };
            if !matches!(
                name,
                "map" | "filter" | "each" | "group_by" | "store_as" | "delete_originals"
            ) {
                let mut arg_values = vec![left_val.clone()];
                for arg in args {
                    arg_values.push(eval_expr(env, &arg.value).await?);
                }
                return eval_builtin_with_args(env, name, arg_values).await;
            }
            (name, args.as_slice())
        }
        _ => return Ok(Value::Null),
    };

    if matches!(func_name, "group_by") {
        if let Value::Array(arr) = left_val {
            let field_name = lambda_args
                .first()
                .and_then(|a| match &a.value {
                    Expr::Ident(name) => Some(name.as_str()),
                    _ => None,
                })
                .unwrap_or("id");

            let mut groups: std::collections::HashMap<String, Vec<Value>> =
                std::collections::HashMap::new();
            for item in arr {
                let key = match &item {
                    Value::Record(r) => r
                        .get(field_name)
                        .and_then(|v| v.as_str())
                        .unwrap_or("_unknown")
                        .to_string(),
                    _ => "_unknown".to_string(),
                };
                groups.entry(key).or_default().push(item);
            }
            let result: Vec<Value> = groups.into_values().map(Value::Array).collect();
            return Ok(Value::Array(result));
        }
        return Ok(left_val);
    }

    if matches!(func_name, "store_as") {
        let type_name = lambda_args
            .first()
            .and_then(|a| match &a.value {
                Expr::Ident(name) => Some(name.clone()),
                _ => None,
            })
            .unwrap_or_default();

        if let Value::Array(arr) = left_val {
            let mut stored = Vec::new();
            for item in arr {
                match item {
                    Value::Record(mut r) => {
                        r.type_name = type_name.clone();
                        env.storage.store(&r).await?;
                        env.stored_count += 1;
                        stored.push(Value::Record(r));
                    }
                    Value::Array(sub) => {
                        for sub_item in sub {
                            if let Value::Record(mut r) = sub_item {
                                r.type_name = type_name.clone();
                                env.storage.store(&r).await?;
                                env.stored_count += 1;
                                stored.push(Value::Record(r));
                            }
                        }
                    }
                    other => stored.push(other),
                }
            }
            return Ok(Value::Array(stored));
        }
        return Ok(Value::Null);
    }

    if matches!(func_name, "delete_originals") {
        if let Value::Array(ref arr) = left_val {
            for item in arr {
                if let Value::Record(r) = item {
                    env.storage.delete(&r.type_name, &r.id).await?;
                }
            }
        }
        return Ok(left_val);
    }

    if matches!(func_name, "map" | "filter" | "each") {
        let lambda = lambda_args.iter().find_map(|arg| match &arg.value {
            Expr::Lambda { params, body } => Some((params.as_slice(), body.as_ref())),
            _ => None,
        });

        if let Some((params, body)) = lambda {
            let arr = match left_val {
                Value::Array(a) => a,
                other => return Ok(other),
            };

            return match func_name {
                "map" => {
                    let mut result = Vec::with_capacity(arr.len());
                    for item in arr {
                        let val = apply_lambda(env, params, body, item).await?;
                        result.push(val);
                    }
                    Ok(Value::Array(result))
                }
                "filter" => {
                    let mut result = Vec::new();
                    for item in arr {
                        let cond = apply_lambda(env, params, body, item.clone()).await?;
                        if cond.is_truthy() {
                            result.push(item);
                        }
                    }
                    Ok(Value::Array(result))
                }
                "each" => {
                    for item in arr {
                        apply_lambda(env, params, body, item).await?;
                    }
                    Ok(Value::Null)
                }
                _ => unreachable!(),
            };
        }
        tracing::warn!("{func_name}() called without a lambda argument");
        return Ok(left_val);
    }

    let mut arg_values = vec![left_val];
    for arg in extra_args {
        arg_values.push(eval_expr(env, &arg.value).await?);
    }
    eval_builtin_with_args(env, func_name, arg_values).await
}

async fn eval_builtin_with_args(
    env: &mut ExecEnv,
    name: &str,
    args: Vec<Value>,
) -> anyhow::Result<Value> {
    match name {
        "store" => {
            for arg in &args {
                if let Value::Record(r) = arg {
                    env.storage.store(r).await?;
                    env.stored_count += 1;
                } else if let Value::Array(arr) = arg {
                    for item in arr {
                        if let Value::Record(r) = item {
                            env.storage.store(r).await?;
                            env.stored_count += 1;
                        }
                    }
                }
            }
            Ok(Value::Null)
        }
        "extract" => {
            let text = args.first().map(value_to_text).unwrap_or_default();
            if let Some(ref llm) = env.llm {
                match llm.extract(&text).await {
                    Ok(facts) => Ok(Value::Array(facts)),
                    Err(e) => {
                        tracing::error!(error = %e, "extract() piped LLM call failed");
                        Ok(Value::Array(vec![]))
                    }
                }
            } else {
                tracing::warn!("extract() called but no extractor configured");
                Ok(Value::Array(vec![]))
            }
        }
        "summarize" => {
            let text = args.first().map(value_to_text).unwrap_or_default();
            if let Some(ref llm) = env.llm {
                match llm.summarize(&text).await {
                    Ok(summary) => Ok(Value::String(summary)),
                    Err(e) => {
                        tracing::error!(error = %e, "summarize() piped LLM call failed");
                        Ok(Value::String(String::new()))
                    }
                }
            } else {
                tracing::warn!("summarize() called but no extractor configured");
                Ok(Value::String(String::new()))
            }
        }
        "len" => match args.first() {
            Some(Value::Array(a)) => Ok(Value::Int(a.len() as i64)),
            _ => Ok(Value::Int(0)),
        },
        _ => {
            tracing::debug!("piped call to unknown function: {name}");
            Ok(args.first().cloned().unwrap_or(Value::Null))
        }
    }
}

fn eval_binop(left: &Value, op: &BinOp, right: &Value) -> Value {
    match op {
        BinOp::Add => match (left, right) {
            (Value::Int(a), Value::Int(b)) => Value::Int(a + b),
            (Value::Float(a), Value::Float(b)) => Value::Float(a + b),
            (Value::Int(a), Value::Float(b)) => Value::Float(*a as f64 + b),
            (Value::Float(a), Value::Int(b)) => Value::Float(a + *b as f64),
            (Value::String(a), Value::String(b)) => Value::String(format!("{a}{b}")),
            _ => Value::Null,
        },
        BinOp::Sub => match (left, right) {
            (Value::Int(a), Value::Int(b)) => Value::Int(a - b),
            (Value::Float(a), Value::Float(b)) => Value::Float(a - b),
            (Value::Int(a), Value::Float(b)) => Value::Float(*a as f64 - b),
            (Value::Float(a), Value::Int(b)) => Value::Float(a - *b as f64),
            _ => Value::Null,
        },
        BinOp::Mul => match (left, right) {
            (Value::Int(a), Value::Int(b)) => Value::Int(a * b),
            (Value::Float(a), Value::Float(b)) => Value::Float(a * b),
            (Value::Int(a), Value::Float(b)) => Value::Float(*a as f64 * b),
            (Value::Float(a), Value::Int(b)) => Value::Float(a * *b as f64),
            _ => Value::Null,
        },
        BinOp::Div => match (left, right) {
            (Value::Int(a), Value::Int(b)) if *b != 0 => Value::Int(a / b),
            (Value::Float(a), Value::Float(b)) if *b != 0.0 => Value::Float(a / b),
            (Value::Int(a), Value::Float(b)) if *b != 0.0 => Value::Float(*a as f64 / b),
            (Value::Float(a), Value::Int(b)) if *b != 0 => Value::Float(a / *b as f64),
            _ => Value::Null,
        },
        BinOp::Mod => match (left, right) {
            (Value::Int(a), Value::Int(b)) if *b != 0 => Value::Int(a % b),
            _ => Value::Null,
        },
        BinOp::Eq => Value::Bool(left == right),
        BinOp::Ne => Value::Bool(left != right),
        BinOp::Lt => cmp_values(left, right, |a, b| a < b),
        BinOp::Le => cmp_values(left, right, |a, b| a <= b),
        BinOp::Gt => cmp_values(left, right, |a, b| a > b),
        BinOp::Ge => cmp_values(left, right, |a, b| a >= b),
        BinOp::And => Value::Bool(left.is_truthy() && right.is_truthy()),
        BinOp::Or => Value::Bool(left.is_truthy() || right.is_truthy()),
    }
}

fn cmp_values(left: &Value, right: &Value, cmp: fn(f64, f64) -> bool) -> Value {
    match (left, right) {
        (Value::Int(a), Value::Int(b)) => Value::Bool(cmp(*a as f64, *b as f64)),
        (Value::Float(a), Value::Float(b)) => Value::Bool(cmp(*a, *b)),
        (Value::Int(a), Value::Float(b)) => Value::Bool(cmp(*a as f64, *b)),
        (Value::Float(a), Value::Int(b)) => Value::Bool(cmp(*a, *b as f64)),
        (Value::String(a), Value::String(b)) => Value::Bool(
            a.cmp(b)
                == if cmp(0.0, 1.0) {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                },
        ),
        _ => Value::Bool(false),
    }
}
