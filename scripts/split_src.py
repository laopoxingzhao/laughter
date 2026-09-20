#!/usr/bin/env python3
"""Split large Rust sources into submodules (no behavior change)."""
from pathlib import Path

def read_lines(p):
    return Path(p).read_text(encoding="utf-8").splitlines(keepends=True)

def write(p, text):
    path = Path(p)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    print(f"wrote {p} ({len(text.splitlines())} lines)")

def find(cl, pred, what):
    for i, l in enumerate(cl):
        if pred(l):
            return i
    raise SystemExit(f"not found: {what}")

def split_check():
    cl = read_lines("src/sema/check.rs")
    fold_start = find(cl, lambda l: l.strip().startswith("fn fold(&self"), "fold")
    declare_fun = find(cl, lambda l: l.strip().startswith("fn declare_fun"), "declare_fun")
    check_stmt = find(cl, lambda l: l.strip().startswith("fn check_stmt"), "check_stmt")
    expr_start = find(cl, lambda l: l.strip().startswith("fn expr_ty"), "expr_ty")
    bin_result = find(cl, lambda l: l.strip().startswith("fn bin_result"), "bin_result")
    fold_bin = find(cl, lambda l: l.strip().startswith("fn fold_bin"), "fold_bin")
    always_ret = find(cl, lambda l: l.strip().startswith("fn always_returns"), "always_ret")
    body_start = find(cl, lambda l: l.startswith("pub struct CheckError"), "CheckError")
    while body_start > 0 and cl[body_start - 1].startswith("///"):
        body_start -= 1

    mod_core = cl[body_start:fold_start] + cl[declare_fun:check_stmt]
    header = (
        "//! 类型检查与 const 折叠（模块入口）。\n"
        "//!\n"
        "//! 子模块：`fold`（常量折叠）、`stmt`（语句）、`expr`（表达式与调用）。\n"
        "\n"
        "use std::collections::HashMap;\n"
        "use std::rc::Rc;\n"
        "\n"
        "use crate::runtime::value::Value;\n"
        "use crate::sema::types::Type;\n"
        "use crate::syntax::ast::*;\n"
        "use crate::syntax::token::Span;\n"
        "\n"
        "mod expr;\n"
        "mod fold;\n"
        "mod stmt;\n"
        "\n"
    )
    write(
        "src/sema/check/mod.rs",
        header + "".join(mod_core) + "\n}\n",
    )
    write(
        "src/sema/check/fold.rs",
        "//! const 表达式折叠。\nuse super::*;\n\nimpl<'a> Checker<'a> {\n"
        + "".join(cl[fold_start:declare_fun])
        + "\n}\n\n"
        + "".join(cl[fold_bin:always_ret]),
    )
    write(
        "src/sema/check/stmt.rs",
        "//! 语句 / 函数体检查。\nuse super::*;\n\nimpl<'a> Checker<'a> {\n"
        + "".join(cl[check_stmt:expr_start])
        + "\n}\n\n"
        + "".join(cl[always_ret:]),
    )
    write(
        "src/sema/check/expr.rs",
        "//! 表达式类型检查与函数调用。\nuse super::*;\n\nimpl<'a> Checker<'a> {\n"
        + "".join(cl[expr_start:fold_bin])
        + "\n}\n",
    )
    Path("src/sema/check.rs").unlink(missing_ok=True)

def split_parser():
    cl = read_lines("src/syntax/parser.rs")
    tests = find(cl, lambda l: l.startswith("#[cfg(test)]"), "tests")
    struct_lit = find(cl, lambda l: l.strip().startswith("fn struct_lit_ahead"), "struct_lit")
    ty_fn = find(cl, lambda l: l.strip().startswith("fn ty(&mut self"), "ty")
    block_fn = find(cl, lambda l: l.strip().startswith("fn block(&mut self"), "block")
    stmt_fn = find(cl, lambda l: l.strip().startswith("fn stmt(&mut self"), "stmt")
    expr_fn = find(cl, lambda l: l.strip().startswith("pub fn expr(&mut self"), "expr")

    header = (
        "//! 递归下降语法分析（模块入口：Parser 状态与工具）。\n"
        "//!\n"
        "//! 子模块：`decl`（顶层声明）、`stmt`（语句）、`expr`（表达式）。\n"
        "\n"
        "use crate::syntax::ast::*;\n"
        "use crate::syntax::token::{Span, Token, TokenKind};\n"
        "\n"
        "mod decl;\n"
        "mod expr;\n"
        "mod stmt;\n"
        "\n"
    )
    # ParseError + Parser + new + parse_program + peek..struct_lit_ahead
    # find ParseError
    pe = find(cl, lambda l: l.startswith("pub struct ParseError") or l.startswith("/// 解析错误") or l.startswith("#[derive(Debug)]\n"), "pe")
    pe = find(cl, lambda l: "ParseError" in l and l.startswith("pub struct"), "ParseError struct")
    while pe > 0 and cl[pe - 1].startswith("///"):
        pe -= 1

    mod_body = cl[pe:ty_fn]  # ParseError, Parser, tools, through struct_lit_ahead
    # struct_lit is before ty - good if order is struct_lit then ty
    write(
        "src/syntax/parser/mod.rs",
        header
        + "".join(mod_body)
        + "\n"
        + "".join(cl[tests:]),
    )
    write(
        "src/syntax/parser/decl.rs",
        "//! 顶层声明：import / struct / const / fun / 类型。\nuse super::*;\n\nimpl Parser {\n"
        + "".join(cl[ty_fn:block_fn])
        + "\n}\n",
    )
    write(
        "src/syntax/parser/stmt.rs",
        "//! 语句解析。\nuse super::*;\n\nimpl Parser {\n"
        + "".join(cl[block_fn:expr_fn])
        + "\n}\n",
    )
    write(
        "src/syntax/parser/expr.rs",
        "//! 表达式解析（优先级见模块注释）。\nuse super::*;\n\nimpl Parser {\n"
        + "".join(cl[expr_fn:tests])
        + "\n}\n",
    )
    Path("src/syntax/parser.rs").unlink(missing_ok=True)

