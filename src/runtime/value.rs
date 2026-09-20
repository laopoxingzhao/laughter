//! 运行时值：栈上到底放了什么。
//!
//! **标量**：`Int/Float/Bool/Str`，直接放在栈上，拷贝很便宜。
//!
//! **数组（引用语义）**  
//! 栈上放的是 `Rc<RefCell<Vec<Value>>>`——可以理解成「遥控器」。  
//! `let b = a` 只是又拿了一个遥控器；`b[0] = 1` 会改到同一台「电视」。
//!
//! **结构体（值语义）**  
//! 字段直接内嵌在 `StructVal` 里。`let b = a` 是整份字段拷贝（复印件）。  
//! 改 `b.x` 不会影响 `a.x`。  
//! 注意：若字段本身是 string/数组，拷贝的是「遥控器」，仍会共享底层数据（浅拷贝）。
//!
//! `display()` 的输出即 `print`/`to_string` 看到的内容。

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

pub type ArrayHandle = Rc<RefCell<Vec<Value>>>;

/// 结构体实例（值语义：整份拷贝，不是遥控器）。
///
/// - `name`：结构体类型名，例如 `"Point"`（方法查找时用它拼 `Point.sum`）
/// - `fields`：字段表 `(字段名, 字段值)`，顺序与声明一致
#[derive(Debug, Clone)]
pub struct StructVal {
    pub name: String,
    pub fields: Vec<(String, Value)>,
}

impl StructVal {
    /// 按字段名查找字段值（不修改结构体）。
    pub fn get(&self, f: &str) -> Option<&Value> {
        self.fields.iter().find(|(n, _)| n == f).map(|(_, v)| v)
    }
    /// 按字段名写入；成功 true，字段不存在 false。
    pub fn set(&mut self, f: &str, v: Value) -> bool {
        if let Some(s) = self.fields.iter_mut().find(|(n, _)| n == f) {
            s.1 = v;
            true
        } else {
            false
        }
    }
}

/// 栈上运行时的一个值。`display()` 的结果就是 `print` 打出来的内容。
///
/// | 变体 | 中文 | 语义 |
/// |------|------|------|
/// | `Int(i64)` | 64 位整数 | 直接拷贝 |
/// | `Float(f64)` | 64 位浮点 | 直接拷贝 |
/// | `Bool(bool)` | 布尔 | 直接拷贝 |
/// | `Str(Rc<str>)` | 字符串 | 共享只读数据 |
/// | `Array(Rc<RefCell<Vec<..>>>)` | 数组 | **引用**：多个变量可改同一数组 |
/// | `Struct(StructVal)` | 结构体 | **值**：赋值时字段拷贝 |
#[derive(Debug, Clone)]
pub enum Value {
    /// 整数
    Int(i64),
    /// 浮点数
    Float(f64),
    /// 真/假
    Bool(bool),
    /// 字符串（引用计数，只读共享）
    Str(Rc<str>),
    /// 数组句柄（遥控器）
    Array(ArrayHandle),
    /// 结构体值（复印件）
    Struct(StructVal),
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Bool(_) => "bool",
            Value::Str(_) => "string",
            Value::Array(_) => "array",
            Value::Struct(_) => "struct",
        }
    }

    /// 显示成 print/to_string 的文本（数组 `[a, b]`，结构体 `Name { f: v }`）。
    pub fn display(&self) -> String {
        match self {
            Value::Int(n) => n.to_string(),
            Value::Float(n) => {
                if n.fract() == 0.0 && n.is_finite() {
                    format!("{n:.1}")
                } else {
                    n.to_string()
                }
            }
            Value::Bool(b) => b.to_string(),
            Value::Str(s) => s.to_string(),
            Value::Array(a) => {
                let items: Vec<String> = a.borrow().iter().map(|v| v.display()).collect();
                format!("[{}]", items.join(", "))
            }
            Value::Struct(s) => {
                let items: Vec<String> = s
                    .fields
                    .iter()
                    .map(|(n, v)| format!("{n}: {}", v.display()))
                    .collect();
                format!("{} {{ {} }}", s.name, items.join(", "))
            }
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display())
    }
}
