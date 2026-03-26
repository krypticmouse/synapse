use crate::types::Type;

// ═══════════════════════════════════════════════════════════════
// TOP-LEVEL PROGRAM
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub enum Item {
    Config(ConfigBlock),
    Channel(ChannelDef),
    Namespace(NamespaceDef),
    Memory(MemoryDef),
    Handler(HandlerDef),
    Query(QueryDef),
    Update(UpdateDef),
    Policy(PolicyDef),
    ExternFn(ExternFnDef),
}

// ═══════════════════════════════════════════════════════════════
// CONFIG
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct ConfigBlock {
    pub entries: Vec<ConfigEntry>,
}

#[derive(Debug, Clone)]
pub struct ConfigEntry {
    pub key: String,
    pub value: ConfigValue,
}

#[derive(Debug, Clone)]
pub enum ConfigValue {
    FnCall { name: String, arg: String },
    None,
    Auto,
    Dict(Vec<(String, ConfigValue)>),
}

// ═══════════════════════════════════════════════════════════════
// CHANNEL
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct ChannelDef {
    pub name: String,
    pub source: String,
    pub config: Vec<ConfigEntry>,
    pub poll_interval: Option<Duration>,
    pub events: Vec<ChannelEventHandler>,
}

#[derive(Debug, Clone)]
pub struct ChannelEventHandler {
    pub event: String,
    pub target: Option<String>,
    pub params: Vec<Param>,
    pub body: Vec<Stmt>,
}

// ═══════════════════════════════════════════════════════════════
// NAMESPACE
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct NamespaceDef {
    pub name: String,
    pub items: Vec<Item>,
}

// ═══════════════════════════════════════════════════════════════
// MEMORY
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct MemoryDef {
    pub name: String,
    pub fields: Vec<FieldDef>,
    /// Field names to index (from standalone @index name)
    pub indexes: Vec<String>,
    /// Invariant expressions (from standalone @invariant expr)
    pub invariants: Vec<Expr>,
}

#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: String,
    pub ty: Type,
    pub default: Option<Expr>,
    pub decorators: Vec<Decorator>,
}

#[derive(Debug, Clone)]
pub enum Decorator {
    Index(String),
    Invariant(Expr),
    Extern,
}

// ═══════════════════════════════════════════════════════════════
// HANDLER
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct HandlerDef {
    pub event: String,
    pub params: Vec<Param>,
    pub body: Vec<Stmt>,
}

// ═══════════════════════════════════════════════════════════════
// QUERY
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct QueryDef {
    pub name: String,
    pub params: Vec<Param>,
    pub return_ty: Type,
    pub body: QueryBody,
}

#[derive(Debug, Clone)]
pub struct QueryBody {
    pub from: Vec<String>,
    pub where_clause: Option<Expr>,
    pub order_by: Option<OrderByClause>,
    pub limit: Option<Expr>,
}

#[derive(Debug, Clone)]
pub struct OrderByClause {
    pub expr: Expr,
    pub direction: SortDir,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortDir {
    Asc,
    Desc,
}

// ═══════════════════════════════════════════════════════════════
// UPDATE
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct UpdateDef {
    pub target: String,
    pub rules: Vec<UpdateRule>,
}

#[derive(Debug, Clone)]
pub enum UpdateRule {
    OnAccess {
        body: Vec<Stmt>,
    },
    OnConflict {
        old_name: String,
        new_name: String,
        body: Vec<Stmt>,
    },
    Every {
        interval: Duration,
        body: Vec<Stmt>,
    },
}

// ═══════════════════════════════════════════════════════════════
// POLICY
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct PolicyDef {
    pub name: String,
    pub rules: Vec<UpdateRule>,
}

// ═══════════════════════════════════════════════════════════════
// EXTERN FUNCTION
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct ExternFnDef {
    pub name: String,
    pub params: Vec<Param>,
    pub return_ty: Option<Type>,
}

// ═══════════════════════════════════════════════════════════════
// SHARED PRIMITIVES
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Duration {
    pub value: u64,
    pub unit: DurationUnit,
}

impl Duration {
    pub fn to_secs(&self) -> u64 {
        match self.unit {
            DurationUnit::Second => self.value,
            DurationUnit::Minute => self.value * 60,
            DurationUnit::Hour => self.value * 3600,
            DurationUnit::Day => self.value * 86400,
            DurationUnit::Week => self.value * 604800,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DurationUnit {
    Second,
    Minute,
    Hour,
    Day,
    Week,
}

// ═══════════════════════════════════════════════════════════════
// STATEMENTS
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub enum Stmt {
    Let {
        name: String,
        value: Expr,
    },
    Assign {
        target: Expr,
        value: Expr,
    },
    If {
        condition: Expr,
        then_body: Vec<Stmt>,
        else_body: Option<Vec<Stmt>>,
    },
    For {
        var: String,
        iter: Expr,
        body: Vec<Stmt>,
    },
    Return(Option<Expr>),
    Expr(Expr),
}

// ═══════════════════════════════════════════════════════════════
// EXPRESSIONS
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub enum Expr {
    // Literals
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Null,
    Duration(Duration),

    // Identifiers & access
    Ident(String),
    FieldAccess {
        object: Box<Expr>,
        field: String,
    },
    OptionalChain {
        object: Box<Expr>,
        field: String,
    },
    IndexAccess {
        object: Box<Expr>,
        index: Box<Expr>,
    },

    // Operators
    Binary {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
    },

    // Calls
    Call {
        func: Box<Expr>,
        args: Vec<CallArg>,
    },

    // Pipe chain: left |> right
    Pipe {
        left: Box<Expr>,
        right: Box<Expr>,
    },

    // Struct construction: Name { field: value, ... }
    StructInit {
        name: String,
        fields: Vec<FieldInit>,
    },

    // Lambda: param => body  OR  (p1, p2) => body
    Lambda {
        params: Vec<String>,
        body: Box<Expr>,
    },

    // Array literal: [a, b, c]
    Array(Vec<Expr>),

    // Inline query expression: from X where ... order by ... limit ...
    InlineQuery(Box<QueryBody>),

    // Alias: expr as name (used in query WHERE for scoring aliases)
    Alias {
        expr: Box<Expr>,
        alias: String,
    },
}

#[derive(Debug, Clone)]
pub struct CallArg {
    pub name: Option<String>,
    pub value: Expr,
}

#[derive(Debug, Clone)]
pub struct FieldInit {
    pub name: String,
    pub value: Expr,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOp {
    Neg,
    Not,
}
