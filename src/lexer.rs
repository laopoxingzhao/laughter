//! 词法分析：源码字符串 → `Token` 流（以 `Eof` 结尾）。
//! 负责跳过空白与 `//` 行注释，识别关键字/字面量/运算符，并维护行列号。

use crate::token::{Span, Token, TokenKind};

#[derive(Debug)]
pub struct LexError {
    pub message: String,
    pub span: Span,
}

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

    pub fn tokenize(mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token()?;
            let is_eof = tok.kind == TokenKind::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn peek2(&self) -> Option<u8> {
        self.src.get(self.pos + 1).copied()
    }

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

    fn skip_trivia(&mut self) {
        loop {
            match self.peek() {
                Some(b' ') | Some(b'\t') | Some(b'\r') | Some(b'\n') => {
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

    fn next_token(&mut self) -> Result<Token, LexError> {
        self.skip_trivia();
        let span = self.span();
        let Some(c) = self.peek() else {
            return Ok(Token::new(TokenKind::Eof, span));
        };

        if c.is_ascii_alphabetic() || c == b'_' {
            return Ok(self.ident_or_keyword(span));
        }
        if c.is_ascii_digit() {
            return self.number(span);
        }
        if c == b'"' {
            return self.string(span);
        }

        // 多字符运算符优先匹配
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
            b'-' => {
                self.bump();
                if self.peek() == Some(b'>') {
                    self.bump();
                    TokenKind::Arrow
                } else {
                    TokenKind::Minus
                }
            }
            b'/' => {
                self.bump();
                TokenKind::Slash
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
                    return Err(LexError {
                        message: "expected `&&`".into(),
                        span,
                    });
                }
            }
            b'|' => {
                self.bump();
                if self.peek() == Some(b'|') {
                    self.bump();
                    TokenKind::OrOr
                } else {
                    return Err(LexError {
                        message: "expected `||`".into(),
                        span,
                    });
                }
            }
            _ => {
                return Err(LexError {
                    message: format!("unexpected character `{}`", c as char),
                    span,
                });
            }
        };
        Ok(Token::new(kind, span))
    }

    fn ident_or_keyword(&mut self, span: Span) -> Token {
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
            "let" => TokenKind::Let,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
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

    fn number(&mut self, span: Span) -> Result<Token, LexError> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.bump();
            } else {
                break;
            }
        }
        let mut is_float = false;
        if self.peek() == Some(b'.') && self.peek2().is_some_and(|c| c.is_ascii_digit()) {
            is_float = true;
            self.bump();
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.bump();
                } else {
                    break;
                }
            }
        }
        let text = std::str::from_utf8(&self.src[start..self.pos]).unwrap();
        if is_float {
            let n: f64 = text.parse().map_err(|_| LexError {
                message: format!("invalid float literal `{text}`"),
                span,
            })?;
            Ok(Token::new(TokenKind::Float(n), span))
        } else {
            let n: i64 = text.parse().map_err(|_| LexError {
                message: format!("invalid integer literal `{text}`"),
                span,
            })?;
            Ok(Token::new(TokenKind::Int(n), span))
        }
    }

    fn string(&mut self, span: Span) -> Result<Token, LexError> {
        self.bump(); // 消费开头的引号"
        let mut out = String::new();
        loop {
            match self.bump() {
                None => {
                    return Err(LexError {
                        message: "unterminated string".into(),
                        span,
                    })
                }
                Some(b'"') => break,
                Some(b'\\') => match self.bump() {
                    Some(b'n') => out.push('\n'),
                    Some(b't') => out.push('\t'),
                    Some(b'\\') => out.push('\\'),
                    Some(b'"') => out.push('"'),
                    _ => {
                        return Err(LexError {
                            message: "invalid escape sequence".into(),
                            span,
                        })
                    }
                },
                Some(c) => {
                    // 收集以 c 为首的完整 UTF-8 字符
                    if c < 0x80 {
                        out.push(c as char);
                    } else {
                        // 多字节：c 已消费首字节，继续收后续 continuation 字节
                        let mut bytes = vec![c];
                        while bytes.len() < 4 {
                            match self.peek() {
                                Some(nb) if nb & 0xC0 == 0x80 => {
                                    bytes.push(nb);
                                    self.bump();
                                }
                                _ => break,
                            }
                        }
                        match std::str::from_utf8(&bytes) {
                            Ok(s) => out.push_str(s),
                            Err(_) => {
                                return Err(LexError {
                                    message: "invalid UTF-8 in string".into(),
                                    span,
                                })
                            }
                        }
                    }
                }
            }
        }
        Ok(Token::new(TokenKind::Str(out), span))
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
    fn keywords_and_idents() {
        let k = kinds("fun let if else while return true false int float bool string void foo");
        assert_eq!(
            k,
            vec![
                TokenKind::Fun,
                TokenKind::Let,
                TokenKind::If,
                TokenKind::Else,
                TokenKind::While,
                TokenKind::Return,
                TokenKind::True,
                TokenKind::False,
                TokenKind::TyInt,
                TokenKind::TyFloat,
                TokenKind::TyBool,
                TokenKind::TyString,
                TokenKind::TyVoid,
                TokenKind::Ident("foo".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn numbers_strings_ops() {
        let k = kinds(r#"1 2.5 "hi\n" + - * / % == != < <= > >= && || ! = -> [] () {} ,: ;"#);
        assert_eq!(
            k,
            vec![
                TokenKind::Int(1),
                TokenKind::Float(2.5),
                TokenKind::Str("hi\n".into()),
                TokenKind::Plus,
                TokenKind::Minus,
                TokenKind::Star,
                TokenKind::Slash,
                TokenKind::Percent,
                TokenKind::EqEq,
                TokenKind::BangEq,
                TokenKind::Lt,
                TokenKind::LtEq,
                TokenKind::Gt,
                TokenKind::GtEq,
                TokenKind::AndAnd,
                TokenKind::OrOr,
                TokenKind::Bang,
                TokenKind::Assign,
                TokenKind::Arrow,
                TokenKind::LBracket,
                TokenKind::RBracket,
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::LBrace,
                TokenKind::RBrace,
                TokenKind::Comma,
                TokenKind::Colon,
                TokenKind::Semi,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn line_col_tracking() {
        let toks = Lexer::new("let x\n= 1;").tokenize().unwrap();
        assert_eq!(toks[0].span, Span::new(1, 1));
        assert_eq!(toks[2].span, Span::new(2, 1));
    }

    #[test]
    fn comments_skipped() {
        let k = kinds("1 // comment\n2");
        assert_eq!(
            k,
            vec![TokenKind::Int(1), TokenKind::Int(2), TokenKind::Eof]
        );
    }

    #[test]
    fn illegal_char() {
        assert!(Lexer::new("let @").tokenize().is_err());
    }
}
