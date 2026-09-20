//! 运行时值：VM 栈上的数据。
//! 数组用 `Rc<RefCell<Vec<Value>>>`，调用时按句柄共享（引用语义），便于教学实现且无需 GC。

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

pub type ArrayHandle = Rc<RefCell<Vec<Value>>>;

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(Rc<str>),
    Array(ArrayHandle),
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Bool(_) => "bool",
            Value::Str(_) => "string",
            Value::Array(_) => "array",
        }
    }

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
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display())
    }
}
