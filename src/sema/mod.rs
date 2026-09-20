//! 语义层：类型、检查、常量折叠。

pub mod check;
pub mod types;

pub use check::{CheckError, Checker};
pub use types::Type;
