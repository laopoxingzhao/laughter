//! 栈机操作码。
//!
//! 枚举从 0 连续编号（`repr(u8)`），字节码里直接存 `op as u8`；
//! `width()` 说明指令含操作数在内的总字节宽度，便于反汇编。

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
