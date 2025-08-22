use crate::utilities::{ pascal_to_snake_case, OutputFile };
use rune_parser::{ types::StructDefinition, RuneFileDescription };
use std::path::Path;

pub fn output_parser(file_descriptions: &Vec<RuneFileDescription>, output_path: &Path) {
    let parser_file_string: String = String::from("runic_parser.c");

    let mut parser_file: OutputFile = OutputFile::new(format!("{0}/{1}", output_path.to_str().unwrap(), parser_file_string));

    // Create a list with all declared structs across all files
    let mut struct_definitions: Vec<StructDefinition> = Vec::with_capacity(0x40);

    for file in file_descriptions {
        if !file.definitions.structs.is_empty() {
            struct_definitions.append(&mut file.definitions.structs.clone());
        }
    }

    // Sort the list alphabetically
    struct_definitions.sort_by(|a, b| a.name.to_ascii_uppercase().cmp(&b.name.to_ascii_uppercase()));


    // Disclaimers
    // ————————————

    // ...

    // Inclusions
    // ———————————

    parser_file.add_line(String::from("#include \"rune.h\""));
    parser_file.add_newline();

    // External parser definitions
    // ————————————————————————————

    if !struct_definitions.is_empty() {
        for i in 0..struct_definitions.len() {
            parser_file.add_line(format!("extern message_info_t {0}_parser;", pascal_to_snake_case(&struct_definitions[i].name)));
        }
        parser_file.add_newline();
    }

    // Parser
    // ———————

    // Define parser array
    parser_file.add_line(String::from("static message_info_t* PROTOCOL parser_array[RUNE_PARSER_COUNT] = {"));

    for i in 0..struct_definitions.len() {
        let end: String = match i == struct_definitions.len() - 1 {
            false => String::from(","),
            true  => String::new()
        };

        parser_file.add_line(format!("    &{0}_parser{1}", pascal_to_snake_case(&struct_definitions[i].name), end));
    }

    parser_file.add_line(String::from("};"));
    parser_file.add_newline();

    // Get parser function
    // ————————————————————

    parser_file.add_line(String::from("/** Get the parser struct of a given message type from its index */"));
    parser_file.add_line(String::from("inline message_info_t* get_parser(RUNE_PARSER_INDEX_TYPE index) {"));
    parser_file.add_line(String::from("    return parser_array[index - 1];"));
    parser_file.add_line(String::from("}"));

    parser_file.output_file();
}
