//! Laughter 教学语言运行时库。
//!
//! **完整流程**
//! ```text
//! .lg 文件
//!   → syntax：词法 + 语法 → AST
//!   → sema：类型检查 + const 折叠
//!   → codegen：AST → 字节码
//!   → runtime：栈机执行 → 打印结果
//! ```
//!
//! **你应该从哪读起**
//! 1. 文档：`docs/CODE_TOUR.md`（小白导读）→ `docs/LANGUAGE.md`（语言规则）
//! 2. 代码：`src/syntax/lexer.rs` → `parser.rs` → `sema/check.rs` → `codegen/compile.rs` → `runtime/vm.rs`
//!
//! **常用 API**
//! - `run_file` / `compile_file`：按路径（支持 import）——CLI 用这个
//! - `run_source` / `compile_source`：单文件字符串（测试常用，无 import）

pub mod codegen;
pub mod module_loader;
pub mod runtime;
pub mod sema;
pub mod syntax;

pub use module_loader::{compile_file, compile_path, run_file, run_path};
pub use runtime::vm::{compile_source, run_source, run_source_file};
