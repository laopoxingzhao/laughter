//! Laughter 教学语言运行时库。
//!
//! 管线：`syntax` → `sema` → `codegen` → `runtime`。
//! 契约见 `docs/LANGUAGE.md`。

pub mod codegen;
pub mod module_loader;
pub mod runtime;
pub mod sema;
pub mod syntax;

pub use module_loader::{compile_file, compile_path, run_file, run_path};
pub use runtime::vm::{compile_source, run_source, run_source_file};
