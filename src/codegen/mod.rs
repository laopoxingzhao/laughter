//! 代码生成：操作码、Chunk、编译器、`.lgb` / `.lgpack`（类 JAR）。

pub mod chunk;
pub mod compile;
pub mod lgb;
pub mod lgpack;
pub mod op;

pub use chunk::{Chunk, Function, Module, StructType};
pub use compile::{CompileError, Compiler};
pub use op::Op;
