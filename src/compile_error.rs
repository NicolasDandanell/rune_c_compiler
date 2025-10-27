#[derive(Debug, Clone)]
pub enum CompilerError {
    InvalidArgument,
    InvalidInputPath,
    FileSystemError
}
