//! 类型检查与 `const` 常量折叠。
//!
//! **在做什么**：程序能解析成功，不代表语义正确。这里回答：
//! - 变量有没有定义？类型对不对？
//! - `1 + 2.5` 这种 int/float 混用要不要报错？（要）
//! - 函数实参个数、返回类型是否匹配？
//! - `break` 是否写在循环外？（要报错）
//! - `const M = N * 2 + 1` 在编译期能否算出结果？（能，就折叠）
//!
//! **为什么需要**：把错误尽量提前到「运行前」，避免 VM 跑到一半才崩。
//!
//! **输出**：检查通过后返回 const 表；错误里带 `Span`（行、列）。

use std::collections::HashMap;
use std::rc::Rc;

use crate::runtime::value::Value;
use crate::sema::types::Type;
use crate::syntax::ast::*;
use crate::syntax::token::Span;

/// 语义错误：诊断信息 + 源位置。
#[derive(Debug)]
pub struct CheckError {
    pub message: String,
    pub span: Span,
}

/// 函数/方法签名（参数类型列表 + 返回类型）。
#[derive(Clone)]
pub struct FunInfo {
    pub params: Vec<Type>,
    pub ret: Type,
}

/// 内建函数名：不可被用户重定义。
const BUILTINS: &[&str] = &[
    "print",
    "len",
    "str_at",
    "str_sub",
    "to_string",
    "push",
    "pop",
    "input",
];

pub struct Checker<'a> {
    program: &'a Program,
    functions: HashMap<String, FunInfo>,
    methods: HashMap<String, HashMap<String, FunInfo>>,
    structs: HashMap<String, Vec<(String, Type)>>,
    consts: HashMap<String, (Type, Value)>,
    scopes: Vec<HashMap<String, Type>>,
    ret: Type,
    loop_depth: u32,
    top_level: bool,
}

impl<'a> Checker<'a> {
    pub fn new(program: &'a Program) -> Self {
        Self {
            program,
            functions: HashMap::new(),
            methods: HashMap::new(),
            structs: HashMap::new(),
            consts: HashMap::new(),
            scopes: vec![HashMap::new()],
            ret: Type::Void,
            loop_depth: 0,
            top_level: true,
        }
    }

    /// 检查入口：先登记 struct/函数/const，再检查函数体与顶层语句。
    /// 返回折叠后的 const 表，供编译器 `emit_const`。
    pub fn check(mut self) -> Result<HashMap<String, Value>, CheckError> {
        for s in self.program.structs() {
            self.declare_struct(s)?;
        }
        for f in self.program.functions() {
            self.declare_fun(f)?;
        }
        for c in self.program.consts() {
            self.declare_const(c)?;
        }
        for f in self.program.functions() {
            self.check_fun(f)?;
        }
        self.top_level = true;
        self.ret = Type::Void;
        self.loop_depth = 0;
        self.scopes = vec![HashMap::new()];
        for st in self.program.top_level_stmts() {
            self.check_stmt(st)?;
        }
        Ok(self.consts.into_iter().map(|(k, (_, v))| (k, v)).collect())
    }

    fn ty(&self, t: &TypeExpr, span: Span) -> Result<Type, CheckError> {
        Type::from_ast(t, &self.structs).map_err(|m| CheckError { message: m, span })
    }

    fn declare_struct(&mut self, s: &StructDecl) -> Result<(), CheckError> {
        if self.structs.contains_key(&s.name.name) {
            return Err(CheckError {
                message: format!("struct `{}` already defined", s.name.name),
                span: s.name.span,
            });
        }
        let mut fields = vec![];
        let mut seen = HashMap::new();
        for f in &s.fields {
            let ty = self.ty(&f.ty, f.name.span)?;
            if seen.insert(f.name.name.clone(), ()).is_some() {
                return Err(CheckError {
                    message: format!("duplicate field `{}`", f.name.name),
                    span: f.name.span,
                });
            }
            fields.push((f.name.name.clone(), ty));
        }
        self.structs.insert(s.name.name.clone(), fields);
        Ok(())
    }

