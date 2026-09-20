use std::env;
use std::fs;
use std::process::ExitCode;

use laughter::vm::{compile_source_file, run_source_file};

fn usage() -> ! {
    eprintln!(
        "Laughter — teaching language compiler + VM\n\n\
         USAGE:\n\
         \x20 laughter run <file.lg>      Compile and execute\n\
         \x20 laughter check <file.lg>    Typecheck only\n\
         \x20 laughter disasm <file.lg>   Print bytecode\n"
    );
    std::process::exit(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        usage();
    }
    let cmd = args[0].as_str();
    let path = args.get(1);

    let Some(path) = path else {
        eprintln!("error: missing file argument");
        usage();
    };

    let src = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read `{path}`: {e}");
            return ExitCode::from(1);
        }
    };

    match cmd {
        "run" => match run_source_file(path, &src) {
            Ok(lines) => {
                for line in lines {
                    println!("{line}");
                }
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::from(1)
            }
        },
        "check" => match compile_source_file(path, &src) {
            Ok(_) => {
                println!("OK: {path}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::from(1)
            }
        },
        "disasm" => match compile_source_file(path, &src) {
            Ok(module) => {
                for f in &module.functions {
                    let label = if f.name == "$toplevel" {
                        "<toplevel>".to_string()
                    } else {
                        format!(
                            "fn {} (arity={}, locals={}, void={})",
                            f.name, f.arity, f.locals, f.is_void
                        )
                    };
                    print!("{}", f.chunk.disassemble(&label));
                    if !f.chunk.constants.is_empty() {
                        println!("constants:");
                        for (i, c) in f.chunk.constants.iter().enumerate() {
                            println!("  [{i}] {}", c.display());
                        }
                    }
                    println!();
                }
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::from(1)
            }
        },
        "help" | "--help" | "-h" => usage(),
        other => {
            eprintln!("error: unknown command `{other}`");
            usage();
        }
    }
}
