use std::path::PathBuf;

use clap::Parser;

use crate::parser::lexer::{Lexer, TokenKind};

mod parser;

#[derive(Parser)]
struct Args {
    #[arg(short, long)]
    pub file: PathBuf,
}

fn main() -> miette::Result<()> {
    let args = Args::parse();

    let src = std::fs::read_to_string(args.file).unwrap();
    let mut lexer = Lexer::new(src.as_bytes());

    /*
    while true {
        let token = match lexer.next() {
            Ok(token) => token,
            Err(e) => Err(e.with_source_code(src.clone()))?,
        };
        println!("{token:?}");
        if token.kind == TokenKind::Eof {
            break;
        }
    }
    */

    Ok(())
}
