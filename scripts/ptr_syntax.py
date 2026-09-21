#!/usr/bin/env python3
"""Complete pointer + modern syntax implementation patches."""
from pathlib import Path

def patch(path, pairs):
    p = Path(path)
    t = p.read_text(encoding="utf-8")
    for old, new in pairs:
        if old not in t:
            print("MISS", path, old.splitlines()[0][:50])
            continue
        t = t.replace(old, new, 1)
    p.write_text(t, encoding="utf-8")
    print("ok", path)

# --- op.rs: remove duplicate CallMethod at line 63 ---
patch("src/codegen/op.rs", [
(
"""    Loop,
    Call,
    CallMethod,
    Return,
""",
"""    Loop,
    Call,
    Return,
""",
),
])

# --- parser/decl.rs ty() ---
patch("src/syntax/parser/decl.rs", [
(
"""    pub(crate) fn ty(&mut self) -> Result<TypeExpr, ParseError> {
        let t = self.advance();
        let base = match &t.kind {
""",
"""    pub(crate) fn ty(&mut self) -> Result<TypeExpr, ParseError> {
        // `&T` / `&mut T`：先吃 &，可选 mut，再解析内层类型
        if self.check(&TokenKind::Amp) {
            let at = self.advance();
            let mutable = if self.check(&TokenKind::Mut) {
                self.advance();
                true
            } else {
                false
            };
            let inner = self.ty()?;
            return Ok(TypeExpr::Ref {
                mutable,
                inner: Box::new(inner),
            });
            // at reserved for future span
            #[allow(unreachable_code)]
            {
                let _ = at;
                unreachable!()
            }
        }
        let t = self.advance();
        let base = match &t.kind {
""",
),
])

# --- parser/mod.rs fun keyword Fn ---
patch("src/syntax/parser/mod.rs", [
(
"            if self.check(&TokenKind::Fun) {",
"            if self.check(&TokenKind::Fun) || self.check(&TokenKind::Fn) {",
),
])

# --- parser/expr.rs primary + unary ---
patch("src/syntax/parser/expr.rs", [
(
"""    pub(crate) fn unary(&mut self) -> Result<Expr, ParseError> {
""",
"""    /// 一元：`-` `!` 以及 `&` `&mut` `*`（指针）
    pub(crate) fn unary(&mut self) -> Result<Expr, ParseError> {
        if self.check(&TokenKind::Amp) {
            let t = self.advance();
            let mutable = if self.check(&TokenKind::Mut) {
                self.advance();
                true
            } else {
                false
            };
            let target = self.unary()?;
            return Ok(Expr::Ref {
                mutable,
                target: Box::new(target),
                span: t.span,
            });
        }
        if self.check(&TokenKind::Star) {
            let t = self.advance();
            let ptr = self.unary()?;
            return Ok(Expr::Deref {
                ptr: Box::new(ptr),
                span: t.span,
            });
        }
""",
),
(
"""            TokenKind::Str(v) => {
""",
"""            TokenKind::InterpStr(raw) => {
                self.advance();
                let parts = crate::syntax::parser::expr::split_interp(&raw, t.span)?;
                return Ok(Expr::Interp {
                    parts,
                    span: t.span,
                });
            }
            TokenKind::Nil => {
                self.advance();
                return Ok(Expr::Nil { span: t.span });
            }
            TokenKind::Str(v) => {
""",
),
(
"""                            let n = self.expect_ident()?;
                            self.expect(TokenKind::Colon, "`:`")?;
                            let v = self.expr()?;
                            fields.push((n, v));
""",
"""                            let n = self.expect_ident()?;
                            // 简写 `Point { x, y }` 或完整 `Point { x: 1 }`
                            if self.check(&TokenKind::Colon) {
                                self.advance();
                                let v = self.expr()?;
                                fields.push(FieldInit {
                                    name: n,
                                    value: Some(v),
                                });
                            } else {
                                fields.push(FieldInit {
                                    name: n,
                                    value: None,
                                });
                            }
""",
),
])

