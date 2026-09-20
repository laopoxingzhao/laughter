//! `.lgb` 字节码文件格式：把内存中的 `Module` 写入磁盘，以及读回。
//!
//! ## 文件布局（小端 little-endian）
//!
//! ```text
//! [0..4]   魔数 "LGB1"
//! [4..6]   版本号 u16（当前 FORMAT_VERSION = 1）
//! [6..8]   保留 u16（对齐，填 0）
//! [8..10]  main_index：u16，0xFFFF 表示无 main
//! [10..12] toplevel_index u16
//! [12..]   结构体布局表
//! [..]     函数表
//! ```
//!
//! ### 结构体表
//! ```text
//! count: u16
//! 每个结构体:
//!   name_len: u16, name: UTF-8
//!   field_count: u16
//!   每个字段: field_len: u16, field: UTF-8
//! ```
//!
//! ### 函数
//! ```text
//! count: u16
//! 每个函数:
//!   name_len: u16, name: UTF-8
//!   arity: u8, is_void: u8 (0/1)
//!   locals: u16
//!   const_count: u16
//!   每个常量: tag u8 + 载荷
//!       0 = Int   i64
//!       1 = Float f64
//!       2 = Bool  u8
//!       3 = Str   u16 长度 + UTF-8
//!   code_len: u32
//!   code: code_len 字节
//!   lines_len: u32（应等于 code_len）
//!   lines: 每个 u32
//! ```
//!
//! ## 命令
//! - `laughter compile foo.lg [-o foo.lgb]`
//! - `laughter exec foo.lgb`
//! - `laughter disasm` 同时接受 `.lg` 与 `.lgb`
//!
//! 常量池只含标量与字符串（与 `Op::Const` 一致）；数组/结构体在运行时构造。

use std::path::Path;

use crate::codegen::chunk::{Chunk, Function, Module, StructType};
use crate::runtime::value::Value;
use crate::runtime::vm::Vm;

pub const MAGIC: &[u8; 4] = b"LGB1";
pub const FORMAT_VERSION: u16 = 1;
const NO_MAIN: u16 = 0xFFFF;

#[derive(Debug)]
pub struct BytecodeError {
    pub message: String,
}

impl std::fmt::Display for BytecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "字节码错误: {}", self.message)
    }
}

fn err(msg: impl Into<String>) -> BytecodeError {
    BytecodeError {
        message: msg.into(),
    }
}

/// 把 `Module` 编码为 `.lgb` 字节。
/// 把内存里的 Module 编码成 .lgb 字节。
/// 步骤：
/// 1. 写文件头：魔数 LGB1、版本、main 下标
/// 2. 写结构体布局表
/// 3. 对每个函数：名字、arity、void、常量池、指令、行号表
pub fn encode_module(module: &Module) -> Result<Vec<u8>, BytecodeError> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    let main = module.main_index.unwrap_or(NO_MAIN as usize);
    if main > NO_MAIN as usize {
        return Err(err("main_index too large"));
    }
    out.extend_from_slice(&(main as u16).to_le_bytes());
    out.extend_from_slice(&(module.toplevel_index as u16).to_le_bytes());

    // struct types
    if module.struct_types.len() > u16::MAX as usize {
        return Err(err("too many struct types"));
    }
    out.extend_from_slice(&(module.struct_types.len() as u16).to_le_bytes());
    for st in &module.struct_types {
        write_str(&mut out, &st.name)?;
        if st.fields.len() > u16::MAX as usize {
            return Err(err("too many fields"));
        }
        out.extend_from_slice(&(st.fields.len() as u16).to_le_bytes());
        for f in &st.fields {
            write_str(&mut out, f)?;
        }
    }

    // functions
    if module.functions.len() > u16::MAX as usize {
        return Err(err("too many functions"));
    }
    out.extend_from_slice(&(module.functions.len() as u16).to_le_bytes());
    for f in &module.functions {
        write_str(&mut out, &f.name)?;
        out.push(f.arity);
        out.push(if f.is_void { 1 } else { 0 });
        out.extend_from_slice(&f.locals.to_le_bytes());
        if f.chunk.constants.len() > u16::MAX as usize {
            return Err(err("too many constants"));
        }
        out.extend_from_slice(&(f.chunk.constants.len() as u16).to_le_bytes());
        for c in &f.chunk.constants {
            write_const(&mut out, c)?;
        }
        let code_len = f.chunk.code.len();
        if code_len > u32::MAX as usize {
            return Err(err("code too large"));
        }
        out.extend_from_slice(&(code_len as u32).to_le_bytes());
        out.extend_from_slice(&f.chunk.code);
        let lines_len = f.chunk.lines.len();
        out.extend_from_slice(&(lines_len as u32).to_le_bytes());
        for l in &f.chunk.lines {
            out.extend_from_slice(&l.to_le_bytes());
        }
    }
    Ok(out)
}

