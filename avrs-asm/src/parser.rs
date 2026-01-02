/// Grammar
/// program     ::= { line } EOF
/// line        ::= [ statement ] NEWLINE
/// statement   ::= instruction
///                | directive
///                | label
///
/// label       ::= IDENT COLON
/// instruction ::= mnemonic [ operand_list ]
/// mnemonic    ::= IDENT
/// operand_list ::= operand { COMMA operand }
///
/// directive   ::= DOT IDENT [ directive_args ]
/// directive_args ::= operand_list
/// operand     ::= register
///               | memory
///               | expression
///
/// memory     ::= LBRACKET register [ expression ] RBRACKET
///               
/// expression ::= additive
///
/// additive   ::= multiplicative { (PLUS | MINUS) multiplicative }
/// multiplicative ::= unary { (STAR | SLASH) unary }
/// unary      ::= (PLUS | MINUS) unary
///              | primary
/// primary    ::= NUMBER
///              | IDENT
///              | LPAREN expression RPAREN
///
use std::{io::Read, num::ParseIntError};

use clap::error::Result;
use miette::SourceSpan;

pub mod tagged;
pub mod lexer;
pub mod reader;
pub mod buflexer;

use crate::parser::{buflexer::BufferedLexer, lexer::{LexerError, Token, TokenKind}, tagged::Tagged};

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum ParserError {
    #[error("error while parsing '{topic}': unexpected token '{received:?}', expected: {expected:?}")]
    UnexpectedToken {
        topic: String,
        received: TokenKind,
        expected: Vec<TokenKind>,
        #[label("error occured while reading here")]
        span: SourceSpan,
    },
    #[error("failed to parse number: {error}")]
    #[diagnostic(
        help="valid numericals are in format '0-9'"
    )]
    InvalidNumber {
        error: ParseIntError,
        #[label("error occured while reading here")]
        span: SourceSpan,
    },
    #[error("unexpected lexer error: {0}")]
    Lexer(#[from] LexerError),
}

#[derive(Debug)]
pub struct Line {
    pub label: Option<String>,
    pub statement: Option<Statement>,
    pub span: SourceSpan,
}

#[derive(Debug)]
pub enum Statement {
    Instruction(ParsedStatement),
    Directive(ParsedStatement),
}

#[derive(Debug)]
pub struct ParsedStatement {
    pub name: String,
    pub operands: Vec<Operand>,
    pub span: SourceSpan,
}

pub type Operand = Tagged<OperandKind>;
#[derive(Debug)]
pub enum OperandKind {
    Register(u8),
    Expression(Expression),
}

type Expression = Tagged<ExpressionKind>;
#[derive(Debug)]
pub enum ExpressionKind {
    Number(i64),
    Unary {
        op: UnaryOp,
        expression: Box<Expression>,
    },
    Binary {
        left: Box<Expression>,
        op: BinaryOp,
        right: Box<Expression>,
    }
}

