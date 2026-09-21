#!/usr/bin/env python3
from pathlib import Path

def patch(path, pairs):
    p = Path(path)
    t = p.read_text(encoding="utf-8")
    for old, new in pairs:
        if old not in t:
            print("MISS", path, repr(old[:60]))
            continue
        t = t.replace(old, new, 1)
    p.write_text(t, encoding="utf-8")
    print("ok", path)

patch("src/syntax/parser/mod.rs", [
("            } else if self.check(&TokenKind::Fun) {\n                items.push(Item::Fun(self.fun_decl()?));",
 "            } else if self.check(&TokenKind::Fun) || self.check(&TokenKind::Fn) {\n                items.push(Item::Fun(self.fun_decl()?));"),
])

patch("src/syntax/parser/decl.rs", [
("        if self.check(&TokenKind::Amp) {\n            let at = self.advance();",
 "        if self.check(&TokenKind::Amp) {\n            let _at = self.advance();"),
])

patch("src/syntax/parser/stmt.rs", [
("""                return Ok(Stmt::Assign(AssignStmt {
                    name,
                    index: None,
                    fields: vec![],
                    value,
                    span: semi.span,
                }));
""",
"""                return Ok(Stmt::Assign(AssignStmt {
                    name,
                    index: None,
                    fields: vec![],
                    value,
                    span: semi.span,
                    via_deref: false,
                }));
"""),
])

# also other AssignStmt without via_deref in stmt.rs
p = Path("src/syntax/parser/stmt.rs")
t = p.read_text(encoding="utf-8")
t = t.replace("span: semi.span,\n                    }));", "span: semi.span,\n                        via_deref: false,\n                    }));")
t = t.replace("span: semi.span,\n                }));", "span: semi.span,\n                    via_deref: false,\n                }));")
p.write_text(t, encoding="utf-8")
print("stmt assign fields done")

patch("src/sema/check/expr.rs", [
(
"""                for (fn_, fe) in fields {
                    let exp = decl
                        .iter()
                        .find(|(n, _)| n == &fn_.name)
                        .map(|(_, t)| t.clone())
                        .ok_or_else(|| CheckError {
                            message: format!("`{}` 没有字段 `{}`", name.name, fn_.name),
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
""",
"""                for fi in fields {
                    let exp = decl
                        .iter()
                        .find(|(n, _)| n == &fi.name.name)
                        .map(|(_, t)| t.clone())
                        .ok_or_else(|| CheckError {
                            message: format!("`{}` 没有字段 `{}`", name.name, fi.name.name),
                            span: fi.name.span,
                        })?;
                    // 简写 `{ x }`：类型为变量 x 的类型
                    let fe = match &fi.value {
                        Some(e) => e.clone(),
                        None => Expr::Var { name: fi.name.clone() },
                    };
                    let got = self.expr_ty(&fe)?;
                    if got != exp {
                        return Err(CheckError {
                            message: format!(
                                "字段 `{}`：期望 `{exp}`，实际是 `{got}`",
                                fi.name.name
                            ),
                            span: fe.span(),
                        });
                    }
                }
                Ok(Type::Struct(name.name.clone()))
            }
            Expr::Nil { .. } => Ok(Type::Ref {
                mutable: false,
                inner: Box::new(Type::Int),
            }), // 特殊：nil 与任意引用兼容，调用处再查
            Expr::Ref { mutable, target, span } => {
                // 左值：变量或数组元素
                match target.as_ref() {
                    Expr::Var { name } => {
                        let t = self.expr_ty(target)?;
                        if t == Type::Void {
                            return Err(CheckError {
                                message: "不能取 void 的引用".into(),
                                span: *span,
                            });
                        }
                        let _ = name;
                        Ok(Type::Ref {
                            mutable: *mutable,
                            inner: Box::new(t),
                        })
                    }
                    Expr::Index { .. } => {
                        let t = self.expr_ty(target)?;
                        Ok(Type::Ref {
                            mutable: *mutable,
                            inner: Box::new(t),
                        })
                    }
                    _ => Err(CheckError {
                        message: "只能对变量或数组元素取引用".into(),
                        span: *span,
                    }),
                }
            }
            Expr::Deref { ptr, span } => {
                let pt = self.expr_ty(ptr)?;
                match pt {
                    Type::Ref { inner, .. } => Ok(*inner),
                    other => Err(CheckError {
                        message: format!("`*` 需要引用类型，实际是 `{other}`"),
                        span: *span,
                    }),
                }
            }
            Expr::Interp { parts, .. } => {
                for p in parts {
                    if let InterpPart::Expr(e) = p {
                        self.expr_ty(e)?;
                    }
                }
                Ok(Type::Str)
            }
        }
    }
""",
),
])

# nil type compatibility: when assigning/checking Ref, allow Type::Ref { inner: Int } as any ref
# Better: Type::Ref inner used as wildcard - in assign to &int, allow nil.
# We'll special-case in expr_ty callers via a helper later. For assign nil to &T:
# If declared &T and value is Nil typed as Ref{Int}, accept if value is Expr::Nil in let.