def split_compile():
    cl = read_lines("src/codegen/compile.rs")
    compile_fn = find(cl, lambda l: l.strip().startswith("pub fn compile("), "compile")
    chunk_fn = find(cl, lambda l: l.strip().startswith("fn chunk(&mut self"), "chunk")
    assign_fn = find(cl, lambda l: l.strip().startswith("fn assign(&mut self"), "assign")
    expr_fn = find(cl, lambda l: l.strip().startswith("fn expr(&mut self"), "expr")

    header = (
        "//! AST → 字节码（模块入口：Compiler 结构与总控）。\n"
        "//!\n"
        "//! 子模块：`stmt`（语句/循环/赋值）、`expr`（表达式与调用）。\n"
        "\n"
        "use std::collections::{HashMap, HashSet};\n"
        "use std::rc::Rc;\n"
        "\n"
        "use crate::codegen::chunk::{Chunk, Function, Module, StructType};\n"
        "use crate::codegen::op::Op;\n"
        "use crate::runtime::value::Value;\n"
        "use crate::syntax::ast::*;\n"
        "use crate::syntax::token::Span;\n"
        "\n"
        "mod expr;\n"
        "mod stmt;\n"
        "\n"
    )
    ce = find(cl, lambda l: l.startswith("pub struct CompileError") or (l.startswith("#[derive(Debug)]") and "CompileError" in "".join(cl[0:40])), "CompileError")
    ce = find(cl, lambda l: l.startswith("pub struct CompileError"), "CompileError")
    while ce > 0 and cl[ce - 1].startswith("///"):
        ce -= 1
    # Through compile() end + helpers chunk/f/void_call
    # Order: CompileError, From, Local, FnC, LoopP, Compiler, compile, chunk, f, void_call, block, stmt, assign...
    write(
        "src/codegen/compile/mod.rs",
        header + "".join(cl[ce:assign_fn]).replace("fn block(&mut self", "fn block(&mut self"),
    )
    # Wait assign_fn is after stmt - compile helpers include stmt. Better:
    # mod: ce : compile end + chunk f void_call - find fn block
    block_fn = find(cl, lambda l: l.strip().startswith("fn block(&mut self"), "block")
    write(
        "src/codegen/compile/mod.rs",
        header + "".join(cl[ce:block_fn]) + "\n",
    )
    write(
        "src/codegen/compile/stmt.rs",
        "//! 语句、赋值、if/while/for 编译。\nuse super::*;\n\nimpl Compiler {\n"
        + "".join(cl[block_fn:expr_fn])
        + "\n}\n",
    )
    write(
        "src/codegen/compile/expr.rs",
        "//! 表达式与调用编译。\nuse super::*;\n\nimpl Compiler {\n"
        + "".join(cl[expr_fn:])
        + "\n}\n",
    )
    Path("src/codegen/compile.rs").unlink(missing_ok=True)

def split_vm():
    cl = read_lines("src/runtime/vm.rs")
    loop_run = find(cl, lambda l: l.strip().startswith("fn loop_run"), "loop_run")
    parse_fn = find(cl, lambda l: l.strip().startswith("fn parse(src:"), "parse")
    compile_src = find(cl, lambda l: l.strip().startswith("pub fn compile_source"), "compile_source")
    vme = find(cl, lambda l: l.startswith("pub struct VmError"), "VmError")
    while vme > 0 and cl[vme - 1].startswith("///"):
        vme -= 1

    header = (
        "//! 栈式虚拟机（模块入口：Vm 与帧）。\n"
        "//!\n"
        "//! 子模块：`exec`（主解释循环）、`source`（run_source 单文件管线）。\n"
        "\n"
        "use std::cell::RefCell;\n"
        "use std::rc::Rc;\n"
        "\n"
        "use crate::codegen::chunk::Module;\n"
        "use crate::codegen::op::Op;\n"
        "use crate::runtime::value::{ArrayHandle, StructVal, Value};\n"
        "\n"
        "mod exec;\n"
        "mod source;\n"
        "\n"
    )
    # VmError through peek (before loop_run)
    write(
        "src/runtime/vm/mod.rs",
        header + "".join(cl[vme:loop_run]),
    )
    write(
        "src/runtime/vm/exec.rs",
        "//! 主解释循环。\nuse super::*;\n\nimpl<'m> Vm<'m> {\n"
        + "".join(cl[loop_run:parse_fn])
        + "\n}\n\n"
        + "".join(cl[parse_fn:compile_src])
        + "".join(cl[compile_src:]),
    )
    # source fns go to source.rs; parse helper with them
    write(
        "src/runtime/vm/exec.rs",
        "//! 主解释循环。\nuse super::*;\n\nimpl<'m> Vm<'m> {\n"
        + "".join(cl[loop_run:parse_fn])
        + "\n}\n",
    )
    write(
        "src/runtime/vm/source.rs",
        "//! 单文件源码编译/执行（测试与库 API；不支持 import）。\nuse super::*;\n\n"
        + "".join(cl[parse_fn:]),
    )
    Path("src/runtime/vm.rs").unlink(missing_ok=True)

def split_ast():
    cl = read_lines("src/syntax/ast.rs")
    # keep one file if ~428 lines - optional skip
    print("ast.rs kept as single file (already moderate)")

def main():
    split_check()
    split_parser()
    split_compile()
    split_vm()
    print("all splits done")

if __name__ == "__main__":
    main()
