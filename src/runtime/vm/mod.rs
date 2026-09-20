//! 栈式虚拟机（模块入口：Vm 与帧）。
//!
//! 子模块：`exec`（主解释循环）、`source`（run_source 单文件管线）。

use std::cell::RefCell;
use std::rc::Rc;

use crate::codegen::chunk::Module;
use crate::codegen::op::Op;
use crate::runtime::value::{ArrayHandle, StructVal, Value};

mod exec;
mod source;

pub use source::{compile_source, compile_source_file, run_source, run_source_file};

pub struct VmError {
    pub message: String,
    pub line: u32,
}

/// 一次函数调用的「现场」记录（调用帧）。
///
/// | 字段 | 中文 | 含义 |
/// |------|------|------|
/// | `func` | 函数编号 | 正在执行 `module.functions` 里的哪一个 |
/// | `ip` | 指令指针 | 下一条字节码的下标（instruction pointer） |
/// | `base` | 帧基址 | 本函数局部变量在全局栈上的起始下标 |
///
/// 局部槽 `n` 对应全局栈下标 `base + n`。
struct Frame {
    func: usize,
    ip: usize,
    base: usize,
}

/// 栈式虚拟机。
///
/// | 字段 | 中文 |
/// |------|------|
/// | `module` | 待执行的编译结果（函数表等） |
/// | `stack` | 操作数栈：指令压入/弹出 `Value` |
/// | `frames` | 调用帧栈（每次调用压一层） |
/// | `max_depth` | 最大递归深度，防止无限递归把主机栈打爆 |
pub struct Vm<'m> {
    module: &'m Module,
    stack: Vec<Value>,
    frames: Vec<Frame>,
    max_depth: usize,
}

impl<'m> Vm<'m> {
    pub fn new(module: &'m Module) -> Self {
        Self {
            module,
            stack: vec![],
            frames: vec![],
            max_depth: 1024,
        }
    }

    /// 从入口函数（有 `main` 则 main，否则 toplevel）开始执行，返回 `print` 的各行。
    pub fn run(&mut self) -> Result<Vec<String>, VmError> {
        let entry = self.module.main_index.unwrap_or(self.module.toplevel_index);
        self.push_frame(entry)?;
        let mut out = vec![];
        self.loop_run(&mut out)
    }

    fn push_frame(&mut self, func: usize) -> Result<(), VmError> {
        if self.frames.len() >= self.max_depth {
            return Err(VmError {
                message: format!("stack overflow (recursion depth {})", self.max_depth),
                line: 0,
            });
        }
        let argc = self.module.functions[func].arity as usize;
        if self.stack.len() < argc {
            return Err(VmError {
                message: "missing arguments".into(),
                line: 0,
            });
        }
        let base = self.stack.len() - argc;
        self.frames.push(Frame { func, ip: 0, base });
        Ok(())
    }

    fn line(&self) -> u32 {
        let f = self.frames.last().unwrap();
        self.module.functions[f.func]
            .chunk
            .lines
            .get(f.ip)
            .copied()
            .unwrap_or(0)
    }

    fn u16(&self, at: usize) -> Result<u16, VmError> {
        let f = self.frames.last().unwrap();
        let code = &self.module.functions[f.func].chunk.code;
        if at + 1 >= code.len() {
            return Err(VmError {
                message: "truncated instruction".into(),
                line: 0,
            });
        }
        Ok(u16::from_le_bytes([code[at], code[at + 1]]))
    }

    fn pop(&mut self, line: u32) -> Result<Value, VmError> {
        self.stack.pop().ok_or_else(|| VmError {
            message: "stack underflow".into(),
            line,
        })
    }

    fn peek(&self, line: u32) -> Result<&Value, VmError> {
        self.stack.last().ok_or_else(|| VmError {
            message: "stack underflow".into(),
            line,
        })
    }
}