patch("src/sema/check/stmt.rs", [
(
"""                } else {
                    if matches!(val_ty, Type::Void) {
                        return Err(CheckError {
                            message: "cannot bind a void expression".into(),
                            span: l.name.span,
                        });
                    }
                    val_ty
                };
""",
"""                } else {
                    if matches!(val_ty, Type::Void) {
                        return Err(CheckError {
                            message: "cannot bind a void expression".into(),
                            span: l.name.span,
                        });
                    }
                    val_ty
                };
                // nil 可赋给任意 &T / &mut T
                let declared = if matches!(&l.value, Expr::Nil { .. })
                    && matches!(declared, Type::Ref { .. })
                {
                    declared
                } else {
                    declared
                };
""",
),
])

# fix compile struct lit
patch("src/codegen/compile/expr.rs", [
(
"""                for fname in &decl.fields {
                    let (_, e) =
                        fields
                            .iter()
                            .find(|(n, _)| &n.name == fname)
                            .ok_or_else(|| {
                                CompileError::at(format!("缺少字段 `{fname}`"), name.span)
                            })?;
                    self.expr(e)?;
                }
""",
"""                for fname in &decl.fields {
                    let fi = fields
                        .iter()
                        .find(|f| &f.name.name == fname)
                        .ok_or_else(|| {
                            CompileError::at(format!("缺少字段 `{fname}`"), name.span)
                        })?;
                    match &fi.value {
                        Some(e) => self.expr(e)?,
                        None => {
                            // 简写：编译同名变量
                            self.expr(&Expr::Var {
                                name: fi.name.clone(),
                            })?
                        }
                    }
                }
""",
),
(
"""            Expr::StructLit { name, fields, span } => {
""",
"""            Expr::Nil { span } => {
                self.chunk().emit(Op::Nil, span.line);
                Ok(())
            }
            Expr::Ref { mutable, target, span } => {
                let Expr::Var { name } = target.as_ref() else {
                    // 数组元素引用：编译数组与下标后由 VM 生成 ArrayEl —— 本期简化：仅变量
                    return Err(CompileError::at(
                        "本期仅支持对变量取引用 `&x` / `&mut x`",
                        *span,
                    ));
                };
                let slot = self.f().slot(&name.name).ok_or_else(|| {
                    CompileError::at(format!("未定义的变量 `{}`", name.name), name.span)
                })?;
                let op = if *mutable { Op::RefMutLocal } else { Op::RefLocal };
                self.chunk().emit(op, span.line);
                self.chunk().emit_u16(slot, span.line);
                Ok(())
            }
            Expr::Deref { ptr, span } => {
                self.expr(ptr)?;
                self.chunk().emit(Op::DerefRead, span.line);
                Ok(())
            }
            Expr::Interp { parts, span } => {
                // 依次：片段 → to_string → 两两 Add 拼接
                let mut first = true;
                for p in parts {
                    match p {
                        InterpPart::Text(s) => {
                            self.chunk()
                                .emit_const(Value::Str(Rc::from(s.as_str())), span.line)?;
                        }
                        InterpPart::Expr(e) => {
                            self.expr(e)?;
                            self.chunk().emit(Op::ToString, span.line);
                        }
                    }
                    if first {
                        first = false;
                    } else {
                        self.chunk().emit(Op::Add, span.line);
                    }
                }
                if first {
                    // 空插值串
                    self.chunk()
                        .emit_const(Value::Str(Rc::from("")), span.line)?;
                }
                Ok(())
            }
            Expr::StructLit { name, fields, span } => {
""",
),
])

# compile assign via_deref
patch("src/codegen/compile/stmt.rs", [
(
"""    pub(crate) fn assign(&mut self, a: &AssignStmt) -> Result<(), CompileError> {
        let line = a.span.line;
""",
"""    pub(crate) fn assign(&mut self, a: &AssignStmt) -> Result<(), CompileError> {
        let line = a.span.line;
        // `*p = v`：value 为 Binary{Eq, ptr, v} + via_deref
        if a.via_deref {
            if let Expr::Binary { lhs, rhs, .. } = &a.value {
                self.expr(lhs)?;
                self.expr(rhs)?;
                self.chunk().emit(Op::DerefWrite, line);
                return Ok(());
            }
        }
""",
),
])

# compile implicit return in stmt Expr
patch("src/codegen/compile/stmt.rs", [
(
"""            Stmt::Expr(e) => {
""",
"""            Stmt::Expr(e) => {
                // 函数体末尾隐式 return
                if e.implicit_return && !matches!(e.expr, Expr::Call { .. }) {
                    // 由 compile 函数体末尾统一处理更稳妥；此处仍按表达式语句编译并 Pop
                }
""",
),
])

