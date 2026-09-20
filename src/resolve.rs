//! 语义检查：作用域、结构体、方法、循环、函数签名。
//! import 由 `loader` 合并进 Program 后再进入本模块（命名空间符号名为 `ns.sym`）。

use std::collections::HashMap;

use crate::ast::*;
use crate::token::Span;
use crate::types::Type;

#[derive(Debug)]
pub struct CheckError {
    pub message: String,
    pub span: Span,
}

#[derive(Clone)]
pub struct FunInfo {
    pub params: Vec<Type>,
    pub ret: Type,
}

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
    scopes: Vec<HashMap<String, Type>>,
    current_ret: Type,
    loop_depth: u32,
    at_top_level: bool,
}

impl<'a> Checker<'a> {
    pub fn new(program: &'a Program) -> Self {
        Self {
            program,
            functions: HashMap::new(),
            methods: HashMap::new(),
            structs: HashMap::new(),
            scopes: vec![HashMap::new()],
            current_ret: Type::Void,
            loop_depth: 0,
            at_top_level: true,
        }
    }

    pub fn check(mut self) -> Result<(), CheckError> {
        for s in self.program.structs() {
            self.declare_struct(s)?;
        }
        for f in self.program.functions() {
            self.declare_function(f)?;
        }
        for f in self.program.functions() {
            self.check_function(f)?;
        }
        self.at_top_level = true;
        self.current_ret = Type::Void;
        self.loop_depth = 0;
        self.scopes = vec![HashMap::new()];
        for stmt in self.program.top_level_stmts() {
            self.check_stmt(stmt)?;
        }
        Ok(())
    }

    fn ty(&self, t: &TypeExpr, span: Span) -> Result<Type, CheckError> {
        Type::from_ast(t, &self.structs).map_err(|m| CheckError { message: m, span })
    }

    fn declare_struct(&mut self, s: &StructDecl) -> Result<(), CheckError> {
        if BUILTINS.contains(&s.name.name.as_str()) {
            return Err(CheckError {
                message: format!("`{}` is already used", s.name.name),
                span: s.name.span,
            });
        }
        if self.structs.contains_key(&s.name.name) {
            return Err(CheckError {
                message: format!("struct `{}` is already defined", s.name.name),
                span: s.name.span,
            });
        }
        let mut fields = Vec::new();
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

    fn fun_info(&self, f: &FunDecl) -> Result<(Vec<Type>, Type), CheckError> {
        let mut params = Vec::new();
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
        Ok((params, ret))
    }

    fn declare_function(&mut self, f: &FunDecl) -> Result<(), CheckError> {
        if BUILTINS.contains(&f.name.name.as_str()) && f.on_type.is_none() {
            return Err(CheckError {
                message: format!("`{}` is a builtin and cannot be redefined", f.name.name),
                span: f.name.span,
            });
        }
        let (params, ret) = self.fun_info(f)?;

        if let Some(on) = &f.on_type {
            if !self.structs.contains_key(&on.name) {
                return Err(CheckError {
                    message: format!("unknown struct `{}`", on.name),
                    span: on.span,
                });
            }
            if f.params.is_empty() {
                return Err(CheckError {
                    message: "method requires a receiver as first parameter".into(),
                    span: f.name.span,
                });
            }
            let recv = Type::Struct(on.name.clone());
            if params[0] != recv {
                return Err(CheckError {
                    message: format!(
                        "method receiver must be `{}`, found `{}`",
                        on.name, params[0]
                    ),
                    span: f.params[0].name.span,
                });
            }
            if self.functions.contains_key(&f.name.name) {
                return Err(CheckError {
                    message: format!(
                        "method `{}` conflicts with function of the same name",
                        f.name.name
                    ),
                    span: f.name.span,
                });
            }
            let entry = self.methods.entry(on.name.clone()).or_default();
            if entry.contains_key(&f.name.name) {
                return Err(CheckError {
                    message: format!("method `{}` already defined on `{}`", f.name.name, on.name),
                    span: f.name.span,
                });
            }
            entry.insert(f.name.name.clone(), FunInfo { params, ret });
            return Ok(());
        }

        if f.name.name == "main" && !f.name.name.contains('.') {
            if !f.params.is_empty() {
                return Err(CheckError {
                    message: "`main` must take no parameters".into(),
                    span: f.name.span,
                });
            }
            if !matches!(f.ret, TypeExpr::Void) {
                return Err(CheckError {
                    message: "`main` must return `void`".into(),
                    span: f.name.span,
                });
            }
        }
        if self.functions.contains_key(&f.name.name) {
            return Err(CheckError {
                message: format!("function `{}` is already defined", f.name.name),
                span: f.name.span,
            });
        }
        // 方法名与全局函数冲突
        if self.methods.values().any(|m| m.contains_key(&f.name.name)) {
            return Err(CheckError {
                message: format!("function `{}` conflicts with a method", f.name.name),
                span: f.name.span,
            });
        }
        self.functions
            .insert(f.name.name.clone(), FunInfo { params, ret });
        Ok(())
    }

    fn check_function(&mut self, f: &FunDecl) -> Result<(), CheckError> {
        let (_, ret) = self.fun_info(f)?;
        self.current_ret = ret;
        self.at_top_level = false;
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
        if !matches!(self.current_ret, Type::Void) && !block_always_returns(&f.body) {
            return Err(CheckError {
                message: format!(
                    "function `{}` must return `{}` on all paths",
                    f.name.name, self.current_ret
                ),
                span: f.name.span,
            });
        }
        Ok(())
    }

    fn check_block(&mut self, b: &Block) -> Result<(), CheckError> {
        self.push_scope();
        let r = self.check_stmts(&b.stmts);
        self.pop_scope();
        r
    }

    fn check_stmts(&mut self, stmts: &[Stmt]) -> Result<(), CheckError> {
        for s in stmts {
            self.check_stmt(s)?;
        }
        Ok(())
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn lookup_var(&self, name: &str) -> Option<Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(t) = scope.get(name) {
                return Some(t.clone());
            }
        }
        None
    }

