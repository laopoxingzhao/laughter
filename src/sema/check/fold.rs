//! const 表达式折叠。
use super::*;

impl<'a> Checker<'a> {
    pub(crate) fn fold(&self, e: &Expr) -> Result<(Type, Value), CheckError> {
        match e {
            Expr::Int { value, .. } => Ok((Type::Int, Value::Int(*value))),
            Expr::Float { value, .. } => Ok((Type::Float, Value::Float(*value))),
            Expr::Bool { value, .. } => Ok((Type::Bool, Value::Bool(*value))),
            Expr::Str { value, .. } => Ok((Type::Str, Value::Str(Rc::from(value.as_str())))),
            Expr::Var { name } => self
                .consts
                .get(&name.name)
                .cloned()
                .ok_or_else(|| CheckError {
                    message: format!("`{}` is not a compile-time constant", name.name),
                    span: name.span,
                }),
            Expr::Unary { op, expr, span } => {
                let (t, v) = self.fold(expr)?;
                match (op, t, v) {
                    (UnOp::Neg, Type::Int, Value::Int(n)) => Ok((Type::Int, Value::Int(-n))),
                    (UnOp::Neg, Type::Float, Value::Float(n)) => {
                        Ok((Type::Float, Value::Float(-n)))
                    }
                    (UnOp::Not, Type::Bool, Value::Bool(b)) => Ok((Type::Bool, Value::Bool(!b))),
                    _ => Err(CheckError {
                        message: "invalid unary in const".into(),
                        span: *span,
                    }),
                }
            }
            Expr::Binary { op, lhs, rhs, span } => {
                let (lt, lv) = self.fold(lhs)?;
                let (rt, rv) = self.fold(rhs)?;
                fold_bin(*op, lt, lv, rt, rv).map_err(|m| CheckError {
                    message: m,
                    span: *span,
                })
            }
            other => Err(CheckError {
                message: "not a compile-time constant expression".into(),
                span: other.span(),
            }),
        }
    }
}

fn fold_bin(op: BinOp, lt: Type, lv: Value, rt: Type, rv: Value) -> Result<(Type, Value), String> {
    use BinOp::*;
    match (op, &lv, &rv) {
        (Add, Value::Int(a), Value::Int(b)) => Ok((Type::Int, Value::Int(a.wrapping_add(*b)))),
        (Sub, Value::Int(a), Value::Int(b)) => Ok((Type::Int, Value::Int(a.wrapping_sub(*b)))),
        (Mul, Value::Int(a), Value::Int(b)) => Ok((Type::Int, Value::Int(a.wrapping_mul(*b)))),
        (Div, Value::Int(a), Value::Int(b)) if *b != 0 => {
            Ok((Type::Int, Value::Int(a.wrapping_div(*b))))
        }
        (Rem, Value::Int(a), Value::Int(b)) if *b != 0 => {
            Ok((Type::Int, Value::Int(a.wrapping_rem(*b))))
        }
        (Div | Rem, Value::Int(_), Value::Int(0)) => Err("division by zero in const".into()),
        (Add, Value::Float(a), Value::Float(b)) => Ok((Type::Float, Value::Float(a + b))),
        (Sub, Value::Float(a), Value::Float(b)) => Ok((Type::Float, Value::Float(a - b))),
        (Mul, Value::Float(a), Value::Float(b)) => Ok((Type::Float, Value::Float(a * b))),
        (Div, Value::Float(a), Value::Float(b)) if *b != 0.0 => {
            Ok((Type::Float, Value::Float(a / b)))
        }
        (Add, Value::Str(a), Value::Str(b)) => {
            let mut s = a.to_string();
            s.push_str(b);
            Ok((Type::Str, Value::Str(Rc::from(s.as_str()))))
        }
        (Eq, Value::Int(a), Value::Int(b)) => Ok((Type::Bool, Value::Bool(a == b))),
        (Ne, Value::Int(a), Value::Int(b)) => Ok((Type::Bool, Value::Bool(a != b))),
        (Lt, Value::Int(a), Value::Int(b)) => Ok((Type::Bool, Value::Bool(a < b))),
        (Le, Value::Int(a), Value::Int(b)) => Ok((Type::Bool, Value::Bool(a <= b))),
        (Gt, Value::Int(a), Value::Int(b)) => Ok((Type::Bool, Value::Bool(a > b))),
        (Ge, Value::Int(a), Value::Int(b)) => Ok((Type::Bool, Value::Bool(a >= b))),
        (Eq, Value::Bool(a), Value::Bool(b)) => Ok((Type::Bool, Value::Bool(a == b))),
        (Ne, Value::Bool(a), Value::Bool(b)) => Ok((Type::Bool, Value::Bool(a != b))),
        (And, Value::Bool(a), Value::Bool(b)) => Ok((Type::Bool, Value::Bool(*a && *b))),
        (Or, Value::Bool(a), Value::Bool(b)) => Ok((Type::Bool, Value::Bool(*a || *b))),
        _ => Err(format!("cannot fold `{lt}` {op:?} `{rt}`")),
    }
}
