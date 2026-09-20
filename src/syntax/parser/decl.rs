//! 顶层声明：import / struct / const / fun / 类型。
use super::*;

impl Parser {
    pub(crate) fn ty(&mut self) -> Result<TypeExpr, ParseError> {
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

    pub(crate) fn import_item(&mut self) -> Result<ImportItem, ParseError> {
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

    /// `struct 名 { 字段: 类型, ... }`
    /// 步骤：吃 struct → 名字 → `{` → 循环「字段名 : 类型」→ `}`
    pub(crate) fn struct_decl(&mut self) -> Result<StructDecl, ParseError> {
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

    pub(crate) fn const_decl(&mut self) -> Result<ConstDecl, ParseError> {
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
    /// 函数/方法声明。
    /// 步骤：fun → 名字（若是 `Type.method` 则记录 on_type）→ 参数表 → `->` 返回类型 → 函数体块
    pub(crate) fn fun_decl(&mut self) -> Result<FunDecl, ParseError> {
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
}
