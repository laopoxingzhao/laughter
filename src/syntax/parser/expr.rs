//! 表达式解析（优先级见模块注释）。
//!
//! **每一步在做什么**：像算术课从「优先级最低」的运算开始拆括号。
//! 调用链：expr → range → or → and → equality → cmp → term → factor
//!         → unary → postfix → primary
//! 每一层：先解析更「紧」的左边，再看当前层运算符是否出现，出现则继续解析右边并组装 Binary。

use super::*;

impl Parser {
    /// 表达式入口：允许 `a..b`（范围；主要给 for 使用）。
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

    /// `||` 层：左边解析完后，连续吃 `|| 右边`。
    pub(crate) fn or(&mut self) -> Result<Expr, ParseError> {
        // 步骤1：先解析优先级更高的 &&
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

    /// `&&` 层。
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

    /// 一元 `-` / `!`：若有则吃掉运算符，再解析右边（可以连续多个一元）。
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

    /// 后缀：在「原子」表达式后面反复吃 `.字段` `.方法()` `[下标]`。
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

    /// 原子：数字/字符串/true/false/变量/(表达式)/[数组]/结构体字面量/函数调用。
    /// 这是优先级最高的一层，不再向更深层拆分。
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
                message: format!("期望表达式，实际是 {other}"),
                span: t.span,
            }),
        }
    }
}
