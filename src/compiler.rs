//! 编译器：AST → 栈式字节码。
//!
//! 局部变量按声明顺序占用栈槽（函数参数在前）；`Call` 后接**函数索引**（不是 argc）。
//! 控制流用 `Jump`/`JumpIfFalse`/`Loop` 相对偏移，编译时先占位再 `patch_jump`。
//! `for x in arr` 脱糖为下标 `while`；结构体字面量按声明字段顺序求值后 `NewStruct`。

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::ast::*;
use crate::bytecode::{Chunk, Function, Module, Op, StructType};
use crate::token::Span;
use crate::value::Value;

pub struct CompileError {
    pub message: String,
    pub line: u32,
    pub col: u32,
}

impl CompileError {
    fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            line: span.line,
            col: span.col,
        }
    }

    fn from_msg(message: String, line: u32) -> Self {
        Self {
            message,
            line,
            col: 0,
        }
    }
}

impl From<String> for CompileError {
    fn from(message: String) -> Self {
        Self::from_msg(message, 0)
    }
}

struct Local {
    name: String,
    depth: i32,
}

struct FnCompiler {
    name: String,
    arity: u8,
    chunk: Chunk,
    locals: Vec<Local>,
    scope_depth: i32,
}

impl FnCompiler {
    fn new(name: String, arity: u8) -> Self {
        Self {
            name,
            arity,
            chunk: Chunk::new(),
            locals: Vec::new(),
            scope_depth: 0,
        }
    }

    fn add_local(&mut self, name: String) {
        self.locals.push(Local {
            name,
            depth: self.scope_depth,
        });
    }

    fn resolve_local(&self, name: &str) -> Option<u16> {
        self.locals
            .iter()
            .rev()
            .position(|l| l.name == name)
            .map(|idx| (self.locals.len() - 1 - idx) as u16)
    }

    fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    fn end_scope(&mut self, line: u32) {
        self.scope_depth -= 1;
        while let Some(l) = self.locals.last() {
            if l.depth > self.scope_depth {
                self.locals.pop();
                self.chunk.emit_op(Op::Pop, line);
            } else {
                break;
            }
        }
    }
}

pub struct Compiler {
    function_index: HashMap<String, usize>,
    functions: Vec<FnCompiler>,
    void_fns: HashMap<String, ()>,
    struct_index: HashMap<String, usize>,
    struct_types: Vec<StructType>,
    /// break/continue 回填
    loops: Vec<LoopCtx>,
    current: usize,
}

struct LoopCtx {
    breaks: Vec<usize>,
    continues: Vec<usize>,
}

impl Compiler {
    pub fn compile(program: &Program) -> Result<Module, CompileError> {
        let mut seen = HashSet::new();
        for f in program.functions() {
            if !seen.insert(f.name.name.clone()) {
                return Err(CompileError::new(
                    format!("duplicate function `{}`", f.name.name),
                    f.name.span,
                ));
            }
        }

        let mut struct_index = HashMap::new();
        let mut struct_types = Vec::new();
        for s in program.structs() {
            if struct_index.contains_key(&s.name.name) {
                return Err(CompileError::new(
                    format!("duplicate struct `{}`", s.name.name),
                    s.name.span,
                ));
            }
            struct_index.insert(s.name.name.clone(), struct_types.len());
            struct_types.push(StructType {
                name: s.name.name.clone(),
                fields: s.fields.iter().map(|f| f.name.name.clone()).collect(),
            });
        }

        let mut function_index = HashMap::new();
        let mut void_fns = HashMap::new();

        let mut fn_decls: Vec<&FunDecl> = program.functions().collect();
        fn_decls.sort_by_key(|f| f.span.line);

        for (i, f) in fn_decls.iter().enumerate() {
            let key = match &f.on_type {
                Some(t) => format!("{}.{}", t.name, f.name.name),
                None => f.name.name.clone(),
            };
            function_index.insert(key.clone(), i);
            let is_void = f.ret.is_void();
            if is_void {
                void_fns.insert(key, ());
            }
        }
        let toplevel_index = fn_decls.len();
        function_index.insert("$toplevel".to_string(), toplevel_index);
        void_fns.insert("$toplevel".to_string(), ());
        // void 内建：print / push
        void_fns.insert("print".to_string(), ());
        void_fns.insert("push".to_string(), ());

        let mut functions: Vec<FnCompiler> = Vec::new();
        for f in &fn_decls {
            let fname = match &f.on_type {
                Some(t) => format!("{}.{}", t.name, f.name.name),
                None => f.name.name.clone(),
            };
            let mut c = FnCompiler::new(fname, f.params.len() as u8);
            for p in &f.params {
                c.add_local(p.name.name.clone());
            }
            functions.push(c);
        }
        functions.push(FnCompiler::new("$toplevel".to_string(), 0));

        let mut compiler = Compiler {
            function_index,
            functions,
            void_fns,
            struct_index,
            struct_types,
            loops: Vec::new(),
            current: 0,
        };

        for (i, f) in fn_decls.iter().enumerate() {
            compiler.current = i;
            for stmt in &f.body.stmts {
                compiler.compile_stmt(stmt)?;
            }
            let line = f.body.span.line;
            compiler.chunk().emit_op(Op::Return, line);
        }

        compiler.current = toplevel_index;
        for stmt in program.top_level_stmts() {
            compiler.compile_stmt(stmt)?;
        }
        compiler.chunk().emit_op(Op::Return, 0);

        let void_set = compiler.void_fns.clone();
        let struct_types = compiler.struct_types.clone();
        let mut out_functions = Vec::new();
        let mut main_index = None;
        for fc in compiler.functions {
            if fc.name == "main" {
                main_index = Some(out_functions.len());
            }
            let is_void = fc.name == "$toplevel" || void_set.contains_key(&fc.name);
            out_functions.push(Function {
                name: fc.name,
                arity: fc.arity,
                locals: fc.locals.len() as u16,
                is_void,
                chunk: fc.chunk,
            });
        }

        Ok(Module {
            functions: out_functions,
            main_index,
            toplevel_index,
            struct_types,
            globals: Vec::new(),
        })
    }

