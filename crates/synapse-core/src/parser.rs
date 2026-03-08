use chumsky::prelude::*;

use crate::ast::*;
use crate::errors::{Result, SynapseError};
use crate::lexer::{tokenize, Token};
use crate::types::Type;

// ═══════════════════════════════════════════════════════════════
// PUBLIC API
// ═══════════════════════════════════════════════════════════════

pub fn parse(source: &str) -> Result<Program> {
    let tokens = tokenize(source);
    let toks: Vec<Token> = tokens.into_iter().map(|(t, _)| t).collect();

    // Build parsers in a dedicated thread with larger stack to handle
    // chumsky's deep type nesting in debug builds
    let result = std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            let r = build_program_parser()
                .parse(&toks)
                .into_result()
                .map_err(|errs| {
                    let msgs: Vec<String> = errs.iter().map(|e| format!("{e:?}")).collect();
                    SynapseError::Parse(msgs.join("; "))
                });
            r
        })
        .expect("spawn parser thread")
        .join()
        .expect("parser thread panicked");

    result
}

// ═══════════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════════

fn ident<'a>() -> impl Parser<'a, &'a [Token], String, extra::Err<Rich<'a, Token>>> + Clone {
    select! { Token::Ident(s) => s.clone() }
}

/// Accept identifiers AND contextual keywords that may appear as names
fn any_name<'a>() -> impl Parser<'a, &'a [Token], String, extra::Err<Rich<'a, Token>>> + Clone {
    select! {
        Token::Ident(s) => s.clone(),
        Token::TyString => "string".to_string(),
        Token::TyInt => "int".to_string(),
        Token::TyFloat => "float".to_string(),
        Token::TyBool => "bool".to_string(),
        Token::TyTimestamp => "timestamp".to_string(),
        Token::Memory => "memory".to_string(),
        Token::Query => "query".to_string(),
        Token::Update => "update".to_string(),
        Token::From => "from".to_string(),
        Token::Where => "where".to_string(),
        Token::Order => "order".to_string(),
        Token::Limit => "limit".to_string(),
        Token::Policy => "policy".to_string(),
    }
}

// ═══════════════════════════════════════════════════════════════
// TYPE PARSER
// ═══════════════════════════════════════════════════════════════

fn type_parser<'a>() -> impl Parser<'a, &'a [Token], Type, extra::Err<Rich<'a, Token>>> + Clone {
    let base = choice((
        just(Token::TyString).to(Type::String),
        just(Token::TyInt).to(Type::Int),
        just(Token::TyBool).to(Type::Bool),
        just(Token::TyTimestamp).to(Type::Timestamp),
        just(Token::TyFloat)
            .then(
                select! { Token::FloatLiteral(v) => v }
                    .or(select! { Token::IntLiteral(v) => v as f64 })
                    .then_ignore(just(Token::Comma))
                    .then(
                        select! { Token::FloatLiteral(v) => v }
                            .or(select! { Token::IntLiteral(v) => v as f64 })
                    )
                    .delimited_by(just(Token::LBracket), just(Token::RBracket))
                    .or_not(),
            )
            .map(|(_, bounds)| match bounds {
                Some((min, max)) => Type::BoundedFloat { min, max },
                std::option::Option::None => Type::Float,
            }),
        ident().map(Type::Named),
    ));

    base.then(
        choice((
            just(Token::Question).to('?'),
            just(Token::LBracket)
                .then_ignore(just(Token::RBracket))
                .to('['),
        ))
        .repeated()
        .collect::<Vec<_>>(),
    )
    .map(|(mut ty, mods)| {
        for m in mods {
            ty = match m {
                '?' => Type::Optional(Box::new(ty)),
                '[' => Type::Array(Box::new(ty)),
                _ => unreachable!(),
            };
        }
        ty
    })
}

// ═══════════════════════════════════════════════════════════════
// DURATION PARSER
// ═══════════════════════════════════════════════════════════════

