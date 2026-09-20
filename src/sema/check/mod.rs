//! 类型检查与 const 折叠（模块入口）。
//!
//! 子模块：`fold`（常量折叠）、`stmt`（语句）、`expr`（表达式与调用）。

use std::collections::HashMap;
use std::rc::Rc;

use crate::runtime::value::Value;
use crate::sema::types::Type;
use crate::syntax::ast::*;
use crate::syntax::token::Span;

mod expr;
mod fold;
mod stmt;

use stmt::always_returns;

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
}