    fn chunk(&mut self) -> &mut Chunk {
        &mut self.functions[self.current].chunk
    }

    fn fn_mut(&mut self) -> &mut FnCompiler {
        &mut self.functions[self.current]
    }

    fn is_void_call_target(&self, name: &str) -> bool {
        self.void_fns.contains_key(name)
    }

    fn compile_block(&mut self, block: &Block) -> Result<(), CompileError> {
        self.fn_mut().begin_scope();
        for stmt in &block.stmts {
            self.compile_stmt(stmt)?;
        }
        self.fn_mut().end_scope(block.span.line);
        Ok(())
    }

    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), CompileError> {
        match stmt {
            Stmt::Let(l) => self.compile_let(l),
            Stmt::Assign(a) => self.compile_assign(a),
            Stmt::If(i) => self.compile_if(i),
            Stmt::While(w) => self.compile_while(w),
            Stmt::For(f) => self.compile_for(f),
            Stmt::Break(span) => {
                if self.loops.is_empty() {
                    return Err(CompileError::new("`break` outside loop", *span));
                }
                let j = self.chunk().emit_jump(Op::Jump, span.line);
                self.loops.last_mut().unwrap().breaks.push(j);
                Ok(())
            }
            Stmt::Continue(span) => {
                if self.loops.is_empty() {
                    return Err(CompileError::new("`continue` outside loop", *span));
                }
                let j = self.chunk().emit_jump(Op::Jump, span.line);
                self.loops.last_mut().unwrap().continues.push(j);
                Ok(())
            }
            Stmt::Return(r) => {
                match &r.value {
                    None => self.chunk().emit_op(Op::Return, r.span.line),
                    Some(e) => {
                        self.compile_expr(e)?;
                        self.chunk().emit_op(Op::Return, r.span.line);
                    }
                }
                Ok(())
            }
            Stmt::Expr(e) => {
                let leaves_value = match &e.expr {
                    Expr::Call { callee, .. }
                        if callee.name == "print"
                            || callee.name == "push"
                            || self.is_void_call_target(&callee.name) =>
                    {
                        false
                    }
                    _ => true,
                };
                self.compile_expr(&e.expr)?;
                if leaves_value {
                    self.chunk().emit_op(Op::Pop, e.span.line);
                }
                Ok(())
            }
            Stmt::Block(b) => self.compile_block(b),
        }
    }

    fn compile_let(&mut self, l: &LetStmt) -> Result<(), CompileError> {
        self.compile_expr(&l.value)?;
        self.fn_mut().add_local(l.name.name.clone());
        Ok(())
    }

    fn compile_assign(&mut self, a: &AssignStmt) -> Result<(), CompileError> {
        let line = a.span.line;
        if let Some(idx) = &a.index {
            let slot = self.fn_mut().resolve_local(&a.name.name).ok_or_else(|| {
                CompileError::new(format!("undefined variable `{}`", a.name.name), a.name.span)
            })?;
            self.chunk().emit_op(Op::GetLocal, line);
            self.chunk().emit_u16(slot, line);
            self.compile_expr(idx)?;
            if a.fields.is_empty() {
                self.compile_expr(&a.value)?;
                self.chunk().emit_op(Op::SetIndex, line);
                return Ok(());
            }
            // a[i].f... = v
            self.chunk().emit_op(Op::GetIndex, line);
            for (i, field) in a.fields.iter().enumerate() {
                if i + 1 < a.fields.len() {
                    let cidx = self
                        .chunk()
                        .add_constant(Value::Str(Rc::from(field.name.as_str())))?;
                    self.chunk().emit_op(Op::GetField, line);
                    self.chunk().emit_u16(cidx, line);
                }
            }
            self.compile_expr(&a.value)?;
            let last = a.fields.last().unwrap();
            let cidx = self
                .chunk()
                .add_constant(Value::Str(Rc::from(last.name.as_str())))?;
            self.chunk().emit_op(Op::SetField, line);
            self.chunk().emit_u16(cidx, line);
            return Ok(());
        }

        if !a.fields.is_empty() {
            let slot = self.fn_mut().resolve_local(&a.name.name).ok_or_else(|| {
                CompileError::new(format!("undefined variable `{}`", a.name.name), a.name.span)
            })?;
            self.chunk().emit_op(Op::GetLocal, line);
            self.chunk().emit_u16(slot, line);
            for (i, field) in a.fields.iter().enumerate() {
                if i + 1 < a.fields.len() {
                    let cidx = self
                        .chunk()
                        .add_constant(Value::Str(Rc::from(field.name.as_str())))?;
                    self.chunk().emit_op(Op::GetField, line);
                    self.chunk().emit_u16(cidx, line);
                }
            }
            self.compile_expr(&a.value)?;
            let last = a.fields.last().unwrap();
            let cidx = self
                .chunk()
                .add_constant(Value::Str(Rc::from(last.name.as_str())))?;
            self.chunk().emit_op(Op::SetField, line);
            self.chunk().emit_u16(cidx, line);
            return Ok(());
        }

        self.compile_expr(&a.value)?;
        let slot = self.fn_mut().resolve_local(&a.name.name).ok_or_else(|| {
            CompileError::new(format!("undefined variable `{}`", a.name.name), a.name.span)
        })?;
        self.chunk().emit_op(Op::SetLocal, line);
        self.chunk().emit_u16(slot, line);
        self.chunk().emit_op(Op::Pop, line);
        Ok(())
    }

    fn compile_if(&mut self, i: &IfStmt) -> Result<(), CompileError> {
        self.compile_expr(&i.cond)?;
        let line = i.span.line;
        // JumpIfFalse 只 peek 条件不弹栈；两条路径都必须自己 Pop。
        let then_jump = self.chunk().emit_jump(Op::JumpIfFalse, line);
        self.chunk().emit_op(Op::Pop, line);
        self.compile_block(&i.then_block)?;
        match &i.else_branch {
            None => {
                // true 路径须 Jump 越过 false 路径的 Pop，否则会误弹局部槽。
                let end_jump = self.chunk().emit_jump(Op::Jump, line);
                self.chunk().patch_jump(then_jump)?;
                self.chunk().emit_op(Op::Pop, line);
                self.chunk().patch_jump(end_jump)?;
            }
            Some(branch) => {
                let else_jump = self.chunk().emit_jump(Op::Jump, line);
                self.chunk().patch_jump(then_jump)?;
                self.chunk().emit_op(Op::Pop, line);
                match branch {
                    ElseBranch::Block(b) => self.compile_block(b)?,
                    ElseBranch::If(n) => self.compile_if(n)?,
                }
                self.chunk().patch_jump(else_jump)?;
            }
        }
        Ok(())
    }

    fn compile_while(&mut self, w: &WhileStmt) -> Result<(), CompileError> {
        let line = w.span.line;
        let loop_start = self.chunk().code.len();
        self.loops.push(LoopCtx {
            breaks: Vec::new(),
            continues: Vec::new(),
        });
        self.compile_expr(&w.cond)?;
        let exit = self.chunk().emit_jump(Op::JumpIfFalse, line);
        self.chunk().emit_op(Op::Pop, line);
        self.compile_block(&w.body)?;
        // continue → 回到条件
        let cont = self.chunk().code.len();
        let ctx = self.loops.pop().unwrap();
        for j in ctx.continues {
            self.chunk().patch_jump_to(j, cont)?;
        }
        self.chunk().emit_loop(loop_start, line)?;
        self.chunk().patch_jump(exit)?;
        self.chunk().emit_op(Op::Pop, line);
        let end = self.chunk().code.len();
        for j in ctx.breaks {
            self.chunk().patch_jump_to(j, end)?;
        }
        Ok(())
    }

    /// for：数组 for-in 或 `a..b` 范围；支持 break/continue
    fn compile_for(&mut self, f: &ForStmt) -> Result<(), CompileError> {
        let line = f.span.line;
        if let Expr::Range { start, end, .. } = &f.iter {
            self.fn_mut().begin_scope();
            self.compile_expr(start)?;
            self.fn_mut().add_local(f.var.name.clone());
            self.compile_expr(end)?;
            self.fn_mut().add_local("\0for_end".into());
            let i_slot = self.fn_mut().resolve_local(&f.var.name).unwrap();
            let end_slot = self.fn_mut().resolve_local("\0for_end").unwrap();

            let loop_start = self.chunk().code.len();
            self.loops.push(LoopCtx {
                breaks: Vec::new(),
                continues: Vec::new(),
            });
            self.chunk().emit_op(Op::GetLocal, line);
            self.chunk().emit_u16(i_slot, line);
            self.chunk().emit_op(Op::GetLocal, line);
            self.chunk().emit_u16(end_slot, line);
            self.chunk().emit_op(Op::Lt, line);
            let exit = self.chunk().emit_jump(Op::JumpIfFalse, line);
            self.chunk().emit_op(Op::Pop, line);

            self.fn_mut().begin_scope();
            for stmt in &f.body.stmts {
                self.compile_stmt(stmt)?;
            }
            self.fn_mut().end_scope(line);

            // continue 目标：增量
            let inc = self.chunk().code.len();
            let ctx = self.loops.pop().unwrap();
            for j in ctx.continues {
                self.chunk().patch_jump_to(j, inc)?;
            }
            self.chunk().emit_op(Op::GetLocal, line);
            self.chunk().emit_u16(i_slot, line);
            self.chunk().emit_const(Value::Int(1), line)?;
            self.chunk().emit_op(Op::Add, line);
            self.chunk().emit_op(Op::SetLocal, line);
            self.chunk().emit_u16(i_slot, line);
            self.chunk().emit_op(Op::Pop, line);
            self.chunk().emit_loop(loop_start, line)?;
            self.chunk().patch_jump(exit)?;
            self.chunk().emit_op(Op::Pop, line);
            let end_pc = self.chunk().code.len();
            for j in ctx.breaks {
                self.chunk().patch_jump_to(j, end_pc)?;
            }
            self.fn_mut().end_scope(line);
            return Ok(());
        }

        // 数组 for-in
        self.fn_mut().begin_scope();
        self.compile_expr(&f.iter)?;
        self.fn_mut().add_local("\0for_arr".into());
        self.chunk().emit_const(Value::Int(0), line)?;
        self.fn_mut().add_local("\0for_i".into());

        let arr_slot = self
            .fn_mut()
            .resolve_local("\0for_arr")
            .ok_or_else(|| CompileError::new("for-in temp missing", f.span))?;
        let i_slot = self
            .fn_mut()
            .resolve_local("\0for_i")
            .ok_or_else(|| CompileError::new("for-in temp missing", f.span))?;

        let loop_start = self.chunk().code.len();
        self.loops.push(LoopCtx {
            breaks: Vec::new(),
            continues: Vec::new(),
        });
        self.chunk().emit_op(Op::GetLocal, line);
        self.chunk().emit_u16(i_slot, line);
        self.chunk().emit_op(Op::GetLocal, line);
        self.chunk().emit_u16(arr_slot, line);
        self.chunk().emit_op(Op::Len, line);
        self.chunk().emit_op(Op::Lt, line);
        let exit = self.chunk().emit_jump(Op::JumpIfFalse, line);
        self.chunk().emit_op(Op::Pop, line);

        self.chunk().emit_op(Op::GetLocal, line);
        self.chunk().emit_u16(arr_slot, line);
        self.chunk().emit_op(Op::GetLocal, line);
        self.chunk().emit_u16(i_slot, line);
        self.chunk().emit_op(Op::GetIndex, line);
        self.fn_mut().begin_scope();
        self.fn_mut().add_local(f.var.name.clone());
        for stmt in &f.body.stmts {
            self.compile_stmt(stmt)?;
        }
        self.fn_mut().end_scope(line);

        let inc = self.chunk().code.len();
        let ctx = self.loops.pop().unwrap();
        for j in ctx.continues {
            self.chunk().patch_jump_to(j, inc)?;
        }
        self.chunk().emit_op(Op::GetLocal, line);
        self.chunk().emit_u16(i_slot, line);
        self.chunk().emit_const(Value::Int(1), line)?;
        self.chunk().emit_op(Op::Add, line);
        self.chunk().emit_op(Op::SetLocal, line);
        self.chunk().emit_u16(i_slot, line);
        self.chunk().emit_op(Op::Pop, line);

        self.chunk().emit_loop(loop_start, line)?;
        self.chunk().patch_jump(exit)?;
        self.chunk().emit_op(Op::Pop, line);
        let end_pc = self.chunk().code.len();
        for j in ctx.breaks {
            self.chunk().patch_jump_to(j, end_pc)?;
        }
        self.fn_mut().end_scope(line);
        Ok(())
    }

    fn compile_expr(&mut self, expr: &Expr) -> Result<(), CompileError> {
        match expr {
            Expr::Int { value, span } => {
                self.chunk().emit_const(Value::Int(*value), span.line)?;
                Ok(())
            }
            Expr::Float { value, span } => {
                self.chunk().emit_const(Value::Float(*value), span.line)?;
                Ok(())
            }
            Expr::Bool { value, span } => {
                if *value {
                    self.chunk().emit_op(Op::True, span.line);
                } else {
                    self.chunk().emit_op(Op::False, span.line);
                }
                Ok(())
            }
            Expr::Str { value, span } => {
                let v = Value::Str(Rc::from(value.as_str()));
                self.chunk().emit_const(v, span.line)?;
                Ok(())
            }
            Expr::Var { name } => {
                let slot = self.fn_mut().resolve_local(&name.name).ok_or_else(|| {
                    CompileError::new(format!("undefined variable `{}`", name.name), name.span)
                })?;
                self.chunk().emit_op(Op::GetLocal, name.span.line);
                self.chunk().emit_u16(slot, name.span.line);
                Ok(())
            }
            Expr::Unary { op, expr, span } => {
                self.compile_expr(expr)?;
                match op {
                    UnOp::Neg => self.chunk().emit_op(Op::Neg, span.line),
                    UnOp::Not => self.chunk().emit_op(Op::Not, span.line),
                }
                Ok(())
            }
            Expr::Binary { op, lhs, rhs, span } => self.compile_binary(*op, lhs, rhs, *span),
            Expr::Call { callee, args, span } => self.compile_call(&callee.name, args, *span),
            Expr::MethodCall {
                recv,
                method,
                args,
                span,
            } => {
                if let Expr::Var { name } = recv.as_ref() {
                    let dotted = format!("{}.{}", name.name, method.name);
                    if self.function_index.contains_key(&dotted) {
                        return self.compile_call(&dotted, args, *span);
                    }
                }
                self.compile_expr(recv)?;
                for a in args {
                    self.compile_expr(a)?;
                }
                let cidx = self
                    .chunk()
                    .add_constant(Value::Str(Rc::from(method.name.as_str())))?;
                self.chunk().emit_op(Op::CallMethod, span.line);
                self.chunk().emit_u16(cidx, span.line);
                self.chunk().emit_u16(args.len() as u16, span.line);
                Ok(())
            }
            Expr::Range { start, end, span } => {
                // 范围字面量仅应出现在 for；单独求值时编译为错误提示用空数组
                let _ = (start, end);
                Err(CompileError::new(
                    "range `a..b` is only allowed in `for`",
                    *span,
                ))
            }
            Expr::Index { base, index, span } => {
                self.compile_expr(base)?;
                self.compile_expr(index)?;
                self.chunk().emit_op(Op::GetIndex, span.line);
                Ok(())
            }
            Expr::Array { elems, span } => {
                for e in elems {
                    self.compile_expr(e)?;
                }
                self.chunk().emit_op(Op::NewArray, span.line);
                self.chunk().emit_u16(elems.len() as u16, span.line);
                Ok(())
            }
            Expr::Field { base, name, span } => {
                self.compile_expr(base)?;
                let cidx = self
                    .chunk()
                    .add_constant(Value::Str(Rc::from(name.name.as_str())))?;
                self.chunk().emit_op(Op::GetField, span.line);
                self.chunk().emit_u16(cidx, span.line);
                Ok(())
            }
            Expr::StructLit { name, fields, span } => {
                let idx = *self.struct_index.get(&name.name).ok_or_else(|| {
                    CompileError::new(format!("unknown struct `{}`", name.name), name.span)
                })?;
                let decl = self.struct_types[idx].clone();
                for fname in &decl.fields {
                    let (_n, e) =
                        fields
                            .iter()
                            .find(|(n, _)| &n.name == fname)
                            .ok_or_else(|| {
                                CompileError::new(
                                    format!("missing field `{fname}` in `{}`", name.name),
                                    name.span,
                                )
                            })?;
                    self.compile_expr(e)?;
                }
                self.chunk().emit_op(Op::NewStruct, span.line);
                self.chunk().emit_u16(idx as u16, span.line);
                self.chunk().emit_u16(decl.fields.len() as u16, span.line);
                Ok(())
            }
        }
    }

    fn compile_binary(
        &mut self,
        op: BinOp,
        lhs: &Expr,
        rhs: &Expr,
        span: Span,
    ) -> Result<(), CompileError> {
        let line = span.line;
        match op {
            BinOp::And => {
                self.compile_expr(lhs)?;
                let jump = self.chunk().emit_jump(Op::JumpIfFalse, line);
                self.chunk().emit_op(Op::Pop, line);
                self.compile_expr(rhs)?;
                self.chunk().patch_jump(jump)?;
                Ok(())
            }
            BinOp::Or => {
                self.compile_expr(lhs)?;
                let jump = self.chunk().emit_jump(Op::JumpIfTrue, line);
                self.chunk().emit_op(Op::Pop, line);
                self.compile_expr(rhs)?;
                self.chunk().patch_jump(jump)?;
                Ok(())
            }
            _ => {
                self.compile_expr(lhs)?;
                self.compile_expr(rhs)?;
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
                self.chunk().emit_op(o, line);
                Ok(())
            }
        }
    }

    fn compile_call(&mut self, name: &str, args: &[Expr], span: Span) -> Result<(), CompileError> {
        let line = span.line;
        match name {
            "print" => {
                if args.len() != 1 {
                    return Err(CompileError::new("`print` takes 1 argument", span));
                }
                self.compile_expr(&args[0])?;
                self.chunk().emit_op(Op::Print, line);
                return Ok(());
            }
            "len" => {
                if args.len() != 1 {
                    return Err(CompileError::new("`len` takes 1 argument", span));
                }
                self.compile_expr(&args[0])?;
                self.chunk().emit_op(Op::Len, line);
                return Ok(());
            }
            "str_at" => {
                for a in args {
                    self.compile_expr(a)?;
                }
                self.chunk().emit_op(Op::StrAt, line);
                return Ok(());
            }
            "str_sub" => {
                for a in args {
                    self.compile_expr(a)?;
                }
                self.chunk().emit_op(Op::StrSub, line);
                return Ok(());
            }
            "to_string" => {
                if args.len() != 1 {
                    return Err(CompileError::new("`to_string` takes 1 argument", span));
                }
                self.compile_expr(&args[0])?;
                self.chunk().emit_op(Op::ToString, line);
                return Ok(());
            }
            "push" => {
                for a in args {
                    self.compile_expr(a)?;
                }
                self.chunk().emit_op(Op::Push, line);
                return Ok(());
            }
            "pop" => {
                if args.len() != 1 {
                    return Err(CompileError::new("`pop` takes 1 argument", span));
                }
                self.compile_expr(&args[0])?;
                self.chunk().emit_op(Op::ArrayPop, line);
                return Ok(());
            }
            "input" => {
                self.chunk().emit_op(Op::Input, line);
                return Ok(());
            }
            _ => {}
        }
        let idx = *self
            .function_index
            .get(name)
            .ok_or_else(|| CompileError::new(format!("undefined function `{name}`"), span))?;
        for a in args {
            self.compile_expr(a)?;
        }
        self.chunk().emit_op(Op::Call, line);
        self.chunk().emit_u16(idx as u16, line);
        Ok(())
    }
}
