//! 词法分析：源码字符串 → Token 流。
//!
//! **在做什么**：像用荧光笔把一段中文按「词」划开。  
//! 例如 `let x = 10;` 会变成：关键字 `let`、标识符 `x`、`=`、整数 `10`、`;`。
//!
//! **不管什么**：不判断语法对错（那是 Parser 的事），只保证每个词可识别。
//!
//! **给小白的提示**：
//! - `Token` = 一个词 + 它在源码里的位置（`Span` 的行/列）
//! - `// ...` 是注释，词法阶段直接跳过，不会出现在 Token 列表里
//! - 出错时（如 `@`）返回 `LexError`，带上位置方便报 `file:line:col`

use crate::syntax::token::{Span, Token, TokenKind};

#[derive(Debug)]
pub struct LexError {
    pub message: String,
    pub span: Span,
}

/// 词法分析器：`src` 为源码字节，`pos`/`line`/`col` 跟踪当前位置。
pub struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
    line: u32,
    col: u32,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            src: src.as_bytes(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    /// 把整个源文件切成 Token。
    /// 步骤：循环「跳过空白/注释 → 取一个词」→ 收集 → 直到 Eof。
    /// 最后总是附加 `Eof`，方便解析器判断结束。
    pub fn tokenize(mut self) -> Result<Vec<Token>, LexError> {
        let mut out = Vec::new();
        loop {
            let t = self.next_token()?;
            let eof = t.kind == TokenKind::Eof;
            out.push(t);
            if eof {
                return Ok(out);
            }
        }
    }

    /// 看当前字节，不前进。
    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn peek2(&self) -> Option<u8> {
        self.src.get(self.pos + 1).copied()
    }

    /// 读走当前字节并前进；若为换行则行号+1、列归1。
    fn bump(&mut self) -> Option<u8> {
        let c = self.peek()?;
        self.pos += 1;
        if c == b'\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    fn span(&self) -> Span {
        Span::new(self.line, self.col)
    }

    /// 跳过空白与 `//` 到行尾的注释（注释不进入 Token 流）。
    fn skip_trivia(&mut self) {
        loop {
            match self.peek() {
                Some(b' ' | b'\t' | b'\r' | b'\n') => {
                    self.bump();
                }
                Some(b'/') if self.peek2() == Some(b'/') => {
                    while let Some(c) = self.peek() {
                        if c == b'\n' {
                            break;
                        }
                        self.bump();
                    }
                }
                _ => break,
            }
        }
    }

    /// 读取下一个 Token；先跳过 trivia，再按首字符分类。
    fn next_token(&mut self) -> Result<Token, LexError> {
        self.skip_trivia();
        let span = self.span();
        let Some(c) = self.peek() else {
            return Ok(Token::new(TokenKind::Eof, span));
        };
        if c.is_ascii_alphabetic() || c == b'_' {
            // s"..."：插值字符串
            if c == b's' && self.peek2() == Some(b'"') {
                let start = self.bump();
                let _ = start;
                return self.interp_string(span);
            }
            return Ok(self.ident_kw(span));
        }
        if c.is_ascii_digit() {
            return self.number(span);
        }
        if c == b'"' {
            return self.string(span);
        }
        let kind = match c {
            b'(' => {
                self.bump();
                TokenKind::LParen
            }
            b')' => {
                self.bump();
                TokenKind::RParen
            }
            b'{' => {
                self.bump();
                TokenKind::LBrace
            }
            b'}' => {
                self.bump();
                TokenKind::RBrace
            }
            b'[' => {
                self.bump();
                TokenKind::LBracket
            }
            b']' => {
                self.bump();
                TokenKind::RBracket
            }
            b',' => {
                self.bump();
                TokenKind::Comma
            }
            b':' => {
                self.bump();
                TokenKind::Colon
            }
            b';' => {
                self.bump();
                TokenKind::Semi
            }
            b'+' => {
                self.bump();
                TokenKind::Plus
            }
            b'*' => {
                self.bump();
                TokenKind::Star
            }
            b'%' => {
                self.bump();
                TokenKind::Percent
            }
            b'/' => {
                self.bump();
                TokenKind::Slash
            }
            b'-' => {
                self.bump();
                if self.peek() == Some(b'>') {
                    self.bump();
                    TokenKind::Arrow
                } else {
                    TokenKind::Minus
                }
            }
            b'=' => {
                self.bump();
                if self.peek() == Some(b'=') {
                    self.bump();
                    TokenKind::EqEq
                } else {
                    TokenKind::Assign
                }
            }
            b'!' => {
                self.bump();
                if self.peek() == Some(b'=') {
                    self.bump();
                    TokenKind::BangEq
                } else {
                    TokenKind::Bang
                }
            }
            b'<' => {
                self.bump();
                if self.peek() == Some(b'=') {
                    self.bump();
                    TokenKind::LtEq
                } else {
                    TokenKind::Lt
                }
            }
            b'>' => {
                self.bump();
                if self.peek() == Some(b'=') {
                    self.bump();
                    TokenKind::GtEq
                } else {
                    TokenKind::Gt
                }
            }
            b'&' => {
                self.bump();
                if self.peek() == Some(b'&') {
                    self.bump();
                    TokenKind::AndAnd
                } else {
                    TokenKind::Amp
                }
            }
            b'|' => {
                self.bump();
                if self.peek() == Some(b'|') {
                    self.bump();
                    TokenKind::OrOr
                } else {
                    return Err(LexError {
                        message: "期望 `||`".into(),
                        span,
                    });
                }
            }
            b'.' => {
                self.bump();
                if self.peek() == Some(b'.') {
                    self.bump();
                    TokenKind::DotDot
                } else {
                    TokenKind::Dot
                }
            }
            _ => {
                return Err(LexError {
                    message: format!("意外的字符 `{}`", c as char),
                    span,
                })
            }
        };
        Ok(Token::new(kind, span))
    }

    /// 标识符或关键字：最长匹配字母/数字/下划线，再查关键字表。
    fn ident_kw(&mut self, span: Span) -> Token {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == b'_' {
                self.bump();
            } else {
                break;
            }
        }
        let text = std::str::from_utf8(&self.src[start..self.pos]).unwrap();
        let kind = match text {
            "fun" => TokenKind::Fun,
            "fn" => TokenKind::Fn,
            "struct" => TokenKind::Struct,
            "const" => TokenKind::Const,
            "import" => TokenKind::Import,
            "as" => TokenKind::As,
            "let" => TokenKind::Let,
            "mut" => TokenKind::Mut,
            "nil" => TokenKind::Nil,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "return" => TokenKind::Return,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "int" => TokenKind::TyInt,
            "float" => TokenKind::TyFloat,
            "bool" => TokenKind::TyBool,
            "string" => TokenKind::TyString,
            "void" => TokenKind::TyVoid,
            _ => TokenKind::Ident(text.to_string()),
        };
        Token::new(kind, span)
    }

    /// 数字字面量：整数或 `1.5` 形式的浮点（小数点后必须跟数字才算浮点）。
    /// 数字：先吃连续数字，若出现「小数点+后继数字」则再吃小数部分。
    /// 插值字符串 `s"..."`：内容原样存进 InterpStr，`{...}` 由解析器拆表达式。
    fn interp_string(&mut self, span: Span) -> Result<Token, LexError> {
        // 此时已消费开头的 s，需再消费引号
        self.bump();
        let mut s = String::new();
        loop {
            match self.bump() {
                None => {
                    return Err(LexError {
                        message: "插值字符串未闭合".into(),
                        span,
                    })
                }
                Some(b'"') => break,
                Some(b'\\') => match self.bump() {
                    Some(b'n') => s.push('\n'),
                    Some(b't') => s.push('\t'),
                    Some(b'\\') => s.push('\\'),
                    Some(b'"') => s.push('"'),
                    _ => {
                        return Err(LexError {
                            message: "无效的转义字符".into(),
                            span,
                        })
                    }
                },
                Some(c) if c < 0x80 => s.push(c as char),
                Some(c) => {
                    let mut bytes = vec![c];
                    while matches!(self.peek(), Some(b) if b & 0xC0 == 0x80) {
                        bytes.push(self.bump().unwrap());
                    }
                    match std::str::from_utf8(&bytes) {
                        Ok(t) => s.push_str(t),
                        Err(_) => {
                            return Err(LexError {
                                message: "字符串中含无效 UTF-8".into(),
                                span,
                            })
                        }
                    }
                }
            }
        }
        Ok(Token::new(TokenKind::InterpStr(s), span))
    }

    fn number(&mut self, span: Span) -> Result<Token, LexError> {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.bump();
        }
        let mut is_float = false;
        if self.peek() == Some(b'.') && matches!(self.peek2(), Some(c) if c.is_ascii_digit()) {
            is_float = true;
            self.bump();
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.bump();
            }
        }
        let text = std::str::from_utf8(&self.src[start..self.pos]).unwrap();
        if is_float {
            let n: f64 = text.parse().map_err(|_| LexError {
                message: format!("无效的浮点数 `{text}`"),
                span,
            })?;
            Ok(Token::new(TokenKind::Float(n), span))
        } else {
            let n: i64 = text.parse().map_err(|_| LexError {
                message: format!("无效的整数 `{text}`"),
                span,
            })?;
            Ok(Token::new(TokenKind::Int(n), span))
        }
    }

    /// 字符串字面量：双引号包裹，支持 `\n \t \\ \"`；遇 EOF 未闭合则报错。
    /// 字符串：吃掉开头引号，直到结束引号；处理 \n 等转义。
    fn string(&mut self, span: Span) -> Result<Token, LexError> {
        self.bump();
        let mut s = String::new();
        loop {
            match self.bump() {
                None => {
                    return Err(LexError {
                        message: "字符串未闭合".into(),
                        span,
                    })
                }
                Some(b'"') => break,
                Some(b'\\') => match self.bump() {
                    Some(b'n') => s.push('\n'),
                    Some(b't') => s.push('\t'),
                    Some(b'\\') => s.push('\\'),
                    Some(b'"') => s.push('"'),
                    _ => {
                        return Err(LexError {
                            message: "无效的转义字符".into(),
                            span,
                        })
                    }
                },
                Some(c) if c < 0x80 => s.push(c as char),
                Some(c) => {
                    let mut bytes = vec![c];
                    while matches!(self.peek(), Some(b) if b & 0xC0 == 0x80) {
                        bytes.push(self.bump().unwrap());
                    }
                    match std::str::from_utf8(&bytes) {
                        Ok(t) => s.push_str(t),
                        Err(_) => {
                            return Err(LexError {
                                message: "字符串中含无效 UTF-8".into(),
                                span,
                            })
                        }
                    }
                }
            }
        }
        Ok(Token::new(TokenKind::Str(s), span))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<TokenKind> {
        Lexer::new(src)
            .tokenize()
            .unwrap()
            .into_iter()
            .map(|t| t.kind)
            .collect()
    }

    #[test]
    fn keywords_and_ops() {
        let k = kinds("fun struct const for in break && || -> ..");
        assert!(k.contains(&TokenKind::Fun));
        assert!(k.contains(&TokenKind::Struct));
        assert!(k.contains(&TokenKind::Const));
        assert!(k.contains(&TokenKind::DotDot));
        assert!(k.contains(&TokenKind::Arrow));
    }

    #[test]
    fn numbers_strings() {
        let k = kinds(r#"1 2.5 "a\n""#);
        assert_eq!(k[0], TokenKind::Int(1));
        assert_eq!(k[1], TokenKind::Float(2.5));
        assert_eq!(k[2], TokenKind::Str("a\n".into()));
    }
}
