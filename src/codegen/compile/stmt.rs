//! 语句、赋值、if/while/for 编译。
use super::*;

impl Compiler {
    pub(crate) fn block(&mut self, b: &Block) -> Result<(), CompileError> {
        self.f().begin();
        for s in &b.stmts {
            self.stmt(s)?;
        }
        self.f().end(b.span.line);
        Ok(())
    }

    /// 编译语句：按种类分派（let/赋值/if/while/for/break/continue/return/块）。
    pub(crate) fn stmt(&mut self, s: &Stmt) -> Result<(), CompileError> {
        match s {
            Stmt::Let(l) => {
                self.expr(&l.value)?;
                self.f().add_local(l.name.name.clone());
                Ok(())
            }
            Stmt::Assign(a) => self.assign(a),
            Stmt::If(i) => self.if_stmt(i),
            // while：循环头 → 条件 → 假跳出；真则 Pop 条件、执行体；continue→条件，break→出口
            Stmt::While(w) => {
                let line = w.span.line;
                let start = self.chunk().code.len();
                self.loops.push(LoopP {
                    breaks: vec![],
                    continues: vec![],
                });
                self.expr(&w.cond)?;
                let exit = self.chunk().emit_jump(Op::JumpIfFalse, line);
                self.chunk().emit(Op::Pop, line);
                self.block(&w.body)?;
                let cont = self.chunk().code.len();
                let ctx = self.loops.pop().unwrap();
                for j in ctx.continues {
                    self.chunk().patch_to(j, cont)?;
                }
                self.chunk().emit_loop(start, line)?;
                self.chunk().patch_to_end(exit)?;
                self.chunk().emit(Op::Pop, line);
                let end = self.chunk().code.len();
                for j in ctx.breaks {
                    self.chunk().patch_to(j, end)?;
                }
                Ok(())
            }
            Stmt::For(fr) => self.for_stmt(fr),
            // break：只发射向前 Jump，偏移在循环编译结束时回填到出口
            Stmt::Break(sp) => {
                if self.loops.is_empty() {
                    return Err(CompileError::at("`break` 只能用在循环内", *sp));
                }
                let j = self.chunk().emit_jump(Op::Jump, sp.line);
                self.loops.last_mut().unwrap().breaks.push(j);
                Ok(())
            }
            // continue：Jump 回填到增量/回边，避免数组 for 死循环
            Stmt::Continue(sp) => {
                if self.loops.is_empty() {
                    return Err(CompileError::at("`continue` 只能用在循环内", *sp));
                }
                let j = self.chunk().emit_jump(Op::Jump, sp.line);
                self.loops.last_mut().unwrap().continues.push(j);
                Ok(())
            }
            // return：有值则先压栈再 Return
            Stmt::Return(r) => {
                match &r.value {
                    None => self.chunk().emit(Op::Return, r.span.line),
                    Some(e) => {
                        self.expr(e)?;
                        self.chunk().emit(Op::Return, r.span.line);
                    }
                }
                Ok(())
            }
            Stmt::Expr(e) => {
                // 函数体末尾隐式 return
                if e.implicit_return && !matches!(e.expr, Expr::Call { .. }) {
                    // 由 compile 函数体末尾统一处理更稳妥；此处仍按表达式语句编译并 Pop
                }
                let void = match &e.expr {
                    Expr::Call { callee, .. } => self.void_call(&callee.name),
                    Expr::MethodCall { method, .. } => self.void_call(&method.name),
                    _ => false,
                };
                self.expr(&e.expr)?;
                if !void {
                    self.chunk().emit(Op::Pop, e.span.line);
                }
                Ok(())
            }
            Stmt::Block(b) => self.block(b),
        }
    }

