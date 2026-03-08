use synapse_core::ast::*;

use super::handler::eval_expr;
use super::ExecEnv;
use crate::storage::{Condition, ConditionOp, QueryFilter};
use crate::value::{Record, Value};

/// Execute a query definition against storage.
pub async fn exec_query(env: &mut ExecEnv, query: &QueryDef) -> anyhow::Result<Vec<Value>> {
    let body = &query.body;

    let mut filter = QueryFilter::default();

    // Process order by
    if let Some(ref ob) = body.order_by {
        if let Expr::Ident(field) = &ob.expr {
            filter.order_by = Some((field.clone(), ob.direction == SortDir::Asc));
        }
    }

    // Process limit
    if let Some(ref lim) = body.limit {
        let val = eval_expr(env, lim).await?;
        if let Value::Int(n) = val {
            filter.limit = Some(n as usize);
        }
    }

    // Process where clause into conditions
    if let Some(ref where_expr) = body.where_clause {
        extract_conditions(env, where_expr, &mut filter.conditions).await;
    }

    // Query each source type and combine results
    let mut all_results = vec![];
    for source in &body.from {
        let records = env.storage.query(source, &filter).await?;
        for r in records {
            all_results.push(Value::Record(r));
        }
    }

    Ok(all_results)
}

/// Extract simple field-comparison conditions from a where clause expression.
/// Complex conditions (semantic_match, graph_match) are currently passed through.
async fn extract_conditions(env: &mut ExecEnv, expr: &Expr, conditions: &mut Vec<Condition>) {
    match expr {
        // field == value
        Expr::Binary { left, op, right } => {
            let cond_op = match op {
                BinOp::Eq => Some(ConditionOp::Eq),
                BinOp::Ne => Some(ConditionOp::Ne),
                BinOp::Lt => Some(ConditionOp::Lt),
                BinOp::Le => Some(ConditionOp::Le),
                BinOp::Gt => Some(ConditionOp::Gt),
                BinOp::Ge => Some(ConditionOp::Ge),
                BinOp::And => {
                    // Recurse into both sides
                    Box::pin(extract_conditions(env, left, conditions)).await;
                    Box::pin(extract_conditions(env, right, conditions)).await;
                    return;
                }
                _ => None,
            };

            if let (Some(op), Expr::Ident(field)) = (cond_op, left.as_ref()) {
                if let Ok(val) = eval_expr(env, right).await {
                    conditions.push(Condition {
                        field: field.clone(),
                        op,
                        value: val,
                    });
                }
            }
        }
        // Function calls like semantic_match(), graph_match() are handled
        // at query time by the storage backends, not as filter conditions
        _ => {}
    }
}
