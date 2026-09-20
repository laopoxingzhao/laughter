//! CLI：`laughter run|check|disasm <file.lg>`

use std::env;
use std::process::ExitCode;

use laughter::module_loader::{compile_file, run_file};

fn usage() -> ! {
    eprintln!(
        "Laughter — 教学语言编译器 + VM\n\n\
         用法:\n\
         \x20 laughter run <file.lg>      类型检查 + 编译 + 执行\n\
         \x20 laughter check <file.lg>    词法/语法/语义 + 编译，不执行\n\
         \x20 laughter disasm <file.lg>   打印字节码反汇编与常量表\n\
         \n语言契约: docs/LANGUAGE.md\n"
    );
    std::process::exit(2)
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
        "run" => match run_file(path) {
            Ok(lines) => {
                for l in lines {
                    println!("{l}");
                }
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::from(1)
            }
        },
        "check" => match compile_file(path) {
            Ok(_) => {
                println!("OK: {path}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::from(1)
            }
        },
        "disasm" => match compile_file(path) {
            Ok(m) => {
                for f in &m.functions {
                    let label = if f.name == "$toplevel" {
                        "<toplevel>".to_string()
                    } else {
                        format!(
                            "fn {} (arity={}, locals={}, void={})",
                            f.name, f.arity, f.locals, f.is_void
                        )
                    };
                    print!("{}", f.chunk.disassemble(&label));
                    println!();
                }
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::from(1)
            }
        },
        "help" | "-h" | "--help" => usage(),
        other => {
            eprintln!("error: 未知命令 `{other}`");
            usage();
        }
    }
}
