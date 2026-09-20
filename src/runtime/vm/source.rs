//! 单文件源码编译/执行（测试与库 API；不支持 import）。
use super::*;

/// 解析：Lexer 切词 → Parser 组 AST；失败信息带 `文件:行:列`。
fn parse(src: &str, file: &str) -> Result<crate::syntax::ast::Program, String> {
    use crate::syntax::lexer::Lexer;
    use crate::syntax::parser::Parser;
    // 步骤1：切词
    let toks = Lexer::new(src)
        .tokenize()
        .map_err(|e| format!("{file}:{}:{}: 错误: {}", e.span.line, e.span.col, e.message))?;
    // 步骤2：组装 AST
    Parser::new(toks)
        .parse_program()
        .map_err(|e| format!("{file}:{}:{}: 错误: {}", e.span.line, e.span.col, e.message))
}

/// 单文件字符串 → Module（测试常用）。步骤同 compile_source_file。
pub fn compile_source(src: &str) -> Result<Module, String> {
    compile_source_file("<input>", src)
}

/// 单文件编译：parse → 若含 import 则报错 → Checker → Compiler
pub fn compile_source_file(file: &str, src: &str) -> Result<Module, String> {
    use crate::codegen::compile::Compiler;
    use crate::sema::check::Checker;
    // 步骤1：parse
    let program = parse(src, file)?;
    // 步骤2：单文件 API 不支持 import，尽早报错
    if program.imports().next().is_some() {
        return Err(format!(
            "{file}: error: `import` requires a file path — use `laughter run <file.lg>`"
        ));
    }
    // 步骤3：类型检查 + const 折叠
    let consts = Checker::new(&program)
        .check()
        .map_err(|e| format!("{file}:{}:{}: 错误: {}", e.span.line, e.span.col, e.message))?;
    // 步骤4：生成字节码 Module
    Compiler::compile(&program, consts)
        .map_err(|e| format!("{file}:{}:{}: 错误: {}", e.line, e.col, e.message))
}

/// 单文件源码直接执行（无 import）。
pub fn run_source(src: &str) -> Result<Vec<String>, String> {
    run_source_file("<input>", src)
}

/// 编译并执行单文件源码：compile → Vm::run；诊断前缀为 `file`。
pub fn run_source_file(file: &str, src: &str) -> Result<Vec<String>, String> {
    // 步骤1：编译
    let module = compile_source_file(file, src)?;
    // 步骤2：在栈机上执行，收集 print 行
    let mut vm = Vm::new(&module);
    vm.run().map_err(|e| {
        if e.line == 0 {
            format!("{file}: 运行时错误: {}", e.message)
        } else {
            format!("{file}:{}: 运行时错误: {}", e.line, e.message)
        }
    })
}
