//! 多文件模块加载：解析 `import`，合并为单一 `Program` 再检查/编译。
//! 只合并顶层 `fun`/`struct` 声明，不执行被导入文件的顶层语句。
//! `import "x.lg"` 扁平合入；`import "x.lg" as ns` 将符号命名为 `ns.name`。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::ast::*;
use crate::bytecode::Module;
use crate::compiler::Compiler;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::resolve::Checker;

pub fn parse_source(src: &str) -> Result<Program, String> {
    parse_source_as("<input>", src)
}

fn parse_source_as(file: &str, src: &str) -> Result<Program, String> {
    let tokens = Lexer::new(src).tokenize().map_err(|e| {
        format!(
            "{file}:{}:{}: error: {}",
            e.span.line, e.span.col, e.message
        )
    })?;
    Parser::new(tokens).parse_program().map_err(|e| {
        format!(
            "{file}:{}:{}: error: {}",
            e.span.line, e.span.col, e.message
        )
    })
}

fn resolve_import(from_file: &Path, rel: &str) -> Result<PathBuf, String> {
    if rel.split('/').any(|seg| seg == "..") {
        return Err(format!("import path must not contain `..`: {rel}"));
    }
    let base = from_file.parent().unwrap_or_else(|| Path::new("."));
    let mut p = base.to_path_buf();
    for seg in rel.split('/') {
        p.push(seg);
    }
    Ok(p)
}

fn load_program(
    path: &Path,
    stack: &mut Vec<PathBuf>,
    out_items: &mut Vec<Item>,
    ns_aliases: &mut HashMap<String, PathBuf>,
) -> Result<(), String> {
    let canon = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if stack.iter().any(|p| p == &canon) {
        return Err(format!("cyclic import involving {}", path.display()));
    }
    let src = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read `{}`: {e}", path.display()))?;
    let program = parse_source_as(&path.display().to_string(), &src)?;

    stack.push(canon);
    let mut pending_imports: Vec<(ImportItem, PathBuf)> = Vec::new();
    for imp in program.imports() {
        let target = resolve_import(path, &imp.path)?;
        if !target.exists() {
            return Err(format!(
                "{}:{}: cannot resolve import `{}`",
                path.display(),
                imp.span.line,
                imp.path
            ));
        }
        pending_imports.push((imp.clone(), target));
    }

    for (imp, target) in pending_imports {
        let mut sub_items = Vec::new();
        load_program(&target, stack, &mut sub_items, ns_aliases)?;
        match &imp.alias {
            None => {
                for it in sub_items {
                    match it {
                        Item::Fun(_) | Item::Struct(_) | Item::Const(_) => out_items.push(it),
                        _ => {}
                    }
                }
            }
            Some(alias) => {
                if ns_aliases
                    .insert(alias.name.clone(), target.clone())
                    .is_some()
                {
                    return Err(format!("duplicate import alias `{}`", alias.name));
                }
                for it in sub_items {
                    match it {
                        Item::Fun(mut f) => {
                            let base = f
                                .name
                                .name
                                .rsplit('.')
                                .next()
                                .unwrap_or(&f.name.name)
                                .to_string();
                            f.name = Ident {
                                name: format!("{}.{}", alias.name, base),
                                span: f.name.span,
                            };
                            // 若原为方法 Type.m，扁平命名空间下改为 ns.m（接收者类型仍在签名中）
                            f.on_type = None;
                            out_items.push(Item::Fun(f));
                        }
                        Item::Struct(mut s) => {
                            let base = s
                                .name
                                .name
                                .rsplit('.')
                                .next()
                                .unwrap_or(&s.name.name)
                                .to_string();
                            s.name = Ident {
                                name: format!("{}.{}", alias.name, base),
                                span: s.name.span,
                            };
                            out_items.push(Item::Struct(s));
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    stack.pop();

    for it in program.items {
        match it {
            Item::Import(_) => {}
            other => out_items.push(other),
        }
    }
    Ok(())
}

pub fn compile_path(path: &Path) -> Result<Module, String> {
    let mut stack = Vec::new();
    let mut items = Vec::new();
    let mut ns_aliases = HashMap::new();
    load_program(path, &mut stack, &mut items, &mut ns_aliases)?;
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

pub fn run_path(path: &Path) -> Result<Vec<String>, String> {
    let module = compile_path(path)?;
    let mut vm = crate::vm::Vm::new(&module);
    let label = path.display().to_string();
    vm.run().map_err(|e| {
        if e.line == 0 {
            format!("{label}: runtime error: {}", e.message)
        } else {
            format!("{label}:{}: runtime error: {}", e.line, e.message)
        }
    })
}

pub fn run_file(path: &str) -> Result<Vec<String>, String> {
    run_path(Path::new(path))
}

pub fn compile_file(path: &str) -> Result<Module, String> {
    compile_path(Path::new(path))
}
