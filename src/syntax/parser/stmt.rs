//! 语句解析。
use super::*;

impl Parser {
    /// 语句块 `{ ... }`：
    /// 1. 吃掉 `{`
    /// 2. 循环解析语句，直到 `}` 或文件结束
    /// 3. 吃掉 `}`，组装 Block
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
    /// 语句分派：关键字开头的专用语句优先，否则尝试赋值/表达式语句。
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
                    via_deref: false,
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
                        via_deref: false,
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
                        via_deref: false,
                    }));
                }
            }
            self.pos = save;
        }

        // `*p = v`：解引用赋值
        if self.check(&TokenKind::Star) {
            let save = self.pos;
            self.advance();
            let ptr = self.expr()?;
            if self.check(&TokenKind::Assign) {
                self.advance();
                let value = self.expr()?;
                let semi = self.expect(TokenKind::Semi, "`;`")?;
                // 用临时名字表示通过指针赋值；编译器看 via_deref + value
                return Ok(Stmt::Assign(AssignStmt {
                    name: Ident {
                        name: String::new(),
                        span: semi.span,
                    },
                    index: None,
                    fields: vec![],
                    value: Expr::Binary {
                        op: crate::syntax::ast::BinOp::Eq,
                        lhs: Box::new(ptr),
                        rhs: Box::new(value),
                        span: semi.span,
                    },
                    span: semi.span,
                    via_deref: true,
                }));
            }
            self.pos = save;
        }

        let expr = self.expr()?;
        let span = expr.span();
        // 无分号且后面是 `}`：函数体末尾隐式 return
        if self.check(&TokenKind::RBrace) || self.check(&TokenKind::Eof) {
            return Ok(Stmt::Expr(ExprStmt {
                expr,
                span,
                implicit_return: true,
            }));
        }
        self.expect(TokenKind::Semi, "`;` after expression")?;
        Ok(Stmt::Expr(ExprStmt {
            expr,
            span,
            implicit_return: false,
        }))
    }

    /// `let 名字 [: 类型] = 值;`
    /// 步骤：吃 let → 名字 → 可选 `: 类型` → `=` → 表达式 → `;`
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

    /// `if 条件 { 块 } [else { 块 } | else if ...]`
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

    /// `while 条件 { 块 }`
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

    /// `for 变量 in 迭代式 { 块 }`
    /// 迭代式可以是数组或 `a..b` 范围（在 expr 里已能解析 Range）。
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
