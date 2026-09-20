//! 表达式类型检查与函数调用。
use super::*;

impl<'a> Checker<'a> {
    pub(crate) fn expr_ty(&mut self, e: &Expr) -> Result<Type, CheckError> {
        match e {
            Expr::Int { .. } => Ok(Type::Int),
            Expr::Float { .. } => Ok(Type::Float),
            Expr::Bool { .. } => Ok(Type::Bool),
            Expr::Str { .. } => Ok(Type::Str),
            Expr::Var { name } => {
                if let Some((t, _)) = self.consts.get(&name.name) {
                    return Ok(t.clone());
                }
                self.lookup(&name.name).ok_or_else(|| CheckError {
                    message: format!("undefined variable `{}`", name.name),
                    span: name.span,
                })
            }
            Expr::Range { start, end, span } => {
                if self.expr_ty(start)? != Type::Int || self.expr_ty(end)? != Type::Int {
                    return Err(CheckError {
                        message: "range bounds must be `int`".into(),
                        span: *span,
                    });
                }
                Ok(Type::Array(Box::new(Type::Int)))
            }
            Expr::Unary { op, expr, span } => {
                let t = self.expr_ty(expr)?;
                match op {
                    UnOp::Neg if t.is_numeric() => Ok(t),
                    UnOp::Not if t == Type::Bool => Ok(Type::Bool),
                    _ => Err(CheckError {
                        message: format!("bad unary on `{t}`"),
                        span: *span,
                    }),
                }
            }
            Expr::Binary { op, lhs, rhs, span } => {
                let lt = self.expr_ty(lhs)?;
                let rt = self.expr_ty(rhs)?;
                bin_result(*op, &lt, &rt).map_err(|m| CheckError {
                    message: m,
                    span: *span,
                })
            }
            Expr::Call { callee, args, span } => self.call(&callee.name, args, *span),
            Expr::MethodCall {
                recv,
                method,
                args,
                span,
            } => {
                if let Expr::Var { name } = recv.as_ref() {
                    let dotted = format!("{}.{}", name.name, method.name);
                    if self.functions.contains_key(&dotted) {
                        return self.call(&dotted, args, *span);
                    }
                    if self.structs.contains_key(&name.name) {
                        let info = self
                            .methods
                            .get(&name.name)
                            .and_then(|m| m.get(&method.name))
                            .cloned()
                            .ok_or_else(|| CheckError {
                                message: format!("no method `{}` on `{}`", method.name, name.name),
                                span: method.span,
                            })?;
                        return self.call_info(&method.name, args, &info, *span);
                    }
                }
                let rt = self.expr_ty(recv)?;
                let Type::Struct(sn) = rt else {
                    return Err(CheckError {
                        message: format!("cannot call method on `{rt}`"),
                        span: *span,
                    });
                };
                let info = self
                    .methods
                    .get(&sn)
                    .and_then(|m| m.get(&method.name))
                    .cloned()
                    .ok_or_else(|| CheckError {
                        message: format!("no method `{}` on `{sn}`", method.name),
                        span: method.span,
                    })?;
                if args.len() + 1 != info.params.len() {
                    return Err(CheckError {
                        message: format!(
                            "`{}` expects {} arg(s) after receiver",
                            method.name,
                            info.params.len() - 1
                        ),
                        span: *span,
                    });
                }
                for (i, (a, p)) in args.iter().zip(info.params[1..].iter()).enumerate() {
                    let at = self.expr_ty(a)?;
                    if &at != p {
                        return Err(CheckError {
                            message: format!(
                                "argument {} of `{}`: expected `{p}`, found `{at}`",
                                i + 1,
                                method.name
                            ),
                            span: a.span(),
                        });
                    }
                }
                Ok(info.ret)
            }
            Expr::Index { base, index, span } => {
                let bt = self.expr_ty(base)?;
                if self.expr_ty(index)? != Type::Int {
                    return Err(CheckError {
                        message: "index must be `int`".into(),
                        span: *span,
                    });
                }
                match bt {
                    Type::Array(e) => Ok(*e),
                    other => Err(CheckError {
                        message: format!("cannot index `{other}`"),
                        span: *span,
                    }),
                }
            }
            Expr::Field { base, name, span } => {
                let bt = self.expr_ty(base)?;
                match &bt {
                    Type::Struct(n) => self.field_ty(n, name),
                    other => Err(CheckError {
                        message: format!("cannot access field on `{other}`"),
                        span: *span,
                    }),
                }
            }
            Expr::Array { elems, span } => {
                if elems.is_empty() {
                    return Err(CheckError {
                        message: "empty `[]` requires annotation".into(),
                        span: *span,
                    });
                }
                let first = self.expr_ty(&elems[0])?;
                for e in &elems[1..] {
                    let t = self.expr_ty(e)?;
                    if t != first {
                        return Err(CheckError {
                            message: format!("array elements must be `{first}`, found `{t}`"),
                            span: e.span(),
                        });
                    }
                }
                Ok(Type::Array(Box::new(first)))
            }
            Expr::StructLit { name, fields, span } => {
                let decl = self
                    .structs
                    .get(&name.name)
                    .cloned()
                    .ok_or_else(|| CheckError {
                        message: format!("unknown struct `{}`", name.name),
                        span: name.span,
                    })?;
                if fields.len() != decl.len() {
                    return Err(CheckError {
                        message: format!("`{}` expects {} field(s)", name.name, decl.len()),
                        span: *span,
                    });
                }
                for (fn_, fe) in fields {
                    let exp = decl
                        .iter()
                        .find(|(n, _)| n == &fn_.name)
                        .map(|(_, t)| t.clone())
                        .ok_or_else(|| CheckError {
                            message: format!("no field `{}` on `{}`", fn_.name, name.name),
                            span: fn_.span,
                        })?;
                    let got = self.expr_ty(fe)?;
                    if got != exp {
                        return Err(CheckError {
                            message: format!(
                                "field `{}`: expected `{exp}`, found `{got}`",
                                fn_.name
                            ),
                            span: fe.span(),
                        });
                    }
                }
                Ok(Type::Struct(name.name.clone()))
            }
        }
    }

