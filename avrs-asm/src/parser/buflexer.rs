use std::io::Read;

use crate::parser::lexer::{LexerError, Token};

use super::lexer::Lexer;

pub struct BufferedLexer<T: Read> {
    lexer: Lexer<T>,
    current: Option<Token>,
}

impl<T: Read> BufferedLexer<T> {
    pub fn new(reader: T) -> Self {
        Self { 
            lexer: Lexer::new(reader),
            current: None,
        }
    }

    pub fn peek(&mut self) -> Result<Token, LexerError> {
        if let Some(token) = &self.current {
            Ok(token.clone())
        } else {
            let token = self.lexer.parse_token()?;
            self.current.replace(token.clone());
            Ok(token) 
        }
    }

    pub fn advance(&mut self) -> Result<Token, LexerError> {
        if let Some(token) = self.current.take() {
            Ok(token)
        } else {
            self.lexer.parse_token()
        }
    }
}
