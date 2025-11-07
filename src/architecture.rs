use crate::{compile_error::CompilerError, output::*};

#[derive(Clone, Debug, PartialEq)]
pub enum Architecture {
    _32Bit,
    _64Bit
}

impl Architecture {
    pub fn from_value(value: usize) -> Result<Architecture, CompilerError> {
        match value {
            32 => Ok(Architecture::_32Bit),
            64 => Ok(Architecture::_64Bit),
            _ => {
                error!("Invalid architecture passed. Got {0}, and valid values are: {1}", value, Architecture::valid_values());
                Err(CompilerError::InvalidArgument)
            }
        }
    }

    pub fn byte_size(&self) -> usize {
        match self {
            Architecture::_32Bit => 4,
            Architecture::_64Bit => 8
        }
    }

    fn valid_values() -> String {
        String::from("64, 32")
    }
}
