//! 单文件源码编译/执行（测试与库 API；不支持 import）。
use super::*;

fn parse(src: &str, file: &str) -> Result<crate::syntax::ast::Program, String> {
    use crate::syntax::lexer::Lexer;
    use crate::syntax::parser::Parser;
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

/// 单文件字符串 → Module（测试常用）。步骤同 compile_source_file。
pub fn compile_source(src: &str) -> Result<Module, String> {
    compile_source_file("<input>", src)
}

/// 单文件编译：parse → 若含 import 则报错 → Checker → Compiler
pub fn compile_source_file(file: &str, src: &str) -> Result<Module, String> {
    use crate::codegen::compile::Compiler;
    use crate::sema::check::Checker;
    let program = parse(src, file)?;
    if program.imports().next().is_some() {
        return Err(format!(
            "{file}: error: `import` requires a file path — use `laughter run <file.lg>`"
        ));
    }
    let consts = Checker::new(&program).check().map_err(|e| {
        format!(
            "{file}:{}:{}: error: {}",
            e.span.line, e.span.col, e.message
        )
    })?;
    Compiler::compile(&program, consts)
        .map_err(|e| format!("{file}:{}:{}: error: {}", e.line, e.col, e.message))
}

/// 单文件源码直接执行（无 import）。
pub fn run_source(src: &str) -> Result<Vec<String>, String> {
    run_source_file("<input>", src)
}

pub fn run_source_file(file: &str, src: &str) -> Result<Vec<String>, String> {
    let module = compile_source_file(file, src)?;
    let mut vm = Vm::new(&module);
    vm.run().map_err(|e| {
        if e.line == 0 {
            format!("{file}: runtime error: {}", e.message)
        } else {
            format!("{file}:{}: runtime error: {}", e.line, e.message)
        }
    })
}
