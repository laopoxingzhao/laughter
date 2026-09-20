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
    /// 语义检查总入口。步骤：
    /// 1. 登记 struct / fun / const（const 会折叠）
    /// 2. 检查每个函数体
    /// 3. 检查顶层语句（有 main 时顶层不执行但仍须合法）
    /// 4. 返回折叠后的 const 表
    pub fn check(mut self) -> Result<HashMap<String, Value>, CheckError> {
        // 步骤1：登记结构体（供字段/类型标注解析）
        for s in self.program.structs() {
            self.declare_struct(s)?;
        }
        // 步骤2：登记函数与方法签名
        for f in self.program.functions() {
            self.declare_fun(f)?;
        }
        // 步骤3：登记并折叠 const（此时已知结构体/函数名，可查重）
        for c in self.program.consts() {
            self.declare_const(c)?;
        }
        // 步骤4：逐个检查函数体
        for f in self.program.functions() {
            self.check_fun(f)?;
        }
        self.top_level = true;
        self.ret = Type::Void;
        self.loop_depth = 0;
        self.scopes = vec![HashMap::new()];
        // 步骤5：检查顶层语句（有 main 时运行期不执行，但仍须类型合法）
        for st in self.program.top_level_stmts() {
            self.check_stmt(st)?;
        }
        // 步骤6：返回「名字 → 折叠后的值」，供 codegen 发 Const
        Ok(self.consts.into_iter().map(|(k, (_, v))| (k, v)).collect())
    }

    fn ty(&self, t: &TypeExpr, span: Span) -> Result<Type, CheckError> {
        Type::from_ast(t, &self.structs).map_err(|m| CheckError { message: m, span })
    }

    /// 登记结构体：查重 → 逐字段解析类型 → 查重字段名 → 写入 structs 表。
    fn declare_struct(&mut self, s: &StructDecl) -> Result<(), CheckError> {
        if self.structs.contains_key(&s.name.name) {
            return Err(CheckError {
                message: format!("结构体 `{}` 重复定义", s.name.name),
                span: s.name.span,
            });
        }
        let mut fields = vec![];
        let mut seen = HashMap::new();
        for f in &s.fields {
            let ty = self.ty(&f.ty, f.name.span)?;
            if seen.insert(f.name.name.clone(), ()).is_some() {
                return Err(CheckError {
                    message: format!("字段 `{}` 重复", f.name.name),
                    span: f.name.span,
                });
            }
            fields.push((f.name.name.clone(), ty));
        }
        self.structs.insert(s.name.name.clone(), fields);
        Ok(())
    }

    /// 登记 const：查重 → 解析标注类型 → 折叠右侧表达式 → 核对类型一致。
    fn declare_const(&mut self, c: &ConstDecl) -> Result<(), CheckError> {
        if BUILTINS.contains(&c.name.name.as_str())
            || self.structs.contains_key(&c.name.name)
            || self.functions.contains_key(&c.name.name)
            || self.consts.contains_key(&c.name.name)
        {
            return Err(CheckError {
                message: format!("`{}` 已定义", c.name.name),
                span: c.name.span,
            });
        }
        let declared = self.ty(&c.ty, c.name.span)?;
        if !matches!(declared, Type::Int | Type::Float | Type::Bool | Type::Str) {
            return Err(CheckError {
                message: format!("const 类型必须是标量，实际是 `{declared}`"),
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
    /// 登记函数/方法：解析参数与返回类型；方法校验接收者类型。
    fn declare_fun(&mut self, f: &FunDecl) -> Result<(), CheckError> {
        if f.on_type.is_none() && BUILTINS.contains(&f.name.name.as_str()) {
            return Err(CheckError {
                message: format!("`{}` 是内建函数，不可重定义", f.name.name),
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
                    message: format!("未知结构体 `{}`", on.name),
                    span: on.span,
                });
            }
            if f.params.is_empty() || params[0] != Type::Struct(on.name.clone()) {
                return Err(CheckError {
                    message: format!("方法接收者类型必须是 `{}`", on.name),
                    span: f.name.span,
                });
            }
            if self.functions.contains_key(&f.name.name) {
                return Err(CheckError {
                    message: format!("方法 `{}` 与同名函数冲突", f.name.name),
                    span: f.name.span,
                });
            }
            let e = self.methods.entry(on.name.clone()).or_default();
            if e.contains_key(&f.name.name) {
                return Err(CheckError {
                    message: format!("方法 `{}` 已定义", f.name.name),
                    span: f.name.span,
                });
            }
            e.insert(f.name.name.clone(), FunInfo { params, ret });
            return Ok(());
        }

        if f.name.name == "main" && (!f.params.is_empty() || !matches!(f.ret, TypeExpr::Void)) {
            return Err(CheckError {
                message: "`main` 必须是 `fun main() -> void`".into(),
                span: f.name.span,
            });
        }
        if self.functions.contains_key(&f.name.name)
            || self.methods.values().any(|m| m.contains_key(&f.name.name))
        {
            return Err(CheckError {
                message: format!("函数 `{}` 重复定义", f.name.name),
                span: f.name.span,
            });
        }
        self.functions
            .insert(f.name.name.clone(), FunInfo { params, ret });
        Ok(())
    }

    /// 检查函数体：新作用域 + 参数槽 → 检查语句 → 非 void 须所有路径 return。
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

    /// 从内向外查变量类型（作用域栈顶优先）。
    fn lookup(&self, n: &str) -> Option<Type> {
        self.scopes.iter().rev().find_map(|s| s.get(n).cloned())
    }

    fn declare_var(&mut self, n: &str, t: Type, span: Span) -> Result<(), CheckError> {
        let sc = self.scopes.last_mut().unwrap();
        if sc.contains_key(n) {
            return Err(CheckError {
                message: format!("变量 `{n}` 在本作用域已声明"),
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
                message: format!("结构体 `{st}` 没有字段 `{}`", field.name),
                span: field.span,
            })
    }
}
