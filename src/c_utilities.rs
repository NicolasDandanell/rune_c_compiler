use rune_parser::{
    RuneFileDescription,
    scanner::NumericLiteral,
    types::{ArraySize, ArrayType, DefineValue, FieldIndex, FieldType, Primitive, StructDefinition, StructMember, UserDefinitionLink}
};

use crate::{c_standard::CStandard, compile_error::CompilerError, output::*};

// String helper functions
// ————————————————————————

/// Output the amount of ' ' spaces
pub fn spaces(amount: usize) -> String {
    let mut spaces = String::with_capacity(0x40);

    for _ in 0..amount {
        spaces.push(' ');
    }

    spaces
}

/// Convert NamedVariable to named_variable
pub fn pascal_to_snake_case(pascal: &String) -> String {
    let mut snake: String = String::with_capacity(0x40);

    for i in 0..pascal.len() {
        let letter: char = pascal.chars().nth(i).unwrap();

        if i != 0 && letter.is_ascii_uppercase() {
            snake.push('_');
        }

        snake.push(letter.to_ascii_lowercase());
    }

    snake
}

/// Convert NamedVariable to NAMED_VARIABLE
pub fn pascal_to_uppercase(pascal: &String) -> String {
    let mut uppecase: String = String::with_capacity(0x40);

    for i in 0..pascal.len() {
        let letter: char = pascal.chars().nth(i).unwrap();

        if i != 0 && letter.is_ascii_uppercase() {
            uppecase.push('_');
        }

        uppecase.push(letter.to_ascii_uppercase());
    }

    uppecase
}

// C Configuration
// ————————————————

#[derive(Debug, Clone)]
pub struct CompileConfigurations {
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
            // Add struct definition amount to amount of messages
            amount_of_messages += file.definitions.structs.len();

