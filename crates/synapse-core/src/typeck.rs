use std::collections::HashMap;

use crate::ast::*;
use crate::errors::{Result, SynapseError};
use crate::types::Type;

/// Symbol table for type checking
#[derive(Debug, Clone, Default)]
pub struct TypeEnv {
    /// memory_name -> { field_name -> type }
    memories: HashMap<String, HashMap<String, Type>>,
    /// query_name -> (params, return_type)
    queries: HashMap<String, (Vec<(String, Type)>, Type)>,
    /// handler_name -> params
    handlers: HashMap<String, Vec<(String, Type)>>,
    /// extern_fn_name -> (params, return_type)
    extern_fns: HashMap<String, (Vec<(String, Type)>, Option<Type>)>,
    /// Local variable scope stack
    scopes: Vec<HashMap<String, Type>>,
    /// Errors accumulated during type checking
    errors: Vec<String>,
}

impl TypeEnv {
    pub fn new() -> Self {
        Self::default()
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn define_local(&mut self, name: &str, ty: Type) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), ty);
        }
    }

    fn lookup(&self, name: &str) -> Option<&Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty);
            }
        }
        // Check if it's a memory type name
        if self.memories.contains_key(name) {
            return None; // It's a type, not a value
        }
        None
    }

    fn error(&mut self, msg: impl Into<String>) {
        self.errors.push(msg.into());
    }
}

/// Type-check a full program. Returns accumulated errors.
pub fn check(program: &Program) -> Result<TypeEnv> {
    let mut env = TypeEnv::new();

    // First pass: register all type declarations
    register_items(&mut env, &program.items);

    // Second pass: check handler/query/update bodies
    check_items(&mut env, &program.items);

    if env.errors.is_empty() {
        Ok(env)
    } else {
        Err(SynapseError::Validation(env.errors.join("\n")))
    }
}

fn register_items(env: &mut TypeEnv, items: &[Item]) {
    for item in items {
        match item {
            Item::Memory(mem) => {
                let mut fields = HashMap::new();
                for field in &mem.fields {
                    fields.insert(field.name.clone(), field.ty.clone());
                }
                env.memories.insert(mem.name.clone(), fields);
            }
            Item::Query(q) => {
                let params: Vec<_> = q
                    .params
                    .iter()
                    .map(|p| (p.name.clone(), p.ty.clone()))
                    .collect();
                env.queries
                    .insert(q.name.clone(), (params, q.return_ty.clone()));
            }
            Item::Handler(h) => {
                let params: Vec<_> = h
                    .params
                    .iter()
                    .map(|p| (p.name.clone(), p.ty.clone()))
                    .collect();
                env.handlers.insert(h.event.clone(), params);
            }
            Item::ExternFn(ef) => {
                let params: Vec<_> = ef
                    .params
                    .iter()
                    .map(|p| (p.name.clone(), p.ty.clone()))
                    .collect();
                env.extern_fns
                    .insert(ef.name.clone(), (params, ef.return_ty.clone()));
            }
            Item::Namespace(ns) => {
                register_items(env, &ns.items);
            }
            _ => {}
        }
    }
}

fn check_items(env: &mut TypeEnv, items: &[Item]) {
    for item in items {
        match item {
            Item::Handler(h) => {
                env.push_scope();
                for p in &h.params {
                    env.define_local(&p.name, p.ty.clone());
                }
                check_stmts(env, &h.body);
                env.pop_scope();
            }
            Item::Query(q) => {
                env.push_scope();
                for p in &q.params {
                    env.define_local(&p.name, p.ty.clone());
                }
                check_query_body(env, &q.body);
                env.pop_scope();
            }
            Item::Update(u) => {
                // Check that target memory exists
                if !env.memories.contains_key(&u.target) {
                    env.error(format!("update references unknown memory '{}'", u.target));
                }
                for rule in &u.rules {
                    check_update_rule(env, &u.target, rule);
                }
            }
            Item::Policy(p) => {
                for rule in &p.rules {
                    env.push_scope();
                    check_update_rule_body(env, rule);
                    env.pop_scope();
                }
            }
            Item::Memory(mem) => {
                check_memory(env, mem);
            }
            Item::Namespace(ns) => {
                check_items(env, &ns.items);
            }
            _ => {}
        }
    }
}

