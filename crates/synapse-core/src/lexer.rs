use logos::Logos;
use std::fmt;

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\n\r]+")]
pub enum Token {
    #[regex(r"#[^\n]*", logos::skip, priority = 100, allow_greedy = true)]
    Comment,

    // ───────────────────────────────────────────────────────────────
    // KEYWORDS
    // ───────────────────────────────────────────────────────────────
    #[token("config")]
    Config,

    #[token("memory")]
    Memory,

    #[token("namespace")]
    Namespace,

    #[token("query")]
    Query,

    #[token("update")]
    Update,

    #[token("on")]
    On,

    #[token("fn")]
    Fn,

    #[token("if")]
    If,

    #[token("else")]
    Else,

    #[token("let")]
    Let,

    #[token("mut")]
    Mut,

    #[token("return")]
    Return,

    #[token("for")]
    For,

    #[token("in")]
    In,

    #[token("policy")]
    Policy,

    #[token("none")]
    None,

    // ───────────────────────────────────────────────────────────────
    // DECORATORS
    // ───────────────────────────────────────────────────────────────
    #[token("@extern")]
    Extern,

    #[token("@index")]
    Index,

    #[token("@invariant")]
    Invariant,

    // ───────────────────────────────────────────────────────────────
    // UPDATE BLOCKS
    // ───────────────────────────────────────────────────────────────
    #[token("on_access")]
    OnAccess,

    #[token("on_conflict")]
    OnConflict,

    #[token("every")]
    Every,

    // ───────────────────────────────────────────────────────────────
    // QUERY CLAUSES
    // ───────────────────────────────────────────────────────────────
    #[token("where")]
    Where,

    #[token("order")]
    Order,

    #[token("limit")]
    Limit,

    #[token("by")]
    By,

    #[token("asc")]
    Asc,

    #[token("desc")]
    Desc,

    #[token("from")]
    From,

    #[token("and")]
    And,

    #[token("or")]
    Or,

    #[token("not")]
    Not,

    // ───────────────────────────────────────────────────────────────
    // TYPES
    // ───────────────────────────────────────────────────────────────
    #[token("string")]
    TyString,

    #[token("int")]
    TyInt,

    #[token("float")]
    TyFloat,

    #[token("bool")]
    TyBool,

    #[token("timestamp")]
    TyTimestamp,

    // ───────────────────────────────────────────────────────────────
    // VALUES
    // ───────────────────────────────────────────────────────────────
    #[token("true")]
    True,

    #[token("false")]
    False,

    #[token("null")]
    Null,

    // ───────────────────────────────────────────────────────────────
    // LITERALS
    // ───────────────────────────────────────────────────────────────
    #[regex(r"\d+\.\d+", |lex| lex.slice().parse::<f64>().unwrap(), priority = 10)]
    FloatLiteral(f64),

    #[regex(r"\d+s", |lex| lex.slice()[..lex.slice().len()-1].parse::<u64>().unwrap())]
    DurationSec(u64),

    #[regex(r"\d+m", |lex| lex.slice()[..lex.slice().len()-1].parse::<u64>().unwrap())]
    DurationMin(u64),

    #[regex(r"\d+h", |lex| lex.slice()[..lex.slice().len()-1].parse::<u64>().unwrap())]
    DurationHour(u64),

    #[regex(r"\d+d", |lex| lex.slice()[..lex.slice().len()-1].parse::<u64>().unwrap())]
    DurationDay(u64),

    #[regex(r"\d+w", |lex| lex.slice()[..lex.slice().len()-1].parse::<u64>().unwrap())]
    DurationWeek(u64),

    #[regex(r"\d+", |lex| lex.slice().parse::<i64>().unwrap(), priority = 5)]
    IntLiteral(i64),

    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string(), priority = 1)]
    Ident(std::string::String),

    #[regex(r#""[^"]*""#, |lex| { let s = lex.slice(); s[1..s.len()-1].to_string() })]
    #[regex(r"'[^']*'", |lex| { let s = lex.slice(); s[1..s.len()-1].to_string() })]
    StringLiteral(std::string::String),

    // ───────────────────────────────────────────────────────────────
    // OPERATORS
    // ───────────────────────────────────────────────────────────────
    #[token("+")]
    Plus,

    #[token("-")]
    Minus,

    #[token("*")]
    Star,

    #[token("/")]
    Slash,

    #[token("%")]
    Percent,

    #[token("=")]
    Eq,

    #[token("==")]
    EqEq,

    #[token("!=")]
    BangEq,

    #[token("<")]
    Lt,

    #[token("<=")]
    LtEq,

    #[token(">")]
    Gt,

    #[token(">=")]
    GtEq,

    #[token("|>")]
    PipeArrow,

    #[token("?")]
    Question,

    // ───────────────────────────────────────────────────────────────
    // DELIMITERS
    // ───────────────────────────────────────────────────────────────
    #[token(":")]
    Colon,

    #[token(";")]
    Semi,

    #[token(",")]
    Comma,

    #[token("(")]
    LParen,

    #[token(")")]
    RParen,

    #[token("[")]
    LBracket,

    #[token("]")]
    RBracket,

    #[token("{")]
    LBrace,

    #[token("}")]
    RBrace,

    #[token(".")]
    Dot,

    #[token("..")]
    DotDot,

    #[token("->")]
    Arrow,

    #[token("=>")]
    FatArrow,

    #[token("|")]
    Pipe,
}