# --- helper split_interp at end of parser/expr.rs ---
patch("src/syntax/parser/expr.rs", [
(
"impl Parser {\n",
"""/// 把 `s"..."` 的原始内容拆成字面量片段与 `{expr}` 片段。
/// 表达式再交给 Parser 二次解析。
pub(crate) fn split_interp(raw: &str, span: Span) -> Result<Vec<InterpPart>, ParseError> {
    let mut parts = vec![];
    let mut buf = String::new();
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' {
            if !buf.is_empty() {
                parts.push(InterpPart::Text(std::mem::take(&mut buf)));
            }
            let mut expr_src = String::new();
            let mut closed = false;
            while let Some(ch) = chars.next() {
                if ch == '}' {
                    closed = true;
                    break;
                }
                expr_src.push(ch);
            }
            if !closed {
                return Err(ParseError {
                    message: "插值表达式缺少 `}`".into(),
                    span,
                });
            }
            let toks = crate::syntax::lexer::Lexer::new(&expr_src)
                .tokenize()
                .map_err(|e| ParseError {
                    message: format!("插值表达式词法错误: {}", e.message),
                    span,
                })?;
            let mut p = Parser::new(toks);
            let e = p.expr().map_err(|e| ParseError {
                message: format!("插值表达式语法错误: {}", e.message),
                span,
            })?;
            parts.push(InterpPart::Expr(e));
        } else if c == '\\\\' {
            match chars.next() {
                Some('n') => buf.push('\\n'),
                Some('t') => buf.push('\\t'),
                Some('\\\\') => buf.push('\\\\'),
                Some('"') => buf.push('"'),
                Some('{') => buf.push('{'),
                Some('}') => buf.push('}'),
                _ => {
                    return Err(ParseError {
                        message: "插值字符串转义无效".into(),
                        span,
                    })
                }
            }
        } else {
            buf.push(c);
        }
    }
    if !buf.is_empty() {
        parts.push(InterpPart::Text(buf));
    }
    Ok(parts)
}

impl Parser {
""",
),
])

# --- parser/stmt.rs: via_deref + implicit return ---
patch("src/syntax/parser/stmt.rs", [
(
"""                    return Ok(Stmt::Assign(AssignStmt {
                        name,
                        index: None,
                        fields: vec![],
                        value,
                        span: semi.span,
                    }));
""",
"""                    return Ok(Stmt::Assign(AssignStmt {
                        name,
                        index: None,
                        fields: vec![],
                        value,
                        span: semi.span,
                        via_deref: false,
                    }));
""",
),
(
"""                    return Ok(Stmt::Assign(AssignStmt {
                        name,
                        index: Some(index),
                        fields,
                        value,
                        span: semi.span,
                    }));
""",
"""                    return Ok(Stmt::Assign(AssignStmt {
                        name,
                        index: Some(index),
                        fields,
                        value,
                        span: semi.span,
                        via_deref: false,
                    }));
""",
),
(
"""                    return Ok(Stmt::Assign(AssignStmt {
                        name,
                        index: None,
                        fields,
                        value,
                        span: semi.span,
                    }));
""",
"""                    return Ok(Stmt::Assign(AssignStmt {
                        name,
                        index: None,
                        fields,
                        value,
                        span: semi.span,
                        via_deref: false,
                    }));
""",
),
(
"""        let expr = self.expr()?;
        let span = expr.span();
        self.expect(TokenKind::Semi, "`;` after expression")?;
        Ok(Stmt::Expr(ExprStmt { expr, span }))
    }
""",
"""        // `*p = v`：解引用赋值
        if self.check(&TokenKind::Star) {
            let save = self.pos;
            self.advance();
            let ptr = self.expr()?;
            if self.check(&TokenKind::Assign) {
                self.advance();
                let value = self.expr()?;
                let semi = self.expect(TokenKind::Semi, "`;`")?;
                // 用临时名字表示通过指针赋值；编译器看 via_deref + value
                return Ok(Stmt::Assign(AssignStmt {
                    name: Ident {
                        name: String::new(),
                        span: semi.span,
                    },
                    index: None,
                    fields: vec![],
                    value: Expr::Binary {
                        op: crate::syntax::ast::BinOp::Eq,
                        lhs: Box::new(ptr),
                        rhs: Box::new(value),
                        span: semi.span,
                    },
                    span: semi.span,
                    via_deref: true,
                }));
            }
            self.pos = save;
        }

        let expr = self.expr()?;
        let span = expr.span();
        // 无分号且后面是 `}`：函数体末尾隐式 return
        if self.check(&TokenKind::RBrace) || self.check(&TokenKind::Eof) {
            return Ok(Stmt::Expr(ExprStmt {
                expr,
                span,
                implicit_return: true,
            }));
        }
        self.expect(TokenKind::Semi, "`;` after expression")?;
        Ok(Stmt::Expr(ExprStmt {
            expr,
            span,
            implicit_return: false,
        }))
    }
""",
),
])

print("syntax patches done")