    /// 编译赋值语句。
    ///
    /// 三种形态，指令序列不同：
    ///
    /// 1) `x = e`  
    ///    算出 `e` → `SetLocal 槽x` → `Pop`（运算结果副本）
    ///
    /// 2) `a[i] = e`  
    ///    先压 `a` 和 `i`，再算 `e`，最后 `SetIndex`（数组是句柄，就地改）
    ///
    /// 3) `p.f = e`（及多层 `p.a.b = e`）  
    ///    结构体是值，需要：压根对象 → 取出父节点 → 算 `e` → `SetField` 沿路径写回 → `SetLocal`
    ///
    /// `arr[i].f = e`：在栈上保留 `[arr, i]`，再取出元素副本改字段，最后 `SetIndex` 写回数组。
    /// 编译赋值。
    /// 1) `x = e`：算 e → SetLocal → Pop
    /// 2) `a[i] = e`：压 a、i、e → SetIndex
    /// 3) `p.f = e`：值语义，取出/写回见下方分支
    pub(crate) fn assign(&mut self, a: &AssignStmt) -> Result<(), CompileError> {
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
        let slot = self.f().slot(&a.name.name).ok_or_else(|| {
            CompileError::at(format!("未定义的变量 `{}`", a.name.name), a.name.span)
        })?;

        if a.fields.is_empty() {
            match &a.index {
                None => {
                    self.expr(&a.value)?;
                    self.chunk().emit(Op::SetLocal, line);
                    self.chunk().emit_u16(slot, line);
                    self.chunk().emit(Op::Pop, line);
                }
                Some(idx) => {
                    self.chunk().emit(Op::GetLocal, line);
                    self.chunk().emit_u16(slot, line);
                    self.expr(idx)?;
                    self.expr(&a.value)?;
                    self.chunk().emit(Op::SetIndex, line);
                }
            }
            return Ok(());
        }

        // field path on local (with optional index root)
        match &a.index {
            None => {
                // [root] + 父链 + value → SetField 逆序写回 → SetLocal
                self.chunk().emit(Op::GetLocal, line);
                self.chunk().emit_u16(slot, line);
                if a.fields.len() > 1 {
                    self.chunk().emit(Op::GetLocal, line);
                    self.chunk().emit_u16(slot, line);
                    for f in &a.fields[..a.fields.len() - 1] {
                        let c = self
                            .chunk()
                            .add_const(Value::Str(Rc::from(f.name.as_str())))?;
                        self.chunk().emit(Op::GetField, line);
                        self.chunk().emit_u16(c, line);
                    }
                }
                self.expr(&a.value)?;
                for f in a.fields.iter().rev() {
                    let c = self
                        .chunk()
                        .add_const(Value::Str(Rc::from(f.name.as_str())))?;
                    self.chunk().emit(Op::SetField, line);
                    self.chunk().emit_u16(c, line);
                }
                self.chunk().emit(Op::SetLocal, line);
                self.chunk().emit_u16(slot, line);
                self.chunk().emit(Op::Pop, line);
                Ok(())
            }
            Some(idx) => {
                if a.fields.len() > 1 {
                    return Err(CompileError::at(
                        "multi-level field assign on array element: use a temporary",
                        a.span,
                    ));
                }
                self.chunk().emit(Op::GetLocal, line);
                self.chunk().emit_u16(slot, line);
                self.expr(idx)?;
                self.chunk().emit(Op::GetLocal, line);
                self.chunk().emit_u16(slot, line);
                self.expr(idx)?;
                self.chunk().emit(Op::GetIndex, line);
                self.expr(&a.value)?;
                let c = self
                    .chunk()
                    .add_const(Value::Str(Rc::from(a.fields[0].name.as_str())))?;
                self.chunk().emit(Op::SetField, line);
                self.chunk().emit_u16(c, line);
                self.chunk().emit(Op::SetIndex, line);
                Ok(())
            }
        }
    }

    /// 编译 if。步骤：
    /// 1. 编译条件（结果在栈顶）
    /// 2. JumpIfFalse 到 else/出口；true 路径先 Pop 条件
    /// 3. 编译 then 块；有 else 则 Jump 过 else，false 路径 Pop 后编译 else
    /// 4. 两路都要 Pop 条件，避免栈残留
    /// 编译 if：条件 → JumpIfFalse → 两路都要 Pop 条件，避免栈残留。
    pub(crate) fn if_stmt(&mut self, i: &IfStmt) -> Result<(), CompileError> {
        let line = i.span.line;
        self.expr(&i.cond)?;
        let then_j = self.chunk().emit_jump(Op::JumpIfFalse, line);
        self.chunk().emit(Op::Pop, line);
        self.block(&i.then_block)?;
        match &i.else_branch {
            None => {
                let end_j = self.chunk().emit_jump(Op::Jump, line);
                self.chunk().patch_to_end(then_j)?;
                self.chunk().emit(Op::Pop, line);
                self.chunk().patch_to_end(end_j)?;
            }
            Some(br) => {
                let else_j = self.chunk().emit_jump(Op::Jump, line);
                self.chunk().patch_to_end(then_j)?;
                self.chunk().emit(Op::Pop, line);
                match br {
                    ElseBranch::Block(b) => self.block(b)?,
                    ElseBranch::If(n) => self.if_stmt(n)?,
                }
                self.chunk().patch_to_end(else_j)?;
            }
        }
        Ok(())
    }