/// Tokenize source code, returning (Token, span) pairs.
pub fn tokenize(source: &str) -> Vec<(Token, std::ops::Range<usize>)> {
    Token::lexer(source)
        .spanned()
        .filter_map(|(tok, span)| tok.ok().map(|t| (t, span)))
        .collect()
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Comment => unreachable!("comments are skipped"),
            Token::Config => write!(f, "config"),
            Token::Memory => write!(f, "memory"),
            Token::Namespace => write!(f, "namespace"),
            Token::Query => write!(f, "query"),
            Token::Update => write!(f, "update"),
            Token::On => write!(f, "on"),
            Token::Fn => write!(f, "fn"),
            Token::If => write!(f, "if"),
            Token::Else => write!(f, "else"),
            Token::Let => write!(f, "let"),
            Token::Mut => write!(f, "mut"),
            Token::Return => write!(f, "return"),
            Token::For => write!(f, "for"),
            Token::In => write!(f, "in"),
            Token::Policy => write!(f, "policy"),
            Token::None => write!(f, "none"),
            Token::Extern => write!(f, "@extern"),
            Token::Index => write!(f, "@index"),
            Token::Invariant => write!(f, "@invariant"),
            Token::OnAccess => write!(f, "on_access"),
            Token::OnConflict => write!(f, "on_conflict"),
            Token::Every => write!(f, "every"),
            Token::Where => write!(f, "where"),
            Token::Order => write!(f, "order"),
            Token::Limit => write!(f, "limit"),
            Token::By => write!(f, "by"),
            Token::Asc => write!(f, "asc"),
            Token::Desc => write!(f, "desc"),
            Token::From => write!(f, "from"),
            Token::And => write!(f, "and"),
            Token::Or => write!(f, "or"),
            Token::Not => write!(f, "not"),
            Token::TyString => write!(f, "string"),
            Token::TyInt => write!(f, "int"),
            Token::TyFloat => write!(f, "float"),
            Token::TyBool => write!(f, "bool"),
            Token::TyTimestamp => write!(f, "timestamp"),
            Token::True => write!(f, "true"),
            Token::False => write!(f, "false"),
            Token::Null => write!(f, "null"),
            Token::FloatLiteral(v) => write!(f, "{v}"),
            Token::DurationSec(v) => write!(f, "{v}s"),
            Token::DurationMin(v) => write!(f, "{v}m"),
            Token::DurationHour(v) => write!(f, "{v}h"),
            Token::DurationDay(v) => write!(f, "{v}d"),
            Token::DurationWeek(v) => write!(f, "{v}w"),
            Token::IntLiteral(v) => write!(f, "{v}"),
            Token::Ident(s) => write!(f, "{s}"),
            Token::StringLiteral(s) => write!(f, "\"{s}\""),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Star => write!(f, "*"),
            Token::Slash => write!(f, "/"),
            Token::Percent => write!(f, "%"),
            Token::Eq => write!(f, "="),
            Token::EqEq => write!(f, "=="),
            Token::BangEq => write!(f, "!="),
            Token::Lt => write!(f, "<"),
            Token::LtEq => write!(f, "<="),
            Token::Gt => write!(f, ">"),
            Token::GtEq => write!(f, ">="),
            Token::PipeArrow => write!(f, "|>"),
            Token::Question => write!(f, "?"),
            Token::Colon => write!(f, ":"),
            Token::Semi => write!(f, ";"),
            Token::Comma => write!(f, ","),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::LBracket => write!(f, "["),
            Token::RBracket => write!(f, "]"),
            Token::LBrace => write!(f, "{{"),
            Token::RBrace => write!(f, "}}"),
            Token::Dot => write!(f, "."),
            Token::DotDot => write!(f, ".."),
            Token::Arrow => write!(f, "->"),
            Token::FatArrow => write!(f, "=>"),
            Token::Pipe => write!(f, "|"),
        }
    }
}