    pub(crate) fn call(
        &mut self,
        name: &str,
        args: &[Expr],
        span: Span,
    ) -> Result<Type, CheckError> {
        match name {
            "print" => {
                if args.len() != 1 {
                    return Err(CheckError {
                        message: "`print` takes 1 argument".into(),
                        span,
                    });
                }
                let t = self.expr_ty(&args[0])?;
                if !t.printable() {
                    return Err(CheckError {
                        message: "cannot print void".into(),
                        span,
                    });
                }
                return Ok(Type::Void);
            }
            "len" => {
                if args.len() != 1 {
                    return Err(CheckError {
                        message: "`len` takes 1 argument".into(),
                        span,
                    });
                }
                let t = self.expr_ty(&args[0])?;
                if !matches!(t, Type::Array(_) | Type::Str) {
                    return Err(CheckError {
                        message: format!("`len` expects array or string, found `{t}`"),
                        span,
                    });
                }
                return Ok(Type::Int);
            }
            "str_at" => {
                self.arity(name, args, 2, span)?;
                self.arg_is(&args[0], &Type::Str, name, span)?;
                self.arg_is(&args[1], &Type::Int, name, span)?;
                return Ok(Type::Str);
            }
            "str_sub" => {
                self.arity(name, args, 3, span)?;
                self.arg_is(&args[0], &Type::Str, name, span)?;
                self.arg_is(&args[1], &Type::Int, name, span)?;
                self.arg_is(&args[2], &Type::Int, name, span)?;
                return Ok(Type::Str);
            }
            "to_string" => {
                self.arity(name, args, 1, span)?;
                self.expr_ty(&args[0])?;
                return Ok(Type::Str);
            }
            "push" => {
                self.arity(name, args, 2, span)?;
                let at = self.expr_ty(&args[0])?;
                let Type::Array(elem) = at else {
                    return Err(CheckError {
                        message: format!("`push` expects array, found `{at}`"),
                        span,
                    });
                };
                let vt = self.expr_ty(&args[1])?;
                if vt != *elem {
                    return Err(CheckError {
                        message: format!("`push` expects `{elem}`, found `{vt}`"),
                        span,
                    });
                }
                return Ok(Type::Void);
            }
            "pop" => {
                self.arity(name, args, 1, span)?;
                match self.expr_ty(&args[0])? {
                    Type::Array(e) => return Ok(*e),
                    other => {
                        return Err(CheckError {
                            message: format!("`pop` expects array, found `{other}`"),
                            span,
                        })
                    }
                }
            }
            "input" => {
                self.arity(name, args, 0, span)?;
                return Ok(Type::Str);
            }
            _ => {}
        }
        let info = self
            .functions
            .get(name)
            .cloned()
            .ok_or_else(|| CheckError {
                message: format!("undefined function `{name}`"),
                span,
            })?;
        self.call_info(name, args, &info, span)
    }

