use crate::CompileConfigurations;
use rune_parser::{ types::{ ArraySize, DefineValue, FieldSlot, FieldType, StructDefinition, StructMember, UserDefinitionLink }, RuneFileDescription };
use std::{ fs::{ File, remove_file }, io::Write, path::Path };

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

        if i != 0 && letter.is_ascii_uppercase() { snake.push('_'); }

        snake.push(letter.to_ascii_lowercase());
    }

    snake
}

/// Convert NamedVariable to NAMED_VARIABLE
pub fn pascal_to_uppercase(pascal: &String) -> String {
    let mut uppecase: String = String::with_capacity(0x40);

    for i in 0..pascal.len() {

        let letter: char = pascal.chars().nth(i).unwrap();

        if i != 0 && letter.is_ascii_uppercase() { uppecase.push('_'); }

        uppecase.push(letter.to_ascii_uppercase());
    }

    uppecase
}

// C Configuration
// ————————————————

pub struct CConfigurations {
    // Configurations
    pub compiler_configurations: CompileConfigurations,

    // Data definitions
    pub field_size_type_size:    usize,
    pub field_offset_type_size:  usize,
    pub message_size_type_size:  usize,
    pub parser_index_type_size:  usize,
}

impl CConfigurations {
    pub fn parse(file_descriptions: &Vec<RuneFileDescription>, configurations: &CompileConfigurations) -> CConfigurations {
        let mut amount_of_messages:   usize = 0;
        let mut largest_message_size: usize = 0;

        // Get the largest overall message size, and the amount of messages
        for file in file_descriptions {

            // Add struct definition amount to amount of messages
            amount_of_messages += file.definitions.structs.len();

            for struct_definition in &file.definitions.structs {

                let estimated_size: usize = struct_definition.estimate_size(configurations);

                if estimated_size > largest_message_size {
                    largest_message_size = estimated_size;
                }
            }
        };

        // println!("Out of {0} messages, the largest one found was estimated at {1} bytes\n", amount_of_messages, largest_message_size);

        // Get the unsigned integer size needed to describe the number of messages
        let parser_index_type_size: usize = match amount_of_messages {
            0x00000000..=0x000000FF => 1,
            0x00000100..=0x0000FFFF => 2,
            0x00010000..=0xFFFFFFFF => 4,
            // 8 byte option is probably not needed, but add anyway...
            _                       => 8
        };

        // Field size type and offset size type will be based on the largest message size
        let message_size_type_size: usize = match largest_message_size {
            0 => panic!("Largest message had size 0! Something went horribly wrong!"),
            0x00000001..=0x000000FF => 1,
            0x00000100..=0x0000FFFF => 2,
            0x00010000..=0xFFFFFFFF => 4,
            // 8 byte option is probably not needed, but add anyway...
            _                       => 8
        };

        let field_size_type_size:   usize = message_size_type_size;
        let field_offset_type_size: usize = message_size_type_size;


        CConfigurations {
            compiler_configurations: configurations.clone(),
            field_size_type_size,
            field_offset_type_size,
            message_size_type_size,
            parser_index_type_size
        }
    }
}

// Field type methods
// ———————————————————

pub trait CFieldType {
    fn c_initializer(&self) -> String;
    fn create_c_variable(&self, name: &String, spacing: usize) -> String;
    fn primitive_c_size(&self) -> usize;
    fn to_c_type(&self) -> String;
}

impl CFieldType for FieldType {
    fn to_c_type(&self) -> String {
        match self {
            FieldType::Boolean => String::from("bool"),
            FieldType::UByte   => String::from("uint8_t"),
            FieldType::Byte    => String::from("int8_t"),

            FieldType::UShort  => String::from("uint16_t"),
            FieldType::Short   => String::from("int16_t"),

            FieldType::Float   => String::from("float"),
            FieldType::UInt    => String::from("uint32_t"),
            FieldType::Int     => String::from("int32_t"),

            FieldType::Double  => String::from("double"),
            FieldType::ULong   => String::from("uint64_t"),
            FieldType::Long    => String::from("int64_t"),

            FieldType::UserDefined(string) => format!("{0}_t", pascal_to_snake_case(string)),

            // This will return the string of the underlying type
            FieldType::Array(underlying_type, _) => underlying_type.to_c_type(),

            FieldType::Empty => panic!("Empty fields have no type!")
        }
    }

