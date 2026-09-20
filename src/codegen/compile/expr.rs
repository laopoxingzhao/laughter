//! 表达式与调用编译。
use super::*;

impl Compiler {
    pub(crate) fn expr(&mut self, e: &Expr) -> Result<(), CompileError> {
        match e {
            Expr::Int { value, span } => {
                self.chunk().emit_const(Value::Int(*value), span.line)?;
                Ok(())
            }
            Expr::Float { value, span } => {
                self.chunk().emit_const(Value::Float(*value), span.line)?;
                Ok(())
            }
            Expr::Bool { value, span } => {
                let op = if *value { Op::True } else { Op::False };
                self.chunk().emit(op, span.line);
                Ok(())
            }
            Expr::Str { value, span } => {
                let v = Value::Str(Rc::from(value.as_str()));
                self.chunk().emit_const(v, span.line)?;
                Ok(())
            }
            Expr::Var { name } => {
                if let Some(v) = self.consts.get(&name.name).cloned() {
                    self.chunk().emit_const(v, name.span.line)?;
                    return Ok(());
                }
                let s = self.f().slot(&name.name).ok_or_else(|| {
                    CompileError::at(format!("undefined `{}`", name.name), name.span)
                })?;
                self.chunk().emit(Op::GetLocal, name.span.line);
                self.chunk().emit_u16(s, name.span.line);
                Ok(())
            }
            Expr::Unary { op, expr, span } => {
                self.expr(expr)?;
                let o = match op {
                    UnOp::Neg => Op::Neg,
                    UnOp::Not => Op::Not,
                };
                self.chunk().emit(o, span.line);
                Ok(())
            }
            Expr::Binary { op, lhs, rhs, span } => self.bin(*op, lhs, rhs, *span),
            Expr::Call { callee, args, span } => self.call(&callee.name, args, *span),
            Expr::MethodCall {
                recv,
                method,
                args,
                span,
            } => {
                if let Expr::Var { name } = recv.as_ref() {
                    let d = format!("{}.{}", name.name, method.name);
                    if self.fidx.contains_key(&d) {
                        return self.call(&d, args, *span);
                    }
                }
                self.expr(recv)?;
                for a in args {
                    self.expr(a)?;
                }
                let c = self
                    .chunk()
                    .add_const(Value::Str(Rc::from(method.name.as_str())))?;
                self.chunk().emit(Op::CallMethod, span.line);
                self.chunk().emit_u16(c, span.line);
                self.chunk().emit_u16(args.len() as u16, span.line);
                Ok(())
            }
            Expr::Range { span, .. } => Err(CompileError::at("range only allowed in `for`", *span)),
            Expr::Index { base, index, span } => {
                self.expr(base)?;
                self.expr(index)?;
                self.chunk().emit(Op::GetIndex, span.line);
                Ok(())
            }
            Expr::Field { base, name, span } => {
                self.expr(base)?;
                let c = self
                    .chunk()
                    .add_const(Value::Str(Rc::from(name.name.as_str())))?;
                self.chunk().emit(Op::GetField, span.line);
                self.chunk().emit_u16(c, span.line);
                Ok(())
            }
            Expr::Array { elems, span } => {
                for x in elems {
                    self.expr(x)?;
                }
                self.chunk().emit(Op::NewArray, span.line);
                self.chunk().emit_u16(elems.len() as u16, span.line);
                Ok(())
            }
            Expr::StructLit { name, fields, span } => {
                let idx = *self.sidx.get(&name.name).ok_or_else(|| {
                    CompileError::at(format!("unknown struct `{}`", name.name), name.span)
                })?;
                let decl = self.stypes[idx].clone();
                for fname in &decl.fields {
                    let (_, e) =
                        fields
                            .iter()
                            .find(|(n, _)| &n.name == fname)
                            .ok_or_else(|| {
                                CompileError::at(format!("missing field `{fname}`"), name.span)
                            })?;
                    self.expr(e)?;
                }
                self.chunk().emit(Op::NewStruct, span.line);
                self.chunk().emit_u16(idx as u16, span.line);
                self.chunk().emit_u16(decl.fields.len() as u16, span.line);
                Ok(())
            }
        }
    }

    pub(crate) fn bin(
        &mut self,
        op: BinOp,
        lhs: &Expr,
        rhs: &Expr,
        span: Span,
    ) -> Result<(), CompileError> {
        let line = span.line;
        match op {
            BinOp::And => {
                self.expr(lhs)?;
                let j = self.chunk().emit_jump(Op::JumpIfFalse, line);
                self.chunk().emit(Op::Pop, line);
                self.expr(rhs)?;
                self.chunk().patch_to_end(j)?;
                Ok(())
            }
            BinOp::Or => {
                self.expr(lhs)?;
                let j = self.chunk().emit_jump(Op::JumpIfTrue, line);
                self.chunk().emit(Op::Pop, line);
                self.expr(rhs)?;
                self.chunk().patch_to_end(j)?;
                Ok(())
            }
            _ => {
                self.expr(lhs)?;
                self.expr(rhs)?;
                let o = match op {
                    BinOp::Add => Op::Add,
                    BinOp::Sub => Op::Sub,
                    BinOp::Mul => Op::Mul,
                    BinOp::Div => Op::Div,
                    BinOp::Rem => Op::Rem,
                    BinOp::Eq => Op::Eq,
                    BinOp::Ne => Op::Ne,
                    BinOp::Lt => Op::Lt,
                    BinOp::Le => Op::Le,
                    BinOp::Gt => Op::Gt,
                    BinOp::Ge => Op::Ge,
                    BinOp::And | BinOp::Or => unreachable!(),
                };
                self.chunk().emit(o, line);
                Ok(())
            }
        }
    }

    pub(crate) fn call(
        &mut self,
        name: &str,
        args: &[Expr],
        span: Span,
    ) -> Result<(), CompileError> {
        let line = span.line;
        match name {
            "print" => {
                self.expr(&args[0])?;
                self.chunk().emit(Op::Print, line);
                return Ok(());
            }
            "len" => {
                self.expr(&args[0])?;
                self.chunk().emit(Op::Len, line);
                return Ok(());
            }
            "str_at" | "str_sub" | "to_string" | "push" | "pop" | "input" => {
                for a in args {
                    self.expr(a)?;
                }
                let o = match name {
                    "str_at" => Op::StrAt,
                    "str_sub" => Op::StrSub,
                    "to_string" => Op::ToString,
                    "push" => Op::Push,
                    "pop" => Op::ArrayPop,
                    _ => Op::Input,
                };
                self.chunk().emit(o, line);
                return Ok(());
            }
            _ => {}
        }
        let idx = *self
            .fidx
            .get(name)
            .ok_or_else(|| CompileError::at(format!("undefined function `{name}`"), span))?;
        for a in args {
            self.expr(a)?;
        }
        self.chunk().emit(Op::Call, line);
        self.chunk().emit_u16(idx as u16, line);
        Ok(())
    }
}
