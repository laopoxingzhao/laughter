//! 递归下降语法分析：Token 流 → AST。
//!
//! **在做什么**：把一串「词」组装成语法树。  
//! 递归下降 = 每个语法规则对应一个函数，函数之间互相调用。
//!
//! **举例**：`1 + 2 * 3` 会解析成树：`Add(1, Mul(2, 3))`，因为 `*` 优先级更高。
//!
//! **优先级（低→高）**：  
//! `||` → `&&` → `== !=` → 比较 → `+ -` → `* / %` → 一元 `- !` → `[]` `. ()`
//!
//! **易错点**：`for x in arr { ... }` 里 `arr {` 不能被当成结构体字面量。  
//! 所以只有 `{` 后面紧跟 `字段名:` 时，才认作结构体字面量（见 `struct_lit_ahead`）。

use crate::syntax::ast::*;
use crate::syntax::token::{Span, Token, TokenKind};

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

/// 解析器状态：`tokens` 为完整 Token 流，`pos` 为当前下标。
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    /// 解析整个程序；循环识别顶层 item，直到 `Eof`。
    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut items = Vec::new();
        while !self.check(&TokenKind::Eof) {
            if self.check(&TokenKind::Struct) {
                items.push(Item::Struct(self.struct_decl()?));
            } else if self.check(&TokenKind::Const) {
                items.push(Item::Const(self.const_decl()?));
            } else if self.check(&TokenKind::Fun) {
                items.push(Item::Fun(self.fun_decl()?));
            } else if self.check(&TokenKind::Import) {
                items.push(Item::Import(self.import_item()?));
            } else {
                items.push(Item::Stmt(self.stmt()?));
            }
        }
        Ok(Program { items })
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    fn check(&self, k: &TokenKind) -> bool {
        std::mem::discriminant(self.kind()) == std::mem::discriminant(k)
    }

    fn advance(&mut self) -> Token {
        let t = self.tokens[self.pos.min(self.tokens.len() - 1)].clone();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        t
    }

    fn expect(&mut self, k: TokenKind, what: &str) -> Result<Token, ParseError> {
        if self.check(&k) {
            Ok(self.advance())
        } else {
            Err(ParseError {
                message: format!("expected {what}, found {}", self.kind()),
                span: self.peek().span,
            })
        }
    }

    fn expect_ident(&mut self) -> Result<Ident, ParseError> {
        let t = self.peek().clone();
        if let TokenKind::Ident(n) = t.kind {
            self.advance();
            Ok(Ident {
                name: n,
                span: t.span,
            })
        } else {
            Err(ParseError {
                message: format!("expected identifier, found {}", t.kind),
                span: t.span,
            })
        }
    }

    /// 仅 `{ ident :` 视为结构体字面量；`{ }` 留给控制流空块。
    fn struct_lit_ahead(&self) -> bool {
        if self.pos + 2 >= self.tokens.len() {
            return false;
        }
        matches!(&self.tokens[self.pos + 1].kind, TokenKind::Ident(_))
            && matches!(&self.tokens[self.pos + 2].kind, TokenKind::Colon)
    }

    /// 解析类型：标量关键字或 `Named`（结构体名），可跟 `[]` 表示数组。
    fn ty(&mut self) -> Result<TypeExpr, ParseError> {
        let t = self.advance();
        let base = match &t.kind {
            TokenKind::TyInt => TypeExpr::Int,
            TokenKind::TyFloat => TypeExpr::Float,
            TokenKind::TyBool => TypeExpr::Bool,
            TokenKind::TyString => TypeExpr::String,
            TokenKind::TyVoid => TypeExpr::Void,
            TokenKind::Ident(n) => TypeExpr::Named(n.clone()),
            _ => {
                return Err(ParseError {
                    message: format!("expected a type, found {}", t.kind),
                    span: t.span,
                })
            }
        };
        if self.check(&TokenKind::LBracket) {
            self.advance();
            self.expect(TokenKind::RBracket, "`]`")?;
            if base.is_void() {
                return Err(ParseError {
                    message: "`void[]` is not a type".into(),
                    span: t.span,
                });
            }
            Ok(TypeExpr::Array(Box::new(base)))
        } else {
            Ok(base)
        }
    }

    fn import_item(&mut self) -> Result<ImportItem, ParseError> {
        let start = self.expect(TokenKind::Import, "`import`")?.span;
        let t = self.advance();
        let path = match t.kind {
            TokenKind::Str(s) => s,
            other => {
                return Err(ParseError {
                    message: format!("expected string path, found {other}"),
                    span: t.span,
                })
            }
        };
        if path.split('/').any(|s| s == "..") {
            return Err(ParseError {
                message: "import path must not contain `..`".into(),
                span: t.span,
            });
        }
        let alias = if self.check(&TokenKind::As) {
            self.advance();
            Some(self.expect_ident()?)
        } else {
            None
        };
        self.expect(TokenKind::Semi, "`;` after import")?;
        Ok(ImportItem {
            path,
            alias,
            span: start,
        })
    }

    fn struct_decl(&mut self) -> Result<StructDecl, ParseError> {
        let start = self.expect(TokenKind::Struct, "`struct`")?.span;
        let name = self.expect_ident()?;
        self.expect(TokenKind::LBrace, "`{`")?;
        let mut fields = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            let fname = self.expect_ident()?;
            self.expect(TokenKind::Colon, "`:`")?;
            let ty = self.ty()?;
            if ty.is_void() {
                return Err(ParseError {
                    message: "field type cannot be void".into(),
                    span: fname.span,
                });
            }
            fields.push(FieldDecl { name: fname, ty });
            if self.check(&TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        self.expect(TokenKind::RBrace, "`}`")?;
        if self.check(&TokenKind::Semi) {
            self.advance();
        }
        Ok(StructDecl {
            name,
            fields,
            span: start,
        })
    }

    fn const_decl(&mut self) -> Result<ConstDecl, ParseError> {
        let start = self.expect(TokenKind::Const, "`const`")?.span;
        let name = self.expect_ident()?;
        self.expect(TokenKind::Colon, "`:`")?;
        let ty = self.ty()?;
        self.expect(TokenKind::Assign, "`=`")?;
        let value = self.expr()?;
        self.expect(TokenKind::Semi, "`;`")?;
        Ok(ConstDecl {
            name,
            ty,
            value,
            span: start,
        })
    }

    /// `fun` 声明：支持 `fun name(...)` 与方法 `fun Type.name(self: Type, ...)`。
    fn fun_decl(&mut self) -> Result<FunDecl, ParseError> {
        let start = self.expect(TokenKind::Fun, "`fun`")?.span;
        let first = self.expect_ident()?;
        let (on_type, name) = if self.check(&TokenKind::Dot) {
            self.advance();
            let m = self.expect_ident()?;
            (Some(first), m)
        } else {
            (None, first)
        };
        self.expect(TokenKind::LParen, "`(`")?;
        let mut params = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                let n = self.expect_ident()?;
                self.expect(TokenKind::Colon, "`:`")?;
                let ty = self.ty()?;
                params.push(Param { name: n, ty });
                if self.check(&TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.expect(TokenKind::RParen, "`)`")?;
        self.expect(TokenKind::Arrow, "`->`")?;
        let ret = self.ty()?;
        let body = self.block()?;
        Ok(FunDecl {
            name,
            on_type,
            params,
            ret,
            body,
            span: start,
        })
    }

    fn block(&mut self) -> Result<Block, ParseError> {
        let start = self.expect(TokenKind::LBrace, "`{`")?.span;
        let mut stmts = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            stmts.push(self.stmt()?);
        }
        self.expect(TokenKind::RBrace, "`}`")?;
        Ok(Block { stmts, span: start })
    }

    /// 语句解析入口。
    ///
    /// 顺序很重要：先认 `let/if/while/...` 这些「以关键字开头」的语句；
    /// 再看是不是赋值（`x = ...`、`a[i] = ...`、`p.f = ...`）；
    /// 剩下的当作表达式语句（例如 `print(1);`），结尾必须有 `;`。
    fn stmt(&mut self) -> Result<Stmt, ParseError> {
        if self.check(&TokenKind::Let) {
            return Ok(Stmt::Let(self.let_stmt()?));
        }
        if self.check(&TokenKind::If) {
            return Ok(Stmt::If(self.if_stmt()?));
        }
        if self.check(&TokenKind::While) {
            return Ok(Stmt::While(self.while_stmt()?));
        }
        if self.check(&TokenKind::For) {
            return Ok(Stmt::For(self.for_stmt()?));
        }
        if self.check(&TokenKind::Break) {
            let t = self.advance();
            self.expect(TokenKind::Semi, "`;`")?;
            return Ok(Stmt::Break(t.span));
        }
        if self.check(&TokenKind::Continue) {
            let t = self.advance();
            self.expect(TokenKind::Semi, "`;`")?;
            return Ok(Stmt::Continue(t.span));
        }
        if self.check(&TokenKind::Return) {
            let t = self.advance();
            let value = if self.check(&TokenKind::Semi) {
                None
            } else {
                Some(self.expr()?)
            };
            self.expect(TokenKind::Semi, "`;`")?;
            return Ok(Stmt::Return(ReturnStmt {
                value,
                span: t.span,
            }));
        }
        if self.check(&TokenKind::LBrace) {
            return Ok(Stmt::Block(self.block()?));
        }

        // assign forms
        if matches!(self.kind(), TokenKind::Ident(_)) {
            let save = self.pos;
            let name = self.expect_ident()?;
            if self.check(&TokenKind::Assign) {
                self.advance();
                let value = self.expr()?;
                let semi = self.expect(TokenKind::Semi, "`;`")?;
                return Ok(Stmt::Assign(AssignStmt {
                    name,
                    index: None,
                    fields: vec![],
                    value,
                    span: semi.span,
                }));
            }
            if self.check(&TokenKind::LBracket) {
                self.advance();
                let index = self.expr()?;
                self.expect(TokenKind::RBracket, "`]`")?;
                let mut fields = vec![];
                while self.check(&TokenKind::Dot) {
                    self.advance();
                    fields.push(self.expect_ident()?);
                }
                if self.check(&TokenKind::Assign) {
                    self.advance();
                    let value = self.expr()?;
                    let semi = self.expect(TokenKind::Semi, "`;`")?;
                    return Ok(Stmt::Assign(AssignStmt {
                        name,
                        index: Some(index),
                        fields,
                        value,
                        span: semi.span,
                    }));
                }
            }
            if self.check(&TokenKind::Dot) {
                let mut fields = vec![];
                while self.check(&TokenKind::Dot) {
                    self.advance();
                    fields.push(self.expect_ident()?);
                }
                if self.check(&TokenKind::Assign) {
                    self.advance();
                    let value = self.expr()?;
                    let semi = self.expect(TokenKind::Semi, "`;`")?;
                    return Ok(Stmt::Assign(AssignStmt {
                        name,
                        index: None,
                        fields,
                        value,
                        span: semi.span,
                    }));
                }
            }
            self.pos = save;
        }

        let expr = self.expr()?;
        let span = expr.span();
        self.expect(TokenKind::Semi, "`;` after expression")?;
        Ok(Stmt::Expr(ExprStmt { expr, span }))
    }

    fn let_stmt(&mut self) -> Result<LetStmt, ParseError> {
        let start = self.expect(TokenKind::Let, "`let`")?.span;
        let name = self.expect_ident()?;
        let ty = if self.check(&TokenKind::Colon) {
            self.advance();
            Some(self.ty()?)
        } else {
            None
        };
        self.expect(TokenKind::Assign, "`=`")?;
        let value = self.expr()?;
        self.expect(TokenKind::Semi, "`;`")?;
        Ok(LetStmt {
            name,
            ty,
            value,
            span: start,
        })
    }

    fn if_stmt(&mut self) -> Result<IfStmt, ParseError> {
        let start = self.expect(TokenKind::If, "`if`")?.span;
        let cond = self.expr()?;
        let then_block = self.block()?;
        let else_branch = if self.check(&TokenKind::Else) {
            self.advance();
            if self.check(&TokenKind::If) {
                Some(ElseBranch::If(Box::new(self.if_stmt()?)))
            } else {
                Some(ElseBranch::Block(self.block()?))
            }
        } else {
            None
        };
        Ok(IfStmt {
            cond,
            then_block,
            else_branch,
            span: start,
        })
    }

    fn while_stmt(&mut self) -> Result<WhileStmt, ParseError> {
        let start = self.expect(TokenKind::While, "`while`")?.span;
        let cond = self.expr()?;
        let body = self.block()?;
        Ok(WhileStmt {
            cond,
            body,
            span: start,
        })
    }

    fn for_stmt(&mut self) -> Result<ForStmt, ParseError> {
        let start = self.expect(TokenKind::For, "`for`")?.span;
        let var = self.expect_ident()?;
        self.expect(TokenKind::In, "`in`")?;
        let iter = self.expr()?;
        let body = self.block()?;
        Ok(ForStmt {
            var,
            iter,
            body,
            span: start,
        })
    }

    /// 表达式入口：允许 `a..b`（仅供 for-in 使用；语义阶段会再限制）。
    pub fn expr(&mut self) -> Result<Expr, ParseError> {
        self.range_expr()
    }

    fn range_expr(&mut self) -> Result<Expr, ParseError> {
        let lhs = self.or()?;
        if self.check(&TokenKind::DotDot) {
            self.advance();
            let rhs = self.or()?;
            let span = lhs.span();
            return Ok(Expr::Range {
                start: Box::new(lhs),
                end: Box::new(rhs),
                span,
            });
        }
        Ok(lhs)
    }

    fn or(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.and()?;
        while self.check(&TokenKind::OrOr) {
            self.advance();
            let rhs = self.and()?;
            let span = lhs.span();
            lhs = Expr::Binary {
                op: BinOp::Or,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                span,
            };
        }
        Ok(lhs)
    }

    fn and(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.equality()?;
        while self.check(&TokenKind::AndAnd) {
            self.advance();
            let rhs = self.equality()?;
            let span = lhs.span();
            lhs = Expr::Binary {
                op: BinOp::And,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                span,
            };
        }
        Ok(lhs)
    }

    fn equality(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.cmp()?;
        loop {
            let op = if self.check(&TokenKind::EqEq) {
                BinOp::Eq
            } else if self.check(&TokenKind::BangEq) {
                BinOp::Ne
            } else {
                break;
            };
            self.advance();
            let rhs = self.cmp()?;
            let span = lhs.span();
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                span,
            };
        }
        Ok(lhs)
    }

    fn cmp(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.term()?;
        loop {
            let op = if self.check(&TokenKind::Lt) {
                BinOp::Lt
            } else if self.check(&TokenKind::LtEq) {
                BinOp::Le
            } else if self.check(&TokenKind::Gt) {
                BinOp::Gt
            } else if self.check(&TokenKind::GtEq) {
                BinOp::Ge
            } else {
                break;
            };
            self.advance();
            let rhs = self.term()?;
            let span = lhs.span();
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                span,
            };
        }
        Ok(lhs)
    }

    fn term(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.factor()?;
        loop {
            let op = if self.check(&TokenKind::Plus) {
                BinOp::Add
            } else if self.check(&TokenKind::Minus) {
                BinOp::Sub
            } else {
                break;
            };
            self.advance();
            let rhs = self.factor()?;
            let span = lhs.span();
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                span,
            };
        }
        Ok(lhs)
    }

    fn factor(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.unary()?;
        loop {
            let op = if self.check(&TokenKind::Star) {
                BinOp::Mul
            } else if self.check(&TokenKind::Slash) {
                BinOp::Div
            } else if self.check(&TokenKind::Percent) {
                BinOp::Rem
            } else {
                break;
            };
            self.advance();
            let rhs = self.unary()?;
            let span = lhs.span();
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                span,
            };
        }
        Ok(lhs)
    }

    fn unary(&mut self) -> Result<Expr, ParseError> {
        if self.check(&TokenKind::Minus) {
            let t = self.advance();
            let e = self.unary()?;
            return Ok(Expr::Unary {
                op: UnOp::Neg,
                expr: Box::new(e),
                span: t.span,
            });
        }
        if self.check(&TokenKind::Bang) {
            let t = self.advance();
            let e = self.unary()?;
            return Ok(Expr::Unary {
                op: UnOp::Not,
                expr: Box::new(e),
                span: t.span,
            });
        }
        self.postfix()
    }

    fn postfix(&mut self) -> Result<Expr, ParseError> {
        let mut e = self.primary()?;
        loop {
            if self.check(&TokenKind::LBracket) {
                self.advance();
                let idx = self.expr()?;
                let end = self.expect(TokenKind::RBracket, "`]`")?;
                e = Expr::Index {
                    base: Box::new(e),
                    index: Box::new(idx),
                    span: end.span,
                };
                continue;
            }
            if self.check(&TokenKind::Dot) {
                self.advance();
                let name = self.expect_ident()?;
                if self.check(&TokenKind::LParen) {
                    self.advance();
                    let args = self.arg_list()?;
                    let end = self.expect(TokenKind::RParen, "`)`")?;
                    e = Expr::MethodCall {
                        recv: Box::new(e),
                        method: name,
                        args,
                        span: end.span,
                    };
                } else {
                    let span = name.span;
                    e = Expr::Field {
                        base: Box::new(e),
                        name,
                        span,
                    };
                }
                continue;
            }
            break;
        }
        Ok(e)
    }

    fn arg_list(&mut self) -> Result<Vec<Expr>, ParseError> {
        let mut args = vec![];
        if !self.check(&TokenKind::RParen) {
            loop {
                args.push(self.expr()?);
                if self.check(&TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        Ok(args)
    }

    fn primary(&mut self) -> Result<Expr, ParseError> {
        let t = self.peek().clone();
        match t.kind.clone() {
            TokenKind::Int(v) => {
                self.advance();
                Ok(Expr::Int {
                    value: v,
                    span: t.span,
                })
            }
            TokenKind::Float(v) => {
                self.advance();
                Ok(Expr::Float {
                    value: v,
                    span: t.span,
                })
            }
            TokenKind::Str(v) => {
                self.advance();
                Ok(Expr::Str {
                    value: v,
                    span: t.span,
                })
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr::Bool {
                    value: true,
                    span: t.span,
                })
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr::Bool {
                    value: false,
                    span: t.span,
                })
            }
            TokenKind::LParen => {
                self.advance();
                let e = self.expr()?;
                self.expect(TokenKind::RParen, "`)`")?;
                Ok(e)
            }
            TokenKind::LBracket => {
                self.advance();
                let mut elems = vec![];
                if !self.check(&TokenKind::RBracket) {
                    loop {
                        elems.push(self.expr()?);
                        if self.check(&TokenKind::Comma) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                self.expect(TokenKind::RBracket, "`]`")?;
                Ok(Expr::Array {
                    elems,
                    span: t.span,
                })
            }
            TokenKind::Ident(name) => {
                self.advance();
                let ident = Ident {
                    name: name.clone(),
                    span: t.span,
                };
                if self.check(&TokenKind::LParen) {
                    self.advance();
                    let args = self.arg_list()?;
                    self.expect(TokenKind::RParen, "`)`")?;
                    return Ok(Expr::Call {
                        callee: ident,
                        args,
                        span: t.span,
                    });
                }
                if self.check(&TokenKind::LBrace) && self.struct_lit_ahead() {
                    self.advance();
                    let mut fields = vec![];
                    if !self.check(&TokenKind::RBrace) {
                        loop {
                            let n = self.expect_ident()?;
                            self.expect(TokenKind::Colon, "`:`")?;
                            let v = self.expr()?;
                            fields.push((n, v));
                            if self.check(&TokenKind::Comma) {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(TokenKind::RBrace, "`}`")?;
                    return Ok(Expr::StructLit {
                        name: ident,
                        fields,
                        span: t.span,
                    });
                }
                Ok(Expr::Var { name: ident })
            }
            other => Err(ParseError {
                message: format!("expected expression, found {other}"),
                span: t.span,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::lexer::Lexer;

    fn parse(src: &str) -> Program {
        let toks = Lexer::new(src).tokenize().unwrap();
        Parser::new(toks).parse_program().unwrap()
    }

    #[test]
    fn parse_struct_const_fun() {
        let p = parse(
            r#"
            struct P { x: int }
            const N: int = 2 + 3;
            fun P.get(self: P) -> int { return self.x; }
            fun main() -> void { let p = P { x: 1 }; print(p.get()); }
            "#,
        );
        assert_eq!(p.structs().count(), 1);
        assert_eq!(p.consts().count(), 1);
        assert!(p.has_main());
    }

    #[test]
    fn empty_for_body_ok() {
        let p = parse("fun main() -> void { let a: int[] = [1]; for x in a { } }");
        assert!(p.has_main());
    }
}