    fn create_c_variable(&self, name: &String, spacing: usize) -> String {
        match self {
            FieldType::Boolean => format!("bool {0}{1}", spaces(spacing), name),
            FieldType::UByte   => format!("uint8_t {0}{1}", spaces(spacing), name),
            FieldType::Byte    => format!("int8_t {0}{1}", spaces(spacing), name),

            FieldType::UShort  => format!("uint16_t {0}{1}", spaces(spacing), name),
            FieldType::Short   => format!("int16_t {0}{1}", spaces(spacing), name),

            FieldType::Float   => format!("float {0}{1}", spaces(spacing), name),
            FieldType::UInt    => format!("uint32_t {0}{1}", spaces(spacing), name),
            FieldType::Int     => format!("int32_t {0}{1}", spaces(spacing), name),

            FieldType::Double  => format!("double {0}{1}", spaces(spacing), name),
            FieldType::ULong   => format!("uint64_t {0}{1}", spaces(spacing), name),
            FieldType::Long    => format!("int64_t {0}{1}", spaces(spacing), name),

            FieldType::UserDefined(string) => format!("{0}_t {1}{2}", pascal_to_snake_case(string), spaces(spacing), name),

            FieldType::Array(field_type, field_size) => {

                let array_size: String = match field_size {
                    ArraySize::UserDefinition(definition) => definition.identifier.clone(),
                    ArraySize::NumericValue(size) => size.to_string()
                };

                format!("{0} {1}{2}[{3}]", field_type.to_c_type(), spaces(spacing), name, array_size)
            }

            FieldType::Empty => panic!("Cannot create an empty field!")
        }
    }

    // Size is calculated without padding, and is a guesstimate at best
    fn primitive_c_size(&self) -> usize {
        match self {
            FieldType::Boolean => 1,
            FieldType::UByte   => 1,
            FieldType::Byte    => 1,

            FieldType::UShort  => 2,
            FieldType::Short   => 2,

            FieldType::Float   => 4,
            FieldType::UInt    => 4,
            FieldType::Int     => 4,

            FieldType::Double  => 8,
            FieldType::ULong   => 8,
            FieldType::Long    => 8,

            _ => panic!("Cannot call this function on an array or user defined type")
        }
    }

    fn c_initializer(&self) -> String {
        match self {
            FieldType::Boolean                    => String::from("false"),
            FieldType::Byte                       => String::from("0"),
            FieldType::UByte                      => String::from("0"),
            FieldType::Short                      => String::from("0"),
            FieldType::UShort                     => String::from("0"),
            FieldType::Float                      => String::from("0.0"),
            FieldType::Int                        => String::from("0"),
            FieldType::UInt                       => String::from("0"),
            FieldType::Double                     => String::from("0.0"),
            FieldType::Long                       => String::from("0"),
            FieldType::ULong                      => String::from("0"),
            FieldType::Empty                      => panic!("Cannot initialize an empty field!"),
            FieldType::UserDefined(name) => format!("{0}_INIT", pascal_to_uppercase(&name)),
            FieldType::Array(field_type, array_size) =>
                format!("{{ [0 ... {0}] = {1} }}",
                    match array_size {
                        ArraySize::NumericValue(value) => value - 1,
                        ArraySize::UserDefinition(definition) => {
                            let size_value: usize = match definition.value {
                                DefineValue::IntegerLiteral(value) => match value.try_into() {
                                    Err(error) => panic!("Could not parse \"{0:?}\" array size into a positive integer value! Got error {1}", self, error),
                                    Ok(value) => value
                                },
                                _ => panic!("Got \"{0:?}\" array size definition of an invalid type!", self)
                            };
                            size_value - 1
                        }
                    },
                    match field_type.as_ref() {
                        FieldType::Boolean           => String::from("false"),
                        FieldType::Byte              => String::from("0"),
                        FieldType::UByte             => String::from("0"),
                        FieldType::Short             => String::from("0"),
                        FieldType::UShort            => String::from("0"),
                        FieldType::Float             => String::from("0.0"),
                        FieldType::Int               => String::from("0"),
                        FieldType::UInt              => String::from("0"),
                        FieldType::Double            => String::from("0.0"),
                        FieldType::Long              => String::from("0"),
                        FieldType::ULong             => String::from("0"),
                        FieldType::UserDefined(name) => format!("{0}_INIT", pascal_to_uppercase(&name)),
                        FieldType::Array(_, _)       => panic!("Nested arrays are not currently supported"),
                        FieldType::Empty             => panic!("Cannot initialize an empty field!")
                    }
            )
        }
    }
}

