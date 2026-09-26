use rune_parser::RuneFileDescription;

use crate::{
    c_utilities::{CMessageDefinition, CStandard},
    compile_error::CompilerError::{self},
    output::*
};

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
    pub c_standard: CStandard,

    /// Specifies if GNU extensions and other common compiler features are allowed
    pub strict: bool
}

pub struct AttributeStrings {
    pub bitfield_attributes:   String,
    pub enum_attributes:       String,
    pub message_attributes:    String,
    pub metadata_attributes:   String,
    pub descriptor_attributes: String,
    pub struct_attributes:     String
}

pub struct CConfigurations {
    // Configurations
    pub compiler_configurations: CompileConfigurations,

    // Data definitions
    pub field_size_type_size:       usize,
    pub field_offset_type_size:     usize,
    pub message_size_type_size:     usize,
    pub descriptor_index_type_size: usize,

    // Type attribute strings
    pub attributes: AttributeStrings,

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
        let descriptor_index_type_size: usize = match amount_of_messages {
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

        // Parse attribute strings
        // ————————————————————————

        let attributes = parse_attributes(configurations)?;

        Ok(CConfigurations {
            compiler_configurations: configurations.clone(),
            field_size_type_size,
            field_offset_type_size,
            message_size_type_size,
            descriptor_index_type_size,
            attributes,
            largest_message_index
        })
    }
}

enum CAttributes {
    PACKED,
    SECTION(String)
}

impl CAttributes {
    fn string(&self, standard: &CStandard) -> String {
        match self {
            CAttributes::PACKED => match standard {
                CStandard::C23 => String::from("gnu::packed"),
                _ => String::from("packed")
            },
            CAttributes::SECTION(string) => match standard {
                CStandard::C23 => format!("gnu::section(\"{string}\")"),
                _ => format!("section(\"{string}\")")
            }
        }
    }

    fn print_attribute_list(list: &Vec<CAttributes>, configurations: &CompileConfigurations) -> Result<String, CompilerError> {
        if list.is_empty() {
            return Ok(String::new());
        }

        if configurations.strict {
            return Err(CompilerError::SourceAndCStandardMismatch);
        }

        let mut output_string: String = String::with_capacity(0x40);

        // Start of attribute list
        match configurations.c_standard {
            CStandard::C23 => output_string.push_str("[["),
            _ => output_string.push_str("__attribute__((")
        }

        for (index, attribute) in list.iter().enumerate() {
            if index != 0 {
                output_string.push_str(", ");
            }

            output_string.push_str(&attribute.string(&configurations.c_standard));
        }

        match configurations.c_standard {
            CStandard::C23 => output_string.push_str("]] "),
            _ => output_string.push_str(")) ")
        }

        Ok(output_string)
    }
}

fn parse_attributes(configurations: &CompileConfigurations) -> Result<AttributeStrings, CompilerError> {
    // Check that the attribute related flags have not been combined with the 'strict' flag
    // —————————————————————————————————————————————————————————————————————————————————————

    if configurations.strict && configurations.pack_data {
        error!("Cannot combine 'strict' and 'pack_data' flags, as the \"packed\" attribute is a GNU extension");
        return Err(CompilerError::SourceAndCStandardMismatch);
    }

    if configurations.strict && configurations.pack_metadata {
        error!("Cannot combine 'strict' and 'pack_metadata' flags, as the \"packed\" attribute is a GNU extension");
        return Err(CompilerError::SourceAndCStandardMismatch);
    }

    if configurations.strict && configurations.section.is_some() {
        error!("Cannot combine 'strict' and 'data_section' flags, as the \"section\" attribute is a GNU extension");
        return Err(CompilerError::SourceAndCStandardMismatch);
    }

    // Declare attribute lists
    // ————————————————————————

    let mut bitfield_attribute_list: Vec<CAttributes> = Vec::with_capacity(2);
    let enum_attribute_list: Vec<CAttributes> = Vec::with_capacity(2);
    let mut message_attribute_list: Vec<CAttributes> = Vec::with_capacity(2);
    let mut metadata_attribute_list: Vec<CAttributes> = Vec::with_capacity(2);
    let mut descriptor_attribute_list: Vec<CAttributes> = Vec::with_capacity(2);
    let mut struct_attribute_list: Vec<CAttributes> = Vec::with_capacity(2);

    // Parse "packed" attribute
    // —————————————————————————

    if configurations.pack_data {
        // Bitfields
        bitfield_attribute_list.push(CAttributes::PACKED);

        // Enums have backing types, and do not need to be packed

        // Messages
        message_attribute_list.push(CAttributes::PACKED);

        // Structs
        struct_attribute_list.push(CAttributes::PACKED);
    }

    if configurations.pack_metadata {
        metadata_attribute_list.push(CAttributes::PACKED);
    }

    // Parse "section" attribute
    // ——————————————————————————

    if configurations.section.is_some() {
        // Message Descriptor
        descriptor_attribute_list.push(CAttributes::SECTION(configurations.section.clone().unwrap()));
    }

    // Create attribute strings
    // —————————————————————————

    Ok(AttributeStrings {
        bitfield_attributes:   CAttributes::print_attribute_list(&bitfield_attribute_list, configurations)?,
        enum_attributes:       CAttributes::print_attribute_list(&enum_attribute_list, configurations)?,
        message_attributes:    CAttributes::print_attribute_list(&message_attribute_list, configurations)?,
        metadata_attributes:   CAttributes::print_attribute_list(&metadata_attribute_list, configurations)?,
        descriptor_attributes: CAttributes::print_attribute_list(&descriptor_attribute_list, configurations)?,
        struct_attributes:     CAttributes::print_attribute_list(&struct_attribute_list, configurations)?
    })
}
