use rune_parser::types::Primitive;

use crate::{CStandard, CompilerError, c_utilities::spaces, output::*};

// Primitive methods
// ——————————————————

pub trait CPrimitive {
    fn c_size(&self) -> u64;
    fn c_initializer(&self, c_standard: &CStandard) -> String;
    fn create_c_variable(&self, name: &str, spacing: usize, c_standard: &CStandard) -> Result<String, CompilerError>;
    fn to_c_type(&self, c_standard: &CStandard) -> Result<String, CompilerError>;
}

impl CPrimitive for Primitive {
    fn c_size(&self) -> u64 {
        match self {
            Primitive::Bool | Primitive::Char | Primitive::I8 | Primitive::U8 => 1,

            Primitive::I16 | Primitive::U16 => 2,

            Primitive::F32 | Primitive::I32 | Primitive::U32 => 4,

            Primitive::F64 | Primitive::I64 | Primitive::U64 => 8,

            Primitive::I128 | Primitive::U128 => 16
        }
    }

    fn c_initializer(&self, c_standard: &CStandard) -> String {
        match self {
            Primitive::Bool => match c_standard.allows_boolean() {
                true => String::from("false"),
                false => String::from("0")
            },

            Primitive::Char | Primitive::I8 | Primitive::U8 | Primitive::I16 | Primitive::U16 | Primitive::I32 | Primitive::U32 | Primitive::I64 | Primitive::U64 => String::from("0"),

            Primitive::F32 | Primitive::F64 => String::from("0.0"),

            // 128 bit integers are converted into 16 byte arrays in this implementation, due to lack of good 128 bit int support
            Primitive::I128 | Primitive::U128 => String::from("{ 0 }")
        }
    }

    fn create_c_variable(&self, name: &str, spacing: usize, c_standard: &CStandard) -> Result<String, CompilerError> {
        match self {
            Primitive::Bool
            | Primitive::Char
            | Primitive::I8
            | Primitive::U8
            | Primitive::I16
            | Primitive::U16
            | Primitive::F32
            | Primitive::I32
            | Primitive::U32
            | Primitive::F64
            | Primitive::I64
            | Primitive::U64 => Ok(format!("{0} {1}{2}", self.to_c_type(c_standard)?, spaces(spacing), name)),

            // 128 bit integers get converted into a byte array
            Primitive::I128 | Primitive::U128 => Ok(format!("{0} {1}{2}[{3}]", Primitive::U8.to_c_type(c_standard)?, spaces(spacing), name, self.c_size()))
        }
    }

    fn to_c_type(&self, c_standard: &CStandard) -> Result<String, CompilerError> {
        let string: String = match self {
            // 8 Bit
            Primitive::Bool => String::from(match c_standard.allows_boolean() {
                true => "bool",
                false => "char"
            }),
            Primitive::Char => String::from("char"),
            Primitive::I8 => String::from(match c_standard.allows_integer_types() {
                true => "int8_t",
                false => "signed char"
            }),
            Primitive::U8 => String::from(match c_standard.allows_integer_types() {
                true => "uint8_t",
                false => "unsigned char"
            }),

            // 16 Bit
            Primitive::I16 => String::from(match c_standard.allows_integer_types() {
                true => "int16_t",
                false => "signed short"
            }),
            Primitive::U16 => String::from(match c_standard.allows_integer_types() {
                true => "uint16_t",
                false => "unsigned short"
            }),

            // 32 Bit
            Primitive::F32 => String::from("float"),
            Primitive::I32 => String::from(match c_standard.allows_integer_types() {
                true => "int32_t",
                false => "signed long"
            }),
            Primitive::U32 => String::from(match c_standard.allows_integer_types() {
                true => "uint32_t",
                false => "unsigned long"
            }),

            // 64 Bit
            Primitive::F64 => String::from("double"),
            Primitive::I64 => String::from(match c_standard.allows_integer_types() {
                true => "int64_t",
                false => {
                    error!("Cannot guarantee 64 bit integers before C99 standard! Thus they are not allowed if using {0}", c_standard.to_string());
                    return Err(CompilerError::SourceAndCStandardMismatch);
                }
            }),
            Primitive::U64 => String::from(match c_standard.allows_integer_types() {
                true => "uint64_t",
                false => {
                    error!("Cannot guarantee 64 bit integers before C99 standard! Thus they are not allowed if using {0}", c_standard.to_string());
                    return Err(CompilerError::SourceAndCStandardMismatch);
                }
            }),

            // 128 Bit - Devolve into unsigned 16 Byte arrays
            Primitive::I128 | Primitive::U128 => String::from(match c_standard.allows_integer_types() {
                true => "uint8_t[16]",
                false => "unsigned char[16]"
            })
        };
        Ok(string)
    }
}
