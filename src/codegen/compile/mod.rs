//! AST → 字节码（模块入口：Compiler 结构与总控）。
//!
//! 子模块：`stmt`（语句/循环/赋值）、`expr`（表达式与调用）。

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::codegen::chunk::{Chunk, Function, Module, StructType};
use crate::codegen::op::Op;
use crate::runtime::value::Value;
use crate::syntax::ast::*;
use crate::syntax::token::Span;

mod expr;
mod stmt;

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

/// 一个函数的编译上下文：字节码缓冲 + 局部名→槽位 + 作用域深度。
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

/// 循环上下文：收集 break/continue 的 Jump 偏移，供循环出口/增量处回填。
struct LoopP {
    breaks: Vec<usize>,
    continues: Vec<usize>,
}

/// 编译器：先给每个函数（含 `$toplevel`）建索引，再逐个编译函数体。
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
    /// 编译整个程序。`consts` 来自 sema 折叠；输出 `Module` 供 VM 执行/CLI 反汇编。
    /// 编译整个程序。步骤：
    /// 1. 登记结构体布局与函数索引（方法名 `Type.name`）
    /// 2. 为每个函数建 FnC（参数已占槽）
    /// 3. 依次编译函数体，末尾补 Return
    /// 4. 编译顶层语句（合成 $toplevel）
    /// 5. 汇总为 Module
    pub fn compile(
        program: &Program,
        consts: HashMap<String, Value>,
    ) -> Result<Module, CompileError> {
        // 步骤1：扫描所有 struct 声明，建立「类型名 → 字段名列表」
        //         NewStruct / 字段布局会用到这张表
        let mut sidx = HashMap::new();
        let mut stypes = vec![];
        for s in program.structs() {
            sidx.insert(s.name.name.clone(), stypes.len());
            stypes.push(StructType {
                name: s.name.name.clone(),
                fields: s.fields.iter().map(|f| f.name.name.clone()).collect(),
            });
        }

        // 步骤2：给每个函数编号。方法的键名是 `Type.method`
        //         voids 记录无返回值的函数（VM 在 Return 时不压返回值）
        let mut fidx = HashMap::new();
        let mut voids = HashSet::new();
        let mut decls: Vec<&FunDecl> = program.functions().collect();
        decls.sort_by_key(|f| f.span.line);
        // 逐个函数：登记「全名 → 下标」；void 记入集合
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
        // $toplevel 占最后一个下标；print/push 也视为 void 内建
        let tl = decls.len();
        fidx.insert("$toplevel".into(), tl);
        voids.insert("$toplevel".into());
        voids.insert("print".into());
        voids.insert("push".into());

        // 步骤3：为每个函数创建编译上下文 FnC
        //         参数名按顺序登记为槽 0..arity-1
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

        // 步骤4：编译每个函数体；末尾补 Return 兜底
        for (i, f) in decls.iter().enumerate() {
            cx.cur = i;
            let non_void = !f.ret.is_void();
            for (si, st) in f.body.stmts.iter().enumerate() {
                let last = si + 1 == f.body.stmts.len();
                // 现代化：函数体最后一条无分号表达式 → 隐式 return
                if last && non_void {
                    if let Stmt::Expr(e) = st {
                        if e.implicit_return {
                            cx.expr(&e.expr)?;
                            cx.chunk().emit(Op::Return, e.span.line);
                            continue;
                        }
                    }
                }
                cx.stmt(st)?;
            }
            cx.chunk().emit(Op::Return, f.body.span.line);
        }
        // 步骤5：顶层语句 → $toplevel
        cx.cur = tl;
        for st in program.top_level_stmts() {
            cx.stmt(st)?;
        }
        cx.chunk().emit(Op::Return, 0);

        // 步骤6：FnC 列表 → Function 列表，组装 Module
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

    /// 当前正在编译的函数的字节码缓冲。
    fn chunk(&mut self) -> &mut Chunk {
        &mut self.fns[self.cur].chunk
    }

    fn f(&mut self) -> &mut FnC {
        &mut self.fns[self.cur]
    }

    /// 判断某调用目标是否不产生返回值（表达式语句因此不必 Pop）。
    fn void_call(&self, n: &str) -> bool {
        self.voids.contains(n)
    }
}
