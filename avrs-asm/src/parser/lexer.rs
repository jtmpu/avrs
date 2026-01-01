use std::io::{Read};

use miette::SourceSpan;

use crate::parser::tagged::Tagged;

use super::reader::{ReadError, Reader};

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum LexerError {
    #[error("unexpected IO error: {0}")]
    Read(#[from] ReadError),
    #[error("unexpected byte: 0x{byte:02x}")]
    UnexpectedByte {
        byte: u8,
        #[label("error occured while reading here")]
        span: SourceSpan,
    },
    #[error("invalid byte in numerical: 0x{byte:02x}")]
    #[diagnostic(
        help="valid numericals use '0-9'"
    )]
    InvalidNumerical {
        byte: u8,
        #[label("error occured while reading here")]
        span: SourceSpan,
    },
    #[error("invalid byte in identifier: 0x{byte:02x}")]
    #[diagnostic(
        help="valid identifiers contain '0-9a-zA-Z'"
    )]
    InvalidIdentifier {
        byte: u8,
        #[label("error occured while reading here")]
        span: SourceSpan,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Identifier(String),
    Register(String),
    DecimalLiteral(String),
    Dot,
    Comma,
    Colon,
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Newline,
    Eof,
}

pub type Token = Tagged<TokenKind>;
pub struct Lexer<T: Read> {
    reader: Reader<T>,
}

impl<T: Read> Lexer<T> {
    pub fn new(reader: T) -> Self {
        Self {
            reader: Reader::new(reader),
        }
    }

    fn span(&self, len: usize) -> SourceSpan {
        (self.reader.offset().saturating_sub(len), len).into()
    }

    fn parse_comma(&mut self) -> Result<Token, LexerError> {
        let _ = self.reader.read_exact()?; 
        Ok(Token::new(TokenKind::Comma, self.span(1)))
    }

    fn parse_colon(&mut self) -> Result<Token, LexerError> {
        let _ = self.reader.read_exact()?; 
        Ok(Token::new(TokenKind::Colon, self.span(1)))
    }

    fn parse_plus(&mut self) -> Result<Token, LexerError> {
        let _ = self.reader.read_exact()?; 
        Ok(Token::new(TokenKind::Plus, self.span(1)))
    }

    fn parse_minus(&mut self) -> Result<Token, LexerError> {
        let _ = self.reader.read_exact()?; 
        Ok(Token::new(TokenKind::Minus, self.span(1)))
    }

    fn parse_star(&mut self) -> Result<Token, LexerError> {
        let _ = self.reader.read_exact()?; 
        Ok(Token::new(TokenKind::Star, self.span(1)))
    }

    fn parse_slash(&mut self) -> Result<Token, LexerError> {
        let _ = self.reader.read_exact()?; 
        Ok(Token::new(TokenKind::Slash, self.span(1)))
    }

    fn parse_lparen(&mut self) -> Result<Token, LexerError> {
        let _ = self.reader.read_exact()?; 
        Ok(Token::new(TokenKind::LParen, self.span(1)))
    }

    fn parse_rparen(&mut self) -> Result<Token, LexerError> {
        let _ = self.reader.read_exact()?; 
        Ok(Token::new(TokenKind::RParen, self.span(1)))
    }

    fn parse_lbracket(&mut self) -> Result<Token, LexerError> {
        let _ = self.reader.read_exact()?; 
        Ok(Token::new(TokenKind::LBracket, self.span(1)))
    }

    fn parse_rbracket(&mut self) -> Result<Token, LexerError> {
        let _ = self.reader.read_exact()?; 
        Ok(Token::new(TokenKind::RBracket, self.span(1)))
    }

    fn parse_newline(&mut self) -> Result<Token, LexerError> {
        let _ = self.reader.read_exact()?; 
        Ok(Token::new(TokenKind::Newline, self.span(1)))
    }

    fn parse_dot(&mut self) -> Result<Token, LexerError> {
        let _ = self.reader.read_exact()?; 
        Ok(Token::new(TokenKind::Dot, self.span(1)))
    }

    fn parse_number(&mut self) -> Result<Token, LexerError> {
        let mut digit = String::new();
        let first = self.reader.read_exact()?;
        digit.push(first as char);

        while let Some(next) = self.reader.peek()? {
            if next.is_ascii_whitespace() || next.is_ascii_punctuation() {
                break;
            }

            let next = self.reader.read_exact()?;
            if !next.is_ascii_digit() {
                return Err(LexerError::InvalidNumerical { byte: next, span: self.span(digit.len() + 1) });
            }
            digit.push(next as char);
        }
        let length = digit.len();
        Ok(Token::new(TokenKind::DecimalLiteral(digit), self.span(length)))
    }

    fn parse_identifier_register(&mut self) -> Result<Token, LexerError> {
        let mut value = String::new();
        let first = self.reader.read_exact()?;
        value.push(first as char);

        while let Some(next) = self.reader.peek()? {
            if next.is_ascii_whitespace() || next.is_ascii_punctuation() {
                break;
            }

            let next = self.reader.read_exact()?;
            if !next.is_ascii_alphanumeric() {
                return Err(LexerError::InvalidIdentifier { byte: next, span: self.span(value.len() + 1) });
            }
            value.push(next as char);
        }
        
        // check if register-format, or if identifier
        let length = value.len();
        let token = if value.starts_with('r') && value[1..].parse::<usize>().is_ok() {
            Token::new(TokenKind::Register(value), self.span(length))
        } else {
            Token::new(TokenKind::Identifier(value), self.span(length))
        };

        Ok(token)
    }

