//! 抽象语法树（AST）：解析器输出、检查与编译的输入。
//! 节点尽量带 `Span`，错误才能指回源码。

use crate::syntax::token::Span;

/// 源码类型标注。
#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    Int,
    Float,
    Bool,
    String,
    Void,
    /// `T[]`
    Array(Box<TypeExpr>),
    /// 结构体名
    Named(String),
    /// `&T` 或 `&mut T`
    Ref {
        mutable: bool,
        inner: Box<TypeExpr>,
    },
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

/// 插值字符串片段
#[derive(Debug, Clone)]
pub enum InterpPart {
    Text(String),
    Expr(Expr),
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
    pub on_type: Option<Ident>,
    pub params: Vec<Param>,
    pub ret: TypeExpr,
    pub body: Block,
    pub span: Span,
}

/// 结构体字面量里的一个字段初始化
#[derive(Debug, Clone)]
pub struct FieldInit {
    pub name: Ident,
    /// `None` 表示简写 `{ x }`，含义为 `{ x: x }`
    pub value: Option<Expr>,
}

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
    /// `s"..."` 插值串（parts 已拆好）
    Interp {
        parts: Vec<InterpPart>,
        span: Span,
    },
    /// `nil` 空引用
    Nil {
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
    /// `&e` / `&mut e`
    Ref {
        mutable: bool,
        target: Box<Expr>,
        span: Span,
    },
    /// `*p`
    Deref {
        ptr: Box<Expr>,
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
        fields: Vec<FieldInit>,
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
            | Expr::Interp { span, .. }
            | Expr::Nil { span }
            | Expr::Unary { span, .. }
            | Expr::Ref { span, .. }
            | Expr::Deref { span, .. }
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

#[derive(Debug, Clone)]
pub struct AssignStmt {
    pub name: Ident,
    pub index: Option<Expr>,
    pub fields: Vec<Ident>,
    pub value: Expr,
    pub span: Span,
    /// `*p = v` 形式
    pub via_deref: bool,
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

/// 表达式语句；函数体末尾无分号时 `implicit_return=true`
#[derive(Debug, Clone)]
pub struct ExprStmt {
    pub expr: Expr,
    pub span: Span,
    pub implicit_return: bool,
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
    Import(ImportStmt),
    Stmt(Stmt),
}

// 名字冲突时用完整名
pub type ImportStmt = ImportItem;

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