/// 从 `.lgb` 字节解码为 `Module`。
/// 从 .lgb 字节解码回 Module。
/// 步骤：校验魔数与版本 → 读入口信息 → 读结构体表 → 读函数表 → 校验下标范围
pub fn decode_module(bytes: &[u8]) -> Result<Module, BytecodeError> {
    let mut r = Reader { buf: bytes, pos: 0 };
    let magic = r.read_bytes(4)?;
    if magic != MAGIC {
        return Err(err(format!(
            "bad magic (expected LGB1), got {:?}",
            String::from_utf8_lossy(magic)
        )));
    }
    let version = r.u16()?;
    if version != FORMAT_VERSION {
        return Err(err(format!(
            "unsupported .lgb version {version} (this build supports {FORMAT_VERSION})"
        )));
    }
    let _reserved = r.u16()?;
    let main_raw = r.u16()?;
    let main_index = if main_raw == NO_MAIN {
        None
    } else {
        Some(main_raw as usize)
    };
    let toplevel_index = r.u16()? as usize;

    let nst = r.u16()? as usize;
    let mut struct_types = Vec::with_capacity(nst);
    for _ in 0..nst {
        let name = r.string()?;
        let nf = r.u16()? as usize;
        let mut fields = Vec::with_capacity(nf);
        for _ in 0..nf {
            fields.push(r.string()?);
        }
        struct_types.push(StructType { name, fields });
    }

    let nfns = r.u16()? as usize;
    let mut functions = Vec::with_capacity(nfns);
    for _ in 0..nfns {
        let name = r.string()?;
        let arity = r.u8()?;
        let is_void = r.u8()? != 0;
        let locals = r.u16()?;
        let nc = r.u16()? as usize;
        let mut constants = Vec::with_capacity(nc);
        for _ in 0..nc {
            constants.push(read_const(&mut r)?);
        }
        let code_len = r.u32()? as usize;
        let code = r.read_bytes(code_len)?.to_vec();
        let lines_len = r.u32()? as usize;
        if lines_len != code_len && lines_len != 0 {
            // 允许 lines 与 code 等长；不等长时报错（避免静默错位）
            return Err(err(format!(
                "function `{name}`: lines_len {lines_len} != code_len {code_len}"
            )));
        }
        let mut lines = Vec::with_capacity(lines_len);
        for _ in 0..lines_len {
            lines.push(r.u32()?);
        }
        functions.push(Function {
            name,
            arity,
            locals,
            is_void,
            chunk: Chunk {
                code,
                constants,
                lines,
            },
        });
    }

    if toplevel_index >= functions.len() && !functions.is_empty() {
        return Err(err("toplevel_index out of range"));
    }
    if let Some(mi) = main_index {
        if mi >= functions.len() {
            return Err(err("main_index out of range"));
        }
    }

    Ok(Module {
        functions,
        main_index,
        toplevel_index,
        struct_types,
    })
}

/// 写入文件（路径以 `.lgb` 结尾时原样使用，否则追加）。
pub fn write_lgb(path: &Path, bytes: &[u8]) -> Result<(), BytecodeError> {
    std::fs::write(path, bytes).map_err(|e| err(format!("cannot write {}: {e}", path.display())))
}

/// 读取 `.lgb` 文件并解码。
pub fn load_lgb(path: &Path) -> Result<Module, BytecodeError> {
    let bytes =
        std::fs::read(path).map_err(|e| err(format!("cannot read {}: {e}", path.display())))?;
    decode_module(&bytes)
}

/// 执行已解码的 Module，返回 print 行。
/// 在内存 Module 上跑 VM（.lgb 与 .lgpack 共用）。
pub fn exec_module(module: &Module) -> Result<Vec<String>, String> {
    let mut vm = Vm::new(module);
    vm.run().map_err(|e| {
        if e.line == 0 {
            format!("运行时错误: {}", e.message)
        } else {
            format!("运行时错误（第 {} 行）: {}", e.line, e.message)
        }
    })
}

