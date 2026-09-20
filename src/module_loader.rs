//! 多文件模块加载：解析 `import`，合并顶层声明为单一 `Program`。
//!
//! - `import "rel.lg";`：扁平合入目标文件的 struct/const/fun。
//! - `import "rel.lg" as ns;`：符号改名为 `ns.name`（方法名去掉 Type 前缀后挂到 ns 下）。
//! - 只合并**声明**，不执行目标文件顶层语句。
//! - 路径相对当前文件；禁止 `..`；用栈检测环状 import。
//!
//! CLI 的 `run`/`check`/`disasm` 走本模块；`run_source` 不走这里。

use std::path::{Path, PathBuf};

use crate::codegen::chunk::Module;
use crate::codegen::compile::Compiler;
use crate::sema::check::Checker;
use crate::syntax::ast::*;
use crate::syntax::lexer::Lexer;
use crate::syntax::parser::Parser;

fn parse_src(file: &str, src: &str) -> Result<Program, String> {
    let toks = Lexer::new(src).tokenize().map_err(|e| {
        format!(
            "{file}:{}:{}: error: {}",
            e.span.line, e.span.col, e.message
        )
    })?;
    Parser::new(toks).parse_program().map_err(|e| {
        format!(
            "{file}:{}:{}: error: {}",
            e.span.line, e.span.col, e.message
        )
    })
}

fn resolve(base_file: &Path, rel: &str) -> Result<PathBuf, String> {
    if rel.split('/').any(|s| s == "..") {
        return Err(format!("import path must not contain `..`: {rel}"));
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

fn load(path: &Path, stack: &mut Vec<PathBuf>, out: &mut Vec<Item>) -> Result<(), String> {
    let canon = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if stack.iter().any(|p| p == &canon) {
        return Err(format!("cyclic import: {}", path.display()));
    }
    let src = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read `{}`: {e}", path.display()))?;
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
pub fn compile_path(path: &Path) -> Result<Module, String> {
    let mut stack = vec![];
    let mut items = vec![];
    load(path, &mut stack, &mut items)?;
    let program = Program { items };
    let label = path.display().to_string();
    let consts = Checker::new(&program).check().map_err(|e| {
        format!(
            "{label}:{}:{}: error: {}",
            e.span.line, e.span.col, e.message
        )
    })?;
    Compiler::compile(&program, consts)
        .map_err(|e| format!("{label}:{}:{}: error: {}", e.line, e.col, e.message))
}

/// 编译并执行文件；诊断前缀为文件路径。
pub fn run_path(path: &Path) -> Result<Vec<String>, String> {
    let module = compile_path(path)?;
    let mut vm = crate::runtime::vm::Vm::new(&module);
    let label = path.display().to_string();
    vm.run().map_err(|e| {
        if e.line == 0 {
            format!("{label}: runtime error: {}", e.message)
        } else {
            format!("{label}:{}: runtime error: {}", e.line, e.message)
        }
    })
}

pub fn compile_file(path: &str) -> Result<Module, String> {
    compile_path(Path::new(path))
}

pub fn run_file(path: &str) -> Result<Vec<String>, String> {
    run_path(Path::new(path))
}
