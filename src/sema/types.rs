//! 语义类型。
//!
//! 与 AST 里的 `TypeExpr` 对应；`Named(n)` 解析为 `Type::Struct(n)`。
//! 检查器用它判断兼容性；运行时 `Value` 与之大体一一对应（void 无运行时值）。

use std::collections::HashMap;
use std::fmt;

use crate::syntax::ast::TypeExpr;

/// 语义类型：检查器内部使用，比源码里的 `TypeExpr` 更「具体」。
///
/// | 变体 | 中文 | 源码写法 |
/// |------|------|----------|
/// | `Int` | 整数 | `int` |
/// | `Float` | 浮点 | `float` |
/// | `Bool` | 布尔 | `bool` |
/// | `Str` | 字符串 | `string` |
/// | `Void` | 无返回值 | `void` |
/// | `Array(Box<Type>)` | 数组 | `int[]`（元素类型在 Box 里） |
/// | `Struct(名字)` | 结构体 | `Point` |
///
/// 和 `TypeExpr` 的关系：解析得到 `TypeExpr` → 检查时通过结构体表解析成 `Type`。
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    Bool,
    Str,
    Void,
    Array(Box<Type>),
    /// 结构体类型名
    Struct(String),
    /// `&T` / `&mut T`：安全引用（可空）
    Ref {
        mutable: bool,
        inner: Box<Type>,
    },
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
            TypeExpr::Ref { mutable, inner } => Type::Ref {
                mutable: *mutable,
                inner: Box::new(Type::from_ast(inner, structs)?),
            },
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
            Type::Ref { mutable, inner } => {
                if *mutable {
                    write!(f, "&mut {inner}")
                } else {
                    write!(f, "&{inner}")
                }
            }
        }
    }
}
