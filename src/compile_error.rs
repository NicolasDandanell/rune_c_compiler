use std::io::Error;

use rune_parser::RuneParserError;

#[derive(Debug)]
pub enum CompilerError {
    InvalidArgument,
    InvalidInputPath,
    ConfigurationError,
    SourceAndCStandardMismatch,
    ParsingError(RuneParserError),
    LogicError,
    MalformedSource,
    UnsupportedFeature,
    FileSystemError(Error)
}
