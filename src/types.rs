use std::fmt;

use crate::ast::TypeExpr;

/// Semantic type used by the checker and mirrored by runtime values.
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    Bool,
    Str,
    Void,
    Array(Box<Type>),
}

impl Type {
    pub fn from_ast(t: &TypeExpr) -> Result<Type, String> {
        Ok(match t {
            TypeExpr::Int => Type::Int,
            TypeExpr::Float => Type::Float,
            TypeExpr::Bool => Type::Bool,
            TypeExpr::String => Type::Str,
            TypeExpr::Void => Type::Void,
            TypeExpr::Array(inner) => Type::Array(Box::new(Type::from_ast(inner)?)),
        })
    }

    pub fn is_numeric(&self) -> bool {
        matches!(self, Type::Int | Type::Float)
    }

    pub fn can_print(&self) -> bool {
        !matches!(self, Type::Void)
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Int => write!(f, "int"),
            Type::Float => write!(f, "float"),
            Type::Bool => write!(f, "bool"),
            Type::Str => write!(f, "string"),
            Type::Void => write!(f, "void"),
            Type::Array(inner) => write!(f, "{inner}[]"),
        }
    }
}
