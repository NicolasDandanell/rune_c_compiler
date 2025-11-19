use std::fmt::{Display, Formatter};

use rune_parser::RuneFileDescription;

use crate::{c_utilities::CMessageDefinition, compile_error::CompilerError, output::*};

// Architecture
// —————————————

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

// C Standard
// ———————————

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum CStandard {
    // C90 is an alias for C89
    C89 = 0,
    C95 = 1,
    C99 = 2,
    C11 = 3,
    C17 = 4,
    C23 = 5
}

impl CStandard {
    pub fn from_string(string: &str) -> Result<CStandard, CompilerError> {
        match string {
            "c89" | "C89" | "c90" | "C90" => Ok(CStandard::C89),
            "c95" | "C95" => Ok(CStandard::C95),
            "c99" | "C99" => Ok(CStandard::C99),
            "c11" | "C11" => Ok(CStandard::C11),
            "c17" | "C17" => Ok(CStandard::C17),
            "c23" | "C23" => Ok(CStandard::C23),
            _ => {
                error!("Invalid C Standard passed. Got {0}, and valid values are: {1}", string, CStandard::valid_values());
                Err(CompilerError::InvalidArgument)
            }
        }
    }

    fn valid_values() -> String {
        String::from("C89/C90, C95, C99, C11, C17, C23")
    }

    // C99
    // ————

    pub fn allows_boolean(&self) -> bool {
        *self >= CStandard::C99
    }

    pub fn allows_designated_initializers(&self) -> bool {
        *self >= CStandard::C99
    }

    pub fn allows_flexible_array_members(&self) -> bool {
        *self >= CStandard::C99
    }

    pub fn allows_inline(&self) -> bool {
        *self >= CStandard::C99
    }

    pub fn allows_integer_types(&self) -> bool {
        *self >= CStandard::C99
    }

    // C23
    // ————

    pub fn allows_enum_backing_type(&self) -> bool {
        *self >= CStandard::C23
    }
}

impl Display for CStandard {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CStandard::C89 => write!(formatter, "C89"),
            CStandard::C95 => write!(formatter, "C95"),
            CStandard::C99 => write!(formatter, "C99"),
            CStandard::C11 => write!(formatter, "C11"),
            CStandard::C17 => write!(formatter, "C17"),
            CStandard::C23 => write!(formatter, "C23")
        }
    }
}

// C Configuration
// ————————————————

#[derive(Debug, Clone)]
pub struct CompileConfigurations {
    /// Which architecture to optimize for
    pub architecture: Architecture,

    /// Whether or not to pack message data structures
    pub pack_data: bool,

    /// Whether or not to pack parsing metadata structures
    pub pack_metadata: bool,

    /// Whether to declare all rune data in a specific section - Default to None
    pub section: Option<String>,

    /// Whether to size sort structs to optimize packing - Defaults to true
    pub sort: bool,

    /// Specifies which C standard the output source should comply with
    pub c_standard: CStandard
}

pub struct CConfigurations {
    // Configurations
    pub compiler_configurations: CompileConfigurations,

    // Data definitions
    pub field_size_type_size:   usize,
    pub field_offset_type_size: usize,
    pub message_size_type_size: usize,
    pub parser_index_type_size: usize,

    // Largest encountered declared message index
    pub largest_message_index: usize
}

impl CConfigurations {
    pub fn parse(file_descriptions: &Vec<RuneFileDescription>, configurations: &CompileConfigurations) -> Result<CConfigurations, CompilerError> {
        let mut amount_of_messages: usize = 0;
        let mut largest_message_size: usize = 0;
        let mut largest_message_index: usize = 0;

        // Get the largest overall message size, and the amount of messages
        for file in file_descriptions {
            // Add message definition amount to amount of messages
            amount_of_messages += file.definitions.messages.len();

            for message_definition in &file.definitions.messages {
                let estimated_size: usize = message_definition.estimate_size(configurations)? as usize;

                if estimated_size > largest_message_size {
                    largest_message_size = estimated_size;
                }

                for member in &message_definition.fields {
                    if member.index.value() as usize > largest_message_index {
                        largest_message_index = member.index.value() as usize;
                    }
                }
            }
        }

        // Get the unsigned integer size needed to describe the number of messages
        let parser_index_type_size: usize = match amount_of_messages {
            0x00000000..=0x000000FF => 1,
            0x00000100..=0x0000FFFF => 2,
            0x00010000..=0xFFFFFFFF => 4,
            // 8 byte option is probably not needed, but add anyway...
            _ => 8
        };

        // Field size type and offset size type will be based on the largest message size
        let message_size_type_size: usize = match largest_message_size {
            0 => {
                error!("Largest message had size 0! Something went horribly wrong!");
                return Err(CompilerError::ConfigurationError);
            },
            0x00000001..=0x000000FF => 1,
            0x00000100..=0x0000FFFF => 2,
            0x00010000..=0xFFFFFFFF => 4,
            // 8 byte option is probably not needed, but add anyway...
            _ => 8
        };

        let field_size_type_size: usize = message_size_type_size;
        let field_offset_type_size: usize = message_size_type_size;

        Ok(CConfigurations {
            compiler_configurations: configurations.clone(),
            field_size_type_size,
            field_offset_type_size,
            message_size_type_size,
            parser_index_type_size,
            largest_message_index
        })
    }
}
