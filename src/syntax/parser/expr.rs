//! 表达式解析（优先级见模块注释）。
use super::*;

impl Parser {
    pub fn expr(&mut self) -> Result<Expr, ParseError> {
        self.range_expr()
    }

    pub(crate) fn range_expr(&mut self) -> Result<Expr, ParseError> {
        let lhs = self.or()?;
        if self.check(&TokenKind::DotDot) {
            self.advance();
            let rhs = self.or()?;
            let span = lhs.span();
            return Ok(Expr::Range {
                start: Box::new(lhs),
                end: Box::new(rhs),
                span,
            });
        }
        Ok(lhs)
    }

    pub(crate) fn or(&mut self) -> Result<Expr, ParseError> {
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

    pub(crate) fn and(&mut self) -> Result<Expr, ParseError> {
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

    pub(crate) fn equality(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.cmp()?;
        loop {
            let op = if self.check(&TokenKind::EqEq) {
                BinOp::Eq
            } else if self.check(&TokenKind::BangEq) {
                BinOp::Ne
            } else {
                break;
            };
            self.advance();
            let rhs = self.cmp()?;
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

    pub(crate) fn cmp(&mut self) -> Result<Expr, ParseError> {
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

    pub(crate) fn term(&mut self) -> Result<Expr, ParseError> {
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

    pub(crate) fn factor(&mut self) -> Result<Expr, ParseError> {
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

    pub(crate) fn unary(&mut self) -> Result<Expr, ParseError> {
        if self.check(&TokenKind::Minus) {
            let t = self.advance();
            let e = self.unary()?;
            return Ok(Expr::Unary {
                op: UnOp::Neg,
                expr: Box::new(e),
                span: t.span,
            });
        }
        if self.check(&TokenKind::Bang) {
            let t = self.advance();
            let e = self.unary()?;
            return Ok(Expr::Unary {
                op: UnOp::Not,
                expr: Box::new(e),
                span: t.span,
            });
        }
        self.postfix()
    }

    pub(crate) fn postfix(&mut self) -> Result<Expr, ParseError> {
        let mut e = self.primary()?;
        loop {
            if self.check(&TokenKind::LBracket) {
                self.advance();
                let idx = self.expr()?;
                let end = self.expect(TokenKind::RBracket, "`]`")?;
                e = Expr::Index {
                    base: Box::new(e),
                    index: Box::new(idx),
                    span: end.span,
                };
                continue;
            }
            if self.check(&TokenKind::Dot) {
                self.advance();
                let name = self.expect_ident()?;
                if self.check(&TokenKind::LParen) {
                    self.advance();
                    let args = self.arg_list()?;
                    let end = self.expect(TokenKind::RParen, "`)`")?;
                    e = Expr::MethodCall {
                        recv: Box::new(e),
                        method: name,
                        args,
                        span: end.span,
                    };
                } else {
                    let span = name.span;
                    e = Expr::Field {
                        base: Box::new(e),
                        name,
                        span,
                    };
                }
                continue;
            }
            break;
        }
        Ok(e)
    }

    pub(crate) fn arg_list(&mut self) -> Result<Vec<Expr>, ParseError> {
        let mut args = vec![];
        if !self.check(&TokenKind::RParen) {
            loop {
                args.push(self.expr()?);
                if self.check(&TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        Ok(args)
    }

    pub(crate) fn primary(&mut self) -> Result<Expr, ParseError> {
        let t = self.peek().clone();
        match t.kind.clone() {
            TokenKind::Int(v) => {
                self.advance();
                Ok(Expr::Int {
                    value: v,
                    span: t.span,
                })
            }
            TokenKind::Float(v) => {
                self.advance();
                Ok(Expr::Float {
                    value: v,
                    span: t.span,
                })
            }
            TokenKind::Str(v) => {
                self.advance();
                Ok(Expr::Str {
                    value: v,
                    span: t.span,
                })
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr::Bool {
                    value: true,
                    span: t.span,
                })
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr::Bool {
                    value: false,
                    span: t.span,
                })
            }
            TokenKind::LParen => {
                self.advance();
                let e = self.expr()?;
                self.expect(TokenKind::RParen, "`)`")?;
                Ok(e)
            }
            TokenKind::LBracket => {
                self.advance();
                let mut elems = vec![];
                if !self.check(&TokenKind::RBracket) {
                    loop {
                        elems.push(self.expr()?);
                        if self.check(&TokenKind::Comma) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                self.expect(TokenKind::RBracket, "`]`")?;
                Ok(Expr::Array {
                    elems,
                    span: t.span,
                })
            }
            TokenKind::Ident(name) => {
                self.advance();
                let ident = Ident {
                    name: name.clone(),
                    span: t.span,
                };
                if self.check(&TokenKind::LParen) {
                    self.advance();
                    let args = self.arg_list()?;
                    self.expect(TokenKind::RParen, "`)`")?;
                    return Ok(Expr::Call {
                        callee: ident,
                        args,
                        span: t.span,
                    });
                }
                if self.check(&TokenKind::LBrace) && self.struct_lit_ahead() {
                    self.advance();
                    let mut fields = vec![];
                    if !self.check(&TokenKind::RBrace) {
                        loop {
                            let n = self.expect_ident()?;
                            self.expect(TokenKind::Colon, "`:`")?;
                            let v = self.expr()?;
                            fields.push((n, v));
                            if self.check(&TokenKind::Comma) {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(TokenKind::RBrace, "`}`")?;
                    return Ok(Expr::StructLit {
                        name: ident,
                        fields,
                        span: t.span,
                    });
                }
                Ok(Expr::Var { name: ident })
            }
            other => Err(ParseError {
                message: format!("expected expression, found {other}"),
                span: t.span,
            }),
        }
    }
}
