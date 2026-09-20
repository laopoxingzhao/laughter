//! Laughter：教学向编译语言工具链（库入口）。
//!
//! 管线：`Lexer → Parser → Checker → Compiler（字节码）→ VM`。
//! 二进制 CLI 见 `src/bin/laughter.rs`；集成测试通过本 crate 调用 `vm::run_source`。

pub mod ast;
pub mod bytecode;
pub mod compiler;
pub mod lexer;
pub mod loader;
pub mod parser;
pub mod resolve;
pub mod token;
pub mod types;
pub mod value;
pub mod vm;
