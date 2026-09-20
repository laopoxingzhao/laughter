//! 语句解析。
use super::*;

impl Parser {
    pub(crate) fn block(&mut self) -> Result<Block, ParseError> {
        let start = self.expect(TokenKind::LBrace, "`{`")?.span;
        let mut stmts = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            stmts.push(self.stmt()?);
        }
        self.expect(TokenKind::RBrace, "`}`")?;
        Ok(Block { stmts, span: start })
    }

    /// 语句解析入口。
    ///
    /// 顺序很重要：先认 `let/if/while/...` 这些「以关键字开头」的语句；
    /// 再看是不是赋值（`x = ...`、`a[i] = ...`、`p.f = ...`）；
    /// 剩下的当作表达式语句（例如 `print(1);`），结尾必须有 `;`。
    pub(crate) fn stmt(&mut self) -> Result<Stmt, ParseError> {
        if self.check(&TokenKind::Let) {
            return Ok(Stmt::Let(self.let_stmt()?));
        }
        if self.check(&TokenKind::If) {
            return Ok(Stmt::If(self.if_stmt()?));
        }
        if self.check(&TokenKind::While) {
            return Ok(Stmt::While(self.while_stmt()?));
        }
        if self.check(&TokenKind::For) {
            return Ok(Stmt::For(self.for_stmt()?));
        }
        if self.check(&TokenKind::Break) {
            let t = self.advance();
            self.expect(TokenKind::Semi, "`;`")?;
            return Ok(Stmt::Break(t.span));
        }
        if self.check(&TokenKind::Continue) {
            let t = self.advance();
            self.expect(TokenKind::Semi, "`;`")?;
            return Ok(Stmt::Continue(t.span));
        }
        if self.check(&TokenKind::Return) {
            let t = self.advance();
            let value = if self.check(&TokenKind::Semi) {
                None
            } else {
                Some(self.expr()?)
            };
            self.expect(TokenKind::Semi, "`;`")?;
            return Ok(Stmt::Return(ReturnStmt {
                value,
                span: t.span,
            }));
        }
        if self.check(&TokenKind::LBrace) {
            return Ok(Stmt::Block(self.block()?));
        }

        // assign forms
        if matches!(self.kind(), TokenKind::Ident(_)) {
            let save = self.pos;
            let name = self.expect_ident()?;
            if self.check(&TokenKind::Assign) {
                self.advance();
                let value = self.expr()?;
                let semi = self.expect(TokenKind::Semi, "`;`")?;
                return Ok(Stmt::Assign(AssignStmt {
                    name,
                    index: None,
                    fields: vec![],
                    value,
                    span: semi.span,
                }));
            }
            if self.check(&TokenKind::LBracket) {
                self.advance();
                let index = self.expr()?;
                self.expect(TokenKind::RBracket, "`]`")?;
                let mut fields = vec![];
                while self.check(&TokenKind::Dot) {
                    self.advance();
                    fields.push(self.expect_ident()?);
                }
                if self.check(&TokenKind::Assign) {
                    self.advance();
                    let value = self.expr()?;
                    let semi = self.expect(TokenKind::Semi, "`;`")?;
                    return Ok(Stmt::Assign(AssignStmt {
                        name,
                        index: Some(index),
                        fields,
                        value,
                        span: semi.span,
                    }));
                }
            }
            if self.check(&TokenKind::Dot) {
                let mut fields = vec![];
                while self.check(&TokenKind::Dot) {
                    self.advance();
                    fields.push(self.expect_ident()?);
                }
                if self.check(&TokenKind::Assign) {
                    self.advance();
                    let value = self.expr()?;
                    let semi = self.expect(TokenKind::Semi, "`;`")?;
                    return Ok(Stmt::Assign(AssignStmt {
                        name,
                        index: None,
                        fields,
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

    pub(crate) fn let_stmt(&mut self) -> Result<LetStmt, ParseError> {
        let start = self.expect(TokenKind::Let, "`let`")?.span;
        let name = self.expect_ident()?;
        let ty = if self.check(&TokenKind::Colon) {
            self.advance();
            Some(self.ty()?)
        } else {
            None
        };
        self.expect(TokenKind::Assign, "`=`")?;
        let value = self.expr()?;
        self.expect(TokenKind::Semi, "`;`")?;
        Ok(LetStmt {
            name,
            ty,
            value,
            span: start,
        })
    }

    pub(crate) fn if_stmt(&mut self) -> Result<IfStmt, ParseError> {
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

    pub(crate) fn while_stmt(&mut self) -> Result<WhileStmt, ParseError> {
        let start = self.expect(TokenKind::While, "`while`")?.span;
        let cond = self.expr()?;
        let body = self.block()?;
        Ok(WhileStmt {
            cond,
            body,
            span: start,
        })
    }

    pub(crate) fn for_stmt(&mut self) -> Result<ForStmt, ParseError> {
        let start = self.expect(TokenKind::For, "`for`")?.span;
        let var = self.expect_ident()?;
        self.expect(TokenKind::In, "`in`")?;
        let iter = self.expr()?;
        let body = self.block()?;
        Ok(ForStmt {
            var,
            iter,
            body,
            span: start,
        })
    }
}
