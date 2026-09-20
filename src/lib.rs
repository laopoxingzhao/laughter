//! Laughter 教学语言运行时库。
//!
//! 管线：`syntax` → `sema` → `codegen` → `runtime`。
//! 语言契约见 `docs/LANGUAGE.md`；架构说明见 `docs/ARCHITECTURE.md`。
//!
//! 常用入口：
//! - `module_loader::{run_file, compile_file}` — 按路径运行/编译（支持 import）
//! - `runtime::vm::{run_source, compile_source}` — 单文件源码字符串（无 import）

pub mod codegen;
pub mod module_loader;
pub mod runtime;
pub mod sema;
pub mod syntax;

pub use module_loader::{compile_file, compile_path, run_file, run_path};
pub use runtime::vm::{compile_source, run_source, run_source_file};
