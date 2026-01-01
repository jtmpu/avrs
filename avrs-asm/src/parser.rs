/// Grammar
/// program     ::= { line } EOF
/// line        ::= [ label ] [ statement ] NEWLINE
/// label       ::= IDENT COLON
/// statement   ::= instruction
///                | directive
///
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

use miette::SourceSpan;

pub mod tagged;
pub mod lexer;
pub mod reader;
pub mod buflexer;

use crate::parser::{buflexer::BufferedLexer, lexer::{LexerError, TokenKind}, tagged::Tagged};

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

#[derive(Debug)]
pub struct Label {
    name: String,
    span: SourceSpan,
}

#[derive(Debug)]
pub struct Instruction {
    name: String,
    operands: Vec<Operand>,
    span: SourceSpan,
}

pub type Operand = Tagged<OperandKind>;
#[derive(Debug)]
pub enum OperandKind {
    Register(u8),
    Expression(Expression),
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

    pub fn parse_instruction(&mut self) -> Result<Instruction, ParserError> {
        let token = self.lexer.advance()?;
        let mnemonic = match token.kind {
            TokenKind::Identifier(value) => value,
            _ => {
                return Err(ParserError::UnexpectedToken { 
                    topic: "instruction".to_string(),
                    received: token.kind.clone(),
                    expected: vec![TokenKind::RParen],
                    span: token.span,
                });
            }
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
            combined_span(token.span, op.span)
        } else {
            token.span
        };

        let instruction = Instruction {
            name: mnemonic,
            operands,
            span,
        };
        Ok(instruction)
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

    pub fn parse_operand(&mut self) -> Result<Operand, ParserError> {
        let token = self.lexer.peek()?;
        println!("{:?}", token);

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

    pub fn parse_expression(&mut self) -> Result<Expression, ParserError> {
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
    use crate::parser::{Expression, ExpressionKind, OperandKind, Parser };

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
    pub fn test_instruction() {
        let mut parser = create("mov r2, r3");
        let instr = parser.parse_instruction()
            .unwrap();
        assert_eq!(instr.name, "mov");
        match instr.operands[0].kind {
            OperandKind::Register(value) => assert_eq!(2, value),
            _ => panic!("expected register operand, received {:?}", instr.operands[0].kind),
        }
        match instr.operands[1].kind {
            OperandKind::Register(value) => assert_eq!(3, value),
            _ => panic!("expected register operand, received {:?}", instr.operands[0].kind),
        }

        let mut parser = create("ldi r2, (2 + (4 * 3))");
        let instr = parser.parse_instruction()
            .unwrap();
        assert_eq!(instr.name, "ldi");
        match instr.operands[0].kind {
            OperandKind::Register(value) => assert_eq!(2, value),
            _ => panic!("expected register operand, received {:?}", instr.operands[0].kind),
        }
        match &instr.operands[1].kind {
            OperandKind::Expression(value) => assert_eq!(14, value.evaluate()),
            _ => panic!("expected register operand, received {:?}", instr.operands[0].kind),
        }
    }
}