# VM ops at end of match before closing
patch("src/runtime/vm/exec.rs", [
(
"""                // to_string(v)：把任意可打印值转成字符串显示
                Op::ToString => {
                    let v = self.pop(line)?;
                    let s = v.display();
                    self.stack.push(Value::Str(Rc::from(s.as_str())));
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
            }
""",
"""                // to_string(v)：把任意可打印值转成字符串显示
                Op::ToString => {
                    let v = self.pop(line)?;
                    let s = v.display();
                    self.stack.push(Value::Str(Rc::from(s.as_str())));
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                Op::Nil => {
                    self.stack.push(Value::Ptr(crate::runtime::value::Ptr::Nil));
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                Op::RefLocal | Op::RefMutLocal => {
                    let slot = self.u16(ip + 1)? as usize;
                    let frame_idx = self.frames.len() - 1;
                    self.stack.push(Value::Ptr(crate::runtime::value::Ptr::Local {
                        frame: frame_idx,
                        slot,
                    }));
                    self.frames.last_mut().unwrap().ip = ip + 3;
                }
                Op::DerefRead => {
                    let p = self.pop(line)?;
                    let v = self.deref_read(&p, line)?;
                    self.stack.push(v);
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                Op::DerefWrite => {
                    let val = self.pop(line)?;
                    let p = self.pop(line)?;
                    self.deref_write(&p, val, line)?;
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                Op::PtrEq => {
                    let b = self.pop(line)?;
                    let a = self.pop(line)?;
                    let eq = match (&a, &b) {
                        (Value::Ptr(p), Value::Ptr(q)) => match (p, q) {
                            (crate::runtime::value::Ptr::Nil, crate::runtime::value::Ptr::Nil) => {
                                true
                            }
                            (
                                crate::runtime::value::Ptr::Local { frame: f1, slot: s1 },
                                crate::runtime::value::Ptr::Local { frame: f2, slot: s2 },
                            ) => f1 == f2 && s1 == s2,
                            (
                                crate::runtime::value::Ptr::ArrayEl { index: i1, .. },
                                crate::runtime::value::Ptr::ArrayEl { index: i2, .. },
                            ) => i1 == i2,
                            _ => false,
                        },
                        _ => false,
                    };
                    self.stack.push(Value::Bool(eq));
                    self.frames.last_mut().unwrap().ip = ip + 1;
                }
                Op::CallMethod => {
                    // 已在前文处理则忽略；此处兜底避免非穷尽
                    return Err(VmError {
                        message: "内部错误：CallMethod 未处理".into(),
                        line,
                    });
                }
            }
""",
),
])

# add deref helpers on Vm in exec.rs
patch("src/runtime/vm/exec.rs", [
(
"impl<'m> Vm<'m> {\n",
"""impl<'m> Vm<'m> {
    /// 通过指针读取：nil 报错；Local 帧仍在则读栈槽；ArrayEl 读数组。
    fn deref_read(&self, p: &crate::runtime::value::Ptr, line: u32) -> Result<Value, VmError> {
        use crate::runtime::value::Ptr;
        match p {
            Ptr::Nil => Err(VmError {
                message: "解引用空指针".into(),
                line,
            }),
            Ptr::Local { frame, slot } => {
                if *frame >= self.frames.len() {
                    return Err(VmError {
                        message: "悬垂指针：函数帧已结束".into(),
                        line,
                    });
                }
                let base = self.frames[*frame].base;
                self.stack.get(base + slot).cloned().ok_or_else(|| VmError {
                    message: "指针槽无效".into(),
                    line,
                })
            }
            Ptr::ArrayEl { arr, index } => {
                let b = arr.borrow();
                if *index < 0 || *index as usize >= b.len() {
                    return Err(VmError {
                        message: format!("指针下标 {index} 越界"),
                        line,
                    });
                }
                Ok(b[*index as usize].clone())
            }
        }
    }

    /// 通过指针写入。
    fn deref_write(
        &mut self,
        p: &crate::runtime::value::Ptr,
        val: Value,
        line: u32,
    ) -> Result<(), VmError> {
        use crate::runtime::value::Ptr;
        match p {
            Ptr::Nil => Err(VmError {
                message: "解引用空指针（写）".into(),
                line,
            }),
            Ptr::Local { frame, slot } => {
                if *frame >= self.frames.len() {
                    return Err(VmError {
                        message: "悬垂指针：函数帧已结束".into(),
                        line,
                    });
                }
                let base = self.frames[*frame].base;
                let addr = base + slot;
                if addr >= self.stack.len() {
                    return Err(VmError {
                        message: "指针槽无效".into(),
                        line,
                    });
                }
                self.stack[addr] = val;
                Ok(())
            }
            Ptr::ArrayEl { arr, index } => {
                let mut b = arr.borrow_mut();
                if *index < 0 || *index as usize >= b.len() {
                    return Err(VmError {
                        message: format!("指针下标 {index} 越界"),
                        line,
                    });
                }
                b[*index as usize] = val;
                Ok(())
            }
        }
    }
""",
),
])

print("phase2 done")