impl Expression {
    pub fn evaluate(&self) -> i64 {
        match &self.kind {
            ExpressionKind::Number(value) => *value,
            ExpressionKind::Unary { op, expression } => {
                let value = expression.evaluate();
                match op {
                    UnaryOp::Minus => -value,
                    UnaryOp::Plus => value,
                }
            }
            ExpressionKind::Binary { left, op, right } => {
                let left = left.evaluate();
                let right = right.evaluate();
                match op {
                    BinaryOp::Add => left + right,
                    BinaryOp::Sub => left - right,
                    BinaryOp::Mul => left * right,
                    BinaryOp::Div => left / right,
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum UnaryOp {
    Plus,
    Minus,
}

#[derive(Debug)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

fn combined_span(first: SourceSpan, second: SourceSpan) -> SourceSpan {
    let end = first.offset() + first.len();
    let extra = second.offset() - end;
    (first.offset(), extra + second.len()).into()
}

pub struct Parser<T: Read> {
    lexer: BufferedLexer<T>,
}

impl<T: Read> Parser<T> {
    pub fn new(reader: T) -> Self {
        Self { lexer: BufferedLexer::new(reader) }
    }

    pub fn next(&mut self) -> miette::Result<Line> {
        Ok(self.parse_line()?)
    }

    pub fn skip_until_end(&mut self) -> miette::Result<()> {
        while !matches!(self.lexer.peek()?.kind, TokenKind::Newline | TokenKind::Eof) {
            self.lexer.advance()?;
        }
        Ok(())
    }

    fn parse_line(&mut self) -> Result<Line, ParserError> {
        let mut token = self.lexer.advance()?;
        let mut span = token.span;
        let mut label = None;
        let mut statement = None;

        // Check if there's a label
        if let TokenKind::Identifier(value) = &token.kind && let TokenKind::Colon = self.lexer.peek()?.kind {
            // Label
            label.replace(value.to_string());
            token = self.lexer.advance()?;
            // include colon in span
            span = combined_span(span, token.span);
            // move cursor forward again
            token = self.lexer.advance()?;
        }

        // Line has ended
        if matches!(token.kind, TokenKind::Newline | TokenKind::Eof) {
            return Ok(Line { label, statement, span });
        }

        // if line hasn't ended, and we still have a token, there must be a statement
        statement.replace(self.parse_statement(Some(token))?);

        Ok(Line {
            label,
            statement,
            span,
        })
    }

    fn parse_statement(&mut self, token: Option<Token>) -> Result<Statement, ParserError> {
        let mut token = if let Some(token) = token {
            token
        } else {
            self.lexer.advance()?
        };
        let initial = token.span;

        let is_directive = if let TokenKind::Dot = token.kind {
            token = self.lexer.advance()?;
            true
        } else {
            false
        };

        let name = if let TokenKind::Identifier(name) = &token.kind {
            name.to_string()
        } else {
            return Err(ParserError::UnexpectedToken { 
                topic: "statement".to_string(),
                received: token.kind.clone(),
                expected: vec![TokenKind::Identifier(String::new())],
                span: token.span,
            });
        };

        let mut operands = Vec::new();
        while self.peek_is_operand()? {
            let op = self.parse_operand()?;
            operands.push(op);

            let next = self.lexer.peek()?;
            if !matches!(next.kind, TokenKind::Comma) {
                break;
            }

            self.lexer.advance()?;
        }

        let span = if let Some(op) = operands.last() {
            combined_span(initial, op.span)
        } else {
            initial
        };

        let parsed = ParsedStatement { name, operands, span };
        let stmt = if is_directive {
            Statement::Directive(parsed)
        } else {
            Statement::Instruction(parsed)
        };

        Ok(stmt)
    }

    fn peek_is_operand(&mut self) -> Result<bool, ParserError> {
        let token = self.lexer.peek()?;
        let matches = match token.kind {
            TokenKind::Register(_) => true,
            _ => {
                self.peek_is_expression()?
            }
        };
        Ok(matches)
    }

    fn peek_is_expression(&mut self) -> Result<bool, ParserError> {
        let token = self.lexer.peek()?;
        let matches = matches!(token.kind, TokenKind::DecimalLiteral(_) | TokenKind::Plus | TokenKind::Minus | TokenKind::LParen);
        Ok(matches)
    }

    fn parse_operand(&mut self) -> Result<Operand, ParserError> {
        let token = self.lexer.peek()?;

        let operand = match &token.kind {
            TokenKind::Register(_) => self.parse_register()?,
            kind if self.peek_is_expression()? => {
                let expr = self.parse_expression()?;
                let span = expr.span;
                Operand::new(OperandKind::Expression(expr), span)
            }
            _ => {
                return Err(ParserError::UnexpectedToken { 
                    topic: "operand".to_string(),
                    received: token.kind.clone(),
                    expected: vec![TokenKind::LParen, TokenKind::DecimalLiteral(String::new()), TokenKind::Plus, TokenKind::Minus],
                    span: token.span,
                });
            }
        };
        Ok(operand)
    }

    fn parse_register(&mut self) -> Result<Operand, ParserError> {
        let token = self.lexer.advance()?;
        if let TokenKind::Register(value) = token.kind {
            let index = value[1..].parse::<u8>()
                .map_err(|e| ParserError::InvalidNumber { error: e, span: token.span })?;
            Ok(Operand::new(OperandKind::Register(index), token.span))
        } else {
            Err(ParserError::UnexpectedToken { 
                topic: "register".to_string(),
                received: token.kind.clone(),
                expected: vec![TokenKind::Register(String::new())],
                span: token.span,
            })
        }
    }

    fn parse_expression(&mut self) -> Result<Expression, ParserError> {
        self.parse_additive()
    }

    fn parse_additive(&mut self) -> Result<Expression, ParserError> {
        let left = self.parse_multiplicative()?;
        
        let token = self.lexer.peek()?;
        let op = match token.kind {
            TokenKind::Plus => BinaryOp::Add,
            TokenKind::Minus => BinaryOp::Sub,
            _ => {
                return Ok(left);
            }
        };
        self.lexer.advance()?;
        let right = self.parse_multiplicative()?;

        let span = combined_span(left.span, right.span);
        let expression = ExpressionKind::Binary { left: Box::new(left), op, right: Box::new(right) };
        Ok(Expression::new(expression, span))
    }

    fn parse_multiplicative(&mut self) -> Result<Expression, ParserError> {
        let left = self.parse_unary()?;
        
        let token = self.lexer.peek()?;
        let op = match token.kind {
            TokenKind::Star => BinaryOp::Mul,
            TokenKind::Slash => BinaryOp::Div,
            _ => {
                return Ok(left);
            }
        };
        self.lexer.advance()?;
        let right = self.parse_unary()?;

        let span = combined_span(left.span, right.span);
        let expression = ExpressionKind::Binary { left: Box::new(left), op, right: Box::new(right) };
        Ok(Expression::new(expression, span))
    }

    fn parse_unary(&mut self) -> Result<Expression, ParserError> {
        let token = self.lexer.peek()?;
        let op = match token.kind {
            TokenKind::Plus => {
                UnaryOp::Plus
            },
            TokenKind::Minus => {
                UnaryOp::Minus
            },
            _ => {
                return self.parse_primary();
            }
        };
        self.lexer.advance()?;

        let subexpr = self.parse_unary()?;
        let span = combined_span(token.span, subexpr.span);
        let expression = Expression::new(
            ExpressionKind::Unary { 
                op,
                expression: Box::new(subexpr), 
            }, 
            span,
        );
        Ok(expression)
    }

    fn parse_primary(&mut self) -> Result<Expression, ParserError> {
        let token = self.lexer.advance()?;

        match token.kind {
            TokenKind::LParen => {},
            TokenKind::DecimalLiteral(value) => {
                let value: i64 = value.parse()
                    .map_err(|e| ParserError::InvalidNumber { error: e, span: token.span })?;
                return Ok(Expression::new(ExpressionKind::Number(value), token.span));
            }
            _ => {
                return Err(ParserError::UnexpectedToken { 
                    topic: "expression".to_string(),
                    received: token.kind.clone(),
                    expected: vec![TokenKind::DecimalLiteral(String::new())], 
                    span: token.span,
                });
            }
        };

        let expression = self.parse_expression()?;
        let token = self.lexer.advance()?;
        if token.kind != TokenKind::RParen {
            return Err(ParserError::UnexpectedToken { 
                topic: "expression".to_string(),
                received: token.kind.clone(),
                expected: vec![TokenKind::RParen],
                span: token.span,
            });
        }
        Ok(expression)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create(value: &str) -> Parser<&[u8]> {
       Parser::new(value.as_bytes())
    }

    #[test]
    fn test_expression() {
        let mut parser = create("24");
        let expr = parser.parse_expression()
            .unwrap();

        assert_eq!(24, expr.evaluate());

        let mut parser = create("-24");
        let expr = parser.parse_expression()
            .unwrap();
        assert_eq!(-24, expr.evaluate());

        let mut parser = create("-+-24");
        let expr = parser.parse_expression()
            .unwrap();
        assert_eq!(24, expr.evaluate());

        let mut parser = create("10 * 2");
        let expr = parser.parse_expression()
            .unwrap();
        assert_eq!(20, expr.evaluate());

        let mut parser = create("10 / 2");
        let expr = parser.parse_expression()
            .unwrap();
        assert_eq!(5, expr.evaluate());

        let mut parser = create("5 + 2");
        let expr = parser.parse_expression()
            .unwrap();
        assert_eq!(7, expr.evaluate());

        let mut parser = create("24 - 8");
        let expr = parser.parse_expression()
            .unwrap();
        assert_eq!(16, expr.evaluate());

        let mut parser = create("5 + (10 * 2)");
        let expr = parser.parse_expression()
            .unwrap();
        assert_eq!(25, expr.evaluate());
    }

    #[test]
    pub fn test_operand() {
        let mut parser = create("r21");
        let op = parser.parse_operand()
            .unwrap();
        match op.kind {
            OperandKind::Register(value) => assert_eq!(21, value),
            _ => panic!("expected register operand, received {:?}", op.kind),
        }

        let mut parser = create("2+2");
        let op = parser.parse_operand()
            .unwrap();
        match op.kind {
            OperandKind::Expression(expr) => assert_eq!(4, expr.evaluate()),
            _ => panic!("expected expression operand, received {:?}", op.kind),
        }
    }

    #[test]
    fn test_statement() {
        let mut parser = create(".BYTE 4");
        let stmt = parser.parse_statement(None)
            .unwrap();
        match &stmt {
            Statement::Directive(body) => {
                assert_eq!(body.name, "BYTE");
                let op = &body.operands[0];
                match &op.kind {
                    OperandKind::Expression(expr) => assert_eq!(expr.evaluate(), 4),
                    _ => panic!("expected expression, received: {:?}", op.kind),
                }

            }
            _ => panic!("expected directive, received: {:?}", stmt),
        };

        let mut parser = create("ldi r2, (4 + 8)");
        let stmt = parser.parse_statement(None)
            .unwrap();
        match &stmt{
            Statement::Instruction(body) => {
                assert_eq!(body.name, "ldi");
                let op = &body.operands[0];
                match &op.kind {
                    OperandKind::Register(index) => assert_eq!(*index, 2),
                    _ => panic!("expected expression, received: {:?}", op.kind),
                }
                let op = &body.operands[1];
                match &op.kind {
                    OperandKind::Expression(expr) => assert_eq!(expr.evaluate(), 12),
                    _ => panic!("expected expression, received: {:?}", op.kind),
                }

            }
            _ => panic!("expected directive, received: {:?}", stmt),
        };
    }

    #[test]
    fn test_line() {
        let mut parser = create("\n");
        let line = parser.parse_line()
            .unwrap();
        assert!(line.label.is_none());
        assert!(line.statement.is_none());

        let mut parser = create("label:\n");
        let line = parser.parse_line()
            .unwrap();
        assert_eq!("label", line.label.unwrap());
        assert!(line.statement.is_none());

        let mut parser = create("mov r2, r3\n");
        let line = parser.parse_line()
            .unwrap();
        assert!(line.label.is_none());
        assert!(line.statement.is_some());

        let mut parser = create("label: .BYTE 4\n");
        let line = parser.parse_line()
            .unwrap();
        assert_eq!("label", line.label.unwrap());
        assert!(line.statement.is_some());
    }
}