    fn declare_const(&mut self, c: &ConstDecl) -> Result<(), CheckError> {
        if BUILTINS.contains(&c.name.name.as_str())
            || self.structs.contains_key(&c.name.name)
            || self.functions.contains_key(&c.name.name)
            || self.consts.contains_key(&c.name.name)
        {
            return Err(CheckError {
                message: format!("`{}` already defined", c.name.name),
                span: c.name.span,
            });
        }
        let declared = self.ty(&c.ty, c.name.span)?;
        if !matches!(declared, Type::Int | Type::Float | Type::Bool | Type::Str) {
            return Err(CheckError {
                message: format!("const type must be a scalar, found `{declared}`"),
                span: c.span,
            });
        }
        let (vt, val) = self.fold(&c.value)?;
        if vt != declared {
            return Err(CheckError {
                message: format!(
                    "const `{}` declared `{declared}` but value is `{vt}`",
                    c.name.name
                ),
                span: c.span,
            });
        }
        self.consts.insert(c.name.name.clone(), (declared, val));
        Ok(())
    }

    /// 折叠常量表达式。
    ///
    /// 例：`const M: int = 10 * 2 + 1;`
    /// 1. 折叠 `10` → Int(10)
    /// 2. 折叠 `2` → Int(2)
    /// 3. `10 * 2` → Int(20)
    /// 4. `20 + 1` → Int(21)
    ///
    /// 一旦遇到「编译期算不出来」的东西（如调用函数），就报错。
    fn fold(&self, e: &Expr) -> Result<(Type, Value), CheckError> {
        match e {
            Expr::Int { value, .. } => Ok((Type::Int, Value::Int(*value))),
            Expr::Float { value, .. } => Ok((Type::Float, Value::Float(*value))),
            Expr::Bool { value, .. } => Ok((Type::Bool, Value::Bool(*value))),
            Expr::Str { value, .. } => Ok((Type::Str, Value::Str(Rc::from(value.as_str())))),
            Expr::Var { name } => self
                .consts
                .get(&name.name)
                .cloned()
                .ok_or_else(|| CheckError {
                    message: format!("`{}` is not a compile-time constant", name.name),
                    span: name.span,
                }),
            Expr::Unary { op, expr, span } => {
                let (t, v) = self.fold(expr)?;
                match (op, t, v) {
                    (UnOp::Neg, Type::Int, Value::Int(n)) => Ok((Type::Int, Value::Int(-n))),
                    (UnOp::Neg, Type::Float, Value::Float(n)) => {
                        Ok((Type::Float, Value::Float(-n)))
                    }
                    (UnOp::Not, Type::Bool, Value::Bool(b)) => Ok((Type::Bool, Value::Bool(!b))),
                    _ => Err(CheckError {
                        message: "invalid unary in const".into(),
                        span: *span,
                    }),
                }
            }
            Expr::Binary { op, lhs, rhs, span } => {
                let (lt, lv) = self.fold(lhs)?;
                let (rt, rv) = self.fold(rhs)?;
                fold_bin(*op, lt, lv, rt, rv).map_err(|m| CheckError {
                    message: m,
                    span: *span,
                })
            }
            other => Err(CheckError {
                message: "not a compile-time constant expression".into(),
                span: other.span(),
            }),
        }
    }

