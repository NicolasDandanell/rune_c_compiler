use std::io::Error;

use rune_parser::RuneParserError;

#[derive(Debug)]
pub enum CompilerError {
    InvalidArgument,
    InvalidInputPath,
    ParsingError(RuneParserError),
    FileSystemError(Error)
}