fn duration<'a>() -> impl Parser<'a, &'a [Token], Duration, extra::Err<Rich<'a, Token>>> + Clone {
    select! {
        Token::DurationSec(v) => Duration { value: v, unit: DurationUnit::Second },
        Token::DurationMin(v) => Duration { value: v, unit: DurationUnit::Minute },
        Token::DurationHour(v) => Duration { value: v, unit: DurationUnit::Hour },
        Token::DurationDay(v) => Duration { value: v, unit: DurationUnit::Day },
        Token::DurationWeek(v) => Duration { value: v, unit: DurationUnit::Week },
    }
}

// ═══════════════════════════════════════════════════════════════
// PARAM LIST
// ═══════════════════════════════════════════════════════════════

fn params<'a>() -> impl Parser<'a, &'a [Token], Vec<Param>, extra::Err<Rich<'a, Token>>> + Clone {
    any_name()
        .then_ignore(just(Token::Colon))
        .then(type_parser())
        .map(|(name, ty)| Param { name, ty })
        .separated_by(just(Token::Comma))
        .allow_trailing()
        .collect::<Vec<_>>()
}

// ═══════════════════════════════════════════════════════════════
// MONOLITHIC PARSER BUILDER
//
// Everything is built in one function to avoid recreating recursive
// parser combinators on each call. Heavy use of .boxed() to reduce
// type nesting and stack frame sizes.
// ═══════════════════════════════════════════════════════════════

