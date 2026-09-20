use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::*;
use crate::bytecode::{Chunk, Function, Module, Op};
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
    current: usize,
}

impl Compiler {
    pub fn compile(program: &Program) -> Result<Module, CompileError> {
        let mut function_index = HashMap::new();
        let mut void_fns = HashMap::new();

        for f in program.functions() {
            if function_index.contains_key(&f.name.name) {
                return Err(CompileError::new(
                    format!("duplicate function `{}`", f.name.name),
                    f.name.span,
                ));
            }
        }

        let mut fn_decls: Vec<&FunDecl> = program.functions().collect();
        // stable order: source order
        fn_decls.sort_by_key(|f| f.span.line);

        for (i, f) in fn_decls.iter().enumerate() {
            function_index.insert(f.name.name.clone(), i);
            if f.ret.is_void() {
                void_fns.insert(f.name.name.clone(), ());
            }
        }
        let toplevel_index = fn_decls.len();
        function_index.insert("$toplevel".to_string(), toplevel_index);
        void_fns.insert("$toplevel".to_string(), ());

        let mut functions: Vec<FnCompiler> = Vec::new();
        for f in &fn_decls {
            let mut c = FnCompiler::new(f.name.name.clone(), f.params.len() as u8);
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
                // void-producing statements leave nothing on the stack
                let leaves_value = match &e.expr {
                    Expr::Call { callee, .. } if callee.name == "print" => false,
                    Expr::Call { callee, .. } if self.is_void_call_target(&callee.name) => false,
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
                CompileError::new(
                    format!("undefined variable `{}`", a.name.name),
                    a.name.span,
                )
            })?;
            self.chunk().emit_op(Op::GetLocal, line);
            self.chunk().emit_u16(slot, line);
            self.compile_expr(idx)?;
            self.compile_expr(&a.value)?;
            self.chunk().emit_op(Op::SetIndex, line);
            return Ok(());
        }

        self.compile_expr(&a.value)?;
        let slot = self.fn_mut().resolve_local(&a.name.name).ok_or_else(|| {
            CompileError::new(
                format!("undefined variable `{}`", a.name.name),
                a.name.span,
            )
        })?;
        self.chunk().emit_op(Op::SetLocal, line);
        self.chunk().emit_u16(slot, line);
        self.chunk().emit_op(Op::Pop, line);
        Ok(())
    }

    fn compile_if(&mut self, i: &IfStmt) -> Result<(), CompileError> {
        self.compile_expr(&i.cond)?;
        let line = i.span.line;
        let then_jump = self.chunk().emit_jump(Op::JumpIfFalse, line);
        self.chunk().emit_op(Op::Pop, line);
        self.compile_block(&i.then_block)?;
        match &i.else_branch {
            None => {
                self.chunk().patch_jump(then_jump)?;
                self.chunk().emit_op(Op::Pop, line);
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
        self.compile_expr(&w.cond)?;
        let exit = self.chunk().emit_jump(Op::JumpIfFalse, line);
        self.chunk().emit_op(Op::Pop, line);
        self.compile_block(&w.body)?;
        self.chunk().emit_loop(loop_start, line)?;
        self.chunk().patch_jump(exit)?;
        self.chunk().emit_op(Op::Pop, line);
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
            Expr::Binary {
                op,
                lhs,
                rhs,
                span,
            } => self.compile_binary(*op, lhs, rhs, *span),
            Expr::Call { callee, args, span } => self.compile_call(&callee.name, args, *span),
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

    fn compile_call(
        &mut self,
        name: &str,
        args: &[Expr],
        span: Span,
    ) -> Result<(), CompileError> {
        let line = span.line;
        if name == "print" {
            if args.len() != 1 {
                return Err(CompileError::new("`print` takes 1 argument", span));
            }
            self.compile_expr(&args[0])?;
            self.chunk().emit_op(Op::Print, line);
            return Ok(());
        }
        if name == "len" {
            if args.len() != 1 {
                return Err(CompileError::new("`len` takes 1 argument", span));
            }
            self.compile_expr(&args[0])?;
            self.chunk().emit_op(Op::Len, line);
            return Ok(());
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
