//! 语义类型（与 AST 中的 `TypeExpr` 对应）。
//! 检查器用它判断兼容性；运行时 `Value` 与之大体一一对应（void 无运行时值）。

use std::collections::HashMap;
use std::fmt;

use crate::ast::TypeExpr;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    Bool,
    Str,
    Void,
    Array(Box<Type>),
    /// 用户结构体，载荷为类型名
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
            TypeExpr::Array(inner) => Type::Array(Box::new(Type::from_ast(inner, structs)?)),
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
            Type::Struct(n) => write!(f, "{n}"),
        }
    }
}