fn build_program_parser<'a>(
) -> impl Parser<'a, &'a [Token], Program, extra::Err<Rich<'a, Token>>> {
    // ─── Expression parser ───────────────────────────────────
    let expr = recursive(|expr: Recursive<dyn Parser<'a, &'a [Token], Expr, _>>| {
        let literal = select! {
            Token::IntLiteral(n) => Expr::Int(n),
            Token::FloatLiteral(v) => Expr::Float(v),
            Token::StringLiteral(s) => Expr::Str(s.clone()),
            Token::True => Expr::Bool(true),
            Token::False => Expr::Bool(false),
            Token::Null => Expr::Null,
            Token::DurationSec(v) => Expr::Duration(Duration { value: v, unit: DurationUnit::Second }),
            Token::DurationMin(v) => Expr::Duration(Duration { value: v, unit: DurationUnit::Minute }),
            Token::DurationHour(v) => Expr::Duration(Duration { value: v, unit: DurationUnit::Hour }),
            Token::DurationDay(v) => Expr::Duration(Duration { value: v, unit: DurationUnit::Day }),
            Token::DurationWeek(v) => Expr::Duration(Duration { value: v, unit: DurationUnit::Week }),
        }
        .boxed();

        // call arguments: [name:] expr
        let call_arg = ident()
            .then_ignore(just(Token::Colon))
            .or_not()
            .then(expr.clone())
            .map(|(name, value)| CallArg { name, value });

        let args_list = call_arg
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(just(Token::LParen), just(Token::RParen))
            .boxed();

        // struct init: Name { field: value, ... }
        let field_init = ident()
            .then_ignore(just(Token::Colon))
            .then(expr.clone())
            .map(|(name, value)| FieldInit { name, value });

        let struct_init = ident()
            .then(
                field_init
                    .separated_by(just(Token::Comma))
                    .allow_trailing()
                    .collect::<Vec<_>>()
                    .delimited_by(just(Token::LBrace), just(Token::RBrace)),
            )
            .map(|(name, fields)| Expr::StructInit { name, fields })
            .boxed();

        // array literal
        let array_lit = expr
            .clone()
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(just(Token::LBracket), just(Token::RBracket))
            .map(Expr::Array)
            .boxed();

        // lambda: x => body  or  (x, y) => body
        let single_lambda = ident()
            .then_ignore(just(Token::FatArrow))
            .then(expr.clone())
            .map(|(p, body)| Expr::Lambda {
                params: vec![p],
                body: Box::new(body),
            })
            .boxed();

        let multi_lambda = ident()
            .separated_by(just(Token::Comma))
            .collect::<Vec<_>>()
            .delimited_by(just(Token::LParen), just(Token::RParen))
            .then_ignore(just(Token::FatArrow))
            .then(expr.clone())
            .map(|(params, body)| Expr::Lambda {
                params,
                body: Box::new(body),
            })
            .boxed();

        // inline query: from Type where ... order by ... limit ...
        let inline_query = just(Token::From)
            .ignore_then(
                any_name()
                    .separated_by(just(Token::Comma))
                    .at_least(1)
                    .collect::<Vec<_>>(),
            )
            .then(just(Token::Where).ignore_then(expr.clone()).or_not())
            .then(
                just(Token::Order)
                    .ignore_then(just(Token::By))
                    .ignore_then(expr.clone())
                    .then(
                        choice((
                            just(Token::Asc).to(SortDir::Asc),
                            just(Token::Desc).to(SortDir::Desc),
                        ))
                        .or_not(),
                    )
                    .map(|(e, d)| OrderByClause {
                        expr: e,
                        direction: d.unwrap_or(SortDir::Asc),
                    })
                    .or_not(),
            )
            .then(just(Token::Limit).ignore_then(expr.clone()).or_not())
            .map(|(((from, wh), ob), lim)| {
                Expr::InlineQuery(Box::new(QueryBody {
                    from,
                    where_clause: wh,
                    order_by: ob,
                    limit: lim,
                }))
            })
            .boxed();

        // grouped: (expr)
        let grouped = expr
            .clone()
            .delimited_by(just(Token::LParen), just(Token::RParen))
            .boxed();

        let ident_expr = any_name().map(Expr::Ident).boxed();

        // atom: order matters for disambiguation
        let atom = choice((
            literal,
            multi_lambda,
            single_lambda,
            array_lit,
            inline_query,
            struct_init,
            grouped,
            ident_expr,
        ))
        .boxed();

        // postfix: .field, (args), [index], ?.field
        let postfix = atom
            .foldl(
                choice((
                    args_list.clone().map(PfOp::Call),
                    just(Token::Dot).ignore_then(any_name()).map(PfOp::Field),
                    just(Token::Question)
                        .ignore_then(just(Token::Dot))
                        .ignore_then(any_name())
                        .map(PfOp::OptField),
                    expr.clone()
                        .delimited_by(just(Token::LBracket), just(Token::RBracket))
                        .map(PfOp::Idx),
                ))
                .repeated(),
                |left, op| match op {
                    PfOp::Call(args) => Expr::Call {
                        func: Box::new(left),
                        args,
                    },
                    PfOp::Field(f) => Expr::FieldAccess {
                        object: Box::new(left),
                        field: f,
                    },
                    PfOp::OptField(f) => Expr::OptionalChain {
                        object: Box::new(left),
                        field: f,
                    },
                    PfOp::Idx(i) => Expr::IndexAccess {
                        object: Box::new(left),
                        index: Box::new(i),
                    },
                },
            )
            .boxed();

        // unary: not, -
        let unary = choice((
            just(Token::Not).to(UnaryOp::Not),
            just(Token::Minus).to(UnaryOp::Neg),
        ))
        .repeated()
        .foldr(postfix, |op, e| Expr::Unary {
            op,
            operand: Box::new(e),
        })
        .boxed();

        // multiplicative
        let mul = unary.clone().foldl(
            choice((
                just(Token::Star).to(BinOp::Mul),
                just(Token::Slash).to(BinOp::Div),
                just(Token::Percent).to(BinOp::Mod),
            ))
            .then(unary)
            .repeated(),
            |l, (op, r)| Expr::Binary {
                left: Box::new(l),
                op,
                right: Box::new(r),
            },
        ).boxed();

        // additive
        let add = mul.clone().foldl(
            choice((
                just(Token::Plus).to(BinOp::Add),
                just(Token::Minus).to(BinOp::Sub),
            ))
            .then(mul)
            .repeated(),
            |l, (op, r)| Expr::Binary {
                left: Box::new(l),
                op,
                right: Box::new(r),
            },
        ).boxed();

        // comparison
        let cmp = add.clone().foldl(
            choice((
                just(Token::EqEq).to(BinOp::Eq),
                just(Token::BangEq).to(BinOp::Ne),
                just(Token::LtEq).to(BinOp::Le),
                just(Token::Lt).to(BinOp::Lt),
                just(Token::GtEq).to(BinOp::Ge),
                just(Token::Gt).to(BinOp::Gt),
            ))
            .then(add)
            .repeated(),
            |l, (op, r)| Expr::Binary {
                left: Box::new(l),
                op,
                right: Box::new(r),
            },
        ).boxed();

        // logical AND
        let and_expr = cmp.clone().foldl(
            just(Token::And).to(BinOp::And).then(cmp).repeated(),
            |l, (op, r)| Expr::Binary {
                left: Box::new(l),
                op,
                right: Box::new(r),
            },
        ).boxed();

        // logical OR
        let or_expr = and_expr.clone().foldl(
            just(Token::Or).to(BinOp::Or).then(and_expr).repeated(),
            |l, (op, r)| Expr::Binary {
                left: Box::new(l),
                op,
                right: Box::new(r),
            },
        ).boxed();

        // pipe: |>
        or_expr
            .clone()
            .foldl(just(Token::PipeArrow).ignore_then(or_expr).repeated(), |l, r| {
                Expr::Pipe {
                    left: Box::new(l),
                    right: Box::new(r),
                }
            })
            .boxed()
    });

    // ─── Statement parser ────────────────────────────────────
    let stmt = recursive({
        let expr = expr.clone();
        move |stmt: Recursive<dyn Parser<'a, &'a [Token], Stmt, _>>| {
            let block = stmt
                .repeated()
                .collect::<Vec<_>>()
                .delimited_by(just(Token::LBrace), just(Token::RBrace))
                .boxed();

            let let_stmt = just(Token::Let)
                .ignore_then(ident())
                .then_ignore(just(Token::Eq))
                .then(expr.clone())
                .map(|(name, value)| Stmt::Let { name, value })
                .boxed();

            let if_stmt = just(Token::If)
                .ignore_then(expr.clone())
                .then(block.clone())
                .then(just(Token::Else).ignore_then(block.clone()).or_not())
                .map(|((condition, then_body), else_body)| Stmt::If {
                    condition,
                    then_body,
                    else_body,
                })
                .boxed();

            let for_stmt = just(Token::For)
                .ignore_then(ident())
                .then_ignore(just(Token::In))
                .then(expr.clone())
                .then(block)
                .map(|((var, iter), body)| Stmt::For { var, iter, body })
                .boxed();

            let return_stmt = just(Token::Return)
                .ignore_then(expr.clone().or_not())
                .map(Stmt::Return)
                .boxed();

            let expr_or_assign = expr
                .clone()
                .then(just(Token::Eq).ignore_then(expr.clone()).or_not())
                .map(|(lhs, rhs)| match rhs {
                    Some(value) => Stmt::Assign { target: lhs, value },
                    std::option::Option::None => Stmt::Expr(lhs),
                })
                .boxed();

            choice((let_stmt, if_stmt, for_stmt, return_stmt, expr_or_assign)).boxed()
        }
    });

    let stmts_block = stmt
        .repeated()
        .collect::<Vec<_>>()
        .delimited_by(just(Token::LBrace), just(Token::RBrace));

    // ─── Config ──────────────────────────────────────────────
    let config_value = choice((
        any_name()
            .then(
                select! { Token::StringLiteral(s) => s.clone() }
                    .delimited_by(just(Token::LParen), just(Token::RParen)),
            )
            .map(|(name, arg)| ConfigValue::FnCall { name, arg }),
        just(Token::None).to(ConfigValue::None),
    ));

    let config = just(Token::Config)
        .ignore_then(
            any_name()
                .then_ignore(just(Token::Colon))
                .then(config_value)
                .map(|(key, value)| ConfigEntry { key, value })
                .repeated()
                .collect::<Vec<_>>()
                .delimited_by(just(Token::LBrace), just(Token::RBrace)),
        )
        .map(|entries| Item::Config(ConfigBlock { entries }))
        .boxed();

    // ─── Memory ──────────────────────────────────────────────
    let decorator = choice((
        just(Token::Index).ignore_then(any_name()).map(Decorator::Index),
        just(Token::Invariant)
            .ignore_then(expr.clone())
            .map(Decorator::Invariant),
        just(Token::Extern).to(Decorator::Extern),
    ));

    let field_def = decorator
        .repeated()
        .collect::<Vec<_>>()
        .then(any_name())
        .then_ignore(just(Token::Colon))
        .then(type_parser())
        .then(just(Token::Eq).ignore_then(expr.clone()).or_not())
        .map(|(((decorators, name), ty), default)| FieldDef {
            name,
            ty,
            default,
            decorators,
        });

    let memory = just(Token::Memory)
        .ignore_then(ident())
        .then(
            field_def
                .repeated()
                .collect::<Vec<_>>()
                .delimited_by(just(Token::LBrace), just(Token::RBrace)),
        )
        .map(|(name, fields)| Item::Memory(MemoryDef { name, fields }))
        .boxed();

    // ─── Handler ─────────────────────────────────────────────
    let handler = just(Token::On)
        .ignore_then(any_name())
        .then(params().delimited_by(just(Token::LParen), just(Token::RParen)))
        .then(stmts_block.clone())
        .map(|((event, params), body)| Item::Handler(HandlerDef { event, params, body }))
        .boxed();

    // ─── Query ───────────────────────────────────────────────
    let query_body = just(Token::From)
        .ignore_then(
            any_name()
                .separated_by(just(Token::Comma))
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .then(just(Token::Where).ignore_then(expr.clone()).or_not())
        .then(
            just(Token::Order)
                .ignore_then(just(Token::By))
                .ignore_then(expr.clone())
                .then(
                    choice((
                        just(Token::Asc).to(SortDir::Asc),
                        just(Token::Desc).to(SortDir::Desc),
                    ))
                    .or_not(),
                )
                .map(|(e, d)| OrderByClause {
                    expr: e,
                    direction: d.unwrap_or(SortDir::Asc),
                })
                .or_not(),
        )
        .then(just(Token::Limit).ignore_then(expr.clone()).or_not())
        .map(|(((from, wh), ob), lim)| QueryBody {
            from,
            where_clause: wh,
            order_by: ob,
            limit: lim,
        });

    let query = just(Token::Query)
        .ignore_then(ident())
        .then(params().delimited_by(just(Token::LParen), just(Token::RParen)))
        .then_ignore(just(Token::Colon))
        .then(type_parser())
        .then(query_body.delimited_by(just(Token::LBrace), just(Token::RBrace)))
        .map(|(((name, params), return_ty), body)| {
            Item::Query(QueryDef {
                name,
                params,
                return_ty,
                body,
            })
        })
        .boxed();

    // ─── Update rules (shared between update & policy) ───────
    let update_rule = {
        let on_access = just(Token::OnAccess)
            .ignore_then(stmts_block.clone())
            .map(|body| UpdateRule::OnAccess { body })
            .boxed();

        let on_conflict = just(Token::OnConflict)
            .ignore_then(
                ident()
                    .then_ignore(just(Token::Comma))
                    .then(ident())
                    .delimited_by(just(Token::LParen), just(Token::RParen)),
            )
            .then(stmts_block.clone())
            .map(|((old_name, new_name), body)| UpdateRule::OnConflict {
                old_name,
                new_name,
                body,
            })
            .boxed();

        let every = just(Token::Every)
            .ignore_then(duration())
            .then(stmts_block.clone())
            .map(|(interval, body)| UpdateRule::Every { interval, body })
            .boxed();

        choice((on_access, on_conflict, every))
    };

    // ─── Update ──────────────────────────────────────────────
    let update = just(Token::Update)
        .ignore_then(ident())
        .then(
            update_rule
                .clone()
                .repeated()
                .collect::<Vec<_>>()
                .delimited_by(just(Token::LBrace), just(Token::RBrace)),
        )
        .map(|(target, rules)| Item::Update(UpdateDef { target, rules }))
        .boxed();

    // ─── Policy ──────────────────────────────────────────────
    let policy = just(Token::Policy)
        .ignore_then(ident())
        .then(
            update_rule
                .repeated()
                .collect::<Vec<_>>()
                .delimited_by(just(Token::LBrace), just(Token::RBrace)),
        )
        .map(|(name, rules)| Item::Policy(PolicyDef { name, rules }))
        .boxed();

    // ─── Extern fn ───────────────────────────────────────────
    let extern_fn = just(Token::Extern)
        .ignore_then(just(Token::Fn))
        .ignore_then(ident())
        .then(params().delimited_by(just(Token::LParen), just(Token::RParen)))
        .then(just(Token::Colon).ignore_then(type_parser()).or_not())
        .map(|((name, params), return_ty)| {
            Item::ExternFn(ExternFnDef {
                name,
                params,
                return_ty,
            })
        })
        .boxed();

    // ─── Item (any top-level construct) ──────────────────────
    let item = choice((
        config, memory, handler, query, update, policy, extern_fn,
    ))
    .boxed();

    // ─── Namespace or bare item ──────────────────────────────
    let ns_or_item = just(Token::Namespace)
        .ignore_then(ident())
        .then(
            item.clone()
                .repeated()
                .collect::<Vec<_>>()
                .delimited_by(just(Token::LBrace), just(Token::RBrace)),
        )
        .map(|(name, items)| Item::Namespace(NamespaceDef { name, items }))
        .or(item);

    // ─── Program ─────────────────────────────────────────────
    ns_or_item
        .repeated()
        .collect::<Vec<_>>()
        .map(|items| Program { items })
}

