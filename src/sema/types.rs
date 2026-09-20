//! 语义类型。

use std::collections::HashMap;
use std::fmt;

use crate::syntax::ast::TypeExpr;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    Bool,
    Str,
    Void,
    Array(Box<Type>),
    Struct(String),
}

impl Type {
    pub fn from_ast(
        t: &TypeExpr,
        structs: &HashMap<String, Vec<(String, Type)>>,
    ) -> Result<Type, String> {
        Ok(match t {
            TypeExpr::Int => Type::Int,
            TypeExpr::Float => Type::Float,
            TypeExpr::Bool => Type::Bool,
            TypeExpr::String => Type::Str,
            TypeExpr::Void => Type::Void,
            TypeExpr::Array(i) => Type::Array(Box::new(Type::from_ast(i, structs)?)),
            TypeExpr::Named(n) => {
                if !structs.contains_key(n) {
                    return Err(format!("unknown type `{n}`"));
                }
                Type::Struct(n.clone())
            }
        })
    }

    pub fn is_numeric(&self) -> bool {
        matches!(self, Type::Int | Type::Float)
    }

    pub fn printable(&self) -> bool {
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
            Type::Array(t) => write!(f, "{t}[]"),
            Type::Struct(n) => write!(f, "{n}"),
        }
    }
}
