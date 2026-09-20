//! 多文件模块加载：`import` 如何工作。
//!
//! **为什么单独一层**：单文件解析器不知道磁盘上还有别的 `.lg`。  
//! 这里负责「按路径读文件 → 解析 → 把声明拼进同一个程序」。
//!
//! **两种 import**
//! ```text
//! import "math.lg";           // 扁平：math 里的 add 变成全局 add
//! import "math.lg" as math;   // 命名空间：只能写 math.add
//! ```
//!
//! **安全与语义**
//! - 路径相对当前文件；禁止 `..`（防止逃出项目目录）
//! - 用「加载栈」检测 A 导入 B、B 又导入 A 的环
//! - **只合并声明**（struct/const/fun），不执行被导入文件的顶层语句
//!
//! CLI `laughter run foo.lg` 走这里；字符串 API `run_source` 不走这里。

use std::path::{Path, PathBuf};

use crate::codegen::chunk::Module;
use crate::codegen::compile::Compiler;
use crate::sema::check::Checker;
use crate::syntax::ast::*;
use crate::syntax::lexer::Lexer;
use crate::syntax::parser::Parser;

/// 单文件：词法 + 语法 → AST（错误带 `file:line:col`）。
/// 文件路径 + 源码文本 → AST。
fn parse_src(file: &str, src: &str) -> Result<Program, String> {
    let toks = Lexer::new(src)
        .tokenize()
        .map_err(|e| format!("{file}:{}:{}: 错误: {}", e.span.line, e.span.col, e.message))?;
    Parser::new(toks)
        .parse_program()
        .map_err(|e| format!("{file}:{}:{}: 错误: {}", e.span.line, e.span.col, e.message))
}

/// 把 import 相对路径拼到「当前文件所在目录」；禁止 `..`。
fn resolve(base_file: &Path, rel: &str) -> Result<PathBuf, String> {
    if rel.split('/').any(|s| s == "..") {
        return Err(format!("import 路径不能包含 `..`: {rel}"));
    }
    let mut p = base_file
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    for seg in rel.split('/') {
        p.push(seg);
    }
    Ok(p)
}

/// 递归加载一个 `.lg` 文件。
/// 步骤：规范化路径查环 → parse → 递归处理 import → 合并声明到 `out`。
fn load(path: &Path, stack: &mut Vec<PathBuf>, out: &mut Vec<Item>) -> Result<(), String> {
    let canon = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if stack.iter().any(|p| p == &canon) {
        return Err(format!("循环 import: {}", path.display()));
    }
    let src =
        std::fs::read_to_string(path).map_err(|e| format!("无法读取 `{}`: {e}", path.display()))?;
    let file = path.display().to_string();
    let prog = parse_src(&file, &src)?;

    stack.push(canon);
    let imports: Vec<ImportItem> = prog.imports().cloned().collect();
    for imp in imports {
        let target = resolve(path, &imp.path)?;
        if !target.exists() {
            return Err(format!(
                "{file}:{}: cannot resolve import `{}`",
                imp.span.line, imp.path
            ));
        }
        let mut sub = vec![];
        load(&target, stack, &mut sub)?;
        match &imp.alias {
            None => {
                for it in sub {
                    match it {
                        Item::Struct(_) | Item::Const(_) | Item::Fun(_) => out.push(it),
                        _ => {}
                    }
                }
            }
            Some(alias) => {
                for it in sub {
                    match it {
                        Item::Struct(mut s) => {
                            let base = s.name.name.rsplit('.').next().unwrap().to_string();
                            s.name.name = format!("{}.{}", alias.name, base);
                            out.push(Item::Struct(s));
                        }
                        Item::Const(mut c) => {
                            c.name.name = format!("{}.{}", alias.name, c.name.name);
                            out.push(Item::Const(c));
                        }
                        Item::Fun(mut f) => {
                            let base = f.name.name.rsplit('.').next().unwrap().to_string();
                            f.name.name = format!("{}.{}", alias.name, base);
                            f.on_type = None;
                            out.push(Item::Fun(f));
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    stack.pop();

    for it in prog.items {
        match it {
            Item::Import(_) => {}
            other => out.push(other),
        }
    }
    Ok(())
}

/// 从文件路径编译：加载 import 图 → 检查 + const 折叠 → 字节码 Module。
/// 按路径编译（支持 import）。步骤：
/// 1. load：递归读入 import，合并声明
/// 2. Checker：类型检查 + const 折叠
/// 3. Compiler：AST → 字节码 Module
pub fn compile_path(path: &Path) -> Result<Module, String> {
    let mut stack = vec![];
    let mut items = vec![];
    load(path, &mut stack, &mut items)?;
    let program = Program { items };
    let label = path.display().to_string();
    let consts = Checker::new(&program).check().map_err(|e| {
        format!(
            "{label}:{}:{}: 错误: {}",
            e.span.line, e.span.col, e.message
        )
    })?;
    Compiler::compile(&program, consts)
        .map_err(|e| format!("{label}:{}:{}: 错误: {}", e.line, e.col, e.message))
}

/// 编译并执行文件；诊断前缀为文件路径。
/// 按路径编译并执行，返回 print 的各行。
pub fn run_path(path: &Path) -> Result<Vec<String>, String> {
    let module = compile_path(path)?;
    let mut vm = crate::runtime::vm::Vm::new(&module);
    let label = path.display().to_string();
    vm.run().map_err(|e| {
        if e.line == 0 {
            format!("{label}: 运行时错误: {}", e.message)
        } else {
            format!("{label}:{}: 运行时错误: {}", e.line, e.message)
        }
    })
}

pub fn compile_file(path: &str) -> Result<Module, String> {
    compile_path(Path::new(path))
}

pub fn run_file(path: &str) -> Result<Vec<String>, String> {
    run_path(Path::new(path))
}
