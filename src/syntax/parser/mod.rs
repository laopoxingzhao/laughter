//! 递归下降语法分析（模块入口：Parser 状态与工具）。
//!
//! 子模块：`decl`（顶层声明）、`stmt`（语句）、`expr`（表达式）。

use crate::syntax::ast::*;
use crate::syntax::token::{Span, Token, TokenKind};

mod decl;
mod expr;
mod stmt;

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

/// 解析器状态：`tokens` 为完整 Token 流，`pos` 为当前下标。
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    /// 解析整个程序；循环识别顶层 item，直到 `Eof`。
    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut items = Vec::new();
        while !self.check(&TokenKind::Eof) {
            if self.check(&TokenKind::Struct) {
                items.push(Item::Struct(self.struct_decl()?));
            } else if self.check(&TokenKind::Const) {
                items.push(Item::Const(self.const_decl()?));
            } else if self.check(&TokenKind::Fun) {
                items.push(Item::Fun(self.fun_decl()?));
            } else if self.check(&TokenKind::Import) {
                items.push(Item::Import(self.import_item()?));
            } else {
                items.push(Item::Stmt(self.stmt()?));
            }
        }
        Ok(Program { items })
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    fn check(&self, k: &TokenKind) -> bool {
        std::mem::discriminant(self.kind()) == std::mem::discriminant(k)
    }

    fn advance(&mut self) -> Token {
        let t = self.tokens[self.pos.min(self.tokens.len() - 1)].clone();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        t
    }

    fn expect(&mut self, k: TokenKind, what: &str) -> Result<Token, ParseError> {
        if self.check(&k) {
            Ok(self.advance())
        } else {
            Err(ParseError {
                message: format!("期望 {what}，实际是 {}", self.kind()),
                span: self.peek().span,
            })
        }
    }

    fn expect_ident(&mut self) -> Result<Ident, ParseError> {
        let t = self.peek().clone();
        if let TokenKind::Ident(n) = t.kind {
            self.advance();
            Ok(Ident {
                name: n,
                span: t.span,
            })
        } else {
            Err(ParseError {
                message: format!("期望标识符，实际是 {}", t.kind),
                span: t.span,
            })
        }
    }

    /// 仅 `{ ident :` 视为结构体字面量；`{ }` 留给控制流空块。
    fn struct_lit_ahead(&self) -> bool {
        if self.pos + 2 >= self.tokens.len() {
            return false;
        }
        matches!(&self.tokens[self.pos + 1].kind, TokenKind::Ident(_))
            && matches!(&self.tokens[self.pos + 2].kind, TokenKind::Colon)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::lexer::Lexer;

    fn parse(src: &str) -> Program {
        let toks = Lexer::new(src).tokenize().unwrap();
        Parser::new(toks).parse_program().unwrap()
    }

    #[test]
    fn parse_struct_const_fun() {
        let p = parse(
            r#"
            struct P { x: int }
            const N: int = 2 + 3;
            fun P.get(self: P) -> int { return self.x; }
            fun main() -> void { let p = P { x: 1 }; print(p.get()); }
            "#,
        );
        assert_eq!(p.structs().count(), 1);
        assert_eq!(p.consts().count(), 1);
        assert!(p.has_main());
    }

    #[test]
    fn empty_for_body_ok() {
        let p = parse("fun main() -> void { let a: int[] = [1]; for x in a { } }");
        assert!(p.has_main());
    }
}
