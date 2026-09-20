//! Token 与源位置：词法分析的输出单元，解析器的输入。
//! `Span` 记录行/列，供后续阶段拼出 `file:line:col: error:` 诊断。

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub line: u32,
    pub col: u32,
}

impl Span {
    pub fn new(line: u32, col: u32) -> Self {
        Self { line, col }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // 字面量与标识符
    Ident(String),
    Int(i64),
    Float(f64),
    Str(String),

    // 关键字
    Fun,
    Struct,
    For,
    In,
    Break,
    Continue,
    Import,
    As,
    Let,
    If,
    Else,
    While,
    Return,
    True,
    False,
    // 类型关键字
    TyInt,
    TyFloat,
    TyBool,
    TyString,
    TyVoid,

    // 标点
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Colon,
    Semi,
    Dot,
    DotDot,
    Arrow,

    // 运算符
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    EqEq,
    BangEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    AndAnd,
    OrOr,
    Bang,
    Assign,

    Eof,
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Ident(s) => write!(f, "identifier `{s}`"),
            TokenKind::Int(n) => write!(f, "integer `{n}`"),
            TokenKind::Float(n) => write!(f, "float `{n}`"),
            TokenKind::Str(s) => write!(f, "string `{s:?}`"),
            TokenKind::Fun => write!(f, "`fun`"),
            TokenKind::Struct => write!(f, "`struct`"),
            TokenKind::For => write!(f, "`for`"),
            TokenKind::In => write!(f, "`in`"),
            TokenKind::Break => write!(f, "`break`"),
            TokenKind::Continue => write!(f, "`continue`"),
            TokenKind::Import => write!(f, "`import`"),
            TokenKind::As => write!(f, "`as`"),
            TokenKind::Let => write!(f, "`let`"),
            TokenKind::If => write!(f, "`if`"),
            TokenKind::Else => write!(f, "`else`"),
            TokenKind::While => write!(f, "`while`"),
            TokenKind::Return => write!(f, "`return`"),
            TokenKind::True => write!(f, "`true`"),
            TokenKind::False => write!(f, "`false`"),
            TokenKind::TyInt => write!(f, "`int`"),
            TokenKind::TyFloat => write!(f, "`float`"),
            TokenKind::TyBool => write!(f, "`bool`"),
            TokenKind::TyString => write!(f, "`string`"),
            TokenKind::TyVoid => write!(f, "`void`"),
            TokenKind::LParen => write!(f, "`(`"),
            TokenKind::RParen => write!(f, "`)`"),
            TokenKind::LBrace => write!(f, "`{{`"),
            TokenKind::RBrace => write!(f, "`}}`"),
            TokenKind::LBracket => write!(f, "`[`"),
            TokenKind::RBracket => write!(f, "`]`"),
            TokenKind::Comma => write!(f, "`,`"),
            TokenKind::Colon => write!(f, "`:`"),
            TokenKind::Semi => write!(f, "`;`"),
            TokenKind::Dot => write!(f, "`.`"),
            TokenKind::DotDot => write!(f, "`..`"),
            TokenKind::Arrow => write!(f, "`->`"),
            TokenKind::Plus => write!(f, "`+`"),
            TokenKind::Minus => write!(f, "`-`"),
            TokenKind::Star => write!(f, "`*`"),
            TokenKind::Slash => write!(f, "`/`"),
            TokenKind::Percent => write!(f, "`%`"),
            TokenKind::EqEq => write!(f, "`==`"),
            TokenKind::BangEq => write!(f, "`!=`"),
            TokenKind::Lt => write!(f, "`<`"),
            TokenKind::LtEq => write!(f, "`<=`"),
            TokenKind::Gt => write!(f, "`>`"),
            TokenKind::GtEq => write!(f, "`>=`"),
            TokenKind::AndAnd => write!(f, "`&&`"),
            TokenKind::OrOr => write!(f, "`||`"),
            TokenKind::Bang => write!(f, "`!`"),
            TokenKind::Assign => write!(f, "`=`"),
            TokenKind::Eof => write!(f, "end of file"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}
