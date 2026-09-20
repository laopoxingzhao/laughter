//! 语法分析：递归下降，把 `Token` 流解析为 AST。
//! 表达式优先级：`||` → `&&` → 相等 → 比较 → `+-` → `*/%` → 一元 → 后缀索引/调用。

use crate::ast::*;
use crate::token::{Span, Token, TokenKind};

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut items = Vec::new();
        while !self.check(&TokenKind::Eof) {
            if self.check(&TokenKind::Fun) {
                items.push(Item::Fun(self.fun_decl()?));
            } else {
                items.push(Item::Stmt(self.stmt()?));
            }
        }
        Ok(Program { items })
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(self.peek_kind()) == std::mem::discriminant(kind)
    }

    fn check_ident(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::Ident(_))
    }

    fn advance(&mut self) -> Token {
        let t = self.tokens[self.pos].clone();
        if !matches!(t.kind, TokenKind::Eof) {
            self.pos += 1;
        }
        t
    }

    fn expect(&mut self, kind: TokenKind, what: &str) -> Result<Token, ParseError> {
        if self.check(&kind) {
            Ok(self.advance())
        } else {
            Err(ParseError {
                message: format!("expected {what}, found {}", self.peek_kind()),
                span: self.peek().span,
            })
        }
    }

    fn expect_ident(&mut self) -> Result<Ident, ParseError> {
        let tok = self.peek().clone();
        match tok.kind {
            TokenKind::Ident(name) => {
                self.advance();
                Ok(Ident {
                    name,
                    span: tok.span,
                })
            }
            _ => Err(ParseError {
                message: format!("expected identifier, found {}", tok.kind),
                span: tok.span,
            }),
        }
    }

    fn ty(&mut self) -> Result<TypeExpr, ParseError> {
        let tok = self.advance();
        let base = match tok.kind {
            TokenKind::TyInt => TypeExpr::Int,
            TokenKind::TyFloat => TypeExpr::Float,
            TokenKind::TyBool => TypeExpr::Bool,
            TokenKind::TyString => TypeExpr::String,
            TokenKind::TyVoid => TypeExpr::Void,
            _ => {
                return Err(ParseError {
                    message: format!("expected a type, found {}", tok.kind),
                    span: tok.span,
                })
            }
        };
        if self.check(&TokenKind::LBracket) {
            self.advance();
            self.expect(TokenKind::RBracket, "`]` after `[` in array type")?;
            if base.is_void() {
                return Err(ParseError {
                    message: "`void[]` is not a type".into(),
                    span: tok.span,
                });
            }
            Ok(TypeExpr::Array(Box::new(base)))
        } else {
            Ok(base)
        }
    }

    fn fun_decl(&mut self) -> Result<FunDecl, ParseError> {
        let start = self.expect(TokenKind::Fun, "`fun`")?.span;
        let name = self.expect_ident()?;
        self.expect(TokenKind::LParen, "`(` after function name")?;
        let mut params = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                let pname = self.expect_ident()?;
                self.expect(TokenKind::Colon, "`:` after parameter name")?;
                let pty = self.ty()?;
                if pty.is_void() {
                    return Err(ParseError {
                        message: "parameter type cannot be `void`".into(),
                        span: pname.span,
                    });
                }
                params.push(Param {
                    name: pname,
                    ty: pty,
                });
                if self.check(&TokenKind::Comma) {
                    self.advance();
                    continue;
                }
                break;
            }
        }
        self.expect(TokenKind::RParen, "`)` after parameters")?;
        self.expect(TokenKind::Arrow, "`->` after parameters")?;
        let ret = self.ty()?;
        let body = self.block()?;
        Ok(FunDecl {
            name,
            params,
            ret,
            body,
            span: start,
        })
    }

    fn block(&mut self) -> Result<Block, ParseError> {
        let start = self.expect(TokenKind::LBrace, "`{`")?.span;
        let mut stmts = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            stmts.push(self.stmt()?);
        }
        self.expect(TokenKind::RBrace, "`}`")?;
        Ok(Block { stmts, span: start })
    }

    fn stmt(&mut self) -> Result<Stmt, ParseError> {
        if self.check(&TokenKind::Let) {
            return Ok(Stmt::Let(self.let_stmt()?));
        }
        if self.check(&TokenKind::If) {
            return Ok(Stmt::If(self.if_stmt()?));
        }
        if self.check(&TokenKind::While) {
            return Ok(Stmt::While(self.while_stmt()?));
        }
        if self.check(&TokenKind::Return) {
            return Ok(Stmt::Return(self.return_stmt()?));
        }
        if self.check(&TokenKind::LBrace) {
            return Ok(Stmt::Block(self.block()?));
        }

        // 区分赋值语句与表达式语句（回溯保存位置）
        if self.check_ident() {
            let save = self.pos;
            let name = self.expect_ident()?;
            if self.check(&TokenKind::Assign) {
                self.advance();
                let value = self.expr()?;
                let semi = self.expect(TokenKind::Semi, "`;` after assignment")?;
                return Ok(Stmt::Assign(AssignStmt {
                    name,
                    index: None,
                    value,
                    span: semi.span,
                }));
            }
            if self.check(&TokenKind::LBracket) {
                self.advance();
                let index = self.expr()?;
                self.expect(TokenKind::RBracket, "`]` after index")?;
                if self.check(&TokenKind::Assign) {
                    self.advance();
                    let value = self.expr()?;
                    let semi = self.expect(TokenKind::Semi, "`;` after index assignment")?;
                    return Ok(Stmt::Assign(AssignStmt {
                        name,
                        index: Some(index),
                        value,
                        span: semi.span,
                    }));
                }
            }
            self.pos = save;
        }

        let expr = self.expr()?;
        let span = expr.span();
        self.expect(TokenKind::Semi, "`;` after expression")?;
        Ok(Stmt::Expr(ExprStmt { expr, span }))
    }

    fn let_stmt(&mut self) -> Result<LetStmt, ParseError> {
        let start = self.expect(TokenKind::Let, "`let`")?.span;
        let name = self.expect_ident()?;
        let ty = if self.check(&TokenKind::Colon) {
            self.advance();
            Some(self.ty()?)
        } else {
            None
        };
        self.expect(TokenKind::Assign, "`=` in let binding")?;
        let value = self.expr()?;
        self.expect(TokenKind::Semi, "`;` after let")?;
        Ok(LetStmt {
            name,
            ty,
            value,
            span: start,
        })
    }

    fn if_stmt(&mut self) -> Result<IfStmt, ParseError> {
        let start = self.expect(TokenKind::If, "`if`")?.span;
        let cond = self.expr()?;
        let then_block = self.block()?;
        let else_branch = if self.check(&TokenKind::Else) {
            self.advance();
            if self.check(&TokenKind::If) {
                Some(ElseBranch::If(Box::new(self.if_stmt()?)))
            } else {
                Some(ElseBranch::Block(self.block()?))
            }
        } else {
            None
        };
        Ok(IfStmt {
            cond,
            then_block,
            else_branch,
            span: start,
        })
    }

    fn while_stmt(&mut self) -> Result<WhileStmt, ParseError> {
        let start = self.expect(TokenKind::While, "`while`")?.span;
        let cond = self.expr()?;
        let body = self.block()?;
        Ok(WhileStmt {
            cond,
            body,
            span: start,
        })
    }

    fn return_stmt(&mut self) -> Result<ReturnStmt, ParseError> {
        let start = self.expect(TokenKind::Return, "`return`")?.span;
        let value = if self.check(&TokenKind::Semi) {
            None
        } else {
            Some(self.expr()?)
        };
        self.expect(TokenKind::Semi, "`;` after return")?;
        Ok(ReturnStmt { value, span: start })
    }

    // 表达式：or → and → equality → comparison → term → factor → unary → postfix
    pub fn expr(&mut self) -> Result<Expr, ParseError> {
        self.or()
    }

    fn or(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.and()?;
        while self.check(&TokenKind::OrOr) {
            self.advance();
            let rhs = self.and()?;
            let span = lhs.span();
            lhs = Expr::Binary {
                op: BinOp::Or,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                span,
            };
        }
        Ok(lhs)
    }

    fn and(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.equality()?;
        while self.check(&TokenKind::AndAnd) {
            self.advance();
            let rhs = self.equality()?;
            let span = lhs.span();
            lhs = Expr::Binary {
                op: BinOp::And,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                span,
            };
        }
        Ok(lhs)
    }

    fn equality(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.comparison()?;
        loop {
            let op = if self.check(&TokenKind::EqEq) {
                BinOp::Eq
            } else if self.check(&TokenKind::BangEq) {
                BinOp::Ne
            } else {
                break;
            };
            self.advance();
            let rhs = self.comparison()?;
            let span = lhs.span();
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                span,
            };
        }
        Ok(lhs)
    }

    fn comparison(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.term()?;
        loop {
            let op = if self.check(&TokenKind::Lt) {
                BinOp::Lt
            } else if self.check(&TokenKind::LtEq) {
                BinOp::Le
            } else if self.check(&TokenKind::Gt) {
                BinOp::Gt
            } else if self.check(&TokenKind::GtEq) {
                BinOp::Ge
            } else {
                break;
            };
            self.advance();
            let rhs = self.term()?;
            let span = lhs.span();
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                span,
            };
        }
        Ok(lhs)
    }

    fn term(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.factor()?;
        loop {
            let op = if self.check(&TokenKind::Plus) {
                BinOp::Add
            } else if self.check(&TokenKind::Minus) {
                BinOp::Sub
            } else {
                break;
            };
            self.advance();
            let rhs = self.factor()?;
            let span = lhs.span();
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                span,
            };
        }
        Ok(lhs)
    }

    fn factor(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.unary()?;
        loop {
            let op = if self.check(&TokenKind::Star) {
                BinOp::Mul
            } else if self.check(&TokenKind::Slash) {
                BinOp::Div
            } else if self.check(&TokenKind::Percent) {
                BinOp::Rem
            } else {
                break;
            };
            self.advance();
            let rhs = self.unary()?;
            let span = lhs.span();
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                span,
            };
        }
        Ok(lhs)
    }

    fn unary(&mut self) -> Result<Expr, ParseError> {
        if self.check(&TokenKind::Minus) {
            let tok = self.advance();
            let expr = self.unary()?;
            return Ok(Expr::Unary {
                op: UnOp::Neg,
                expr: Box::new(expr),
                span: tok.span,
            });
        }
        if self.check(&TokenKind::Bang) {
            let tok = self.advance();
            let expr = self.unary()?;
            return Ok(Expr::Unary {
                op: UnOp::Not,
                expr: Box::new(expr),
                span: tok.span,
            });
        }
        self.postfix()
    }

    fn postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.primary()?;
        while self.check(&TokenKind::LBracket) {
            self.advance();
            let index = self.expr()?;
            let end = self.expect(TokenKind::RBracket, "`]` after index")?;
            expr = Expr::Index {
                base: Box::new(expr),
                index: Box::new(index),
                span: end.span,
            };
        }
        Ok(expr)
    }

    fn primary(&mut self) -> Result<Expr, ParseError> {
        let tok = self.peek().clone();
        match tok.kind.clone() {
            TokenKind::Int(n) => {
                self.advance();
                Ok(Expr::Int {
                    value: n,
                    span: tok.span,
                })
            }
            TokenKind::Float(n) => {
                self.advance();
                Ok(Expr::Float {
                    value: n,
                    span: tok.span,
                })
            }
            TokenKind::Str(s) => {
                self.advance();
                Ok(Expr::Str {
                    value: s,
                    span: tok.span,
                })
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr::Bool {
                    value: true,
                    span: tok.span,
                })
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr::Bool {
                    value: false,
                    span: tok.span,
                })
            }
            TokenKind::LBracket => {
                self.advance();
                let mut elems = Vec::new();
                if !self.check(&TokenKind::RBracket) {
                    loop {
                        elems.push(self.expr()?);
                        if self.check(&TokenKind::Comma) {
                            self.advance();
                            continue;
                        }
                        break;
                    }
                }
                self.expect(TokenKind::RBracket, "`]` after array elements")?;
                Ok(Expr::Array {
                    elems,
                    span: tok.span,
                })
            }
            TokenKind::LParen => {
                self.advance();
                let e = self.expr()?;
                self.expect(TokenKind::RParen, "`)` after expression")?;
                Ok(e)
            }
            TokenKind::Ident(name) => {
                self.advance();
                let ident = Ident {
                    name: name.clone(),
                    span: tok.span,
                };
                if self.check(&TokenKind::LParen) {
                    self.advance();
                    let mut args = Vec::new();
                    if !self.check(&TokenKind::RParen) {
                        loop {
                            args.push(self.expr()?);
                            if self.check(&TokenKind::Comma) {
                                self.advance();
                                continue;
                            }
                            break;
                        }
                    }
                    self.expect(TokenKind::RParen, "`)` after arguments")?;
                    Ok(Expr::Call {
                        callee: ident,
                        args,
                        span: tok.span,
                    })
                } else {
                    Ok(Expr::Var { name: ident })
                }
            }
            _ => Err(ParseError {
                message: format!("expected expression, found {}", tok.kind),
                span: tok.span,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse(src: &str) -> Program {
        let toks = Lexer::new(src).tokenize().expect("lex");
        Parser::new(toks).parse_program().expect("parse")
    }

    #[test]
    fn parse_fun_and_main() {
        let p = parse(
            r#"
            fun add(a: int, b: int) -> int {
                return a + b;
            }
            fun main() -> void {
                print(add(1, 2));
            }
            "#,
        );
        assert!(p.has_main());
        assert_eq!(p.functions().count(), 2);
    }

    #[test]
    fn parse_precedence() {
        let p = parse("let x = 1 + 2 * 3;");
        let Item::Stmt(Stmt::Let(l)) = &p.items[0] else {
            panic!("expected let");
        };
        match &l.value {
            Expr::Binary {
                op: BinOp::Add,
                rhs,
                ..
            } => match rhs.as_ref() {
                Expr::Binary { op: BinOp::Mul, .. } => {}
                other => panic!("expected mul on rhs, got {other:?}"),
            },
            other => panic!("expected add, got {other:?}"),
        }
    }

    #[test]
    fn parse_array_index_assign() {
        let p = parse("let a = [1,2,3]; a[0] = 9;");
        assert!(matches!(p.items[1], Item::Stmt(Stmt::Assign(_))));
    }

    #[test]
    fn syntax_error_has_span() {
        let toks = Lexer::new("let x = ;").tokenize().unwrap();
        let err = Parser::new(toks).parse_program().unwrap_err();
        assert!(err.span.line >= 1);
    }
}
