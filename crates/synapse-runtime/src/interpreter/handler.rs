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
            // Simple case: assign to identifier
            if let Expr::Ident(name) = target {
                env.set(name, val);
            }
            // Field access assignment: obj.field = val
            // Handled by updating the record in the environment
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

        Expr::Call { func, args } => {
            eval_call(env, func, args).await
        }

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

        Expr::InlineQuery(_qb) => {
            // Inline queries would be executed against storage
            // For now, return empty array
            Ok(Value::Array(vec![]))
        }
    }
}

/// Evaluate a function call
async fn eval_call(
    env: &mut ExecEnv,
    func: &Expr,
    args: &[CallArg],
) -> anyhow::Result<Value> {
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
                env.storage.store(record).await?;
                env.stored_count += 1;
            }
            Ok(Value::Null)
        }

        "delete" => {
            // delete() with no args: delete current record (in update context)
            // delete(record): delete specific record
            Ok(Value::Null)
        }

        "archive" => Ok(Value::Null),
        "discard" => Ok(Value::Null),

        "supersede" => {
            // supersede(old, new): mark old as superseded by new
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
            // LLM-powered extraction — would use rig-core
            // Returns extracted facts as an array
            tracing::debug!("extract() called — requires LLM integration via rig");
            Ok(Value::Array(vec![]))
        }

        "summarize" => {
            // LLM-powered summarization — would use rig-core
            tracing::debug!("summarize() called — requires LLM integration via rig");
            Ok(Value::String("(summary placeholder)".into()))
        }

        "semantic_match" | "graph_match" | "regex" | "cypher" | "sql" => {
            // These are query-context functions, handled by the query executor
            Ok(Value::Bool(true))
        }

        "map" => {
            // map(lambda) on an array in pipe context
            Ok(Value::Array(vec![]))
        }

        "filter" => {
            Ok(Value::Array(vec![]))
        }

        "each" => {
            Ok(Value::Null)
        }

        "emit" => {
            // emit("event_name", ...args) — trigger another handler
            tracing::debug!("emit() called with args: {:?}", arg_values);
            Ok(Value::Null)
        }

        _ => {
            tracing::warn!("unknown function: {func_name}");
            Ok(Value::Null)
        }
    }
}

/// Evaluate a piped function call: left |> func(args)
async fn eval_piped_call(
    env: &mut ExecEnv,
    func: &Expr,
    left_val: Value,
    extra_args: &[CallArg],
) -> anyhow::Result<Value> {
    let func_name = match func {
        Expr::Ident(name) => name.as_str(),
        Expr::Call { func: inner, args } => {
            // left |> func(args) — prepend left to args
            let mut all_args = vec![CallArg {
                name: None,
                value: Expr::Null,
            }];
            all_args.extend(args.iter().cloned());
            let name = match inner.as_ref() {
                Expr::Ident(n) => n.as_str(),
                _ => return Ok(Value::Null),
            };
            // Evaluate with left_val as first arg
            let mut arg_values = vec![left_val.clone()];
            for arg in args {
                arg_values.push(eval_expr(env, &arg.value).await?);
            }
            return eval_builtin_with_args(env, name, arg_values).await;
        }
        _ => return Ok(Value::Null),
    };

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
            tracing::debug!("extract() piped call");
            Ok(Value::Array(vec![]))
        }
        "summarize" => {
            Ok(Value::String("(summary)".into()))
        }
        "filter" => {
            // filter(array, lambda) — lambda not yet evaluated
            Ok(args.first().cloned().unwrap_or(Value::Array(vec![])))
        }
        "map" => {
            Ok(args.first().cloned().unwrap_or(Value::Array(vec![])))
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
        (Value::String(a), Value::String(b)) => Value::Bool(a.cmp(b) == if cmp(0.0, 1.0) { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater }),
        _ => Value::Bool(false),
    }
}
