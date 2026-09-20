//! 语句 / 函数体检查。
use super::*;

impl<'a> Checker<'a> {
    /// 检查一条语句。
    /// 步骤：按语句种类分支 → 求相关表达式类型 → 对照语言规则，不符则报错。
    pub(crate) fn check_stmt(&mut self, s: &Stmt) -> Result<(), CheckError> {
        match s {
            Stmt::Let(l) => {
                let empty = matches!(&l.value, Expr::Array { elems, .. } if elems.is_empty());
                let vt = if empty {
                    if let Some(t) = &l.ty {
                        self.ty(t, l.name.span)?
                    } else {
                        return Err(CheckError {
                            message: "empty `[]` requires a type annotation".into(),
                            span: l.name.span,
                        });
                    }
                } else {
                    self.expr_ty(&l.value)?
                };
                let declared = if let Some(t) = &l.ty {
                    let t = self.ty(t, l.name.span)?;
                    if empty {
                        if !matches!(t, Type::Array(_)) {
                            return Err(CheckError {
                                message: format!("`[]` needs array type, got `{t}`"),
                                span: l.span,
                            });
                        }
                    } else if t != vt {
                        return Err(CheckError {
                            message: format!("let `{}` declared `{t}` but got `{vt}`", l.name.name),
                            span: l.span,
                        });
                    }
                    t
                } else {
                    vt
                };
                self.declare_var(&l.name.name, declared, l.name.span)?;
            }
            Stmt::Assign(a) => {
                if self.consts.contains_key(&a.name.name) {
                    return Err(CheckError {
                        message: format!("cannot assign to const `{}`", a.name.name),
                        span: a.span,
                    });
                }
                let mut cur = self.lookup(&a.name.name).ok_or_else(|| CheckError {
                    message: format!("undefined variable `{}`", a.name.name),
                    span: a.name.span,
                })?;
                let vt = self.expr_ty(&a.value)?;
                if let Some(idx) = &a.index {
                    if self.expr_ty(idx)? != Type::Int {
                        return Err(CheckError {
                            message: "index must be `int`".into(),
                            span: idx.span(),
                        });
                    }
                    match cur {
                        Type::Array(e) => cur = *e,
                        other => {
                            return Err(CheckError {
                                message: format!("cannot index `{other}`"),
                                span: a.name.span,
                            })
                        }
                    }
                }
                for f in &a.fields {
                    match &cur {
                        Type::Struct(n) => cur = self.field_ty(n, f)?,
                        other => {
                            return Err(CheckError {
                                message: format!("cannot access field on `{other}`"),
                                span: f.span,
                            })
                        }
                    }
                }
                if vt != cur {
                    return Err(CheckError {
                        message: format!("cannot assign `{vt}` to `{cur}`"),
                        span: a.span,
                    });
                }
            }
            Stmt::If(i) => self.check_if(i)?,
            Stmt::While(w) => {
                if self.expr_ty(&w.cond)? != Type::Bool {
                    return Err(CheckError {
                        message: "while condition must be `bool`".into(),
                        span: w.cond.span(),
                    });
                }
                self.loop_depth += 1;
                self.check_block(&w.body)?;
                self.loop_depth -= 1;
            }
            Stmt::For(f) => {
                if matches!(f.iter, Expr::Range { .. }) {
                    if let Expr::Range { start, end, span } = &f.iter {
                        if self.expr_ty(start)? != Type::Int || self.expr_ty(end)? != Type::Int {
                            return Err(CheckError {
                                message: "range bounds must be `int`".into(),
                                span: *span,
                            });
                        }
                    }
                    self.scopes.push(HashMap::new());
                    self.declare_var(&f.var.name, Type::Int, f.var.span)?;
                    self.loop_depth += 1;
                    let r = (|| {
                        for s in &f.body.stmts {
                            self.check_stmt(s)?;
                        }
                        Ok(())
                    })();
                    self.loop_depth -= 1;
                    self.scopes.pop();
                    r?;
                } else {
                    let it = self.expr_ty(&f.iter)?;
                    let elem = match it {
                        Type::Array(e) => *e,
                        other => {
                            return Err(CheckError {
                                message: format!("for-in expects array, found `{other}`"),
                                span: f.iter.span(),
                            })
                        }
                    };
                    self.scopes.push(HashMap::new());
                    self.declare_var(&f.var.name, elem, f.var.span)?;
                    self.loop_depth += 1;
                    let r = (|| {
                        for s in &f.body.stmts {
                            self.check_stmt(s)?;
                        }
                        Ok(())
                    })();
                    self.loop_depth -= 1;
                    self.scopes.pop();
                    r?;
                }
            }
            Stmt::Break(sp) | Stmt::Continue(sp) => {
                if self.loop_depth == 0 {
                    return Err(CheckError {
                        message: "`break`/`continue` outside loop".into(),
                        span: *sp,
                    });
                }
            }
            Stmt::Return(r) => {
                if self.top_level {
                    return Err(CheckError {
                        message: "`return` outside function".into(),
                        span: r.span,
                    });
                }
                match &r.value {
                    None if self.ret == Type::Void => {}
                    None => {
                        return Err(CheckError {
                            message: format!("must return `{}`", self.ret),
                            span: r.span,
                        })
                    }
                    Some(_) if self.ret == Type::Void => {
                        return Err(CheckError {
                            message: "void function cannot return a value".into(),
                            span: r.span,
                        })
                    }
                    Some(e) => {
                        let t = self.expr_ty(e)?;
                        if t != self.ret {
                            return Err(CheckError {
                                message: format!("expected `{}`, found `{t}`", self.ret),
                                span: r.span,
                            });
                        }
                    }
                }
            }
            Stmt::Expr(e) => {
                self.expr_ty(&e.expr)?;
            }
            Stmt::Block(b) => self.check_block(b)?,
        }
        Ok(())
    }

    pub(crate) fn check_if(&mut self, i: &IfStmt) -> Result<(), CheckError> {
        if self.expr_ty(&i.cond)? != Type::Bool {
            return Err(CheckError {
                message: "if condition must be `bool`".into(),
                span: i.cond.span(),
            });
        }
        self.check_block(&i.then_block)?;
        match &i.else_branch {
            None => {}
            Some(ElseBranch::Block(b)) => self.check_block(b)?,
            Some(ElseBranch::If(n)) => self.check_if(n)?,
        }
        Ok(())
    }
}

pub(crate) fn always_returns(b: &Block) -> bool {
    b.stmts.iter().any(stmt_returns)
}

pub(crate) fn stmt_returns(s: &Stmt) -> bool {
    match s {
        Stmt::Return(_) => true,
        Stmt::Block(b) => always_returns(b),
        Stmt::If(i) => {
            let t = always_returns(&i.then_block);
            t && match &i.else_branch {
                Some(ElseBranch::Block(b)) => always_returns(b),
                Some(ElseBranch::If(n)) => stmt_returns(&Stmt::If((**n).clone())),
                None => false,
            }
        }
        _ => false,
    }
}
