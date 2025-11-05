use std::path::Path;

use rune_parser::types::{FieldIndex, FieldType, StructMember, UserDefinitionLink};

use crate::{
    RuneFileDescription,
    c_utilities::{CConfigurations, CStructMember, pascal_to_snake_case, spaces},
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

    if !&file.definitions.structs.is_empty() {
        source_file.add_newline();
    }

    // Struct parsers
    // ———————————————

    for struct_definition in &file.definitions.structs {
        let struct_name: String = pascal_to_snake_case(&struct_definition.name);

        // SORT BY INDEX; DO NOT FORGET
        // INDEXES MISSING MUST HAVE AN EMPTY DEFINITION --> .size = 0 will cause the field to be skipped

        // Get highest index number (except verification field)
        let mut highest_index: u64 = 0;
        let mut has_verification: bool = false;

        for member in &struct_definition.members {
            let index: u64 = match member.index {
                FieldIndex::Verifier => {
                    has_verification = true;
                    0
                },
                FieldIndex::Numeric(value) => value
            };

            if index > highest_index {
                highest_index = index;
            }
        }

        let member_count: u64 = highest_index + 1;

        // Index sort all members, adding empty definitions for skipped fields
        let mut index_sorted_members: Vec<StructMember> = Vec::with_capacity(member_count as usize);
        let mut descriptor_list: Vec<String> = Vec::with_capacity(0x20);
        let mut descriptor_flags: u32 = 0;

        // Also get longest member name for spacing reasons
        let mut longest_member_name_size: usize = 0;

        for i in 0..member_count {
            // Empty definition that will be used if index not found in struct list
            let mut member: StructMember = StructMember::index_empty(i)?;

            // Try to find member with index i
            for listed_member in &struct_definition.members {
                let listed_index: u64 = match listed_member.index {
                    FieldIndex::Numeric(index) => index,
                    FieldIndex::Verifier => 0
                };

                if listed_index == i {
                    member = listed_member.clone();

                    // Check name length for spacing
                    if pascal_to_snake_case(&member.identifier).len() > longest_member_name_size {
                        longest_member_name_size = pascal_to_snake_case(&member.identifier).len()
                    }

                    // Check to see if it's a nested message, and add descriptor if so
                    if let UserDefinitionLink::StructLink(link) = &member.user_definition_link {
                        descriptor_list.push(pascal_to_snake_case(&link.name));
                        descriptor_flags += 1 << member.index.value();
                    }
                }
            }

            index_sorted_members.push(member);
        }

        // Handle field descriptors
        // —————————————————————————

        let mut descriptor_list_initializer: String = String::from("NULL");

        // Output field descriptors (if any)
        if !descriptor_list.is_empty() {
            descriptor_list_initializer = format!("&{0}_field_descriptors", struct_name);

            source_file.add_line(format!("const rune_descriptor_t* {0}_field_descriptors[{1}] = {{", struct_name, descriptor_list.len()));

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

        source_file.add_line(format!("const rune_descriptor_t RUNIC_PARSER {0}_descriptor = {{", struct_name));
        source_file.add_line(format!(
            "    {0}.descriptor_flags     {1}={2} 0b{3:0members$b},",
            comment_start,
            space,
            comment_end,
            descriptor_flags,
            members = member_count as usize
        ));
        source_file.add_line(format!("    {0}.field_descriptors    {1}={2} {3},", comment_start, space, comment_end, descriptor_list_initializer));
        source_file.add_line(format!("    {0}.size                 {1}={2} sizeof({3}_t),", comment_start, space, comment_end, struct_name));
        source_file.add_line(format!("    {0}.largest_field        {1}={2} {3},", comment_start, space, comment_end, highest_index));
        source_file.add_line(format!("    {0}.parsing_data         {1}={2} {{", comment_start, space, comment_end));
        source_file.add_line(format!("    {0}    .has_verification {1}={2} {3},", comment_start, space, comment_end, has_verification_string));
        source_file.add_line("    },".to_string());
        source_file.add_line(format!("    {0}.field_info           {1}={2} {{", comment_start, space, comment_end));

        for (counter, member) in index_sorted_members.iter().enumerate() {
            let member_name: String = pascal_to_snake_case(&member.identifier);
            let spacing: usize = longest_member_name_size - member_name.len();

            //  println!("Got spacing {0} from longest member size {1}", spacing, longest_member_name_size);

            let init_char: String = match &member.data_type {
                FieldType::Empty => String::new(),
                _ => String::from(".")
            };

            let end: char = match counter == member_count as usize - 1 {
                false => ',',
                true => ' '
            };

            let size_string: String = member.c_size_definition(c_standard)?;

            let verification_string: String = match has_verification && counter == 0 {
                false => String::from(""),
                true => String::from("Verifier field - ")
            };

            let offset_string: String = match &member.data_type {
                FieldType::Empty => String::from("0"),
                _ => format!("offsetof({0}_t, {1})", struct_name, member_name)
            };

            let comment_spacing = match c_standard.allows_designated_initializers() {
                true => "",
                false => "   "
            };

            source_file.add_line(format!(
                "    /*  {0}{1}{2}: {3}{4}{5} */ {{",
                comment_spacing,
                init_char,
                member_name,
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
