//! 语句 / 函数体检查。
use super::*;

impl<'a> Checker<'a> {
    /// 检查一条语句。
    /// 步骤：按语句种类分支 → 求相关表达式类型 → 对照语言规则，不符则报错。
    pub(crate) fn check_stmt(&mut self, s: &Stmt) -> Result<(), CheckError> {
        // 按语句种类检查；每条路径违反语言规则就返回 Err
        match s {
            // let：先求初始值类型；[] 需要标注；再写入当前作用域
            Stmt::Let(l) => {
                let empty = matches!(&l.value, Expr::Array { elems, .. } if elems.is_empty());
                let vt = if empty {
                    if let Some(t) = &l.ty {
                        self.ty(t, l.name.span)?
                    } else {
                        return Err(CheckError {
                            message: "空数组 `[]` 需要类型标注，如 `int[]`".into(),
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
                                message: format!("`[]` 需要数组类型，实际是 `{t}`"),
                                span: l.span,
                            });
                        }
                    } else if matches!(&l.value, Expr::Nil { .. }) && matches!(t, Type::Ref { .. })
                    {
                        // nil 可赋给任意引用
                    } else if t != vt {
                        return Err(CheckError {
                            message: format!("let `{}` 标注 `{t}`，但初始值是 `{vt}`", l.name.name),
                            span: l.span,
                        });
                    }
                    t
                } else {
                    vt
                };
                self.declare_var(&l.name.name, declared, l.name.span)?;
            }
            // 赋值：不能改 const；求出目标类型后与右值比较
            Stmt::Assign(a) => {
                // `*p = v`：检查 p 为引用且 v 类型匹配
                if a.via_deref {
                    if let Expr::Binary { lhs, rhs, .. } = &a.value {
                        let pt = self.expr_ty(lhs)?;
                        let vt = self.expr_ty(rhs)?;
                        match pt {
                            Type::Ref { mutable, inner } => {
                                if !mutable {
                                    return Err(CheckError {
                                        message: format!(
                                            "不能通过只读引用 `{inner}` 赋值（需要 &mut）"
                                        ),
                                        span: a.span,
                                    });
                                }
                                if vt != *inner {
                                    return Err(CheckError {
                                        message: format!(
                                            "解引用赋值类型不匹配：`{inner}` 与 `{vt}`"
                                        ),
                                        span: a.span,
                                    });
                                }
                                return Ok(());
                            }
                            other => {
                                return Err(CheckError {
                                    message: format!("`*=` 需要引用，实际是 `{other}`"),
                                    span: a.span,
                                })
                            }
                        }
                    }
                }
                if self.consts.contains_key(&a.name.name) {
                    return Err(CheckError {
                        message: format!("不能给 const `{}` 赋值", a.name.name),
                        span: a.span,
                    });
                }
                let mut cur = self.lookup(&a.name.name).ok_or_else(|| CheckError {
                    message: format!("未定义的变量 `{}`", a.name.name),
                    span: a.name.span,
                })?;
                let vt = self.expr_ty(&a.value)?;
                if let Some(idx) = &a.index {
                    if self.expr_ty(idx)? != Type::Int {
                        return Err(CheckError {
                            message: "下标类型必须是 `int`".into(),
                            span: idx.span(),
                        });
                    }
                    match cur {
                        Type::Array(e) => cur = *e,
                        other => {
                            return Err(CheckError {
                                message: format!("无法对 `{other}` 做下标访问"),
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
                                message: format!("无法在 `{other}` 上访问字段"),
                                span: f.span,
                            })
                        }
                    }
                }
                if vt != cur {
                    return Err(CheckError {
                        message: format!("不能把 `{vt}` 赋给 `{cur}`"),
                        span: a.span,
                    });
                }
            }
            Stmt::If(i) => self.check_if(i)?,
            Stmt::While(w) => {
                if self.expr_ty(&w.cond)? != Type::Bool {
                    return Err(CheckError {
                        message: "while 条件必须是 `bool`".into(),
                        span: w.cond.span(),
                    });
                }
                self.loop_depth += 1;
                self.check_block(&w.body)?;
                self.loop_depth -= 1;
            }
            // for：区分「数组」与「范围 a..b」；循环变量进新作用域
            Stmt::For(f) => {
                if matches!(f.iter, Expr::Range { .. }) {
                    if let Expr::Range { start, end, span } = &f.iter {
                        if self.expr_ty(start)? != Type::Int || self.expr_ty(end)? != Type::Int {
                            return Err(CheckError {
                                message: "范围两端类型必须是 `int`".into(),
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
                                message: format!("for-in 需要数组，实际是 `{other}`"),
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
                        message: "`break`/`continue` 只能用在循环内".into(),
                        span: *sp,
                    });
                }
            }
            // return：顶层禁止；类型必须与当前函数返回类型一致
            Stmt::Return(r) => {
                if self.top_level {
                    return Err(CheckError {
                        message: "`return` 只能用在函数内".into(),
                        span: r.span,
                    });
                }
                match &r.value {
                    None if self.ret == Type::Void => {}
                    None => {
                        return Err(CheckError {
                            message: format!("此处必须返回 `{}`", self.ret),
                            span: r.span,
                        })
                    }
                    Some(_) if self.ret == Type::Void => {
                        return Err(CheckError {
                            message: "void 函数不能返回值".into(),
                            span: r.span,
                        })
                    }
                    Some(e) => {
                        let t = self.expr_ty(e)?;
                        if t != self.ret {
                            return Err(CheckError {
                                message: format!("期望返回类型 `{}`，实际是 `{t}`", self.ret),
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
                message: "if 条件必须是 `bool`".into(),
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
        // 现代化：函数体末尾无分号表达式视为隐式 return
        Stmt::Expr(e) => e.implicit_return,
        _ => false,
    }
}
