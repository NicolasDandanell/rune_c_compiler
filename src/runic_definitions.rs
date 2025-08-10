use crate::utilities::{ pascal_to_uppercase, spaces, CConfigurations, OutputFile };
use rune_parser::{ RuneFileDescription, types::StructDefinition };
use std::path::Path;

fn type_from_size(size: usize) -> String {
    match size {
        1 => String::from("uint8_t"),
        2 => String::from("uint16_t"),
        4 => String::from("uint32_t"),
        8 => String::from("uint64_t"),
        _ => panic!("Invalid type size given!")
    }
}

pub fn output_runic_definitions(file_descriptions: &Vec<RuneFileDescription>, configurations: &CConfigurations, output_path: &Path) {
    let definitions_file_string: String = format!("{0}/runic_definitions.h", output_path.to_str().unwrap());

    let mut bitfield_attributes: String = String::with_capacity(0x100);
    let     enum_attributes:     String = String::with_capacity(0x100);
    let mut parser_attributes:   String = String::with_capacity(0x100);
    let mut struct_attributes:   String = String::with_capacity(0x100);

    // Parse "packed" attribute
    // —————————————————————————

    // Bitfields are always packed!
    match bitfield_attributes.is_empty() {
        true  => bitfield_attributes.push_str("packed"),
        false => bitfield_attributes.push_str(", packed")
    }

    // Enums have backing types, and do not need to be packed

    if configurations.compiler_configurations.pack {
        // Parser
        match parser_attributes.is_empty() {
            true  => parser_attributes.push_str("packed"),
            false => parser_attributes.push_str(", packed")
        }

        // Structs
        match struct_attributes.is_empty() {
            true  => struct_attributes.push_str("packed"),
            false => struct_attributes.push_str(", packed")
        }
    }

    // Parse "section" attribute
    // ——————————————————————————

    if configurations.compiler_configurations.section != None {
        let section_name: String = configurations.compiler_configurations.section.clone().unwrap();

        // Parser
        match parser_attributes.is_empty() {
            true  => parser_attributes.push_str(format!("section(\"{0}\")", section_name).as_str()),
            false => parser_attributes.push_str(format!(", section(\"{0}\")", section_name).as_str())
        }
    }

    // Create attribute strings
    // —————————————————————————

    // Runic bitfields must ALWAYS be packed, so this will never be empty
    let runic_bitfield_string: String = format!("__attribute__(({0}))", bitfield_attributes);

    // Enums
    let runic_enum_string: String = match enum_attributes.is_empty() {
        true  => String::new(),
        false => format!("__attribute__(({0}))", enum_attributes)
    };

    // Parser
    let runic_parser_string: String = match parser_attributes.is_empty() {
        true  => String::new(),
        false => format!("__attribute__(({0}))", parser_attributes)
    };

    // Structs
    let runic_struct_string: String = match struct_attributes.is_empty() {
        true  => String::new(),
        false => format!("__attribute__(({0}))", struct_attributes)
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
    let mut definitions_file: OutputFile = OutputFile::new(definitions_file_string);

    // Disclaimers
    // ————————————

    // ...

    // Definitions
    // ————————————

    definitions_file.add_line(format!("#ifndef RUNE_DEFINITIONS_H"));
    definitions_file.add_line(format!("#define RUNE_DEFINITIONS_H"));
    definitions_file.add_newline();

    definitions_file.add_line(format!("// Static definitions"));
    definitions_file.add_line(format!("// ———————————————————"));
    definitions_file.add_newline();

    definitions_file.add_line(format!("#define FIELD_INDEX_BITS      0x1F"));
    definitions_file.add_line(format!("#define NO_VERIFICATION_FIELD INT8_MIN"));
    definitions_file.add_line(format!("#define NO_PARSER             0"));
    definitions_file.add_line(format!("#define TRANSPORT_TYPE_BITS   0xE0"));
    definitions_file.add_line(format!("#define VERIFICATION_FIELD    0x1F"));
    definitions_file.add_newline();

    definitions_file.add_line(format!("// Configuration dependent definitions"));
    definitions_file.add_line(format!("// ————————————————————————————————————"));
    definitions_file.add_newline();

    definitions_file.add_line(format!("/* These definitions are based on the configurations passed by user to get code generator, such as packing, specific data sections, or other */"));
    definitions_file.add_newline();

    definitions_file.add_line(format!("#define RUNIC_BITFIELD {0}", runic_bitfield_string));
    definitions_file.add_line(format!("#define RUNIC_ENUM     {0}", runic_enum_string));
    definitions_file.add_line(format!("#define RUNIC_PARSER   {0}", runic_parser_string));
    definitions_file.add_line(format!("#define RUNIC_STRUCT   {0}", runic_struct_string));
    definitions_file.add_newline();

    definitions_file.add_line(format!("// Message dependent definitions"));
    definitions_file.add_line(format!("// ——————————————————————————————"));
    definitions_file.add_newline();

    definitions_file.add_line(format!("/* These definitions are dependent on the declared data, and will vary to adapt to accommodate the sizes of the declared data structures */"));
    definitions_file.add_newline();

    definitions_file.add_line(format!("#define FIELD_SIZE_TYPE   {0}", type_from_size(configurations.field_size_type_size)));
    definitions_file.add_line(format!("#define FIELD_OFFSET_TYPE {0}", type_from_size(configurations.field_offset_type_size)));
    definitions_file.add_line(format!("#define MESSAGE_SIZE_TYPE {0}", type_from_size(configurations.message_size_type_size)));
    definitions_file.add_line(format!("#define PARSER_INDEX_TYPE {0}", type_from_size(configurations.parser_index_type_size)));
    definitions_file.add_newline();

    definitions_file.add_line(format!("// Parsing array definitions"));
    definitions_file.add_line(format!("// ——————————————————————————"));
    definitions_file.add_newline();

    definitions_file.add_line(format!("/** First entry of the parser array is purposely empty, as NO_PARSER is the index 0 for quick comparison check */"));
    definitions_file.add_line(format!("#define PARSER_COUNT {0}", struct_definitions.len() + 1));
    definitions_file.add_newline();

    // Calculate longest struct name for spacing reasons
    let mut longest_struct_name: usize = 0;

    for struct_definition in &struct_definitions {
        let struct_name: String = pascal_to_uppercase(&struct_definition.name);

        if struct_name.len() > longest_struct_name {
            longest_struct_name = struct_name.len();
        }
    }

    for i in 0..struct_definitions.len() {
        let struct_name: String = pascal_to_uppercase(&struct_definitions[i].name);

        definitions_file.add_line(format!("#define {0}_INDEX {1}{2}",
            struct_name,
            spaces(longest_struct_name - struct_name.len()),
            i + 1
        ));
    }
    definitions_file.add_newline();

    definitions_file.add_line(format!("#endif // RUNIC_DEFINITIONS_H"));

    definitions_file.output_file();
}
