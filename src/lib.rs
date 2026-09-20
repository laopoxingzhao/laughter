//! Laughter 教学语言运行时库。
//!
//! **完整流程**
//! ```text
//! .lg 文件
//!   → syntax：词法 + 语法 → AST
//!   → sema：类型检查 + const 折叠
//!   → codegen：AST → 字节码（内存 Module）
//!   → 可选：codegen::lgb 写入 .lgb 文件
//!   → runtime：栈机执行 → 打印结果
//! ```
//!
//! **建议阅读**
//! 1. `docs/COMPILE_PRIMER.md` → `docs/LANGUAGE.md` → `docs/TOOLS.md`
//! 2. `src/syntax/` → `sema/` → `codegen/` → `runtime/`
//!
//! **常用 API**
//! - `run_file` / `compile_file`：源码路径（支持 import）
//! - `codegen::lgb::{encode_module, decode_module, load_lgb}`：`.lgb` 文件
//! - `run_source`：单文件字符串（测试用，无 import）

pub mod codegen;
pub mod module_loader;
pub mod runtime;
pub mod sema;
pub mod syntax;

pub use module_loader::{compile_file, compile_path, run_file, run_path};
pub use runtime::vm::{compile_source, run_source, run_source_file};