    fn declare_fun(&mut self, f: &FunDecl) -> Result<(), CheckError> {
        if f.on_type.is_none() && BUILTINS.contains(&f.name.name.as_str()) {
            return Err(CheckError {
                message: format!("`{}` is a builtin", f.name.name),
                span: f.name.span,
            });
        }
        let mut params = vec![];
        let mut seen = HashMap::new();
        for p in &f.params {
            let ty = self.ty(&p.ty, p.name.span)?;
            if seen.insert(p.name.name.clone(), ()).is_some() {
                return Err(CheckError {
                    message: format!("duplicate parameter `{}`", p.name.name),
                    span: p.name.span,
                });
            }
            params.push(ty);
        }
        let ret = self.ty(&f.ret, f.span)?;

        if let Some(on) = &f.on_type {
            if !self.structs.contains_key(&on.name) {
                return Err(CheckError {
                    message: format!("unknown struct `{}`", on.name),
                    span: on.span,
                });
            }
            if f.params.is_empty() || params[0] != Type::Struct(on.name.clone()) {
                return Err(CheckError {
                    message: format!("method receiver must be `{}`", on.name),
                    span: f.name.span,
                });
            }
            if self.functions.contains_key(&f.name.name) {
                return Err(CheckError {
                    message: format!("method `{}` conflicts with a function", f.name.name),
                    span: f.name.span,
                });
            }
            let e = self.methods.entry(on.name.clone()).or_default();
            if e.contains_key(&f.name.name) {
                return Err(CheckError {
                    message: format!("method `{}` already defined", f.name.name),
                    span: f.name.span,
                });
            }
            e.insert(f.name.name.clone(), FunInfo { params, ret });
            return Ok(());
        }

        if f.name.name == "main" && (!f.params.is_empty() || !matches!(f.ret, TypeExpr::Void)) {
            return Err(CheckError {
                message: "`main` must be `fun main() -> void`".into(),
                span: f.name.span,
            });
        }
        if self.functions.contains_key(&f.name.name)
            || self.methods.values().any(|m| m.contains_key(&f.name.name))
        {
            return Err(CheckError {
                message: format!("function `{}` already defined", f.name.name),
                span: f.name.span,
            });
        }
        self.functions
            .insert(f.name.name.clone(), FunInfo { params, ret });
        Ok(())
    }

    fn check_fun(&mut self, f: &FunDecl) -> Result<(), CheckError> {
        let (_, ret) = {
            let mut params = vec![];
            for p in &f.params {
                params.push(self.ty(&p.ty, p.name.span)?);
            }
            (params, self.ty(&f.ret, f.span)?)
        };
        self.ret = ret;
        self.top_level = false;
        self.loop_depth = 0;
        self.scopes = vec![HashMap::new()];
        for p in &f.params {
            let ty = self.ty(&p.ty, p.name.span)?;
            self.scopes
                .last_mut()
                .unwrap()
                .insert(p.name.name.clone(), ty);
        }
        self.check_block(&f.body)?;
        if !matches!(self.ret, Type::Void) && !always_returns(&f.body) {
            return Err(CheckError {
                message: format!(
                    "function `{}` must return `{}` on all paths",
                    f.name.name, self.ret
                ),
                span: f.name.span,
            });
        }
        Ok(())
    }

    fn check_block(&mut self, b: &Block) -> Result<(), CheckError> {
        self.scopes.push(HashMap::new());
        let r = (|| {
            for s in &b.stmts {
                self.check_stmt(s)?;
            }
            Ok(())
        })();
        self.scopes.pop();
        r
    }

    fn lookup(&self, n: &str) -> Option<Type> {
        self.scopes.iter().rev().find_map(|s| s.get(n).cloned())
    }

    fn declare_var(&mut self, n: &str, t: Type, span: Span) -> Result<(), CheckError> {
        let sc = self.scopes.last_mut().unwrap();
        if sc.contains_key(n) {
            return Err(CheckError {
                message: format!("variable `{n}` already declared in this scope"),
                span,
            });
        }
        sc.insert(n.to_string(), t);
        Ok(())
    }

    fn field_ty(&self, st: &str, field: &Ident) -> Result<Type, CheckError> {
        self.structs
            .get(st)
            .and_then(|fs| {
                fs.iter()
                    .find(|(n, _)| n == &field.name)
                    .map(|(_, t)| t.clone())
            })
            .ok_or_else(|| CheckError {
                message: format!("struct `{st}` has no field `{}`", field.name),
                span: field.span,
            })
    }

