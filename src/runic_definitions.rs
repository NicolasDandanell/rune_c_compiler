use std::path::Path;

use rune_parser::{
    RuneFileDescription,
    types::{Primitive, StructDefinition}
};

use crate::{
    c_standard::CStandard,
    c_utilities::{CConfigurations, CPrimitive},
    compile_error::CompilerError,
    output::*,
    output_file::OutputFile
};

fn type_from_size(size: usize, c_standard: &CStandard) -> Result<String, CompilerError> {
    match size {
        1 => Primitive::U8.to_c_type(c_standard),
        2 => Primitive::U16.to_c_type(c_standard),
        4 => Primitive::U32.to_c_type(c_standard),
        8 => Primitive::U64.to_c_type(c_standard),
        _ => {
            error!("Invalid type size given! This should not be possible");
            Err(CompilerError::LogicError)
        }
    }
}

pub fn output_runic_definitions(file_descriptions: &Vec<RuneFileDescription>, configurations: &CConfigurations, output_path: &Path) -> Result<(), CompilerError> {
    let c_standard: &CStandard = &configurations.compiler_configurations.c_standard;

    let mut bitfield_attributes: String = String::with_capacity(0x100);
    let enum_attributes: String = String::with_capacity(0x100);
    let mut parser_attributes: String = String::with_capacity(0x100);
    let mut struct_attributes: String = String::with_capacity(0x100);

    let mut metadata_attributes: String = String::with_capacity(0x100);

    // Parse "packed" attribute
    // —————————————————————————

    // Bitfields are always packed!
    match bitfield_attributes.is_empty() {
        true => bitfield_attributes.push_str("packed"),
        false => bitfield_attributes.push_str(", packed")
    }

    // Enums have backing types, and do not need to be packed

    if configurations.compiler_configurations.pack_data {
        // Parser
        match parser_attributes.is_empty() {
            true => parser_attributes.push_str("packed"),
            false => parser_attributes.push_str(", packed")
        }

        // Structs
        match struct_attributes.is_empty() {
            true => struct_attributes.push_str("packed"),
            false => struct_attributes.push_str(", packed")
        }
    }

    if configurations.compiler_configurations.pack_metadata {
        match metadata_attributes.is_empty() {
            true => metadata_attributes.push_str("packed"),
            false => metadata_attributes.push_str(", packed")
        }
    }

    // Parse "section" attribute
    // ——————————————————————————

    if configurations.compiler_configurations.section.is_some() {
        let section_name: String = configurations.compiler_configurations.section.clone().unwrap();

        // Parser
        match parser_attributes.is_empty() {
            true => parser_attributes.push_str(format!("section(\"{0}\")", section_name).as_str()),
            false => parser_attributes.push_str(format!(", section(\"{0}\")", section_name).as_str())
        }
    }

    // Create attribute strings
    // —————————————————————————

    // Runic bitfields must ALWAYS be packed, so this will never be empty
    let runic_bitfield_string: String = format!("__attribute__(({0}))", bitfield_attributes);

    // Enums
    let runic_enum_string: String = match enum_attributes.is_empty() {
        true => String::new(),
        false => format!("__attribute__(({0}))", enum_attributes)
    };

    // Parser
    let runic_parser_string: String = match parser_attributes.is_empty() {
        true => String::new(),
        false => format!("__attribute__(({0}))", parser_attributes)
    };

    // Structs
    let runic_struct_string: String = match struct_attributes.is_empty() {
        true => String::new(),
        false => format!("__attribute__(({0}))", struct_attributes)
    };

    // Metadata
    let runic_metadata_string: String = match metadata_attributes.is_empty() {
        true => String::new(),
        false => format!("__attribute__(({0}))", metadata_attributes)
    };

    // Create a list with all declared structs across all files
    let mut struct_definitions: Vec<StructDefinition> = Vec::with_capacity(0x40);

    for file in file_descriptions {
        if !file.definitions.structs.is_empty() {
            struct_definitions.append(&mut file.definitions.structs.clone());
        }
    }

    // Sort the list alphabetically
    struct_definitions.sort_by(|a, b| a.name.to_ascii_uppercase().cmp(&b.name.to_ascii_uppercase()));

    // Create output file
    let definitions_file_string: String = String::from("runic_definitions.h");

    let mut definitions_file: OutputFile = OutputFile::new(String::from(output_path.to_str().unwrap()), definitions_file_string);

    // Disclaimers
    // ————————————

    // ...

    // Definitions
    // ————————————

    definitions_file.add_line("#ifndef RUNE_DEFINITIONS_H".to_string());
    definitions_file.add_line("#define RUNE_DEFINITIONS_H".to_string());
    definitions_file.add_newline();

    definitions_file.add_line("// Static definitions".to_string());
    definitions_file.add_line("// ———————————————————".to_string());
    definitions_file.add_newline();

    definitions_file.add_line("#define RUNE_FIELD_INDEX_BITS 0x1F".to_string());
    definitions_file.add_line("#define RUNE_PACKAGING_BITS   0xE0".to_string());
    definitions_file.add_newline();

    definitions_file.add_line("// Configuration dependent definitions".to_string());
    definitions_file.add_line("// ————————————————————————————————————".to_string());
    definitions_file.add_newline();

    definitions_file.add_line("/* These definitions are based on the configurations passed by user to get code generator, such as packing, specific data sections, or other */".to_string());
    definitions_file.add_newline();

    definitions_file.add_line(format!("#define RUNIC_BITFIELD {0}", runic_bitfield_string));
    definitions_file.add_line(format!("#define RUNIC_ENUM     {0}", runic_enum_string));
    definitions_file.add_line(format!("#define RUNIC_PARSER   {0}", runic_parser_string));
    definitions_file.add_line(format!("#define RUNIC_STRUCT   {0}", runic_struct_string));
    definitions_file.add_newline();

    definitions_file.add_line("// Message dependent definitions".to_string());
    definitions_file.add_line("// ——————————————————————————————".to_string());
    definitions_file.add_newline();

    definitions_file.add_line("/* These definitions are dependent on the declared data, and will vary to adapt to accommodate the sizes of the declared data structures */".to_string());
    definitions_file.add_newline();

    definitions_file.add_line(format!(
        "#define RUNE_FIELD_SIZE_TYPE   {0}",
        match configurations.compiler_configurations.pack_metadata {
            true => type_from_size(configurations.field_size_type_size, c_standard)?,
            false => String::from("size_t")
        }
    ));
    definitions_file.add_line(format!(
        "#define RUNE_FIELD_OFFSET_TYPE {0}",
        match configurations.compiler_configurations.pack_metadata {
            true => type_from_size(configurations.field_offset_type_size, c_standard)?,
            false => String::from("size_t")
        }
    ));
    definitions_file.add_line(format!(
        "#define RUNE_MESSAGE_SIZE_TYPE {0}",
        match configurations.compiler_configurations.pack_metadata {
            true => type_from_size(configurations.message_size_type_size, c_standard)?,
            false => String::from("size_t")
        }
    ));
    definitions_file.add_line(format!(
        "#define RUNE_PARSER_INDEX_TYPE {0}",
        match configurations.compiler_configurations.pack_metadata {
            true => type_from_size(configurations.parser_index_type_size, c_standard)?,
            false => String::from("size_t")
        }
    ));
    definitions_file.add_line(format!(
        "#define RUNE_FIELD_INFO_COUNT {0}",
        match c_standard.allows_flexible_array_members() {
            true => String::new(),
            false => (configurations.largest_message_index + 1).to_string()
        }
    ));
    definitions_file.add_newline();

    definitions_file.add_line("/** Defines whether and how metadata generated by the rune compiler should be packed optimized */".to_string());
    definitions_file.add_line(format!("#define RUNIC_METADATA {0}", runic_metadata_string));
    definitions_file.add_newline();

    definitions_file.add_line("#endif // RUNIC_DEFINITIONS_H".to_string());

    definitions_file.output_file()
}