            for struct_definition in &file.definitions.structs {
                let estimated_size: usize = struct_definition.estimate_size(configurations)? as usize;

                if estimated_size > largest_message_size {
                    largest_message_size = estimated_size;
                }

                for member in &struct_definition.members {
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

// Numeric value helper functions
// ———————————————————————————————

pub trait CNumericValue {
    fn requires_size(&self) -> u64;
}

impl CNumericValue for NumericLiteral {
    fn requires_size(&self) -> u64 {
        let leading_zeroes = match self {
            NumericLiteral::Boolean(_) => return 1,
            NumericLiteral::PositiveInteger(value, _) => value.leading_zeros() / 8,
            NumericLiteral::NegativeInteger(value, _) => value.leading_zeros() / 8,
            NumericLiteral::Float(value) => value.to_bits().leading_zeros() / 8
        };

        match leading_zeroes {
            0..4 => 8,
            4..6 => 4,
            6..7 => 2,
            7.. => 1
        }
    }
}

// Primitive methods
// ——————————————————

pub trait CPrimitive {
    fn c_size(&self) -> u64;
    fn c_initializer(&self, c_standard: &CStandard) -> String;
    fn create_c_variable(&self, name: &String, spacing: usize, c_standard: &CStandard) -> Result<String, CompilerError>;
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

    fn create_c_variable(&self, name: &String, spacing: usize, c_standard: &CStandard) -> Result<String, CompilerError> {
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
            Primitive::I128 | Primitive::U128 => Ok(format!("{0} {1}{2}[{3}]", Primitive::U8.to_c_type(c_standard)?, spaces(spacing), name, self.c_size().to_string()))
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

// Array Type
// ———————————

pub trait CArrayType {
    fn to_c_type(&self, c_standard: &CStandard) -> Result<String, CompilerError>;
}

impl CArrayType for ArrayType {
    fn to_c_type(&self, c_standard: &CStandard) -> Result<String, CompilerError> {
        match self {
            ArrayType::Primitive(primitive) => primitive.to_c_type(c_standard),
            ArrayType::UserDefined(definition) => Ok(format!("{0}_t", pascal_to_snake_case(definition)))
        }
    }
}

// Field type methods
// ———————————————————

pub trait CFieldType {
    fn c_initializer(&self, c_standard: &CStandard) -> Result<String, CompilerError>;
    fn create_c_variable(&self, name: &String, spacing: usize, c_standard: &CStandard) -> Result<String, CompilerError>;
}

impl CFieldType for FieldType {
    fn create_c_variable(&self, name: &String, spacing: usize, c_standard: &CStandard) -> Result<String, CompilerError> {
        match self {
            FieldType::Primitive(primitive) => primitive.create_c_variable(name, spacing, c_standard),
            FieldType::UserDefined(string) => Ok(format!("{0}_t {1}{2}", pascal_to_snake_case(string), spaces(spacing), name)),
            FieldType::Array(field_type, field_size) => Ok(format!("{0} {1}{2}[{3}]", field_type.to_c_type(c_standard)?, spaces(spacing), name, field_size.to_string())),
            FieldType::Empty => {
                error!("Cannot create an empty field!");
                return Err(CompilerError::LogicError);
            }
        }
    }

    fn c_initializer(&self, c_standard: &CStandard) -> Result<String, CompilerError> {
        let string = match self {
            FieldType::Primitive(primitive) => primitive.c_initializer(c_standard),
            FieldType::UserDefined(name) => format!("{0}_INIT", pascal_to_uppercase(&name)),
            FieldType::Array(array_type, _) => format!(
                "{{ {0} }}",
                match array_type {
                    // Special 128 bit case
                    ArrayType::Primitive(primitive) if *primitive == Primitive::I128 || *primitive == Primitive::U128 => {
                        String::from("0")
                    },
                    ArrayType::Primitive(primitive) => primitive.c_initializer(c_standard),
                    ArrayType::UserDefined(name) => format!("{0}_INIT", pascal_to_uppercase(&name))
                }
            ),
            FieldType::Empty => {
                error!("Cannot initialize an empty field!");
                return Err(CompilerError::LogicError);
            }
        };
        Ok(string)
    }
}

// Struct member methods
// ——————————————————————

pub trait CStructMember {
    fn c_size(&self) -> Result<u64, CompilerError>;
    fn c_size_definition(&self, c_standard: &CStandard) -> Result<String, CompilerError>;
    fn index_empty(index: u64) -> Result<StructMember, CompilerError>;
}

impl CStructMember for StructMember {
    fn index_empty(index: u64) -> Result<StructMember, CompilerError> {
        // Check if value is positive and within the legal values (0 to and including 31)
        let field_index = match index {
            // Legal values
            0..32 => FieldIndex::Numeric(index),
            // Higher than legal values
            32.. => {
                error!("Field index cannot have a value higher than 31!");
                return Err(CompilerError::LogicError);
            }
        };

        Ok(StructMember {
            identifier:           String::from("(empty)"),
            data_type:            FieldType::Empty,
            index:                field_index,
            user_definition_link: UserDefinitionLink::NoLink,
            comment:              None
        })
    }

    fn c_size_definition(&self, c_standard: &CStandard) -> Result<String, CompilerError> {
        let size_string: String = match &self.data_type {
            FieldType::Primitive(primitive) => format!("sizeof({0})", primitive.to_c_type(c_standard)?),
            FieldType::UserDefined(type_name) => format!("sizeof({0}_t)", pascal_to_snake_case(&type_name)),
            FieldType::Array(array_type, array_size) => {
                let type_string: String = match array_type {
                    ArrayType::Primitive(primitive) => format!("sizeof({0})", primitive.to_c_type(c_standard)?),
                    ArrayType::UserDefined(name) => format!("sizeof({0}_t)", pascal_to_snake_case(&name))
                };

                format!("({0} * {1})", type_string, array_size.to_string())
            },
            FieldType::Empty => String::from("0")
        };
        Ok(size_string)
    }

    fn c_size(&self) -> Result<u64, CompilerError> {
        match &self.data_type {
            // Calculate Array size based on (field type * field size)
            FieldType::Array(array_type, field_size) => {
                // Get the array size first
                let array_size: u64 = match field_size {
                    ArraySize::Integer(value, _) => *value,
                    ArraySize::UserDefinition(definition) => match &definition.value {
                        DefineValue::NumericLiteral(value) => match value {
                            NumericLiteral::PositiveInteger(value, _) => *value,
                            _ => {
                                error!("Got \"{0:?}\" array size definition of an invalid type!", self.identifier);
                                return Err(CompilerError::MalformedSource);
                            }
                        },
                        _ => {
                            error!("Got \"{0}\" array size definition of an invalid type!", self.identifier);
                            return Err(CompilerError::MalformedSource);
                        }
                    }
                };

                // Parse the byte size based on the array type
                let total_size: u64 = match array_type {
                    ArrayType::Primitive(primitive) => primitive.c_size() * array_size,
                    ArrayType::UserDefined(definition) => match &self.user_definition_link {
                        UserDefinitionLink::NoLink => {
                            error!("Could not find definition for type {0} while parsing C size. This should not happen!", definition);
                            return Err(CompilerError::MalformedSource);
                        },
                        UserDefinitionLink::BitfieldLink(bitfield_definition) => bitfield_definition.backing_type.c_size() * array_size,
                        UserDefinitionLink::EnumLink(enum_definition) => enum_definition.backing_type.c_size() * array_size,
                        UserDefinitionLink::StructLink(struct_definition) => {
                            let mut struct_size: u64 = 0;

                            // Call this function recursively for each struct member to get size
                            for member in &struct_definition.members {
                                struct_size += member.c_size()?;
                            }

                            struct_size * array_size
                        }
                    }
                };

                Ok(total_size)
            },
            FieldType::Empty => Ok(0),
            FieldType::Primitive(primitive) => Ok(primitive.c_size()),
            FieldType::UserDefined(name) => match &self.user_definition_link {
                UserDefinitionLink::NoLink => {
                    error!("Found no definition link for item {0}!", name);
                    return Err(CompilerError::MalformedSource);
                },
                UserDefinitionLink::BitfieldLink(bitfield_definition) => Ok(bitfield_definition.backing_type.c_size()),
                UserDefinitionLink::EnumLink(enum_definition) => Ok(enum_definition.backing_type.c_size()),
                UserDefinitionLink::StructLink(struct_definition) => {
                    let mut total_size: u64 = 0;

                    for member in &struct_definition.members {
                        total_size += member.c_size()?;
                    }

                    Ok(total_size)
                }
            }
        }
    }
}

// Struct definition methods
// ——————————————————————————

pub trait CStructDefinition {
    fn estimate_size(&self, configurations: &CompileConfigurations) -> Result<u64, CompilerError>;
    fn sort_members(&self) -> Result<Vec<StructMember>, CompilerError>;
}

impl CStructDefinition for StructDefinition {
    /// Sort the members of a struct based on their size alignment to reduce eventual padding
    fn sort_members(&self) -> Result<Vec<StructMember>, CompilerError> {
        let mut full_list: Vec<StructMember> = Vec::with_capacity(0x20);

        let mut aligned_8: Vec<(StructMember, u64)> = Vec::with_capacity(0x20);
        let mut aligned_4: Vec<(StructMember, u64)> = Vec::with_capacity(0x20);
        let mut aligned_2: Vec<(StructMember, u64)> = Vec::with_capacity(0x20);
        let mut aligned_1: Vec<(StructMember, u64)> = Vec::with_capacity(0x20);

        for member in &self.members {
            let size: u64 = member.c_size()?;

            // Zero-size members are discarded
            if size == 0 {
                warning!("Member {0} of struct {1} had size 0.", member.identifier, self.name);
                continue;
            }

            if size % 8 == 0 {
                // First 8 aligned
                aligned_8.push((member.clone(), size));
            } else if member.c_size()? % 4 == 0 {
                // First 4 aligned
                aligned_4.push((member.clone(), size));
            } else if member.c_size()? % 2 == 0 {
                // First 2 aligned
                aligned_2.push((member.clone(), size));
            } else {
                // Lastly non aligned
                aligned_1.push((member.clone(), size));
            }
        }

        // Sort the 1 aligned members by size
        aligned_1.sort_by(|a, b| b.1.cmp(&a.1));

        // Append all member elements into the full sorted list
        full_list.append(&mut aligned_8.into_iter().map(|(member, _)| member).collect());
        full_list.append(&mut aligned_4.into_iter().map(|(member, _)| member).collect());
        full_list.append(&mut aligned_2.into_iter().map(|(member, _)| member).collect());
        full_list.append(&mut aligned_1.into_iter().map(|(member, _)| member).collect());

        Ok(full_list)
    }

    fn estimate_size(&self, configurations: &CompileConfigurations) -> Result<u64, CompilerError> {
        // println!("Estimating size of {0}", struct_definition.name);

        let struct_list: Vec<StructMember> = match configurations.sort {
            true => self.sort_members()?,
            false => self.members.clone()
        };

        // Calculate padding
        let mut total_size: u64 = 0;

        for member in &struct_list {
            // println!("   {0} - {1} bytes", member.identifier, member.c_size());

            // Assume 8 byte alignment target for items > 4 bytes for worst case scenario
            let member_alignment_size: u64 = match member.c_size()? {
                // Members with a size 0 can be skipped
                0 => continue,
                1 => 1,
                2 => 2,
                3..=4 => 4,
                // Assume that anything bigger than 4 bytes needs to align to 8 bytes as a worst case scenario (64 bit targets)
                5.. => 8
            };

            // Estimate padding if packing disabled, and member does not align to the worst case 8 bytes (64 bit targets)
            if !configurations.pack_data && (total_size % member_alignment_size) != 0 {
                // Add padding
                let padding: u64 = member_alignment_size - (total_size % member_alignment_size);
                total_size += padding;
            }

            total_size += member.c_size()?;
        }

        Ok(total_size)
    }
}