fn check_memory(env: &mut TypeEnv, mem: &MemoryDef) {
    let mut seen = std::collections::HashSet::new();
    for field in &mem.fields {
        if !seen.insert(&field.name) {
            env.error(format!(
                "duplicate field '{}' in memory '{}'",
                field.name, mem.name
            ));
        }
        // Check bounded float constraints
        if let Type::BoundedFloat { min, max } = &field.ty {
            if min > max {
                env.error(format!(
                    "bounded float [{},{}] has min > max in field '{}' of memory '{}'",
                    min, max, field.name, mem.name
                ));
            }
        }
        // If there's a default, validate type compatibility
        if let Some(default) = &field.default {
            let inferred = infer_expr_type(env, default);
            if let Some(inferred_ty) = inferred {
                if !types_compatible(&field.ty, &inferred_ty) {
                    env.error(format!(
                        "default value type mismatch for field '{}' in memory '{}': expected {}, got {}",
                        field.name, mem.name, field.ty, inferred_ty
                    ));
                }
            }
        }
    }
}

fn check_query_body(env: &mut TypeEnv, body: &QueryBody) {
    for source in &body.from {
        if !env.memories.contains_key(source) {
            env.error(format!("query references unknown memory '{source}'"));
        }
    }
    if let Some(ref wh) = body.where_clause {
        check_expr(env, wh);
    }
    if let Some(ref ob) = body.order_by {
        check_expr(env, &ob.expr);
    }
    if let Some(ref lim) = body.limit {
        check_expr(env, lim);
    }
}

fn check_update_rule(env: &mut TypeEnv, target: &str, rule: &UpdateRule) {
    env.push_scope();
    // Bring memory fields into scope
    if let Some(fields) = env.memories.get(target).cloned() {
        for (name, ty) in fields {
            env.define_local(&name, ty);
        }
    }
    match rule {
        UpdateRule::OnConflict {
            old_name, new_name, ..
        } => {
            if let Some(fields) = env.memories.get(target).cloned() {
                // old and new are both of the memory type
                env.define_local(old_name, Type::Named(target.to_string()));
                env.define_local(new_name, Type::Named(target.to_string()));
                // Also define fields prefixed with old/new
                for (fname, fty) in &fields {
                    env.define_local(&format!("{old_name}.{fname}"), fty.clone());
                    env.define_local(&format!("{new_name}.{fname}"), fty.clone());
                }
            }
        }
        _ => {}
    }
    check_update_rule_body(env, rule);
    env.pop_scope();
}

fn check_update_rule_body(env: &mut TypeEnv, rule: &UpdateRule) {
    match rule {
        UpdateRule::OnAccess { body }
        | UpdateRule::OnConflict { body, .. }
        | UpdateRule::Every { body, .. } => {
            check_stmts(env, body);
        }
    }
}

fn check_stmts(env: &mut TypeEnv, stmts: &[Stmt]) {
    for stmt in stmts {
        check_stmt(env, stmt);
    }
}

fn check_stmt(env: &mut TypeEnv, stmt: &Stmt) {
    match stmt {
        Stmt::Let { name, value } => {
            check_expr(env, value);
            let ty = infer_expr_type(env, value).unwrap_or(Type::String);
            env.define_local(name, ty);
        }
        Stmt::Assign { target, value } => {
            check_expr(env, target);
            check_expr(env, value);
        }
        Stmt::If {
            condition,
            then_body,
            else_body,
        } => {
            check_expr(env, condition);
            env.push_scope();
            check_stmts(env, then_body);
            env.pop_scope();
            if let Some(else_body) = else_body {
                env.push_scope();
                check_stmts(env, else_body);
                env.pop_scope();
            }
        }
        Stmt::For { var, iter, body } => {
            check_expr(env, iter);
            env.push_scope();
            // Infer element type from iterator
            let elem_ty = match infer_expr_type(env, iter) {
                Some(Type::Array(inner)) => *inner,
                _ => Type::String, // fallback
            };
            env.define_local(var, elem_ty);
            check_stmts(env, body);
            env.pop_scope();
        }
        Stmt::Return(expr) => {
            if let Some(e) = expr {
                check_expr(env, e);
            }
        }
        Stmt::Expr(e) => {
            check_expr(env, e);
        }
    }
}

