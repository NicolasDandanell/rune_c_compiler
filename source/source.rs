use std::path::Path;

use crate::{RuneFileDescription, c_configuration::CConfigurations, c_utilities::CMessageDescriptor, compile_error::CompilerError, output_file::OutputFile};

pub fn output_source(file: &RuneFileDescription, configurations: &CConfigurations, output_path: &Path) -> Result<(), CompilerError> {
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

    source_file.add_line(&format!("#include \"{0}.rune.h\"", file.name));
    source_file.add_newline();

    // Include rune.h
    // ———————————————

    source_file.add_line(&"#include \"rune.h\"".to_string());

    if !&file.definitions.messages.is_empty() {
        source_file.add_newline();
    }

    // Message parsers
    // ————————————————

    for message_definition in &file.definitions.messages {
        // Create message descriptor
        let descriptor: CMessageDescriptor = CMessageDescriptor::from(&message_definition)?;

        // Add field descriptors (if any)
        descriptor.output_field_descriptors(&mut source_file);

        // Add message descriptor
        descriptor.output(&configurations.compiler_configurations, &configurations.attributes.descriptor_attributes, &mut source_file)?;
    }

    source_file.output_file()
}
