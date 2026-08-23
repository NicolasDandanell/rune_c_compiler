use rune_parser::types::{FieldIndex, FieldType, MessageDefinition, MessageField, UserDefinitionLink};

use crate::{
    c_configuration::CompileConfigurations, c_utilities::{CMessageDefinition, CMessageField, pascal_to_snake_case, spaces}, compile_error::CompilerError, output_file::OutputFile
};

pub struct CMessageDescriptor {
    name:             String,
    fields:           Vec<MessageField>,
    descriptor_flags: u32,
    descriptor_list:  Vec<String>
}

impl CMessageDescriptor {
    pub fn from(message: &MessageDefinition) -> Result<CMessageDescriptor, CompilerError> {
        let fields: Vec<MessageField> = message.index_sort_fields()?;

        let mut descriptor_list: Vec<String> = Vec::with_capacity(0x20);
        let mut descriptor_flags: u32 = 0;

        for field in &fields {
            // Check to see if it's a nested message, and add descriptor if so
            if let FieldType::UserDefined(_, UserDefinitionLink::MessageLink(link)) = &field.data_type {
                descriptor_list.push(pascal_to_snake_case(&link.name));
                descriptor_flags += 1 << field.index.value();
            }
        }

        Ok(CMessageDescriptor {
            name:   pascal_to_snake_case(&message.name),
            fields,
            descriptor_flags,
            descriptor_list
        })
    }

    fn member_string(&self, name: &str, value: &str, offset: usize, name_alignment: Option<usize>, comma: bool, configurations: &CompileConfigurations) -> String {
        let comma_string: &str = match comma {
            true  => ",",
            false => ""
        };

        let name_spacing: usize = match name_alignment {
            Some(longest_name) => longest_name - name.len(),
            None => 0
        };

        // Generate the output string
        let output_string: String = match configurations.c_standard.allows_designated_initializers() {
            true => format!("{0}.{1}{2} = {3}{4}", spaces(offset), name, spaces(name_spacing), value, comma_string),
            false => format!("{0}/* .{1}{2} */ {3}{4}", spaces(offset), name, spaces(name_spacing), value, comma_string)
        };

        output_string
    }

    fn field_string(&self, field: &MessageField,  offset: usize, spacing: usize, configurations: &CompileConfigurations) -> Result<String, CompilerError> {
        let verifier_string: &str = match field.index {
            FieldIndex::Verifier => " (Verifier)",
            _ => ""
        };

        let offset_value: &str = match field.data_type {
            FieldType::Empty => "0",
            _ => &format!("offsetof({0}_t, {1})", self.name, field.identifier)
        };

        let offset_string: String = self.member_string("offset", offset_value, offset + spacing, Some("offset".len()), true, configurations);

        let size_value: &str = match field.data_type {
            FieldType::Empty => "0",
            _ => &field.c_size_definition(&configurations.c_standard)?
        };

        let size_string: String = self.member_string("size", size_value, offset + spacing, Some("offset".len()), false, configurations);

        let comma: &str = match self.fields.len() - 1 == field.index.value() as usize {
            true  => "",
            false => ","
        };

        Ok(format!("{0}{{ // {1}: {2}{3}\n{4}\n{5}\n{0}}}{6}",
            spaces(offset),
            field.index.value(),
            field.identifier,
            verifier_string,
            offset_string,
            size_string,
            comma
        ))
    }

    pub fn output_field_descriptors(&self, source_file : &mut OutputFile) {
        // Output field descriptors (if any)
        if !self.descriptor_list.is_empty() {
            source_file.add_line(&format!("const rune_descriptor_t* {0}_field_descriptors[{1}] = {{", self.name, self.descriptor_list.len()));

            for i in 0..self.descriptor_list.len() {
                let comma: String = match i == self.descriptor_list.len() - 1 {
                    true => String::new(),
                    false => String::from(",")
                };
                source_file.add_line(&format!("    &{0}_descriptor{1}", self.descriptor_list[i], comma));
            }

            source_file.add_line(&"};".to_string());
            source_file.add_newline();
        }
    }

    pub fn output(&self, configurations: &CompileConfigurations, descriptor_attributes: &String, source_file : &mut OutputFile) -> Result<(), CompilerError> {
        let offset: usize = 4;
        let spacing: usize = 4;

        let longest_field: Option<usize> = Some("field_descriptors".len());

        // Start descriptor
        source_file.add_line(&format!("const rune_descriptor_t {0}{1}_descriptor = {{", descriptor_attributes, self.name));

        // Field descriptors
        let field_descriptors: &str = match self.descriptor_flags == 0 {
            false => &format!("{0}_field_descriptors", self.name),
            true  => configurations.c_standard.null_string()
        };

        source_file.add_line(&self.member_string("field_descriptors", &field_descriptors, offset, longest_field, true, &configurations));

        // Descriptor flags
        let descriptor_flags: &str = match configurations.c_standard.allows_binary_literals() {
            false => &format!("0x{0:08X}", self.descriptor_flags),
            true  => &format!("0b{0:0fields$b}", self.descriptor_flags as usize, fields = self.fields.len())
        };

        source_file.add_line(&self.member_string("descriptor_flags", descriptor_flags, offset, longest_field, true, &configurations));

        // Size
        let size: &str = &format!("sizeof({0}_t)", self.name);

        source_file.add_line(&self.member_string("size", size, offset, longest_field, true, &configurations));

        // Largest field
        let largest_field: &str = &format!("{0}", self.fields.len() - 1);

        source_file.add_line(&self.member_string("largest_field", largest_field, offset, longest_field, true, &configurations));

        // Parsing data
        source_file.add_line(&self.member_string("parsing_data", "{", offset, longest_field, false, &configurations));
        source_file.add_line(&self.member_string("has_verification", "{", offset + spacing, longest_field, false, &configurations));
        source_file.add_line(&format!("{0}}},", spaces(offset)).to_string());

        // Field info
        source_file.add_line(&self.member_string("field_info", "{", offset, longest_field, false, &configurations));

        for field in &self.fields {
            source_file.add_line(&self.field_string(field, offset + spacing, spacing, configurations)?);
        }

        source_file.add_line(&format!("{0}}}", spaces(offset)).to_string());

        // End descriptor
        source_file.add_line(&"};".to_string());

        Ok(())
    }
}