    /// 编译 `for`。
    ///
    /// **范围 for `a..b`**：
    /// 1. 算 a → 存入循环变量槽；算 b → 存入临时槽
    /// 2. 循环头：`i < end`，假则跳出
    /// 3. 执行循环体
    /// 4. continue 跳到这里：`i = i + 1`，再 Loop 回循环头
    /// 5. break / 正常结束：清理局部并继续后续代码
    ///
    /// **数组 for-in**：思路相同，多了「每次从数组取 arr[i] 绑定循环变量」。
    pub(crate) fn for_stmt(&mut self, f: &ForStmt) -> Result<(), CompileError> {
        let line = f.span.line;
        self.f().begin();
        if let Expr::Range { start, end, .. } = &f.iter {
            self.expr(start)?;
            self.f().add_local(f.var.name.clone());
            self.expr(end)?;
            self.f().add_local("\0end".into());
            let i_s = self.f().slot(&f.var.name).unwrap();
            let e_s = self.f().slot("\0end").unwrap();
            let start_pc = self.chunk().code.len();
            self.loops.push(LoopP {
                breaks: vec![],
                continues: vec![],
            });
            self.chunk().emit(Op::GetLocal, line);
            self.chunk().emit_u16(i_s, line);
            self.chunk().emit(Op::GetLocal, line);
            self.chunk().emit_u16(e_s, line);
            self.chunk().emit(Op::Lt, line);
            let exit = self.chunk().emit_jump(Op::JumpIfFalse, line);
            self.chunk().emit(Op::Pop, line);
            self.f().begin();
            for st in &f.body.stmts {
                self.stmt(st)?;
            }
            self.f().end(line);
            let inc = self.chunk().code.len();
            let ctx = self.loops.pop().unwrap();
            for j in ctx.continues {
                self.chunk().patch_to(j, inc)?;
            }
            self.chunk().emit(Op::GetLocal, line);
            self.chunk().emit_u16(i_s, line);
            self.chunk().emit_const(Value::Int(1), line)?;
            self.chunk().emit(Op::Add, line);
            self.chunk().emit(Op::SetLocal, line);
            self.chunk().emit_u16(i_s, line);
            self.chunk().emit(Op::Pop, line);
            self.chunk().emit_loop(start_pc, line)?;
            self.chunk().patch_to_end(exit)?;
            self.chunk().emit(Op::Pop, line);
            let endpc = self.chunk().code.len();
            for j in ctx.breaks {
                self.chunk().patch_to(j, endpc)?;
            }
        } else {
            self.expr(&f.iter)?;
            self.f().add_local("\0arr".into());
            self.chunk().emit_const(Value::Int(0), line)?;
            self.f().add_local("\0i".into());
            let a_s = self.f().slot("\0arr").unwrap();
            let i_s = self.f().slot("\0i").unwrap();
            let start_pc = self.chunk().code.len();
            self.loops.push(LoopP {
                breaks: vec![],
                continues: vec![],
            });
            self.chunk().emit(Op::GetLocal, line);
            self.chunk().emit_u16(i_s, line);
            self.chunk().emit(Op::GetLocal, line);
            self.chunk().emit_u16(a_s, line);
            self.chunk().emit(Op::Len, line);
            self.chunk().emit(Op::Lt, line);
            let exit = self.chunk().emit_jump(Op::JumpIfFalse, line);
            self.chunk().emit(Op::Pop, line);
            self.chunk().emit(Op::GetLocal, line);
            self.chunk().emit_u16(a_s, line);
            self.chunk().emit(Op::GetLocal, line);
            self.chunk().emit_u16(i_s, line);
            self.chunk().emit(Op::GetIndex, line);
            self.f().begin();
            self.f().add_local(f.var.name.clone());
            for st in &f.body.stmts {
                self.stmt(st)?;
            }
            self.f().end(line);
            let inc = self.chunk().code.len();
            let ctx = self.loops.pop().unwrap();
            for j in ctx.continues {
                self.chunk().patch_to(j, inc)?;
            }
            self.chunk().emit(Op::GetLocal, line);
            self.chunk().emit_u16(i_s, line);
            self.chunk().emit_const(Value::Int(1), line)?;
            self.chunk().emit(Op::Add, line);
            self.chunk().emit(Op::SetLocal, line);
            self.chunk().emit_u16(i_s, line);
            self.chunk().emit(Op::Pop, line);
            self.chunk().emit_loop(start_pc, line)?;
            self.chunk().patch_to_end(exit)?;
            self.chunk().emit(Op::Pop, line);
            let endpc = self.chunk().code.len();
            for j in ctx.breaks {
                self.chunk().patch_to(j, endpc)?;
            }
        }
        self.f().end(line);
        Ok(())
    }
}
