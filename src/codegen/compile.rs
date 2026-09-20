//! AST → 字节码。

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::codegen::chunk::{Chunk, Function, Module, StructType};
use crate::codegen::op::Op;
use crate::runtime::value::Value;
use crate::syntax::ast::*;
use crate::syntax::token::Span;

#[derive(Debug)]
pub struct CompileError {
    pub message: String,
    pub line: u32,
    pub col: u32,
}

impl CompileError {
    fn at(msg: impl Into<String>, span: Span) -> Self {
        Self {
            message: msg.into(),
            line: span.line,
            col: span.col,
        }
    }
}

impl From<String> for CompileError {
    fn from(message: String) -> Self {
        Self {
            message,
            line: 0,
            col: 0,
        }
    }
}

struct Local {
    name: String,
    depth: i32,
}

struct FnC {
    name: String,
    arity: u8,
    chunk: Chunk,
    locals: Vec<Local>,
    depth: i32,
}

impl FnC {
    fn new(name: String, arity: u8) -> Self {
        Self {
            name,
            arity,
            chunk: Chunk::new(),
            locals: vec![],
            depth: 0,
        }
    }
    fn add_local(&mut self, n: String) {
        let d = self.depth;
        self.locals.push(Local { name: n, depth: d });
    }
    fn slot(&self, n: &str) -> Option<u16> {
        self.locals
            .iter()
            .rev()
            .position(|l| l.name == n)
            .map(|i| (self.locals.len() - 1 - i) as u16)
    }
    fn begin(&mut self) {
        self.depth += 1;
    }
    fn end(&mut self, line: u32) {
        self.depth -= 1;
        while let Some(l) = self.locals.last() {
            if l.depth > self.depth {
                self.locals.pop();
                self.chunk.emit(Op::Pop, line);
            } else {
                break;
            }
        }
    }
}

struct LoopP {
    breaks: Vec<usize>,
    continues: Vec<usize>,
}

pub struct Compiler {
    fidx: HashMap<String, usize>,
    fns: Vec<FnC>,
    voids: HashSet<String>,
    sidx: HashMap<String, usize>,
    stypes: Vec<StructType>,
    consts: HashMap<String, Value>,
    loops: Vec<LoopP>,
    cur: usize,
}

impl Compiler {
    pub fn compile(
        program: &Program,
        consts: HashMap<String, Value>,
    ) -> Result<Module, CompileError> {
        let mut sidx = HashMap::new();
        let mut stypes = vec![];
        for s in program.structs() {
            sidx.insert(s.name.name.clone(), stypes.len());
            stypes.push(StructType {
                name: s.name.name.clone(),
                fields: s.fields.iter().map(|f| f.name.name.clone()).collect(),
            });
        }

        let mut fidx = HashMap::new();
        let mut voids = HashSet::new();
        let mut decls: Vec<&FunDecl> = program.functions().collect();
        decls.sort_by_key(|f| f.span.line);
        for (i, f) in decls.iter().enumerate() {
            let key = match &f.on_type {
                Some(t) => format!("{}.{}", t.name, f.name.name),
                None => f.name.name.clone(),
            };
            fidx.insert(key.clone(), i);
            if f.ret.is_void() {
                voids.insert(key);
            }
        }
        let tl = decls.len();
        fidx.insert("$toplevel".into(), tl);
        voids.insert("$toplevel".into());
        voids.insert("print".into());
        voids.insert("push".into());

        let mut fns = vec![];
        for f in &decls {
            let name = match &f.on_type {
                Some(t) => format!("{}.{}", t.name, f.name.name),
                None => f.name.name.clone(),
            };
            let mut c = FnC::new(name, f.params.len() as u8);
            for p in &f.params {
                c.add_local(p.name.name.clone());
            }
            fns.push(c);
        }
        fns.push(FnC::new("$toplevel".into(), 0));

        let mut cx = Compiler {
            fidx,
            fns,
            voids,
            sidx,
            stypes,
            consts,
            loops: vec![],
            cur: 0,
        };

        for (i, f) in decls.iter().enumerate() {
            cx.cur = i;
            for st in &f.body.stmts {
                cx.stmt(st)?;
            }
            cx.chunk().emit(Op::Return, f.body.span.line);
        }
        cx.cur = tl;
        for st in program.top_level_stmts() {
            cx.stmt(st)?;
        }
        cx.chunk().emit(Op::Return, 0);

        let voids = cx.voids.clone();
        let stypes = cx.stypes.clone();
        let mut functions = vec![];
        let mut main_index = None;
        for fc in cx.fns {
            if fc.name == "main" {
                main_index = Some(functions.len());
            }
            functions.push(Function {
                name: fc.name.clone(),
                arity: fc.arity,
                locals: fc.locals.len() as u16,
                is_void: fc.name == "$toplevel" || voids.contains(&fc.name),
                chunk: fc.chunk,
            });
        }
        Ok(Module {
            functions,
            main_index,
            toplevel_index: tl,
            struct_types: stypes,
        })
    }

    fn chunk(&mut self) -> &mut Chunk {
        &mut self.fns[self.cur].chunk
    }

    fn f(&mut self) -> &mut FnC {
        &mut self.fns[self.cur]
    }

    fn void_call(&self, n: &str) -> bool {
        self.voids.contains(n)
    }

    fn block(&mut self, b: &Block) -> Result<(), CompileError> {
        self.f().begin();
        for s in &b.stmts {
            self.stmt(s)?;
        }
        self.f().end(b.span.line);
        Ok(())
    }

    fn stmt(&mut self, s: &Stmt) -> Result<(), CompileError> {
        match s {
            Stmt::Let(l) => {
                self.expr(&l.value)?;
                self.f().add_local(l.name.name.clone());
                Ok(())
            }
            Stmt::Assign(a) => self.assign(a),
            Stmt::If(i) => self.if_stmt(i),
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
            Stmt::Break(sp) => {
                if self.loops.is_empty() {
                    return Err(CompileError::at("`break` outside loop", *sp));
                }
                let j = self.chunk().emit_jump(Op::Jump, sp.line);
                self.loops.last_mut().unwrap().breaks.push(j);
                Ok(())
            }
            Stmt::Continue(sp) => {
                if self.loops.is_empty() {
                    return Err(CompileError::at("`continue` outside loop", *sp));
                }
                let j = self.chunk().emit_jump(Op::Jump, sp.line);
                self.loops.last_mut().unwrap().continues.push(j);
                Ok(())
            }
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

    fn assign(&mut self, a: &AssignStmt) -> Result<(), CompileError> {
        let line = a.span.line;
        let slot = self
            .f()
            .slot(&a.name.name)
            .ok_or_else(|| CompileError::at(format!("undefined `{}`", a.name.name), a.name.span))?;

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

    fn if_stmt(&mut self, i: &IfStmt) -> Result<(), CompileError> {
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

    fn for_stmt(&mut self, f: &ForStmt) -> Result<(), CompileError> {
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

    fn expr(&mut self, e: &Expr) -> Result<(), CompileError> {
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

    fn bin(&mut self, op: BinOp, lhs: &Expr, rhs: &Expr, span: Span) -> Result<(), CompileError> {
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

    fn call(&mut self, name: &str, args: &[Expr], span: Span) -> Result<(), CompileError> {
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
