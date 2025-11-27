use std::path::Path;

use rune_parser::types::{FieldIndex, FieldType, MessageField, UserDefinitionLink};

use crate::{
    RuneFileDescription,
    c_configuration::CConfigurations,
    c_utilities::{CMessageDefinition, CMessageField, pascal_to_snake_case, spaces},
    compile_error::CompilerError,
    output_file::OutputFile
};

pub fn output_source(file: &RuneFileDescription, configurations: &CConfigurations, output_path: &Path) -> Result<(), CompilerError> {
    let c_standard = &configurations.compiler_configurations.c_standard;

    let c_file_string: String = format!(
        "{0}{1}.rune.c",
        match file.relative_path.is_empty() {
            true => String::new(),
            false => format!("/{0}", file.relative_path)
        },
        file.name
    );

    let mut source_file: OutputFile = OutputFile::new(String::from(output_path.to_str().unwrap()), c_file_string);

    // Disclaimers
    // ————————————

    // ...

    // Include own header
    // ———————————————————

    source_file.add_line(format!("#include \"{0}.rune.h\"", file.name));
    source_file.add_newline();

    // Include rune.h
    // ———————————————

    source_file.add_line("#include \"rune.h\"".to_string());

    if !&file.definitions.messages.is_empty() {
        source_file.add_newline();
    }

    // Message parsers
    // ————————————————

    for message_definition in &file.definitions.messages {
        let message_name: String = pascal_to_snake_case(&message_definition.name);

        // SORT BY INDEX; DO NOT FORGET
        // INDEXES MISSING MUST HAVE AN EMPTY DEFINITION --> .size = 0 will cause the field to be skipped

        // Index sort all fields, adding empty definitions for skipped fields
        let index_sorted_fields: Vec<MessageField> = message_definition.index_sort_fields()?;
        let field_count: u64 = index_sorted_fields.len() as u64;

        let mut descriptor_list: Vec<String> = Vec::with_capacity(0x20);
        let mut descriptor_flags: u32 = 0;

        // Also get longest field name for spacing reasons
        let mut longest_field_name_size: usize = 0;

        let has_verification: bool = match index_sorted_fields[0].index {
            FieldIndex::Verifier => true,
            FieldIndex::Numeric(_) => false
        };

        for field in &index_sorted_fields {
            // Check to see if it's a nested message, and add descriptor if so
            if let FieldType::UserDefined(_, UserDefinitionLink::MessageLink(link)) = &field.data_type {
                descriptor_list.push(pascal_to_snake_case(&link.name));
                descriptor_flags += 1 << field.index.value();
            }

            // Check if field is empty, as empty fields will not have a '.' in front of the name
            let not_empty: bool = field.data_type != FieldType::Empty;

            // Check name length for spacing
            if pascal_to_snake_case(&field.identifier).len() + not_empty as usize > longest_field_name_size {
                longest_field_name_size = pascal_to_snake_case(&field.identifier).len() + not_empty as usize;
            }
        }

        // Handle field descriptors
        // —————————————————————————

        let mut descriptor_list_initializer: String = String::from("NULL");

        // Output field descriptors (if any)
        if !descriptor_list.is_empty() {
            descriptor_list_initializer = format!("{0}_field_descriptors", message_name);

            source_file.add_line(format!("const rune_descriptor_t* {0}_field_descriptors[{1}] = {{", message_name, descriptor_list.len()));

            for i in 0..descriptor_list.len() {
                let comma: String = match i == descriptor_list.len() - 1 {
                    true => String::new(),
                    false => String::from(",")
                };
                source_file.add_line(format!("    &{0}_descriptor{1}", descriptor_list[i], comma));
            }

            source_file.add_line("};".to_string());
            source_file.add_newline();
        }

        // Check that standard allows_designated_initializers, and output accordingly
        // ———————————————————————————————————————————————————————————————————————————

        let comment_start: &'static str;
        let comment_end: &'static str;
        let space: &'static str;
        let has_verification_string: String;

        match c_standard.allows_designated_initializers() {
            true => {
                comment_start = "";
                comment_end = "";
                space = "    ";
                has_verification_string = has_verification.to_string();
            },
            false => {
                comment_start = "/* ";
                comment_end = " */";
                space = "";
                has_verification_string = (has_verification as usize).to_string()
            }
        }

        source_file.add_line(format!("const rune_descriptor_t {0}{1}_descriptor = {{", configurations.attributes.descriptor_attributes, message_name));
        source_file.add_line(format!(
            "    {0}.descriptor_flags     {1}={2} 0b{3:0fields$b},",
            comment_start,
            space,
            comment_end,
            descriptor_flags,
            fields = field_count as usize
        ));
        source_file.add_line(format!("    {0}.field_descriptors    {1}={2} {3},", comment_start, space, comment_end, descriptor_list_initializer));
        source_file.add_line(format!("    {0}.size                 {1}={2} sizeof({3}_t),", comment_start, space, comment_end, message_name));
        source_file.add_line(format!("    {0}.largest_field        {1}={2} {3},", comment_start, space, comment_end, field_count - 1));
        source_file.add_line(format!("    {0}.parsing_data         {1}={2} {{", comment_start, space, comment_end));
        source_file.add_line(format!("    {0}    .has_verification {1}={2} {3},", comment_start, space, comment_end, has_verification_string));
        source_file.add_line("    },".to_string());
        source_file.add_line(format!("    {0}.field_info           {1}={2} {{", comment_start, space, comment_end));

        for (counter, field) in index_sorted_fields.iter().enumerate() {
            let field_name: String = pascal_to_snake_case(&field.identifier);
            let spacing: usize = longest_field_name_size - field_name.len() - (field.data_type != FieldType::Empty) as usize;

            let init_char: String = match &field.data_type {
                FieldType::Empty => String::new(),
                _ => String::from(".")
            };

            let end: char = match counter == field_count as usize - 1 {
                false => ',',
                true => ' '
            };

            let size_string: String = field.c_size_definition(c_standard)?;

            let verification_string: String = match has_verification && counter == 0 {
                false => String::from(""),
                true => String::from("Verifier field - ")
            };

            let offset_string: String = match &field.data_type {
                FieldType::Empty => String::from("0"),
                _ => format!("offsetof({0}_t, {1})", message_name, field_name)
            };

            let comment_spacing = match c_standard.allows_designated_initializers() {
                true => "",
                false => "   "
            };

            source_file.add_line(format!(
                "    /*  {0}{1}{2}: {3}{4}{5} */ {{",
                comment_spacing,
                init_char,
                field_name,
                spaces(spacing),
                verification_string,
                counter
            ));
            source_file.add_line(format!("    {0}        .offset ={1} {2},", comment_start, comment_end, offset_string));
            source_file.add_line(format!("    {0}        .size   ={1} {2},", comment_start, comment_end, size_string));

            source_file.add_line(format!("        }}{0}", end));
        }

        source_file.add_line("    }".to_string());
        source_file.add_line("};".to_string());
    }

    source_file.output_file()
}