// Struct member methods
// ——————————————————————

pub trait CStructMember {
    fn c_size(&self) -> usize;
    fn c_size_definition(&self) -> String;
    fn index_empty(index: usize) -> StructMember;
}

impl CStructMember for StructMember {
    fn index_empty(index: usize) -> StructMember {
        // Check if value is positive and within the legal values (0 to and including 31)
        let field_index = match index {
            // Legal values
            0..32 => FieldSlot::NamedSlot(index as usize),
            // Higher than legal values
            32..  => panic!("Field index cannot have a value higher than 31!")
        };

        StructMember {
            ident:                String::from("(empty)"),
            field_type:           FieldType::Empty,
            field_slot:           field_index,
            user_definition_link: UserDefinitionLink::NoLink,
            comment:              None

        }
    }

    fn c_size_definition(&self) -> String {
        let size_string: String = match &self.field_type {
            FieldType::UserDefined(type_name) => format!("sizeof({0}_t)", pascal_to_snake_case(&type_name)),
            FieldType::Array(array_type, array_size) => {
                let type_string: String = match &(**array_type) {
                    FieldType::Array(_, _) => panic!("Nested arrays are not supported!"),
                    FieldType::UserDefined(name) => format!("sizeof({0}_t)", pascal_to_snake_case(&name)),
                    _ => format!("sizeof({0})", array_type.to_c_type())
                };

                match array_size {
                    ArraySize::UserDefinition(define) => format!("({0} * {1})", type_string, define.identifier),
                    ArraySize::NumericValue(size) => format!("({0} * {1})", type_string, size)
                }
            },
            FieldType::Empty => String::from("0"),
            _ => format!("sizeof({0})", self.field_type.to_c_type())
        };
        size_string
    }

