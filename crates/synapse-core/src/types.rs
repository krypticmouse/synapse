use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Type {
    String,
    Int,
    Float,
    BoundedFloat { min: f64, max: f64 },
    Bool,
    Timestamp,
    Optional(Box<Type>),
    Array(Box<Type>),
    Named(std::string::String),
}

impl Type {
    pub fn is_numeric(&self) -> bool {
        matches!(self, Type::Int | Type::Float | Type::BoundedFloat { .. })
    }

    pub fn is_optional(&self) -> bool {
        matches!(self, Type::Optional(_))
    }

    pub fn inner_type(&self) -> &Type {
        match self {
            Type::Optional(inner) | Type::Array(inner) => inner,
            other => other,
        }
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::String => write!(f, "string"),
            Type::Int => write!(f, "int"),
            Type::Float => write!(f, "float"),
            Type::BoundedFloat { min, max } => write!(f, "float[{min},{max}]"),
            Type::Bool => write!(f, "bool"),
            Type::Timestamp => write!(f, "timestamp"),
            Type::Optional(inner) => write!(f, "{inner}?"),
            Type::Array(inner) => write!(f, "{inner}[]"),
            Type::Named(name) => write!(f, "{name}"),
        }
    }
}
