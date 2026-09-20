//! 抽象语法树（AST）。
//!
//! 解析器的输出、语义检查与字节码编译的输入。
//! 尽量在节点上保留 `Span`，错误才能指回源码位置。

use crate::syntax::token::Span;

/// 源码里写出来的类型标注（尚未解析成语义类型 `Type`）。
#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    Int,
    Float,
    Bool,
    String,
    Void,
    Array(Box<TypeExpr>),
    Named(String),
}

impl TypeExpr {
    pub fn is_void(&self) -> bool {
        matches!(self, TypeExpr::Void)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
}

#[derive(Debug, Clone)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FieldDecl {
    pub name: Ident,
    pub ty: TypeExpr,
}

#[derive(Debug, Clone)]
pub struct StructDecl {
    pub name: Ident,
    pub fields: Vec<FieldDecl>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ConstDecl {
    pub name: Ident,
    pub ty: TypeExpr,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ImportItem {
    pub path: String,
    pub alias: Option<Ident>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: Ident,
    pub ty: TypeExpr,
}

#[derive(Debug, Clone)]
pub struct FunDecl {
    pub name: Ident,
    /// `fun Point.sum(self: Point)` → Some("Point")
    pub on_type: Option<Ident>,
    pub params: Vec<Param>,
    pub ret: TypeExpr,
    pub body: Block,
    pub span: Span,
}

/// 表达式节点。`Range` 仅应出现在 `for-in` 的迭代式中。
#[derive(Debug, Clone)]
pub enum Expr {
    Int {
        value: i64,
        span: Span,
    },
    Float {
        value: f64,
        span: Span,
    },
    Bool {
        value: bool,
        span: Span,
    },
    Str {
        value: String,
        span: Span,
    },
    Var {
        name: Ident,
    },
    Unary {
        op: UnOp,
        expr: Box<Expr>,
        span: Span,
    },
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        span: Span,
    },
    Call {
        callee: Ident,
        args: Vec<Expr>,
        span: Span,
    },
    MethodCall {
        recv: Box<Expr>,
        method: Ident,
        args: Vec<Expr>,
        span: Span,
    },
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    Field {
        base: Box<Expr>,
        name: Ident,
        span: Span,
    },
    Array {
        elems: Vec<Expr>,
        span: Span,
    },
    StructLit {
        name: Ident,
        fields: Vec<(Ident, Expr)>,
        span: Span,
    },
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Int { span, .. }
            | Expr::Float { span, .. }
            | Expr::Bool { span, .. }
            | Expr::Str { span, .. }
            | Expr::Unary { span, .. }
            | Expr::Binary { span, .. }
            | Expr::Call { span, .. }
            | Expr::MethodCall { span, .. }
            | Expr::Index { span, .. }
            | Expr::Field { span, .. }
            | Expr::Array { span, .. }
            | Expr::StructLit { span, .. }
            | Expr::Range { span, .. } => *span,
            Expr::Var { name } => name.span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LetStmt {
    pub name: Ident,
    pub ty: Option<TypeExpr>,
    pub value: Expr,
    pub span: Span,
}

/// 赋值目标：`name`、`name[index]`，以及 `fields` 字段路径（可多层）。
#[derive(Debug, Clone)]
pub struct AssignStmt {
    pub name: Ident,
    pub index: Option<Expr>,
    pub fields: Vec<Ident>,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct IfStmt {
    pub cond: Expr,
    pub then_block: Block,
    pub else_branch: Option<ElseBranch>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ElseBranch {
    Block(Block),
    If(Box<IfStmt>),
}

#[derive(Debug, Clone)]
pub struct WhileStmt {
    pub cond: Expr,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ForStmt {
    pub var: Ident,
    pub iter: Expr,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ReturnStmt {
    pub value: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ExprStmt {
    pub expr: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let(LetStmt),
    Assign(AssignStmt),
    If(IfStmt),
    While(WhileStmt),
    For(ForStmt),
    Break(Span),
    Continue(Span),
    Return(ReturnStmt),
    Expr(ExprStmt),
    Block(Block),
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Item {
    Struct(StructDecl),
    Const(ConstDecl),
    Fun(FunDecl),
    Import(ImportItem),
    Stmt(Stmt),
}

/// 一个 `.lg` 文件解析后的完整语法树。
#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Item>,
}

impl Program {
    pub fn structs(&self) -> impl Iterator<Item = &StructDecl> {
        self.items.iter().filter_map(|i| match i {
            Item::Struct(s) => Some(s),
            _ => None,
        })
    }

    pub fn consts(&self) -> impl Iterator<Item = &ConstDecl> {
        self.items.iter().filter_map(|i| match i {
            Item::Const(c) => Some(c),
            _ => None,
        })
    }

    pub fn functions(&self) -> impl Iterator<Item = &FunDecl> {
        self.items.iter().filter_map(|i| match i {
            Item::Fun(f) => Some(f),
            _ => None,
        })
    }

    pub fn imports(&self) -> impl Iterator<Item = &ImportItem> {
        self.items.iter().filter_map(|i| match i {
            Item::Import(x) => Some(x),
            _ => None,
        })
    }

    pub fn top_level_stmts(&self) -> impl Iterator<Item = &Stmt> {
        self.items.iter().filter_map(|i| match i {
            Item::Stmt(s) => Some(s),
            _ => None,
        })
    }

    pub fn has_main(&self) -> bool {
        self.functions()
            .any(|f| f.on_type.is_none() && f.name.name == "main")
    }
}