    fn c_size(&self) -> usize {
        match &self.field_type {

            // Calculate Array size based on (field type * field size)
            FieldType::Array(array_field_type, field_size) => {

                // Get the array size first
                let array_size: usize = match field_size {
                    ArraySize::NumericValue(value) => *value,
                    ArraySize::UserDefinition(definition) => match definition.value {
                        DefineValue::IntegerLiteral(value) => match value.try_into() {
                            Err(error) => panic!("Could not parse \"{0}\" array size into a positive integer value! Got error {1}", self.ident, error),
                            Ok(value) => value
                        },
                        _ => panic!("Got \"{0}\" array size definition of an invalid type!", self.ident)
                    }
                };

                // Parse the byte size based on the array type
                match *array_field_type.to_owned() {
                    FieldType::Array(_, _)    => panic!("Nested arrays not allowed at the moment"),

                    // Parse the user defined type using the member user_definition_link
                    FieldType::UserDefined(type_string) => match &self.user_definition_link {
                        UserDefinitionLink::NoLink                                    => panic!("Could not find definition for type {0} while parsing C size", type_string),
                        UserDefinitionLink::BitfieldLink(bitfield_definition) => bitfield_definition.backing_type.primitive_c_size() * array_size,
                        UserDefinitionLink::EnumLink(enum_definition)             => enum_definition.backing_type.primitive_c_size() * array_size,
                        UserDefinitionLink::StructLink(struct_definition)       => {

                            let mut struct_size = 0;

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

            FieldType::UserDefined(name) => {
                match &self.user_definition_link {
                    UserDefinitionLink::NoLink                                                 => panic!("Found no definition link for item {0}!", name),
                    UserDefinitionLink::BitfieldLink(bitfield_definition) => bitfield_definition.backing_type.primitive_c_size(),
                    UserDefinitionLink::EnumLink(enum_definition)             => enum_definition.backing_type.primitive_c_size(),
                    UserDefinitionLink::StructLink(struct_definition)       => {
                        let mut total_size = 0;

                        for member in &struct_definition.members {
                            total_size += member.c_size();
                        }

                        total_size
                    }
                }
            },

            // Primitives
            _ =>  self.field_type.primitive_c_size()
        }
    }
}

// Struct definition methods
// ——————————————————————————

pub trait CStructDefinition {
    fn estimate_size(&self, configurations: &CompileConfigurations) -> usize;
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
        aligned_1.sort_by(|a, b| b.c_size().cmp(&a.c_size()) );

        full_list.append(&mut aligned_8);
        full_list.append(&mut aligned_4);
        full_list.append(&mut aligned_2);
        full_list.append(&mut aligned_1);

        full_list
    }

    fn estimate_size(&self, configurations: &CompileConfigurations) -> usize {
        // println!("Estimating size of {0}", struct_definition.name);

        let struct_list: Vec<StructMember> = match configurations.sort {
            true  => self.sort_members(),
            false => self.members.clone()
        };

        // Calculate padding
        let mut total_size:  usize = 0;

        for member in &struct_list {
            // println!("   {0} - {1} bytes", member.ident, member.c_size());

            // Assume 8 byte alignment target for items > 4 bytes for worst case scenario
            let member_alignment_size: usize = match member.c_size() {
                // Members with a size 0 can be skipped
                0     => continue,
                1     => 1,
                2     => 2,
                3..=4 => 4,
                // Assume that anything bigger than 4 bytes needs to align to 8 bytes as a worst case scenario (64 bit targets)
                5..   => 8
            };

            // Estimate padding if packing disabled, and member does not align to the worst case 8 bytes (64 bit targets)
            if !configurations.pack_data && (total_size % member_alignment_size) != 0 {
                // Add padding
                let padding: usize = member_alignment_size - (total_size % member_alignment_size);
                total_size += padding;
                // println!("    > Estimated {0} bytes of padding for member \"{1}\"", padding, member.ident);
            }


            total_size += member.c_size();
        }

        // println!("   = Estimated total - {0} bytes\n", total_size);

        total_size
    }
}

// Output file declaration
// ————————————————————————

pub struct OutputFile {
    file_name: String,
    string_buffer: String
}

impl OutputFile {
    pub fn new(file_name: String) -> OutputFile {

        // Create string buffer
        let string_buffer: String = String::with_capacity(0x2000);

        OutputFile {
            file_name,
            string_buffer
        }
    }

    pub fn add_line(&mut self, string: String) {
        self.string_buffer.push_str(format!("{0}\n", string).as_str());
    }

    pub fn add_newline(&mut self) {
        self.string_buffer.push_str("\n");
    }

    pub fn output_file(&self) {

        let output_file_path: &Path = Path::new(&self.file_name);

        // Check if file already exists
        if output_file_path.exists() {
            match remove_file(output_file_path) {
                Err(error) => panic!("Could not delete existing {0} file. Got error {1}", output_file_path.to_str().unwrap(), error),
                Ok(_) => ()
            }
        }

        let mut output_file: File = match File::create(output_file_path) {
            Err(error) => panic!("Could not create output file \"{0}\". Got error {1}", output_file_path.to_str().unwrap(), error),
            Ok(file_result) => file_result
        };

        match output_file.write(self.string_buffer.as_bytes()) {
            Err(error) => panic!("Could not write to \"{0}\" file. Got error {1}", self.file_name, error),
            Ok(_) => match output_file.flush() {
                Err(error) => panic!("Could not flush to \"{0}\" file. Got error {1}", self.file_name, error),
                Ok(_) => ()
            }
         }
    }
}
