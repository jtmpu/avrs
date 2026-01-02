use avrs_core::{instruction::Add, types::Unsigned};
use miette::SourceSpan;

use crate::parser::{Operand, OperandKind, ParsedStatement, Statement};

pub struct AssemblyState {

}

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum InstructionError {
    #[error("failed parsing '{instruction}': {message}")]
    InvalidOperand {
        instruction: &'static str,
        message: String,
        #[label("error occured while reading here")]
        span: SourceSpan,
    },
    #[error("failed parsing '{instruction}': received '{received}' args but expected '{expected}'")]
    InvalidOperandCount {
        instruction: &'static str,
        received: usize,
        expected: usize,
        #[label("error occured while reading here")]
        span: SourceSpan,
    },
    #[error("instruction parsing received unexpected directive")]
    UnexpectedDirective {
        #[label("error occured while reading here")]
        span: SourceSpan,
    },
}

fn get_instruction(statement: &Statement) -> Result<&ParsedStatement, InstructionError> {
    match statement {
        Statement::Instruction(b) => Ok(b),
        Statement::Directive(b) => {
            Err(InstructionError::UnexpectedDirective { span: b.span })
        }
    }
}

pub trait AssemblyParse {
    const NAME: &'static str;
    fn parse(statement: &ParsedStatement, state: &AssemblyState) -> Result<Self, InstructionError>
    where
        Self: std::marker::Sized;
}

impl AssemblyParse for Add {
    const NAME: &'static str = "add";

    fn parse(statement: &ParsedStatement, state: &AssemblyState) -> Result<Self, InstructionError> {
        if statement.operands.len() != 2 {
            return Err(InstructionError::InvalidOperandCount { instruction: Self::NAME, received: statement.operands.len(), expected: 2, span: statement.span });
        }

        let op = &statement.operands[0];
        let rd: Unsigned<0, 31> = match &op.kind {
            OperandKind::Register(index) => (*index).try_into().map_err(|e|{
                InstructionError::InvalidOperand { instruction: Self::NAME, message: format!("{e}"), span: op.span }

            })?,
            x => return Err(InstructionError::InvalidOperand { instruction: Self::NAME, message: format!("expected register, received: {x:?}"), span: op.span }),
        };
        let op = &statement.operands[1];
        let rr = match &op.kind {
            OperandKind::Register(index) => (*index).try_into().map_err(|e|{
                InstructionError::InvalidOperand { instruction: Self::NAME, message: format!("{e}"), span: op.span }

            })?,
            x => return Err(InstructionError::InvalidOperand { instruction: Self::NAME, message: format!("expected register, received: {x:?}"), span: op.span }),
        };

        let add = Add::new(rd, rr);
        Ok(add)
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::Parser;

    use super::*;

    #[test]
    fn test_add() {
        let mut parser = Parser::new("add r1, r2\n".as_bytes());
        let line = parser.next().unwrap();
        let stmt = line.statement.unwrap();
        let add = Add::parse(&stmt, &AssemblyState {  }).unwrap();
        assert_eq!(add.d.inner(), 1);
        assert_eq!(add.r.inner(), 2);
    }

}
