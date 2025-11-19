use std::path::Path;

use rune_parser::{
    RuneFileDescription,
    types::{MessageDefinition, Primitive}
};

use crate::{
    c_configuration::{CConfigurations, CStandard},
    c_utilities::CPrimitive,
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

    // Create a list with all declared messages across all files
    let mut message_definitions: Vec<MessageDefinition> = Vec::with_capacity(0x40);

    for file in file_descriptions {
        if !file.definitions.messages.is_empty() {
            message_definitions.append(&mut file.definitions.messages.clone());
        }
    }

    // Sort the list alphabetically
    message_definitions.sort_by(|a, b| a.name.to_ascii_uppercase().cmp(&b.name.to_ascii_uppercase()));

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

    definitions_file.add_line("// Configuration dependent definitions".to_string());
    definitions_file.add_line("// ————————————————————————————————————".to_string());
    definitions_file.add_newline();

    definitions_file.add_line("/* These definitions are based on the configurations passed by user to get code generator, such as packing, specific data sections, or other */".to_string());
    definitions_file.add_newline();

    definitions_file.add_line(format!("#define RUNIC_METADATA {0}", configurations.attributes.metadata_attributes));
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
            true => type_from_size(configurations.descriptor_index_type_size, c_standard)?,
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

    definitions_file.add_line("#endif // RUNIC_DEFINITIONS_H".to_string());

    definitions_file.output_file()
}
