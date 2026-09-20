//! CLI 入口。
//!
//! | 命令 | 中文 |
//! |------|------|
//! | `run <file.lg>` | 运行源码 |
//! | `check <file.lg>` | 检查+编译，不执行 |
//! | `compile <file.lg> [-o out.lgb]` | 编译为 `.lgb` |
//! | `pack <file.lg> [-o out.lgpack]` | 打包为类 JAR 的 `.lgpack`（ZIP） |
//! | `exec <file.lg\|file.lgb\|file.lgpack>` | 执行 |
//! | `disasm <file.lg\|file.lgb\|file.lgpack>` | 反汇编 |
//! | `list <file.lgpack>` | 列出包内文件（类似 `jar tf`） |
//!
//! 文档：`docs/LANGUAGE.md`、`docs/BYTECODE.md`、`docs/LGPACK.md`

use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use laughter::codegen::lgb::{
    default_output_path, disassemble_module, encode_module, exec_module, load_lgb, write_lgb,
    FORMAT_VERSION,
};
use laughter::codegen::lgpack::{
    disasm_package, exec_package, is_lgpack, list_package, pack_program,
};
use laughter::module_loader::{compile_file, run_file};

fn usage() -> ! {
    eprintln!(
        "Laughter — 教学语言编译器 + VM\n\n\
         用法:\n\
         \x20 laughter run <file.lg>                         执行源码\n\
         \x20 laughter check <file.lg>                       检查+编译\n\
         \x20 laughter compile <file.lg> [-o out.lgb]        编译为 .lgb\n\
         \x20 laughter pack <file.lg> [-o out.lgpack] [--resource f]...\n\
         \x20                                                     打包类 JAR 包\n\
         \x20 laughter exec <file.lg|file.lgb|file.lgpack>   执行\n\
         \x20 laughter disasm <上述任意>                     反汇编\n\
         \x20 laughter list <file.lgpack>                    列出包内条目\n\
         \n文档: docs/LANGUAGE.md  docs/BYTECODE.md  docs/LGPACK.md\n"
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
            if is_lgb(path) || is_lgpack(path) {
                return fail("请对 .lg 使用 run；字节码/包请用 exec");
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
        "check" => match compile_file(path) {
            Ok(_) => {
                println!("OK: {path}");
                ExitCode::SUCCESS
            }
            Err(e) => fail(e),
        },
        "compile" => {
            if !path.ends_with(".lg") && !is_lgb(path) {
                // allow .lg only
            }
            if is_lgb(path) || is_lgpack(path) {
                return fail("`compile` 需要 .lg 源码");
            }
            let out: PathBuf = match args.iter().position(|a| a == "-o" || a == "--output") {
                Some(i) => PathBuf::from(args.get(i + 1).map(|s| s.as_str()).unwrap_or("")),
                None => default_output_path(Path::new(path)),
            };
            if out.as_os_str().is_empty() {
                eprintln!("error: `-o` 后面要跟输出路径");
                return ExitCode::from(2);
            }
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
                        "compiled: {} ({} bytes, lgb v{FORMAT_VERSION})",
                        out.display(),
                        bytes.len()
                    );
                    ExitCode::SUCCESS
                }
                Err(e) => fail(e),
            }
        }
        "pack" => {
            if is_lgpack(path) || is_lgb(path) {
                return fail("`pack` 需要 .lg 入口源码");
            }
            let out: PathBuf = match args.iter().position(|a| a == "-o" || a == "--output") {
                Some(i) => PathBuf::from(args.get(i + 1).map(|s| s.as_str()).unwrap_or("")),
                None => {
                    let mut p = PathBuf::from(path);
                    p.set_extension("lgpack");
                    p
                }
            };
            if out.as_os_str().is_empty() {
                eprintln!("error: `-o` 后面要跟输出路径");
                return ExitCode::from(2);
            }
            let mut resources = Vec::new();
            let mut i = 2;
            while i < args.len() {
                if args[i] == "--resource" || args[i] == "-r" {
                    if let Some(p) = args.get(i + 1) {
                        resources.push(PathBuf::from(p));
                        i += 2;
                        continue;
                    }
                }
                i += 1;
            }
            match pack_program(Path::new(path), &out, &resources, "app.lgb") {
                Ok(()) => {
                    println!("packed: {}", out.display());
                    ExitCode::SUCCESS
                }
                Err(e) => fail(e),
            }
        }
        "exec" => {
            if is_lgpack(path) {
                return match exec_package(Path::new(path)) {
                    Ok(lines) => {
                        for l in lines {
                            println!("{l}");
                        }
                        ExitCode::SUCCESS
                    }
                    Err(e) => fail(e),
                };
            }
            if is_lgb(path) {
                let module = match load_lgb(Path::new(path)) {
                    Ok(m) => m,
                    Err(e) => return fail(e),
                };
                return match exec_module(&module) {
                    Ok(lines) => {
                        for l in lines {
                            println!("{l}");
                        }
                        ExitCode::SUCCESS
                    }
                    Err(e) => fail(e),
                };
            }
            // .lg source
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
        "disasm" => {
            if is_lgpack(path) {
                return match disasm_package(Path::new(path)) {
                    Ok(t) => {
                        print!("{t}");
                        ExitCode::SUCCESS
                    }
                    Err(e) => fail(e),
                };
            }
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
        "list" => match list_package(Path::new(path)) {
            Ok(names) => {
                for n in names {
                    println!("{n}");
                }
                ExitCode::SUCCESS
            }
            Err(e) => fail(e),
        },
        "help" | "-h" | "--help" => usage(),
        other => {
            eprintln!("error: 未知命令 `{other}`");
            usage();
        }
    }
}
