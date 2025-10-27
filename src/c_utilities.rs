use rune_parser::{
    RuneFileDescription,
    scanner::NumericLiteral,
    types::{ArraySize, DefineValue, FieldIndex, FieldType, StructDefinition, StructMember, UserDefinitionLink}
};

use crate::c_standard::CStandard;

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
    pub fn parse(file_descriptions: &Vec<RuneFileDescription>, configurations: &CompileConfigurations) -> CConfigurations {
        let mut amount_of_messages: usize = 0;
        let mut largest_message_size: usize = 0;
        let mut largest_message_index: usize = 0;

        // Get the largest overall message size, and the amount of messages
        for file in file_descriptions {
            // Add struct definition amount to amount of messages
            amount_of_messages += file.definitions.structs.len();

            for struct_definition in &file.definitions.structs {
                let estimated_size: usize = struct_definition.estimate_size(configurations) as usize;

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

        // println!("Out of {0} messages, the largest one found was estimated at {1} bytes\n", amount_of_messages, largest_message_size);

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
            0 => panic!("Largest message had size 0! Something went horribly wrong!"),
            0x00000001..=0x000000FF => 1,
            0x00000100..=0x0000FFFF => 2,
            0x00010000..=0xFFFFFFFF => 4,
            // 8 byte option is probably not needed, but add anyway...
            _ => 8
        };

        let field_size_type_size: usize = message_size_type_size;
        let field_offset_type_size: usize = message_size_type_size;

        CConfigurations {
            compiler_configurations: configurations.clone(),
            field_size_type_size,
            field_offset_type_size,
            message_size_type_size,
            parser_index_type_size,
            largest_message_index
        }
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
            NumericLiteral::PositiveBinary(value) | NumericLiteral::PositiveDecimal(value) | NumericLiteral::PositiveHexadecimal(value) => value.leading_zeros() / 8,
            NumericLiteral::NegativeBinary(value) | NumericLiteral::NegativeDecimal(value) | NumericLiteral::NegativeHexadecimal(value) => value.leading_zeros() / 8,
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

// Field type methods
// ———————————————————

pub trait CFieldType {
    fn c_initializer(&self, c_standard: &CStandard) -> String;
    fn create_c_variable(&self, name: &String, spacing: usize, c_standard: &CStandard) -> String;
    fn primitive_c_size(&self) -> u64;
    fn to_c_type(&self, c_standard: &CStandard) -> String;
}

impl CFieldType for FieldType {
    fn to_c_type(&self, c_standard: &CStandard) -> String {
        match self {
            FieldType::Boolean => String::from(match c_standard.allows_boolean() {
                true => "bool",
                false => "char"
            }),

            FieldType::Char => String::from("char"),

            FieldType::UByte => String::from(match c_standard.allows_integer_types() {
                true => "uint8_t",
                false => "unsigned char"
            }),
            FieldType::Byte => String::from(match c_standard.allows_integer_types() {
                true => "int8_t",
                false => "signed char"
            }),

            FieldType::UShort => String::from(match c_standard.allows_integer_types() {
                true => "uint16_t",
                false => "unsigned short"
            }),
            FieldType::Short => String::from(match c_standard.allows_integer_types() {
                true => "int16_t",
                false => "signed short"
            }),

            FieldType::UInt => String::from(match c_standard.allows_integer_types() {
                true => "uint32_t",
                false => "unsigned long"
            }),
            FieldType::Int => String::from(match c_standard.allows_integer_types() {
                true => "int32_t",
                false => "signed long"
            }),

            FieldType::ULong => String::from(match c_standard.allows_integer_types() {
                true => "uint64_t",
                false => panic!("Cannot guarantee 64 bit integers before C99 standard!")
            }),
            FieldType::Long => String::from(match c_standard.allows_integer_types() {
                true => "int64_t",
                false => panic!("Cannot guarantee 64 bit integers before C99 standard!")
            }),

            FieldType::Float => String::from("float"),
            FieldType::Double => String::from("double"),

            FieldType::UserDefined(string) => format!("{0}_t", pascal_to_snake_case(string)),

            // This will return the string of the underlying type
            FieldType::Array(underlying_type, _) => underlying_type.to_c_type(c_standard),

            FieldType::Empty => panic!("Empty fields have no type!")
        }
    }

    fn create_c_variable(&self, name: &String, spacing: usize, c_standard: &CStandard) -> String {
        match self {
            FieldType::Boolean
            | FieldType::Char
            | FieldType::UByte
            | FieldType::Byte
            | FieldType::UShort
            | FieldType::Short
            | FieldType::Float
            | FieldType::UInt
            | FieldType::Int
            | FieldType::Double
            | FieldType::ULong
            | FieldType::Long => format!("{0} {1}{2}", self.to_c_type(c_standard), spaces(spacing), name),

            FieldType::UserDefined(string) => format!("{0}_t {1}{2}", pascal_to_snake_case(string), spaces(spacing), name),
            FieldType::Array(field_type, field_size) => format!("{0} {1}{2}[{3}]", field_type.to_c_type(c_standard), spaces(spacing), name, field_size.to_string()),
            FieldType::Empty => panic!("Cannot create an empty field!")
        }
    }

    // Size is calculated without padding, and is a guesstimate at best
    fn primitive_c_size(&self) -> u64 {
        match self {
            FieldType::Boolean => 1,
            FieldType::Char => 1,
            FieldType::UByte => 1,
            FieldType::Byte => 1,

            FieldType::UShort => 2,
            FieldType::Short => 2,

            FieldType::Float => 4,
            FieldType::UInt => 4,
            FieldType::Int => 4,

            FieldType::Double => 8,
            FieldType::ULong => 8,
            FieldType::Long => 8,

            FieldType::Empty => 0,
            _ => panic!("Cannot call this function on an array or user defined type")
        }
    }

    fn c_initializer(&self, c_standard: &CStandard) -> String {
        match self {
            FieldType::Boolean => match c_standard.allows_boolean() {
                true => String::from("false"),
                false => String::from("0")
            },
            FieldType::Char => String::from("0"),
            FieldType::Byte => String::from("0"),
            FieldType::UByte => String::from("0"),
            FieldType::Short => String::from("0"),
            FieldType::UShort => String::from("0"),
            FieldType::Float => String::from("0.0"),
            FieldType::Int => String::from("0"),
            FieldType::UInt => String::from("0"),
            FieldType::Double => String::from("0.0"),
            FieldType::Long => String::from("0"),
            FieldType::ULong => String::from("0"),
            FieldType::Empty => panic!("Cannot initialize an empty field!"),
            FieldType::UserDefined(name) => format!("{0}_INIT", pascal_to_uppercase(&name)),
            FieldType::Array(field_type, _) => format!(
                "{{ {0} }}",
                match field_type.as_ref() {
                    FieldType::Boolean => String::from("false"),
                    FieldType::Char | FieldType::Byte | FieldType::UByte | FieldType::Short | FieldType::UShort | FieldType::Int | FieldType::UInt | FieldType::Long | FieldType::ULong =>
                        String::from("0"),
                    FieldType::Float | FieldType::Double => String::from("0.0"),
                    FieldType::UserDefined(name) => format!("{0}_INIT", pascal_to_uppercase(&name)),
                    FieldType::Array(_, _) => panic!("Nested arrays are not currently supported"),
                    FieldType::Empty => panic!("Cannot initialize an empty field!")
                }
            )
        }
    }
}

// Struct member methods
// ——————————————————————

pub trait CStructMember {
    fn c_size(&self) -> u64;
    fn c_size_definition(&self, standard: &CStandard) -> String;
    fn index_empty(index: u64) -> StructMember;
}

impl CStructMember for StructMember {
    fn index_empty(index: u64) -> StructMember {
        // Check if value is positive and within the legal values (0 to and including 31)
        let field_index = match index {
            // Legal values
            0..32 => FieldIndex::Numeric(index),
            // Higher than legal values
            32.. => panic!("Field index cannot have a value higher than 31!")
        };

        StructMember {
            identifier:           String::from("(empty)"),
            data_type:            FieldType::Empty,
            index:                field_index,
            user_definition_link: UserDefinitionLink::NoLink,
            comment:              None
        }
    }

    fn c_size_definition(&self, standard: &CStandard) -> String {
        let size_string: String = match &self.data_type {
            FieldType::UserDefined(type_name) => {
                format!("sizeof({0}_t)", pascal_to_snake_case(&type_name))
            },
            FieldType::Array(array_type, array_size) => {
                let type_string: String = match &(**array_type) {
                    FieldType::Array(_, _) => panic!("Nested arrays are not supported!"),
                    FieldType::UserDefined(name) => {
                        format!("sizeof({0}_t)", pascal_to_snake_case(&name))
                    },
                    _ => format!("sizeof({0})", array_type.to_c_type(standard))
                };

                format!("({0} * {1})", type_string, array_size.to_string())
            },
            FieldType::Empty => String::from("0"),
            _ => format!("sizeof({0})", self.data_type.to_c_type(standard))
        };
        size_string
    }

    fn c_size(&self) -> u64 {
        match &self.data_type {
            // Calculate Array size based on (field type * field size)
            FieldType::Array(array_field_type, field_size) => {
                // Get the array size first
                let array_size: u64 = match field_size {
                    ArraySize::Binary(value) | ArraySize::Decimal(value) | ArraySize::Hexadecimal(value) => *value,
                    ArraySize::UserDefinition(definition) => match &definition.value {
                        DefineValue::NumericLiteral(value) => match value {
                            NumericLiteral::PositiveBinary(binary) => *binary,
                            NumericLiteral::PositiveDecimal(decimal) => *decimal,
                            NumericLiteral::PositiveHexadecimal(hexadecimal) => *hexadecimal,
                            _ => panic!("Got \"{0:?}\" array size definition of an invalid type!", self.identifier)
                        },
                        _ => panic!("Got \"{0}\" array size definition of an invalid type!", self.identifier)
                    }
                };

                // Parse the byte size based on the array type
                match *array_field_type.to_owned() {
                    FieldType::Array(_, _) => panic!("Nested arrays not allowed at the moment"),

                    // Parse the user defined type using the member user_definition_link
                    FieldType::UserDefined(type_string) => match &self.user_definition_link {
                        UserDefinitionLink::NoLink => panic!("Could not find definition for type {0} while parsing C size", type_string),
                        UserDefinitionLink::BitfieldLink(bitfield_definition) => bitfield_definition.backing_type.primitive_c_size() * array_size,
                        UserDefinitionLink::EnumLink(enum_definition) => enum_definition.backing_type.primitive_c_size() * array_size,
                        UserDefinitionLink::StructLink(struct_definition) => {
                            let mut struct_size: u64 = 0;

                            // Call this function recursively for each struct member to get size
                            for member in &struct_definition.members {
                                struct_size += member.c_size();
                            }

                            struct_size * array_size
                        }
                    },

                    // Primitives
                    _ => array_field_type.primitive_c_size() * array_size
                }
            },

            FieldType::UserDefined(name) => match &self.user_definition_link {
                UserDefinitionLink::NoLink => {
                    panic!("Found no definition link for item {0}!", name)
                },
                UserDefinitionLink::BitfieldLink(bitfield_definition) => bitfield_definition.backing_type.primitive_c_size(),
                UserDefinitionLink::EnumLink(enum_definition) => enum_definition.backing_type.primitive_c_size(),
                UserDefinitionLink::StructLink(struct_definition) => {
                    let mut total_size: u64 = 0;

                    for member in &struct_definition.members {
                        total_size += member.c_size();
                    }

                    total_size
                }
            },

            // Primitives
            _ => self.data_type.primitive_c_size()
        }
    }
}

// Struct definition methods
// ——————————————————————————

pub trait CStructDefinition {
    fn estimate_size(&self, configurations: &CompileConfigurations) -> u64;
    fn sort_members(&self) -> Vec<StructMember>;
}

impl CStructDefinition for StructDefinition {
    /// Sort the members of a struct based on their size alignment to reduce eventual padding
    fn sort_members(&self) -> Vec<StructMember> {
        let mut full_list: Vec<StructMember> = Vec::with_capacity(0x20);

        let mut aligned_8: Vec<StructMember> = Vec::with_capacity(0x20);
        let mut aligned_4: Vec<StructMember> = Vec::with_capacity(0x20);
        let mut aligned_2: Vec<StructMember> = Vec::with_capacity(0x20);
        let mut aligned_1: Vec<StructMember> = Vec::with_capacity(0x20);

        for member in &self.members {
            // Zero-size members are discarded
            if member.c_size() == 0 {
                continue;
            }

            if member.c_size() % 8 == 0 {
                // First 8 aligned
                aligned_8.push(member.clone());
            } else if member.c_size() % 4 == 0 {
                // First 4 aligned
                aligned_4.push(member.clone());
            } else if member.c_size() % 2 == 0 {
                // First 2 aligned
                aligned_2.push(member.clone());
            } else {
                // Lastly non aligned
                aligned_1.push(member.clone());
            }
        }

        // Size sort unaligned structs
        aligned_1.sort_by(|a, b| b.c_size().cmp(&a.c_size()));

        full_list.append(&mut aligned_8);
        full_list.append(&mut aligned_4);
        full_list.append(&mut aligned_2);
        full_list.append(&mut aligned_1);

        full_list
    }

    fn estimate_size(&self, configurations: &CompileConfigurations) -> u64 {
        // println!("Estimating size of {0}", struct_definition.name);

        let struct_list: Vec<StructMember> = match configurations.sort {
            true => self.sort_members(),
            false => self.members.clone()
        };

        // Calculate padding
        let mut total_size: u64 = 0;

        for member in &struct_list {
            // println!("   {0} - {1} bytes", member.identifier, member.c_size());

            // Assume 8 byte alignment target for items > 4 bytes for worst case scenario
            let member_alignment_size: u64 = match member.c_size() {
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
                // println!("    > Estimated {0} bytes of padding for member \"{1}\"", padding, member.identifier);
            }

            total_size += member.c_size();
        }

        // println!("   = Estimated total - {0} bytes\n", total_size);

        total_size
    }
}