fn check_expr(env: &mut TypeEnv, expr: &Expr) {
    match expr {
        Expr::Binary { left, right, .. } => {
            check_expr(env, left);
            check_expr(env, right);
        }
        Expr::Unary { operand, .. } => {
            check_expr(env, operand);
        }
        Expr::Call { func, args } => {
            check_expr(env, func);
            for arg in args {
                check_expr(env, &arg.value);
            }
        }
        Expr::Pipe { left, right } => {
            check_expr(env, left);
            check_expr(env, right);
        }
        Expr::FieldAccess { object, .. } | Expr::OptionalChain { object, .. } => {
            check_expr(env, object);
        }
        Expr::IndexAccess { object, index } => {
            check_expr(env, object);
            check_expr(env, index);
        }
        Expr::StructInit { name, fields } => {
            if !env.memories.contains_key(name) {
                env.error(format!("unknown memory type '{name}' in struct init"));
            } else {
                let mem_fields = env.memories.get(name).cloned().unwrap_or_default();
                for fi in fields {
                    check_expr(env, &fi.value);
                    if !mem_fields.contains_key(&fi.name) {
                        env.error(format!("unknown field '{}' in memory '{name}'", fi.name));
                    }
                }
            }
        }
        Expr::Lambda { body, .. } => {
            check_expr(env, body);
        }
        Expr::Array(elems) => {
            for e in elems {
                check_expr(env, e);
            }
        }
        Expr::InlineQuery(qb) => {
            check_query_body(env, qb);
        }
        // Literals and idents don't need recursive checking
        _ => {}
    }
}

/// Basic type inference — returns None when type can't be determined
fn infer_expr_type(env: &TypeEnv, expr: &Expr) -> Option<Type> {
    match expr {
        Expr::Int(_) => Some(Type::Int),
        Expr::Float(_) => Some(Type::Float),
        Expr::Str(_) => Some(Type::String),
        Expr::Bool(_) => Some(Type::Bool),
        Expr::Null => None,
        Expr::Duration(_) => Some(Type::Timestamp),
        Expr::Ident(name) => env.lookup(name).cloned(),
        Expr::Binary { left, op, .. } => match op {
            BinOp::Eq
            | BinOp::Ne
            | BinOp::Lt
            | BinOp::Le
            | BinOp::Gt
            | BinOp::Ge
            | BinOp::And
            | BinOp::Or => Some(Type::Bool),
            _ => infer_expr_type(env, left),
        },
        Expr::Unary { op, operand } => match op {
            UnaryOp::Not => Some(Type::Bool),
            UnaryOp::Neg => infer_expr_type(env, operand),
        },
        Expr::Call { .. } => None, // Can't infer without function signatures
        Expr::Array(elems) => elems
            .first()
            .and_then(|e| infer_expr_type(env, e))
            .map(|t| Type::Array(Box::new(t))),
        _ => None,
    }
}

fn types_compatible(expected: &Type, actual: &Type) -> bool {
    match (expected, actual) {
        (Type::Float, Type::Int) => true, // int -> float promotion
        (Type::BoundedFloat { .. }, Type::Float) => true,
        (Type::BoundedFloat { .. }, Type::Int) => true,
        (Type::Optional(inner), other) => types_compatible(inner, other),
        (a, b) => a == b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    #[test]
    fn check_valid_program() {
        let prog = parser::parse(
            r#"
            memory Note {
                content: string
                created_at: timestamp
            }
            on save(content: string) {
                store(Note {
                    content: content,
                    created_at: now()
                })
            }
            query GetAll(): Note[] {
                from Note
                order by created_at desc
            }
        "#,
        )
        .unwrap();

        let result = check(&prog);
        assert!(result.is_ok());
    }

    #[test]
    fn check_unknown_memory_in_update() {
        let prog = parser::parse(
            r#"
            update NonExistent {
                on_access {
                    x = 1
                }
            }
        "#,
        )
        .unwrap();

        let result = check(&prog);
        assert!(result.is_err());
    }

    #[test]
    fn check_duplicate_fields() {
        let prog = parser::parse(
            r#"
            memory Bad {
                name: string
                name: int
            }
        "#,
        )
        .unwrap();

        let result = check(&prog);
        assert!(result.is_err());
    }
}
