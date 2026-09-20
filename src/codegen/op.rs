//! 栈机操作码一览。
//!
//! **怎么读反汇编**：每条指令前面是地址，`L行号` 是源码行，后面是指令名和操作数。  
//! 例如 `0000 L1 CONST 0 (hello)`：地址 0，源码第 1 行，把常量表第 0 项压栈。
//!
//! **宽度**（`width()`）：指令在 `code` 里占几个字节（含操作数）。
//! - 无操作数：1 字节（如 `Add`、`Pop`）
//! - 带 u16 操作数：3 字节（如 `Const`、`Jump`、`Call`）
//! - 两个 u16：5 字节（如 `SetLocalField`、`NewStruct`）

use std::fmt;

/// 字节码指令。未在 LANGUAGE 中出现的指令属于实现细节。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Op {
    Const = 0,
    True,
    False,
    Pop,
    GetLocal,
    SetLocal,
    SetLocalField,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Neg,
    Not,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Jump,
    JumpIfFalse,
    JumpIfTrue,
    Loop,
    Call,
    CallMethod,
    Return,
    NewArray,
    GetIndex,
    SetIndex,
    NewStruct,
    GetField,
    SetField,
    Len,
    Print,
    Push,
    ArrayPop,
    Input,
    StrAt,
    StrSub,
    ToString,
}

impl Op {
    pub fn from_u8(b: u8) -> Option<Op> {
        // safety: contiguous enum starting at 0
        const MAX: u8 = Op::ToString as u8;
        if b <= MAX {
            Some(unsafe { std::mem::transmute(b) })
        } else {
            None
        }
    }

    /// 指令宽度（含操作数）
    pub fn width(self) -> usize {
        match self {
            Op::Const
            | Op::GetLocal
            | Op::SetLocal
            | Op::Jump
            | Op::JumpIfFalse
            | Op::JumpIfTrue
            | Op::Loop
            | Op::Call
            | Op::CallMethod
            | Op::NewArray
            | Op::GetField
            | Op::SetField => 3,
            Op::SetLocalField | Op::NewStruct => 5,
            _ => 1,
        }
    }
}

impl fmt::Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