    pub fn parse_token(&mut self) -> Result<Token, LexerError> {
        self.reader.skip_until(|b| !super::reader::whitespace(b))?;
        let char = match self.reader.peek()? {
            Some(char) => char,
            None => return Ok(Token::new(TokenKind::Eof, self.span(0))),
        };

        let token = match char {
            b',' => self.parse_comma()?,
            b'.' => self.parse_dot()?,
            b':' => self.parse_colon()?,
            b'+' => self.parse_plus()?,
            b'-' => self.parse_minus()?,
            b'*' => self.parse_star()?,
            b'/' => self.parse_slash()?,
            b'(' => self.parse_lparen()?,
            b')' => self.parse_rparen()?,
            b'[' => self.parse_lbracket()?,
            b']' => self.parse_rbracket()?,
            b'\n' => self.parse_newline()?,
            x if x.is_ascii_digit() => self.parse_number()?,
            x if x.is_ascii_alphabetic() => self.parse_identifier_register()?,
            _ => return Err(LexerError::UnexpectedByte { byte: char, span: self.span(1) }),
        };
        Ok(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create(value: &str) -> Lexer<&[u8]> {
        Lexer::new(value.as_bytes())
    }

    #[test]
    fn test_simple_lex() {
        let cases = vec![
            (TokenKind::Comma, ","),
            (TokenKind::Dot, "."),
            (TokenKind::Colon, ":"),
            (TokenKind::Plus, "+"),
            (TokenKind::Minus, "-"),
            (TokenKind::Star, "*"),
            (TokenKind::Slash, "/"),
            (TokenKind::LParen, "("),
            (TokenKind::RParen, ")"),
            (TokenKind::LBracket, "["),
            (TokenKind::RBracket, "]"),
            (TokenKind::Newline, "\n"),
        ];

        for (expected, input) in cases {
            let mut lexer = create(input);
            let next = lexer.parse_token()
                .unwrap();
            assert_eq!(next.kind, expected);
        }
    }

    #[test]
    fn test_skip_whitespace() {
        let cases = vec![
            (TokenKind::Comma, "  ,"),
            (TokenKind::Colon, ":   "),
            (TokenKind::Plus, "   +   "),
        ];

        for (expected, input) in cases {
            let mut lexer = create(input);
            let next = lexer.parse_token()
                .unwrap();
            assert_eq!(next.kind, expected);
            let next = lexer.parse_token()
                .unwrap();
            assert_eq!(next.kind, TokenKind::Eof);
        }
    }

    #[test]
    fn test_numerical() {
        let mut lexer = create("42");
        let token = lexer.parse_token()
            .unwrap();
        assert_eq!(TokenKind::DecimalLiteral("42".to_string()), token.kind);

        let mut lexer = create("  123456  ");
        let token = lexer.parse_token()
            .unwrap();
        assert_eq!(TokenKind::DecimalLiteral("123456".to_string()), token.kind);

        let mut lexer = create("  42b  ");
        let token = lexer.parse_token();
        assert!(matches!(token, Err(LexerError::InvalidNumerical { byte: b'b', span: _ })));
    }

    #[test]
    fn test_register() {
        let mut lexer = create("r1");
        let token = lexer.parse_token()
            .unwrap();
        assert_eq!(TokenKind::Register("r1".to_string()), token.kind);

        let mut lexer = create(" r321  ");
        let token = lexer.parse_token()
            .unwrap();
        assert_eq!(TokenKind::Register("r321".to_string()), token.kind);

        let mut lexer = create(" r2w  ");
        let token = lexer.parse_token()
            .unwrap();
        assert!(matches!(token.kind, TokenKind::Identifier(_)));
    }

    #[test]
    fn test_identifier() {
        let mut lexer = create("something");
        let token = lexer.parse_token()
            .unwrap();
        assert_eq!(TokenKind::Identifier("something".to_string()), token.kind);

        let mut lexer = create("   secretlabel332: ");
        let token = lexer.parse_token()
            .unwrap();
        assert_eq!(TokenKind::Identifier("secretlabel332".to_string()), token.kind);
    }

    #[test]
    fn test_line() {
        let mut lexer = create("mov r1, r2");
        let token = lexer.parse_token()
            .unwrap();
        assert_eq!(TokenKind::Identifier("mov".to_string()), token.kind);
        let token = lexer.parse_token()
            .unwrap();
        assert_eq!(TokenKind::Register("r1".to_string()), token.kind);
        let token = lexer.parse_token()
            .unwrap();
        assert_eq!(TokenKind::Comma, token.kind);
        let token = lexer.parse_token()
            .unwrap();
        assert_eq!(TokenKind::Register("r2".to_string()), token.kind);
    }

    #[test]
    fn test_multiline() {
        let mut lexer = create("label1:\n   ldi r2, 42\n    add r1, r2");
        let expected_tokens = vec![
            TokenKind::Identifier("label1".to_string()),
            TokenKind::Colon,
            TokenKind::Newline,
            TokenKind::Identifier("ldi".to_string()),
            TokenKind::Register("r2".to_string()),
            TokenKind::Comma,
            TokenKind::DecimalLiteral("42".to_string()),
            TokenKind::Newline,
            TokenKind::Identifier("add".to_string()),
            TokenKind::Register("r1".to_string()),
            TokenKind::Comma,
            TokenKind::Register("r2".to_string()),
            TokenKind::Eof,
        ];

        for expected in expected_tokens {
            let token = lexer.parse_token().unwrap();
            assert_eq!(token.kind, expected);
        }
    }
}
