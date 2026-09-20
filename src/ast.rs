//! 抽象语法树（AST）：解析器的输出，语义检查与字节码编译的输入。
//! 节点尽量带 `Span`，错误信息才能指回源码位置。

use crate::token::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    Int,
    Float,
    Bool,
    String,
    Void,
    Array(Box<TypeExpr>),
    /// 用户结构体类型名
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
pub struct StructFieldDecl {
    pub name: Ident,
    pub ty: TypeExpr,
}

#[derive(Debug, Clone)]
pub struct StructDecl {
    pub name: Ident,
    pub fields: Vec<StructFieldDecl>,
    pub span: Span,
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
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    Array {
        elems: Vec<Expr>,
        span: Span,
    },
    Field {
        base: Box<Expr>,
        name: Ident,
        span: Span,
    },
    StructLit {
        name: Ident,
        fields: Vec<(Ident, Expr)>,
        span: Span,
    },
    /// 仅用于 `for i in a..b` 的半开区间
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
        span: Span,
    },
    /// `recv.method(args)`；recv 为命名空间 `ns` 时是模块函数调用
    MethodCall {
        recv: Box<Expr>,
        method: Ident,
        args: Vec<Expr>,
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
            | Expr::Index { span, .. }
            | Expr::Array { span, .. }
            | Expr::Field { span, .. }
            | Expr::StructLit { span, .. }
            | Expr::Range { span, .. }
            | Expr::MethodCall { span, .. } => *span,
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

/// `name = value` 或 `name[index] = value` 或 `name.field = value`
#[derive(Debug, Clone)]
pub struct AssignStmt {
    pub name: Ident,
    pub index: Option<Expr>,
    /// 字段路径（按访问顺序）：`p.x` → ["x"]，`p.a.b` → ["a","b"]
    pub fields: Vec<Ident>,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct IfStmt {
    pub cond: Expr,
    pub then_block: Block,
    /// `else` 或 `else if`：`else_branch` 为 `If` 时表示 `else if`
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
pub struct Param {
    pub name: Ident,
    pub ty: TypeExpr,
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
pub struct FunDecl {
    pub name: Ident,
    /// `fun Point.sum(self: Point)` 时为 Some(Point)
    pub on_type: Option<Ident>,
    pub params: Vec<Param>,
    pub ret: TypeExpr,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Item {
    Fun(FunDecl),
    Struct(StructDecl),
    Const(ConstDecl),
    Import(ImportItem),
    Stmt(Stmt),
}

#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Item>,
}

impl Program {
    pub fn functions(&self) -> impl Iterator<Item = &FunDecl> {
        self.items.iter().filter_map(|it| match it {
            Item::Fun(f) => Some(f),
            _ => None,
        })
    }

    pub fn structs(&self) -> impl Iterator<Item = &StructDecl> {
        self.items.iter().filter_map(|it| match it {
            Item::Struct(s) => Some(s),
            _ => None,
        })
    }

    pub fn consts(&self) -> impl Iterator<Item = &ConstDecl> {
        self.items.iter().filter_map(|it| match it {
            Item::Const(c) => Some(c),
            _ => None,
        })
    }

    pub fn imports(&self) -> impl Iterator<Item = &ImportItem> {
        self.items.iter().filter_map(|it| match it {
            Item::Import(i) => Some(i),
            _ => None,
        })
    }

    pub fn top_level_stmts(&self) -> impl Iterator<Item = &Stmt> {
        self.items.iter().filter_map(|it| match it {
            Item::Stmt(s) => Some(s),
            _ => None,
        })
    }

    pub fn has_main(&self) -> bool {
        self.functions()
            .any(|f| f.name.name == "main" && f.on_type.is_none())
    }
}