    fn check_stmt(&mut self, s: &Stmt) -> Result<(), CheckError> {
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

    fn check_if(&mut self, i: &IfStmt) -> Result<(), CheckError> {
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

    /// 求表达式类型；方法调用区分「命名空间函数 / 静态 Type.m / 实例 m()」。
    fn expr_ty(&mut self, e: &Expr) -> Result<Type, CheckError> {
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

    fn call(&mut self, name: &str, args: &[Expr], span: Span) -> Result<Type, CheckError> {
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

    fn call_info(
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

    fn arity(&self, n: &str, args: &[Expr], want: usize, span: Span) -> Result<(), CheckError> {
        if args.len() != want {
            return Err(CheckError {
                message: format!("`{n}` takes {want} argument(s)"),
                span,
            });
        }
        Ok(())
    }

    fn arg_is(&mut self, a: &Expr, want: &Type, f: &str, span: Span) -> Result<(), CheckError> {
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

fn fold_bin(op: BinOp, lt: Type, lv: Value, rt: Type, rv: Value) -> Result<(Type, Value), String> {
    use BinOp::*;
    match (op, &lv, &rv) {
        (Add, Value::Int(a), Value::Int(b)) => Ok((Type::Int, Value::Int(a.wrapping_add(*b)))),
        (Sub, Value::Int(a), Value::Int(b)) => Ok((Type::Int, Value::Int(a.wrapping_sub(*b)))),
        (Mul, Value::Int(a), Value::Int(b)) => Ok((Type::Int, Value::Int(a.wrapping_mul(*b)))),
        (Div, Value::Int(a), Value::Int(b)) if *b != 0 => {
            Ok((Type::Int, Value::Int(a.wrapping_div(*b))))
        }
        (Rem, Value::Int(a), Value::Int(b)) if *b != 0 => {
            Ok((Type::Int, Value::Int(a.wrapping_rem(*b))))
        }
        (Div | Rem, Value::Int(_), Value::Int(0)) => Err("division by zero in const".into()),
        (Add, Value::Float(a), Value::Float(b)) => Ok((Type::Float, Value::Float(a + b))),
        (Sub, Value::Float(a), Value::Float(b)) => Ok((Type::Float, Value::Float(a - b))),
        (Mul, Value::Float(a), Value::Float(b)) => Ok((Type::Float, Value::Float(a * b))),
        (Div, Value::Float(a), Value::Float(b)) if *b != 0.0 => {
            Ok((Type::Float, Value::Float(a / b)))
        }
        (Add, Value::Str(a), Value::Str(b)) => {
            let mut s = a.to_string();
            s.push_str(b);
            Ok((Type::Str, Value::Str(Rc::from(s.as_str()))))
        }
        (Eq, Value::Int(a), Value::Int(b)) => Ok((Type::Bool, Value::Bool(a == b))),
        (Ne, Value::Int(a), Value::Int(b)) => Ok((Type::Bool, Value::Bool(a != b))),
        (Lt, Value::Int(a), Value::Int(b)) => Ok((Type::Bool, Value::Bool(a < b))),
        (Le, Value::Int(a), Value::Int(b)) => Ok((Type::Bool, Value::Bool(a <= b))),
        (Gt, Value::Int(a), Value::Int(b)) => Ok((Type::Bool, Value::Bool(a > b))),
        (Ge, Value::Int(a), Value::Int(b)) => Ok((Type::Bool, Value::Bool(a >= b))),
        (Eq, Value::Bool(a), Value::Bool(b)) => Ok((Type::Bool, Value::Bool(a == b))),
        (Ne, Value::Bool(a), Value::Bool(b)) => Ok((Type::Bool, Value::Bool(a != b))),
        (And, Value::Bool(a), Value::Bool(b)) => Ok((Type::Bool, Value::Bool(*a && *b))),
        (Or, Value::Bool(a), Value::Bool(b)) => Ok((Type::Bool, Value::Bool(*a || *b))),
        _ => Err(format!("cannot fold `{lt}` {op:?} `{rt}`")),
    }
}

fn always_returns(b: &Block) -> bool {
    b.stmts.iter().any(stmt_returns)
}

fn stmt_returns(s: &Stmt) -> bool {
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