    fn declare_var(&mut self, name: &str, ty: Type, span: Span) -> Result<(), CheckError> {
        let scope = self.scopes.last_mut().unwrap();
        if scope.contains_key(name) {
            return Err(CheckError {
                message: format!("variable `{name}` is already declared in this scope"),
                span,
            });
        }
        scope.insert(name.to_string(), ty);
        Ok(())
    }

    fn field_type(&self, struct_ty: &str, field: &Ident) -> Result<Type, CheckError> {
        let fields = self.structs.get(struct_ty).ok_or_else(|| CheckError {
            message: format!("unknown struct `{struct_ty}`"),
            span: field.span,
        })?;
        fields
            .iter()
            .find(|(n, _)| n == &field.name)
            .map(|(_, t)| t.clone())
            .ok_or_else(|| CheckError {
                message: format!("struct `{struct_ty}` has no field `{}`", field.name),
                span: field.span,
            })
    }

    fn check_stmt(&mut self, stmt: &Stmt) -> Result<(), CheckError> {
        match stmt {
            Stmt::Let(l) => {
                let empty_array = matches!(&l.value, Expr::Array { elems, .. } if elems.is_empty());
                let val_ty = if empty_array {
                    if let Some(t) = &l.ty {
                        self.ty(t, l.name.span)?
                    } else {
                        return Err(CheckError {
                            message: "empty array literal `[]` requires a `let` type annotation"
                                .into(),
                            span: l.name.span,
                        });
                    }
                } else {
                    self.check_expr(&l.value)?
                };
                let declared = if let Some(t) = &l.ty {
                    let t = self.ty(t, l.name.span)?;
                    if empty_array {
                        if !matches!(t, Type::Array(_)) {
                            return Err(CheckError {
                                message: format!("`[]` requires an array type, got `{t}`"),
                                span: l.span,
                            });
                        }
                    } else if t != val_ty {
                        return Err(CheckError {
                            message: format!(
                                "let `{}` declared as `{t}` but initialized with `{val_ty}`",
                                l.name.name
                            ),
                            span: l.span,
                        });
                    }
                    t
                } else {
                    val_ty
                };
                self.declare_var(&l.name.name, declared, l.name.span)?;
            }
            Stmt::Assign(a) => {
                let mut cur = self.lookup_var(&a.name.name).ok_or_else(|| CheckError {
                    message: format!("undefined variable `{}`", a.name.name),
                    span: a.name.span,
                })?;
                let val_ty = self.check_expr(&a.value)?;
                if let Some(idx) = &a.index {
                    let idx_ty = self.check_expr(idx)?;
                    if idx_ty != Type::Int {
                        return Err(CheckError {
                            message: format!("array index must be `int`, found `{idx_ty}`"),
                            span: idx.span(),
                        });
                    }
                    let Type::Array(elem) = cur else {
                        return Err(CheckError {
                            message: format!("cannot index `{}`, not an array", a.name.name),
                            span: a.name.span,
                        });
                    };
                    cur = *elem;
                }
                for field in &a.fields {
                    let Type::Struct(sname) = &cur else {
                        return Err(CheckError {
                            message: format!("cannot access field `{}` on `{cur}`", field.name),
                            span: field.span,
                        });
                    };
                    cur = self.field_type(sname, field)?;
                }
                if val_ty != cur {
                    return Err(CheckError {
                        message: format!("cannot assign `{val_ty}` to `{cur}`"),
                        span: a.span,
                    });
                }
            }
            Stmt::If(i) => self.check_if(i)?,
            Stmt::While(w) => {
                let c = self.check_expr(&w.cond)?;
                if c != Type::Bool {
                    return Err(CheckError {
                        message: format!("while condition must be `bool`, found `{c}`"),
                        span: w.cond.span(),
                    });
                }
                self.loop_depth += 1;
                self.check_block(&w.body)?;
                self.loop_depth -= 1;
            }
            Stmt::For(f) => {
                let is_range = matches!(f.iter, Expr::Range { .. });
                if is_range {
                    if let Expr::Range { start, end, span } = &f.iter {
                        let s = self.check_expr(start)?;
                        let e = self.check_expr(end)?;
                        if s != Type::Int || e != Type::Int {
                            return Err(CheckError {
                                message: format!("range bounds must be `int`, found `{s}..{e}`"),
                                span: *span,
                            });
                        }
                    }
                    self.push_scope();
                    self.declare_var(&f.var.name, Type::Int, f.var.span)?;
                    self.loop_depth += 1;
                    let r = self.check_stmts(&f.body.stmts);
                    self.loop_depth -= 1;
                    self.pop_scope();
                    r?;
                } else {
                    let it = self.check_expr(&f.iter)?;
                    let elem = match it {
                        Type::Array(e) => *e,
                        other => {
                            return Err(CheckError {
                                message: format!("for-in expects an array, found `{other}`"),
                                span: f.iter.span(),
                            })
                        }
                    };
                    self.push_scope();
                    self.declare_var(&f.var.name, elem, f.var.span)?;
                    self.loop_depth += 1;
                    let r = self.check_stmts(&f.body.stmts);
                    self.loop_depth -= 1;
                    self.pop_scope();
                    r?;
                }
            }
            Stmt::Break(span) | Stmt::Continue(span) => {
                if self.loop_depth == 0 {
                    return Err(CheckError {
                        message: "`break`/`continue` only allowed inside a loop".into(),
                        span: *span,
                    });
                }
            }
            Stmt::Return(r) => {
                if self.at_top_level {
                    return Err(CheckError {
                        message: "`return` is only allowed inside a function".into(),
                        span: r.span,
                    });
                }
                match &r.value {
                    None => {
                        if self.current_ret != Type::Void {
                            return Err(CheckError {
                                message: format!(
                                    "this function must return `{}`",
                                    self.current_ret
                                ),
                                span: r.span,
                            });
                        }
                    }
                    Some(e) => {
                        let t = self.check_expr(e)?;
                        if self.current_ret == Type::Void {
                            return Err(CheckError {
                                message: "this function returns `void`".into(),
                                span: r.span,
                            });
                        }
                        if t != self.current_ret {
                            return Err(CheckError {
                                message: format!(
                                    "expected return type `{}`, found `{t}`",
                                    self.current_ret
                                ),
                                span: r.span,
                            });
                        }
                    }
                }
            }
            Stmt::Expr(e) => {
                self.check_expr(&e.expr)?;
            }
            Stmt::Block(b) => self.check_block(b)?,
        }
        Ok(())
    }

