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

/// 二元运算符（左右各有一个操作数）。
///
/// | 变体 | 中文 | 源码 |
/// |------|------|------|
/// | `Add` | 加 | `+`（数字相加或字符串拼接） |
/// | `Sub` `Mul` `Div` `Rem` | 减 乘 除 取余 | `-` `*` `/` `%` |
/// | `Eq` `Ne` | 等于 / 不等于 | `==` `!=` |
/// | `Lt` `Le` `Gt` `Ge` | 小于 / ≤ / 大于 / ≥ | `<` `<=` `>` `>=` |
/// | `And` `Or` | 逻辑与 / 或（短路） | `&&` `\|\|` |
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

/// 一元运算符（只有一个操作数）。
///
/// | 变体 | 中文 | 源码 |
/// |------|------|------|
/// | `Neg` | 取负 | `-x` |
/// | `Not` | 逻辑非 | `!flag` |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
}

/// 名字 + 位置。`name` 是标识符文本，`span` 用于报错定位。
#[derive(Debug, Clone)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

/// 结构体里的一个字段：`name` 字段名，`ty` 字段类型。
#[derive(Debug, Clone)]
pub struct FieldDecl {
    pub name: Ident,
    pub ty: TypeExpr,
}

/// `struct 名 { 字段... }` 的完整声明。
/// `fields` 按源码顺序；编译时也按这个顺序在栈上排布。
#[derive(Debug, Clone)]
pub struct StructDecl {
    pub name: Ident,
    pub fields: Vec<FieldDecl>,
    pub span: Span,
}

/// `const 名: 类型 = 表达式;`
/// `value` 在语义阶段会被「折叠」成具体数值。
#[derive(Debug, Clone)]
pub struct ConstDecl {
    pub name: Ident,
    pub ty: TypeExpr,
    pub value: Expr,
    pub span: Span,
}

/// `import "路径" [as 别名];`
/// - 无 `alias`：扁平合入，符号用原名
/// - 有 `alias`：变成 `别名.原名`
#[derive(Debug, Clone)]
pub struct ImportItem {
    pub path: String,
    pub alias: Option<Ident>,
    pub span: Span,
}

/// 函数的一个参数：`name` 参数名，`ty` 参数类型。
#[derive(Debug, Clone)]
pub struct Param {
    pub name: Ident,
    pub ty: TypeExpr,
}

/// 函数声明（普通函数或结构体方法）。
///
/// - `name`：函数名（方法时只是方法名，如 `sum`）
/// - `on_type`：`Some("Point")` 表示这是 `Point` 的方法；普通函数为 `None`
/// - `params`：参数列表；方法的第一个参数是接收者 `self`
/// - `ret`：返回类型（可为 `void`）
/// - `body`：函数体语句块
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

/// `let 名字 [: 类型] = 值;`
/// `ty` 为 `None` 时，类型完全由 `value` 推出。
#[derive(Debug, Clone)]
pub struct LetStmt {
    pub name: Ident,
    pub ty: Option<TypeExpr>,
    pub value: Expr,
    pub span: Span,
}

/// 赋值语句：可以是 `x = v`、`a[i] = v`、`p.f = v` 或 `p.a.b = v`。
///
/// - `name`：被赋值的变量
/// - `index`：若有，表示先做数组下标
/// - `fields`：字段路径（可多层）
/// - `value`：等号右边的表达式
#[derive(Debug, Clone)]
pub struct AssignStmt {
    pub name: Ident,
    pub index: Option<Expr>,
    pub fields: Vec<Ident>,
    pub value: Expr,
    pub span: Span,
}

/// `if 条件 { 块 } [else 分支]`
#[derive(Debug, Clone)]
pub struct IfStmt {
    /// 条件，类型必须是 `bool`
    pub cond: Expr,
    /// 条件为真时执行
    pub then_block: Block,
    /// `else` 或 `else if`（见 `ElseBranch`）
    pub else_branch: Option<ElseBranch>,
    pub span: Span,
}

/// `if` 的否则分支。
///
/// | 变体 | 中文 |
/// |------|------|
/// | `Block` | `else { ... }` |
/// | `If` | `else if ...`（嵌套的 if） |
#[derive(Debug, Clone)]
pub enum ElseBranch {
    Block(Block),
    If(Box<IfStmt>),
}

/// `while 条件 { 块 }`：条件为真就一直循环。
#[derive(Debug, Clone)]
pub struct WhileStmt {
    pub cond: Expr,
    pub body: Block,
    pub span: Span,
}

/// `for 变量 in 迭代式 { 块 }`
///
/// `iter` 只能是：
/// - 数组表达式：每次取出一个元素
/// - `start..end`：整数半开区间
#[derive(Debug, Clone)]
pub struct ForStmt {
    /// 循环变量名（作用域仅在循环体内）
    pub var: Ident,
    pub iter: Expr,
    pub body: Block,
    pub span: Span,
}

/// `return;` 或 `return 表达式;`
#[derive(Debug, Clone)]
pub struct ReturnStmt {
    pub value: Option<Expr>,
    pub span: Span,
}

/// 表达式语句：只为了副作用，例如 `print(1);`
#[derive(Debug, Clone)]
pub struct ExprStmt {
    pub expr: Expr,
    pub span: Span,
}

/// 所有语句的总和。
///
/// | 变体 | 中文 |
/// |------|------|
/// | `Let` | 变量声明 |
/// | `Assign` | 赋值 |
/// | `If` / `While` / `For` | 条件与循环 |
/// | `Break` / `Continue` | 循环控制 |
/// | `Return` | 函数返回 |
/// | `Expr` | 表达式语句 |
/// | `Block` | 独立代码块 |
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

/// `{ 语句... }`：进入时新开一层作用域，离开时该层声明的变量失效。
#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

/// 顶层条目：一个 `.lg` 文件里的「大块」。
///
/// | 变体 | 中文 |
/// |------|------|
/// | `Struct` | 结构体声明 |
/// | `Const` | 常量声明 |
/// | `Fun` | 函数/方法 |
/// | `Import` | 导入其它文件 |
/// | `Stmt` | 顶层语句（无 `main` 时才会执行） |
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