/// 根据源文件路径推导默认输出：`foo.lg` → `foo.lgb`
pub fn default_output_path(src: &Path) -> std::path::PathBuf {
    let mut p = src.to_path_buf();
    p.set_extension("lgb");
    p
}

/// 反汇编整个 Module（文本，与 CLI disasm 一致）。
pub fn disassemble_module(module: &Module) -> String {
    let mut out = String::new();
    for f in &module.functions {
        let label = if f.name == "$toplevel" {
            "<toplevel>".to_string()
        } else {
            format!(
                "fn {} (arity={}, locals={}, void={})",
                f.name, f.arity, f.locals, f.is_void
            )
        };
        out.push_str(&f.chunk.disassemble(&label));
        out.push('\n');
    }
    out
}

fn write_str(out: &mut Vec<u8>, s: &str) -> Result<(), BytecodeError> {
    if s.len() > u16::MAX as usize {
        return Err(err("string too long"));
    }
    out.extend_from_slice(&(s.len() as u16).to_le_bytes());
    out.extend_from_slice(s.as_bytes());
    Ok(())
}

fn write_const(out: &mut Vec<u8>, v: &Value) -> Result<(), BytecodeError> {
    match v {
        Value::Int(n) => {
            out.push(0);
            out.extend_from_slice(&n.to_le_bytes());
        }
        Value::Float(n) => {
            out.push(1);
            out.extend_from_slice(&n.to_le_bytes());
        }
        Value::Bool(b) => {
            out.push(2);
            out.push(if *b { 1 } else { 0 });
        }
        Value::Str(s) => {
            out.push(3);
            write_str(out, s)?;
        }
        other => {
            return Err(err(format!(
                "constant of type {} cannot be stored in .lgb",
                other.type_name()
            )))
        }
    }
    Ok(())
}

fn read_const(r: &mut Reader<'_>) -> Result<Value, BytecodeError> {
    match r.u8()? {
        0 => Ok(Value::Int(r.i64()?)),
        1 => Ok(Value::Float(r.f64()?)),
        2 => Ok(Value::Bool(r.u8()? != 0)),
        3 => {
            let s = r.string()?;
            Ok(Value::Str(std::rc::Rc::from(s.as_str())))
        }
        t => Err(err(format!("unknown constant tag {t}"))),
    }
}

struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn read_bytes(&mut self, n: usize) -> Result<&'a [u8], BytecodeError> {
        if self.pos + n > self.buf.len() {
            return Err(err("truncated .lgb file"));
        }
        let s = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    fn u8(&mut self) -> Result<u8, BytecodeError> {
        Ok(self.read_bytes(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, BytecodeError> {
        let b = self.read_bytes(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn u32(&mut self) -> Result<u32, BytecodeError> {
        let b = self.read_bytes(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn i64(&mut self) -> Result<i64, BytecodeError> {
        let b = self.read_bytes(8)?;
        let mut a = [0u8; 8];
        a.copy_from_slice(b);
        Ok(i64::from_le_bytes(a))
    }

    fn f64(&mut self) -> Result<f64, BytecodeError> {
        let b = self.read_bytes(8)?;
        let mut a = [0u8; 8];
        a.copy_from_slice(b);
        Ok(f64::from_le_bytes(a))
    }

    fn string(&mut self) -> Result<String, BytecodeError> {
        let n = self.u16()? as usize;
        let b = self.read_bytes(n)?;
        String::from_utf8(b.to_vec()).map_err(|_| err("invalid UTF-8 in .lgb"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module_loader::compile_file;

    #[test]
    fn roundtrip_fib() {
        let m = compile_file("examples/fib.lg").expect("compile");
        let bytes = encode_module(&m).expect("encode");
        assert!(bytes.starts_with(b"LGB1"));
        let m2 = decode_module(&bytes).expect("decode");
        assert_eq!(m2.functions.len(), m.functions.len());
        assert!(m2.main_index.is_some());
        let out = exec_module(&m2).expect("exec");
        assert_eq!(out, vec!["55"]);
    }

    #[test]
    fn bad_magic() {
        assert!(decode_module(b"NOPExxxx").is_err());
    }
}
