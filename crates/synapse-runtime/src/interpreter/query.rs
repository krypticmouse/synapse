use synapse_dsl::ast::*;

use super::handler::eval_expr;
use super::ExecEnv;
use crate::storage::{Condition, ConditionOp, CypherQuery, GraphMatch, QueryFilter};
use crate::value::Value;

/// Execute a query definition against storage.
pub async fn exec_query(env: &mut ExecEnv, query: &QueryDef) -> anyhow::Result<Vec<Value>> {
    exec_query_body(env, &query.body).await
}

/// Execute a QueryBody against storage (shared by named queries and inline queries).
pub async fn exec_query_body(env: &mut ExecEnv, body: &QueryBody) -> anyhow::Result<Vec<Value>> {
    let mut filter = QueryFilter::default();

    // Store the order-by expression for potential expression evaluation
    let order_expr = body.order_by.as_ref().map(|ob| (ob.expr.clone(), ob.direction));

    // Simple ident ordering goes to the backend filter
    if let Some((ref expr, dir)) = order_expr {
        if let Expr::Ident(field) = expr {
            filter.order_by = Some((field.clone(), dir == SortDir::Asc));
        }
    }

    if let Some(ref lim) = body.limit {
        let val = eval_expr(env, lim).await?;
        if let Value::Int(n) = val {
            filter.limit = Some(n as usize);
        }
    }

    if let Some(ref where_expr) = body.where_clause {
        extract_conditions(env, where_expr, &mut filter).await;
    }

    let mut all_results = vec![];
    for source in &body.from {
        let (records, alias_scores) = env.storage.query_with_scores(source, &filter).await?;
        for mut r in records {
            // Attach alias scores to each record as virtual fields
            for (alias, scores) in &alias_scores {
                if let Some(&score) = scores.get(&r.id) {
                    r.set(alias, Value::Float(score));
                }
            }
            all_results.push(Value::Record(r));
        }
    }

    // Apply expression-based ordering if the order-by is not a simple ident
    if let Some((ref expr, dir)) = order_expr {
        if !matches!(expr, Expr::Ident(_)) {
            all_results.sort_by(|a, b| {
                let score_a = eval_order_expr(expr, a);
                let score_b = eval_order_expr(expr, b);
                let ord = score_a
                    .partial_cmp(&score_b)
                    .unwrap_or(std::cmp::Ordering::Equal);
                if dir == SortDir::Asc { ord } else { ord.reverse() }
            });

            // Apply limit after expression-based ordering
            if let Some(limit) = filter.limit {
                all_results.truncate(limit);
            }
        }
    }

    Ok(all_results)
}

/// Evaluate an order-by expression against a record's fields.
/// Supports arithmetic on alias fields (e.g., `(sm + gm) / 2`).
fn eval_order_expr(expr: &Expr, val: &Value) -> f64 {
    match expr {
        Expr::Ident(name) => {
            if let Value::Record(r) = val {
                r.fields
                    .get(name)
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0)
            } else {
                0.0
            }
        }
        Expr::Float(f) => *f,
        Expr::Int(n) => *n as f64,
        Expr::Binary { left, op, right } => {
            let l = eval_order_expr(left, val);
            let r = eval_order_expr(right, val);
            match op {
                BinOp::Add => l + r,
                BinOp::Sub => l - r,
                BinOp::Mul => l * r,
                BinOp::Div => {
                    if r != 0.0 {
                        l / r
                    } else {
                        0.0
                    }
                }
                BinOp::Mod => {
                    if r != 0.0 {
                        l % r
                    } else {
                        0.0
                    }
                }
                _ => 0.0,
            }
        }
        Expr::Unary { op, operand } => {
            let v = eval_order_expr(operand, val);
            match op {
                UnaryOp::Neg => -v,
                UnaryOp::Not => {
                    if v == 0.0 {
                        1.0
                    } else {
                        0.0
                    }
                }
            }
        }
        Expr::Call { func, args } => {
            if let Expr::Ident(name) = func.as_ref() {
                let arg_vals: Vec<f64> = args.iter().map(|a| eval_order_expr(&a.value, val)).collect();
                match name.as_str() {
                    "min" => arg_vals.iter().copied().fold(f64::INFINITY, f64::min),
                    "max" => arg_vals.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                    _ => 0.0,
                }
            } else {
                0.0
            }
        }
        _ => 0.0,
    }
}

