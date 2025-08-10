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

    // Create a list of all header files which contain struct definitions, then sort them alphabetically
    let mut file_list: Vec<String> = Vec::with_capacity(file_descriptions.len());

    // Include all message headers that include structs
    for file in file_descriptions {
        if !file.definitions.structs.is_empty() {
            file_list.push(pascal_to_snake_case(&file.file_name));
        }
    }

    // Sort list alphabetically
    file_list.sort_by(|a, b| a.to_ascii_uppercase().cmp(&b.to_ascii_uppercase()));

    // Output inclusions to files
    if !file_list.is_empty() {
        for file in file_list {
            parser_file.add_line(format!("#include \"{0}.rune.h\"", file));
        }
        parser_file.add_newline();
    }

    parser_file.add_line(String::from("#include \"rune.h\""));
    parser_file.add_newline();

    // Parser
    // ———————

    // Define parser array
    parser_file.add_line(String::from("message_info_t* PROTOCOL parser_array[PARSER_COUNT] = {"));
    parser_file.add_line(String::from("    NULL,"));

    for i in 0..struct_definitions.len() {
        let end: String = match i == struct_definitions.len() - 1 {
            false => String::from(","),
            true  => String::new()
        };

        parser_file.add_line(format!("    &{0}_parser{1}", pascal_to_snake_case(&struct_definitions[i].name), end));
    }

    parser_file.add_line(String::from("};"));

    parser_file.output_file();
}