    fn check_if(&mut self, i: &IfStmt) -> Result<(), CheckError> {
        let c = self.check_expr(&i.cond)?;
        if c != Type::Bool {
            return Err(CheckError {
                message: format!("if condition must be `bool`, found `{c}`"),
                span: i.cond.span(),
            });
        }
        self.check_block(&i.then_block)?;
        match &i.else_branch {
            None => {}
            Some(ElseBranch::Block(b)) => self.check_block(b)?,
            Some(ElseBranch::If(nested)) => self.check_if(nested)?,
        }
        Ok(())
    }

    fn check_expr(&mut self, expr: &Expr) -> Result<Type, CheckError> {
        match expr {
            Expr::Int { .. } => Ok(Type::Int),
            Expr::Float { .. } => Ok(Type::Float),
            Expr::Bool { .. } => Ok(Type::Bool),
            Expr::Str { .. } => Ok(Type::Str),
            Expr::Var { name } => self.lookup_var(&name.name).ok_or_else(|| CheckError {
                message: format!("undefined variable `{}`", name.name),
                span: name.span,
            }),
            Expr::Range { start, end, span } => {
                let s = self.check_expr(start)?;
                let e = self.check_expr(end)?;
                if s != Type::Int || e != Type::Int {
                    return Err(CheckError {
                        message: format!("range bounds must be `int`, found `{s}..{e}`"),
                        span: *span,
                    });
                }
                Ok(Type::Array(Box::new(Type::Int)))
            }
            Expr::Unary { op, expr, span } => {
                let t = self.check_expr(expr)?;
                match op {
                    UnOp::Neg => {
                        if t.is_numeric() {
                            Ok(t)
                        } else {
                            Err(CheckError {
                                message: format!("cannot negate `{t}`"),
                                span: *span,
                            })
                        }
                    }
                    UnOp::Not => {
                        if t == Type::Bool {
                            Ok(Type::Bool)
                        } else {
                            Err(CheckError {
                                message: format!("`!` expects `bool`, found `{t}`"),
                                span: *span,
                            })
                        }
                    }
                }
            }
            Expr::Binary { op, lhs, rhs, span } => {
                let lt = self.check_expr(lhs)?;
                let rt = self.check_expr(rhs)?;
                match op {
                    BinOp::Add => {
                        if lt == Type::Str && rt == Type::Str {
                            return Ok(Type::Str);
                        }
                        if lt == rt && lt.is_numeric() {
                            return Ok(lt);
                        }
                        Err(CheckError {
                            message: format!("cannot add `{lt}` and `{rt}`"),
                            span: *span,
                        })
                    }
                    BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem => {
                        if lt == rt && lt.is_numeric() {
                            Ok(lt)
                        } else {
                            Err(CheckError {
                                message: format!("cannot apply arithmetic to `{lt}` and `{rt}`"),
                                span: *span,
                            })
                        }
                    }
                    BinOp::Eq | BinOp::Ne => {
                        if lt == rt
                            && matches!(lt, Type::Int | Type::Float | Type::Bool | Type::Str)
                        {
                            Ok(Type::Bool)
                        } else {
                            Err(CheckError {
                                message: format!("cannot compare `{lt}` with `{rt}`"),
                                span: *span,
                            })
                        }
                    }
                    BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                        if lt == rt && lt.is_numeric() {
                            Ok(Type::Bool)
                        } else {
                            Err(CheckError {
                                message: format!("cannot order `{lt}` and `{rt}`"),
                                span: *span,
                            })
                        }
                    }
                    BinOp::And | BinOp::Or => {
                        if lt == Type::Bool && rt == Type::Bool {
                            Ok(Type::Bool)
                        } else {
                            Err(CheckError {
                                message: format!(
                                    "logical operators expect `bool`, found `{lt}` and `{rt}`"
                                ),
                                span: *span,
                            })
                        }
                    }
                }
            }
            Expr::Call { callee, args, span } => self.check_call(&callee.name, args, *span),
            Expr::MethodCall {
                recv,
                method,
                args,
                span,
            } => {
                // 命名空间函数：ns.sym
                if let Expr::Var { name } = recv.as_ref() {
                    let dotted = format!("{}.{}", name.name, method.name);
                    if self.functions.contains_key(&dotted) {
                        return self.check_call(&dotted, args, *span);
                    }
                    // 静态调用 Type.method(recv, ...)
                    if self.structs.contains_key(&name.name) {
                        let minfo = self
                            .methods
                            .get(&name.name)
                            .and_then(|m| m.get(&method.name))
                            .cloned()
                            .ok_or_else(|| CheckError {
                                message: format!("no method `{}` on `{}`", method.name, name.name),
                                span: method.span,
                            })?;
                        return self.check_call_info(&method.name, args, &minfo, *span);
                    }
                }
                let rt = self.check_expr(recv)?;
                let Type::Struct(sname) = rt else {
                    return Err(CheckError {
                        message: format!("cannot call method on `{rt}`"),
                        span: *span,
                    });
                };
                let minfo = self
                    .methods
                    .get(&sname)
                    .and_then(|m| m.get(&method.name))
                    .cloned()
                    .ok_or_else(|| CheckError {
                        message: format!("no method `{}` on `{sname}`", method.name),
                        span: method.span,
                    })?;
                if args.len() + 1 != minfo.params.len() {
                    return Err(CheckError {
                        message: format!(
                            "method `{}` takes {} argument(s) after receiver, found {}",
                            method.name,
                            minfo.params.len() - 1,
                            args.len()
                        ),
                        span: *span,
                    });
                }
                for (i, (arg, pty)) in args.iter().zip(minfo.params[1..].iter()).enumerate() {
                    let at = self.check_expr(arg)?;
                    if &at != pty {
                        return Err(CheckError {
                            message: format!(
                                "argument {} of `{}`: expected `{pty}`, found `{at}`",
                                i + 1,
                                method.name
                            ),
                            span: arg.span(),
                        });
                    }
                }
                Ok(minfo.ret)
            }
            Expr::Index { base, index, span } => {
                let bt = self.check_expr(base)?;
                let it = self.check_expr(index)?;
                if it != Type::Int {
                    return Err(CheckError {
                        message: format!("array index must be `int`, found `{it}`"),
                        span: *span,
                    });
                }
                match bt {
                    Type::Array(elem) => Ok(*elem),
                    other => Err(CheckError {
                        message: format!("cannot index `{other}`, not an array"),
                        span: *span,
                    }),
                }
            }
            Expr::Array { elems, span } => {
                if elems.is_empty() {
                    return Err(CheckError {
                        message: "empty array literal `[]` requires a `let` type annotation".into(),
                        span: *span,
                    });
                }
                let first = self.check_expr(&elems[0])?;
                if matches!(first, Type::Void | Type::Array(_)) {
                    return Err(CheckError {
                        message: format!("cannot create `{first}[]`"),
                        span: *span,
                    });
                }
                for e in &elems[1..] {
                    let t = self.check_expr(e)?;
                    if t != first {
                        return Err(CheckError {
                            message: format!(
                                "array elements must be homogeneous: expected `{first}`, found `{t}`"
                            ),
                            span: e.span(),
                        });
                    }
                }
                Ok(Type::Array(Box::new(first)))
            }
            Expr::Field { base, name, span } => {
                let bt = self.check_expr(base)?;
                match &bt {
                    Type::Struct(sname) => self.field_type(sname, name),
                    other => Err(CheckError {
                        message: format!("cannot access field `{}` on `{other}`", name.name),
                        span: *span,
                    }),
                }
            }
            Expr::StructLit { name, fields, span } => {
                let sfields = self
                    .structs
                    .get(&name.name)
                    .cloned()
                    .ok_or_else(|| CheckError {
                        message: format!("unknown struct `{}`", name.name),
                        span: name.span,
                    })?;
                if fields.len() != sfields.len() {
                    return Err(CheckError {
                        message: format!(
                            "struct `{}` expects {} field(s), found {}",
                            name.name,
                            sfields.len(),
                            fields.len()
                        ),
                        span: *span,
                    });
                }
                let mut seen = std::collections::HashSet::new();
                for (fname, fexpr) in fields {
                    if !seen.insert(fname.name.clone()) {
                        return Err(CheckError {
                            message: format!("duplicate field `{}`", fname.name),
                            span: fname.span,
                        });
                    }
                    let expected = sfields
                        .iter()
                        .find(|(n, _)| n == &fname.name)
                        .map(|(_, t)| t.clone())
                        .ok_or_else(|| CheckError {
                            message: format!(
                                "struct `{}` has no field `{}`",
                                name.name, fname.name
                            ),
                            span: fname.span,
                        })?;
                    let got = self.check_expr(fexpr)?;
                    if got != expected {
                        return Err(CheckError {
                            message: format!(
                                "field `{}`: expected `{expected}`, found `{got}`",
                                fname.name
                            ),
                            span: fexpr.span(),
                        });
                    }
                }
                Ok(Type::Struct(name.name.clone()))
            }
        }
    }

    fn check_call_info(
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
        for (i, (arg, pty)) in args.iter().zip(info.params.iter()).enumerate() {
            let at = self.check_expr(arg)?;
            if &at != pty {
                return Err(CheckError {
                    message: format!(
                        "argument {} of `{name}`: expected `{pty}`, found `{at}`",
                        i + 1
                    ),
                    span: arg.span(),
                });
            }
        }
        Ok(info.ret.clone())
    }

    fn check_call(&mut self, name: &str, args: &[Expr], span: Span) -> Result<Type, CheckError> {
        match name {
            "print" => {
                if args.len() != 1 {
                    return Err(CheckError {
                        message: format!("`print` takes 1 argument, found {}", args.len()),
                        span,
                    });
                }
                let t = self.check_expr(&args[0])?;
                if !t.can_print() {
                    return Err(CheckError {
                        message: "cannot print `void`".into(),
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
                let t = self.check_expr(&args[0])?;
                if !matches!(t, Type::Array(_) | Type::Str) {
                    return Err(CheckError {
                        message: format!("`len` expects array or string, found `{t}`"),
                        span,
                    });
                }
                return Ok(Type::Int);
            }
            "str_at" => {
                if args.len() != 2 {
                    return Err(CheckError {
                        message: "`str_at` takes (string, int)".into(),
                        span,
                    });
                }
                self.expect_arg(&args[0], &Type::Str, "str_at", 1)?;
                self.expect_arg(&args[1], &Type::Int, "str_at", 2)?;
                return Ok(Type::Str);
            }
            "str_sub" => {
                if args.len() != 3 {
                    return Err(CheckError {
                        message: "`str_sub` takes (string, int, int)".into(),
                        span,
                    });
                }
                self.expect_arg(&args[0], &Type::Str, "str_sub", 1)?;
                self.expect_arg(&args[1], &Type::Int, "str_sub", 2)?;
                self.expect_arg(&args[2], &Type::Int, "str_sub", 3)?;
                return Ok(Type::Str);
            }
            "to_string" => {
                if args.len() != 1 {
                    return Err(CheckError {
                        message: "`to_string` takes 1 argument".into(),
                        span,
                    });
                }
                let _t = self.check_expr(&args[0])?;
                return Ok(Type::Str);
            }
            "push" => {
                if args.len() != 2 {
                    return Err(CheckError {
                        message: "`push` takes (array, value)".into(),
                        span,
                    });
                }
                let at = self.check_expr(&args[0])?;
                let Type::Array(elem) = at else {
                    return Err(CheckError {
                        message: format!("`push` expects an array, found `{at}`"),
                        span: args[0].span(),
                    });
                };
                let vt = self.check_expr(&args[1])?;
                if vt != *elem {
                    return Err(CheckError {
                        message: format!("`push` expects `{elem}`, found `{vt}`"),
                        span: args[1].span(),
                    });
                }
                return Ok(Type::Void);
            }
            "pop" => {
                if args.len() != 1 {
                    return Err(CheckError {
                        message: "`pop` takes 1 array".into(),
                        span,
                    });
                }
                let at = self.check_expr(&args[0])?;
                match at {
                    Type::Array(elem) => return Ok(*elem),
                    other => {
                        return Err(CheckError {
                            message: format!("`pop` expects an array, found `{other}`"),
                            span: args[0].span(),
                        })
                    }
                }
            }
            "input" => {
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
        for (i, (arg, pty)) in args.iter().zip(info.params.iter()).enumerate() {
            let at = self.check_expr(arg)?;
            if &at != pty {
                return Err(CheckError {
                    message: format!(
                        "argument {} of `{name}`: expected `{pty}`, found `{at}`",
                        i + 1
                    ),
                    span: arg.span(),
                });
            }
        }
        Ok(info.ret)
    }

    fn expect_arg(
        &mut self,
        arg: &Expr,
        expected: &Type,
        fname: &str,
        idx: usize,
    ) -> Result<(), CheckError> {
        let at = self.check_expr(arg)?;
        if &at != expected {
            return Err(CheckError {
                message: format!(
                    "argument {idx} of `{fname}`: expected `{expected}`, found `{at}`"
                ),
                span: arg.span(),
            });
        }
        Ok(())
    }
}

fn block_always_returns(b: &Block) -> bool {
    stmts_always_return(&b.stmts)
}

fn stmts_always_return(stmts: &[Stmt]) -> bool {
    for s in stmts {
        if stmt_always_returns(s) {
            return true;
        }
    }
    false
}

fn stmt_always_returns(s: &Stmt) -> bool {
    match s {
        Stmt::Return(_) => true,
        Stmt::Block(b) => block_always_returns(b),
        Stmt::If(i) => if_always_returns(i),
        _ => false,
    }
}

fn if_always_returns(i: &IfStmt) -> bool {
    let then = block_always_returns(&i.then_block);
    if !then {
        return false;
    }
    match &i.else_branch {
        Some(ElseBranch::Block(b)) => block_always_returns(b),
        Some(ElseBranch::If(n)) => if_always_returns(n),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn check(src: &str) -> Result<(), CheckError> {
        let toks = Lexer::new(src).tokenize().expect("lex");
        let prog = Parser::new(toks).parse_program().expect("parse");
        Checker::new(&prog).check()
    }

    #[test]
    fn accepts_methods_and_range_for() {
        let src = r#"
        struct Point { x: int, y: int }
        fun Point.sum(self: Point) -> int { return self.x + self.y; }
        fun main() -> void {
            let p = Point { x: 1, y: 2 };
            print(p.sum());
            for i in 0..3 {
                if i == 1 { continue; }
                print(i);
            }
        }
        "#;
        assert!(check(src).is_ok(), "{:?}", check(src));
    }

    #[test]
    fn rejects_break_outside_loop() {
        assert!(check("fun main() -> void { break; }").is_err());
    }

    #[test]
    fn rejects_range_non_int() {
        assert!(check("fun main() -> void { for i in 0..1.5 { print(i); } }").is_err());
    }
}