/// Extract conditions from a where clause expression into the QueryFilter.
async fn extract_conditions(env: &mut ExecEnv, expr: &Expr, filter: &mut QueryFilter) {
    match expr {
        // Handle `expr as alias` — extract the inner expression and register the alias
        Expr::Alias { expr: inner, alias } => {
            // Determine what kind of scoring this alias represents
            if let Expr::Call { func, .. } = inner.as_ref() {
                if let Expr::Ident(name) = func.as_ref() {
                    let kind = match name.as_str() {
                        "semantic_match" => "semantic",
                        "graph_match" | "cypher" => "graph",
                        _ => "unknown",
                    };
                    filter
                        .score_aliases
                        .insert(alias.clone(), kind.to_string());
                }
            }
            Box::pin(extract_conditions(env, inner, filter)).await;
        }

        Expr::Binary { left, op, right } => {
            let cond_op = match op {
                BinOp::Eq => Some(ConditionOp::Eq),
                BinOp::Ne => Some(ConditionOp::Ne),
                BinOp::Lt => Some(ConditionOp::Lt),
                BinOp::Le => Some(ConditionOp::Le),
                BinOp::Gt => Some(ConditionOp::Gt),
                BinOp::Ge => Some(ConditionOp::Ge),
                BinOp::And => {
                    Box::pin(extract_conditions(env, left, filter)).await;
                    Box::pin(extract_conditions(env, right, filter)).await;
                    return;
                }
                BinOp::Or => {
                    let mut left_filter = QueryFilter::default();
                    let mut right_filter = QueryFilter::default();
                    Box::pin(extract_conditions(env, left, &mut left_filter)).await;
                    Box::pin(extract_conditions(env, right, &mut right_filter)).await;
                    filter.or_conditions.extend(left_filter.conditions);
                    filter.or_conditions.extend(right_filter.conditions);
                    filter.or_conditions.extend(left_filter.or_conditions);
                    filter.or_conditions.extend(right_filter.or_conditions);
                    return;
                }
                _ => None,
            };

            if let (Some(op), Expr::Ident(field)) = (cond_op, left.as_ref()) {
                if let Ok(val) = eval_expr(env, right).await {
                    filter.conditions.push(Condition {
                        field: field.clone(),
                        op,
                        value: val,
                    });
                }
            }
        }

        Expr::Call { func, args } => {
            if let Expr::Ident(name) = func.as_ref() {
                match name.as_str() {
                    "graph_match" => {
                        let input = if let Some(arg) = args.first() {
                            eval_expr(env, &arg.value)
                                .await
                                .ok()
                                .and_then(|v| v.as_str().map(|s| s.to_string()))
                                .unwrap_or_default()
                        } else {
                            String::new()
                        };

                        let hops = args
                            .iter()
                            .find_map(|a| {
                                if a.name.as_deref() == Some("hops") {
                                    if let Expr::Int(n) = &a.value {
                                        return Some(*n as usize);
                                    }
                                }
                                None
                            })
                            .unwrap_or(2);

                        // Extract backend parameter
                        let backend = args.iter().find_map(|a| {
                            if a.name.as_deref() == Some("backend") {
                                if let Expr::Str(s) = &a.value {
                                    return Some(s.clone());
                                }
                            }
                            None
                        });
                        if let Some(b) = backend {
                            filter.graph_backend = Some(b);
                        }

                        filter.graph_match = Some(GraphMatch { input, hops });
                    }

                    "cypher" => {
                        if let Some(arg) = args.first() {
                            if let Expr::Str(query_str) = &arg.value {
                                let mut params = std::collections::HashMap::new();
                                for a in args.iter().skip(1) {
                                    if let Some(ref pname) = a.name {
                                        if let Ok(val) = eval_expr(env, &a.value).await {
                                            if let Some(s) = val.as_str() {
                                                params.insert(pname.clone(), s.to_string());
                                            }
                                        }
                                    }
                                }

                                let query_str_owned = query_str.clone();
                                for word in query_str_owned.split('$') {
                                    if let Some(param_name) = word
                                        .split(|c: char| !c.is_alphanumeric() && c != '_')
                                        .next()
                                    {
                                        if !param_name.is_empty()
                                            && !params.contains_key(param_name)
                                        {
                                            let val = env.get(param_name);
                                            if let Some(s) = val.as_str() {
                                                params
                                                    .insert(param_name.to_string(), s.to_string());
                                            }
                                        }
                                    }
                                }

                                filter.cypher_query = Some(CypherQuery {
                                    query: query_str.clone(),
                                    params,
                                });
                            }
                        }
                    }

                    "semantic_match" => {
                        let input = if let Some(arg) = args.first() {
                            eval_expr(env, &arg.value)
                                .await
                                .ok()
                                .and_then(|v| v.as_str().map(|s| s.to_string()))
                                .unwrap_or_default()
                        } else {
                            String::new()
                        };

                        let threshold = args
                            .iter()
                            .find_map(|a| {
                                if a.name.as_deref() == Some("threshold") {
                                    match &a.value {
                                        synapse_dsl::ast::Expr::Float(f) => Some(*f),
                                        synapse_dsl::ast::Expr::Int(n) => Some(*n as f64),
                                        _ => None,
                                    }
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(0.7);

                        // Extract backend parameter
                        let backend = args.iter().find_map(|a| {
                            if a.name.as_deref() == Some("backend") {
                                if let Expr::Str(s) = &a.value {
                                    return Some(s.clone());
                                }
                            }
                            None
                        });
                        if let Some(b) = backend {
                            filter.vector_backend = Some(b);
                        }

                        filter.semantic_match =
                            Some(crate::storage::SemanticMatch { input, threshold });
                    }

                    "regex" => {
                        if let (Some(field_arg), Some(pattern_arg)) = (args.first(), args.get(1)) {
                            if let Expr::Ident(field_name) = &field_arg.value {
                                let pattern = if let Expr::Str(s) = &pattern_arg.value {
                                    s.clone()
                                } else if let Ok(val) = eval_expr(env, &pattern_arg.value).await {
                                    val.as_str().unwrap_or_default().to_string()
                                } else {
                                    String::new()
                                };
                                if !pattern.is_empty() {
                                    if let Ok(re) = regex::Regex::new(&pattern) {
                                        filter.regex_filters.push((field_name.clone(), re));
                                    }
                                }
                            }
                        }
                    }

                    "sql" => {
                        if let Some(arg) = args.first() {
                            if let Expr::Str(raw_sql) = &arg.value {
                                filter.raw_sql = Some(raw_sql.clone());
                            }
                        }
                    }

                    _ => {}
                }
            }
        }

        _ => {}
    }
}
