//! CLI 入口。
//!
//! | 命令 | 中文 | 说明 |
//! |------|------|------|
//! | `run <file.lg>` | 运行源码 | 检查+编译+执行（支持 import） |
//! | `check <file.lg>` | 只检查编译 | 不执行 |
//! | `compile <file.lg> [-o out.lgb]` | 编译为字节码文件 | 默认输出同名 `.lgb` |
//! | `exec <file.lgb>` | 执行字节码文件 | 不再解析源码 |
//! | `disasm <file.lg\|file.lgb>` | 反汇编 | 打印指令与常量表 |
//!
//! 语言契约：`docs/LANGUAGE.md`；字节码格式：`src/codegen/lgb.rs`。

use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use laughter::codegen::lgb::{
    default_output_path, disassemble_module, encode_module, exec_module, load_lgb, write_lgb,
};
use laughter::module_loader::{compile_file, run_file};

fn usage() -> ! {
    eprintln!(
        "Laughter — 教学语言编译器 + VM\n\n\
         用法:\n\
         \x20 laughter run <file.lg>                    编译并执行源码\n\
         \x20 laughter check <file.lg>                  类型检查+编译，不执行\n\
         \x20 laughter compile <file.lg> [-o out.lgb]   编译为 .lgb 字节码文件\n\
         \x20 laughter exec <file.lgb>                  执行 .lgb\n\
         \x20 laughter disasm <file.lg|file.lgb>        反汇编\n\
         \n文档: docs/LANGUAGE.md  docs/CODE_TOUR.md  docs/BYTECODE.md\n"
    );
    std::process::exit(2)
}

fn is_lgb(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("lgb"))
        .unwrap_or(false)
}

fn fail(msg: impl std::fmt::Display) -> ExitCode {
    eprintln!("{msg}");
    ExitCode::from(1)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        usage();
    }
    let cmd = args[0].as_str();
    let Some(path) = args.get(1) else {
        eprintln!("error: 缺少文件参数");
        usage();
    };

    match cmd {
        "run" => {
            if is_lgb(path) {
                return fail("`run` 只接受 .lg 源码；执行字节码请用 `laughter exec <file.lgb>`");
            }
            match run_file(path) {
                Ok(lines) => {
                    for l in lines {
                        println!("{l}");
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => fail(e),
            }
        }
        "check" => {
            if is_lgb(path) {
                return fail("`check` 只接受 .lg 源码");
            }
            match compile_file(path) {
                Ok(_) => {
                    println!("OK: {path}");
                    ExitCode::SUCCESS
                }
                Err(e) => fail(e),
            }
        }
        "compile" => {
            if is_lgb(path) {
                return fail("`compile` 需要 .lg 源码，而不是 .lgb");
            }
            let out: PathBuf = match args.iter().position(|a| a == "-o" || a == "--output") {
                Some(i) => match args.get(i + 1) {
                    Some(p) => PathBuf::from(p),
                    None => {
                        eprintln!("error: `-o` 后面要跟输出路径");
                        return ExitCode::from(2);
                    }
                },
                None => default_output_path(Path::new(path)),
            };
            let module = match compile_file(path) {
                Ok(m) => m,
                Err(e) => return fail(e),
            };
            let bytes = match encode_module(&module) {
                Ok(b) => b,
                Err(e) => return fail(e),
            };
            match write_lgb(&out, &bytes) {
                Ok(()) => {
                    println!(
                        "compiled: {} ({} bytes, version {})",
                        out.display(),
                        bytes.len(),
                        laughter::codegen::lgb::FORMAT_VERSION
                    );
                    ExitCode::SUCCESS
                }
                Err(e) => fail(e),
            }
        }
        "exec" => {
            if !is_lgb(path) {
                return fail("`exec` 需要 .lgb 字节码；源码请用 `laughter run`");
            }
            let module = match load_lgb(Path::new(path)) {
                Ok(m) => m,
                Err(e) => return fail(e),
            };
            match exec_module(&module) {
                Ok(lines) => {
                    for l in lines {
                        println!("{l}");
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => fail(e),
            }
        }
        "disasm" => {
            let text = if is_lgb(path) {
                match load_lgb(Path::new(path)) {
                    Ok(m) => disassemble_module(&m),
                    Err(e) => return fail(e),
                }
            } else {
                match compile_file(path) {
                    Ok(m) => disassemble_module(&m),
                    Err(e) => return fail(e),
                }
            };
            print!("{text}");
            ExitCode::SUCCESS
        }
        "help" | "-h" | "--help" => usage(),
        other => {
            eprintln!("error: 未知命令 `{other}`");
            usage();
        }
    }
}
