//! Token 与源位置。`Span` 用于 `file:line:col` 诊断。

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
    Ident(String),
    Int(i64),
    Float(f64),
    Str(String),
    // keywords
    Fun,
    Struct,
    Const,
    Import,
    As,
    Let,
    If,
    Else,
    While,
    For,
    In,
    Break,
    Continue,
    Return,
    True,
    False,
    TyInt,
    TyFloat,
    TyBool,
    TyString,
    TyVoid,
    // punctuation
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
    // operators
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
        let s = match self {
            TokenKind::Ident(n) => return write!(f, "identifier `{n}`"),
            TokenKind::Int(n) => return write!(f, "integer `{n}`"),
            TokenKind::Float(n) => return write!(f, "float `{n}`"),
            TokenKind::Str(s) => return write!(f, "string `{s:?}`"),
            TokenKind::Fun => "`fun`",
            TokenKind::Struct => "`struct`",
            TokenKind::Const => "`const`",
            TokenKind::Import => "`import`",
            TokenKind::As => "`as`",
            TokenKind::Let => "`let`",
            TokenKind::If => "`if`",
            TokenKind::Else => "`else`",
            TokenKind::While => "`while`",
            TokenKind::For => "`for`",
            TokenKind::In => "`in`",
            TokenKind::Break => "`break`",
            TokenKind::Continue => "`continue`",
            TokenKind::Return => "`return`",
            TokenKind::True => "`true`",
            TokenKind::False => "`false`",
            TokenKind::TyInt => "`int`",
            TokenKind::TyFloat => "`float`",
            TokenKind::TyBool => "`bool`",
            TokenKind::TyString => "`string`",
            TokenKind::TyVoid => "`void`",
            TokenKind::LParen => "`(`",
            TokenKind::RParen => "`)`",
            TokenKind::LBrace => "`{`",
            TokenKind::RBrace => "`}`",
            TokenKind::LBracket => "`[`",
            TokenKind::RBracket => "`]`",
            TokenKind::Comma => "`,`",
            TokenKind::Colon => "`:`",
            TokenKind::Semi => "`;`",
            TokenKind::Dot => "`.`",
            TokenKind::DotDot => "`..`",
            TokenKind::Arrow => "`->`",
            TokenKind::Plus => "`+`",
            TokenKind::Minus => "`-`",
            TokenKind::Star => "`*`",
            TokenKind::Slash => "`/`",
            TokenKind::Percent => "`%`",
            TokenKind::EqEq => "`==`",
            TokenKind::BangEq => "`!=`",
            TokenKind::Lt => "`<`",
            TokenKind::LtEq => "`<=`",
            TokenKind::Gt => "`>`",
            TokenKind::GtEq => "`>=`",
            TokenKind::AndAnd => "`&&`",
            TokenKind::OrOr => "`||`",
            TokenKind::Bang => "`!`",
            TokenKind::Assign => "`=`",
            TokenKind::Eof => "end of file",
        };
        write!(f, "{s}")
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