    pub(crate) fn call_info(
        &mut self,
        name: &str,
        args: &[Expr],
        info: &FunInfo,
        span: Span,
    ) -> Result<Type, CheckError> {
        if args.len() != info.params.len() {
            return Err(CheckError {
                message: format!(
                    "`{name}` takes {} argument(s), found {}",
                    info.params.len(),
                    args.len()
                ),
                span,
            });
        }
        for (i, (a, p)) in args.iter().zip(info.params.iter()).enumerate() {
            let at = self.expr_ty(a)?;
            if &at != p {
                return Err(CheckError {
                    message: format!(
                        "argument {} of `{name}`: expected `{p}`, found `{at}`",
                        i + 1
                    ),
                    span: a.span(),
                });
            }
        }
        Ok(info.ret.clone())
    }

    pub(crate) fn arity(
        &self,
        n: &str,
        args: &[Expr],
        want: usize,
        span: Span,
    ) -> Result<(), CheckError> {
        if args.len() != want {
            return Err(CheckError {
                message: format!("`{n}` takes {want} argument(s)"),
                span,
            });
        }
        Ok(())
    }

    pub(crate) fn arg_is(
        &mut self,
        a: &Expr,
        want: &Type,
        f: &str,
        span: Span,
    ) -> Result<(), CheckError> {
        let t = self.expr_ty(a)?;
        if &t != want {
            return Err(CheckError {
                message: format!("`{f}` expects `{want}`, found `{t}`"),
                span,
            });
        }
        Ok(())
    }
}

fn bin_result(op: BinOp, lt: &Type, rt: &Type) -> Result<Type, String> {
    match op {
        BinOp::Add => {
            if lt == &Type::Str && rt == &Type::Str {
                return Ok(Type::Str);
            }
            if lt == rt && lt.is_numeric() {
                return Ok(lt.clone());
            }
            Err(format!("cannot add `{lt}` and `{rt}`"))
        }
        BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem => {
            if lt == rt && lt.is_numeric() {
                Ok(lt.clone())
            } else {
                Err(format!("cannot apply arithmetic to `{lt}` and `{rt}`"))
            }
        }
        BinOp::Eq | BinOp::Ne => {
            if lt == rt && matches!(lt, Type::Int | Type::Float | Type::Bool | Type::Str) {
                Ok(Type::Bool)
            } else {
                Err(format!("cannot compare `{lt}` with `{rt}`"))
            }
        }
        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
            if lt == rt && lt.is_numeric() {
                Ok(Type::Bool)
            } else {
                Err(format!("cannot order `{lt}` and `{rt}`"))
            }
        }
        BinOp::And | BinOp::Or => {
            if lt == &Type::Bool && rt == &Type::Bool {
                Ok(Type::Bool)
            } else {
                Err("logical ops need `bool`".into())
            }
        }
    }
}