/// Helper enum for postfix operator parsing
#[derive(Debug, Clone)]
enum PfOp {
    Call(Vec<CallArg>),
    Field(String),
    OptField(String),
    Idx(Expr),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_config() {
        let result = parse("config { }");
        assert!(result.is_ok(), "error: {:?}", result.err());
        let prog = result.unwrap();
        assert_eq!(prog.items.len(), 1);
        assert!(matches!(&prog.items[0], Item::Config(_)));
    }

    #[test]
    fn parse_config_with_entries() {
        let result = parse(r#"
            config {
                storage: sqlite("./test.db")
                vector: qdrant("localhost:6333")
                graph: none
            }
        "#);
        assert!(result.is_ok(), "error: {:?}", result.err());
        let prog = result.unwrap();
        if let Item::Config(cfg) = &prog.items[0] {
            assert_eq!(cfg.entries.len(), 3);
            assert_eq!(cfg.entries[0].key, "storage");
            assert_eq!(cfg.entries[2].key, "graph");
            assert!(matches!(&cfg.entries[2].value, ConfigValue::None));
        } else {
            panic!("expected Config");
        }
    }

    #[test]
    fn parse_simple_memory() {
        let result = parse(r#"
            memory Note {
                content: string
                created_at: timestamp
            }
        "#);
        assert!(result.is_ok(), "error: {:?}", result.err());
        let prog = result.unwrap();
        if let Item::Memory(mem) = &prog.items[0] {
            assert_eq!(mem.name, "Note");
            assert_eq!(mem.fields.len(), 2);
            assert_eq!(mem.fields[0].name, "content");
            assert_eq!(mem.fields[0].ty, Type::String);
        } else {
            panic!("expected Memory");
        }
    }

    #[test]
    fn parse_memory_with_optional_and_array() {
        let result = parse(r#"
            memory Fact {
                content: string
                source: string?
                tags: string[]
                confidence: float[0.0,1.0]
            }
        "#);
        assert!(result.is_ok(), "error: {:?}", result.err());
        let prog = result.unwrap();
        if let Item::Memory(mem) = &prog.items[0] {
            assert_eq!(mem.fields[1].ty, Type::Optional(Box::new(Type::String)));
            assert_eq!(mem.fields[2].ty, Type::Array(Box::new(Type::String)));
            assert!(matches!(&mem.fields[3].ty, Type::BoundedFloat { .. }));
        } else {
            panic!("expected Memory");
        }
    }

    #[test]
    fn parse_handler() {
        let result = parse(r#"
            on save(content: string) {
                store(Note {
                    content: content,
                    created_at: now()
                })
            }
        "#);
        assert!(result.is_ok(), "error: {:?}", result.err());
    }

    #[test]
    fn parse_query() {
        let result = parse(r#"
            query GetAll(): Note[] {
                from Note
                order by created_at desc
                limit 10
            }
        "#);
        assert!(result.is_ok(), "error: {:?}", result.err());
    }

    #[test]
    fn parse_update() {
        let result = parse(r#"
            update Fact {
                on_access {
                    accessed_at = now()
                }
                every 24h {
                    confidence = confidence * 0.95
                    if confidence < 0.1 { delete() }
                }
            }
        "#);
        assert!(result.is_ok(), "error: {:?}", result.err());
    }

    #[test]
    fn parse_comments() {
        let result = parse(r#"
            # This is a comment
            memory Note {
                # field comment
                content: string
            }
        "#);
        assert!(result.is_ok(), "error: {:?}", result.err());
    }

    #[test]
    fn parse_extern_fn() {
        let result = parse(r#"
            @extern
            fn search(query: string, limit: int): ArchivalEntry[]
        "#);
        assert!(result.is_ok(), "error: {:?}", result.err());
        let prog = result.unwrap();
        assert!(matches!(&prog.items[0], Item::ExternFn(_)));
    }

    #[test]
    fn parse_namespace() {
        let result = parse(r#"
            namespace test_agent {
                memory Note {
                    content: string
                }
            }
        "#);
        assert!(result.is_ok(), "error: {:?}", result.err());
        let prog = result.unwrap();
        if let Item::Namespace(ns) = &prog.items[0] {
            assert_eq!(ns.name, "test_agent");
            assert_eq!(ns.items.len(), 1);
        } else {
            panic!("expected Namespace");
        }
    }

    #[test]
    fn parse_pipe_chain() {
        let result = parse(r#"
            on process(content: string) {
                content
                    |> extract()
                    |> filter(f => f.confidence > 0.7)
                    |> store()
            }
        "#);
        assert!(result.is_ok(), "error: {:?}", result.err());
    }

    #[test]
    fn parse_on_conflict() {
        let result = parse(r#"
            update Fact {
                on_conflict(old, new) {
                    if new.confidence > old.confidence {
                        supersede(old, new)
                    }
                }
            }
        "#);
        assert!(result.is_ok(), "error: {:?}", result.err());
    }

    #[test]
    fn parse_policy() {
        let result = parse(r#"
            policy Refresh {
                every 1h {
                    from ConnectorState
                    where last_sync < now() - 1h
                }
            }
        "#);
        assert!(result.is_ok(), "error: {:?}", result.err());
    }
}
