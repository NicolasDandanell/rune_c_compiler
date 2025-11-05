use std::path::Path;

use rune_parser::{
    scanner::NumericLiteral,
    types::{BitSize, BitfieldDefinition, BitfieldMember, DefineDefinition, DefineValue, EnumDefinition, Primitive, StructDefinition, StructMember}
};

use crate::{
    RuneFileDescription,
    c_standard::CStandard,
    c_utilities::{CConfigurations, CFieldType, CNumericValue, CPrimitive, CStructDefinition, pascal_to_snake_case, pascal_to_uppercase, spaces},
    compile_error::CompilerError,
    output::*,
    output_file::OutputFile
};

/// Outputs a bitfield definition into the header file
fn output_bitfield(header_file: &mut OutputFile, configurations: &CConfigurations, bitfield_definition: &BitfieldDefinition) -> Result<(), CompilerError> {
    let c_standard = &configurations.compiler_configurations.c_standard;

    // Print comment if present
    match &bitfield_definition.comment {
        Some(comment) => header_file.add_line(format!("/**{0}*/", comment)),
        None => ()
    }

    let bitfield_name: String = pascal_to_snake_case(&bitfield_definition.name);

    let mut little_endian_order: Vec<BitfieldMember> = Vec::with_capacity(bitfield_definition.members.len());
    let mut big_endian_order: Vec<BitfieldMember> = Vec::with_capacity(bitfield_definition.members.len());

    // Get the backing type with signed and unsigned variants
    let backing_type: (Primitive, Primitive) = match bitfield_definition.backing_type {
        Primitive::I8 | Primitive::U8 => (Primitive::U8, Primitive::I8),
        Primitive::I16 | Primitive::U16 => (Primitive::U16, Primitive::I16),
        Primitive::I32 | Primitive::U32 => (Primitive::U32, Primitive::I32),
        Primitive::I64 | Primitive::U64 => (Primitive::U64, Primitive::I64),
        _ => {
            error!("Only integer type primitives can back bitfields");
            return Err(CompilerError::MalformedSource);
        }
    };

    // Calculate required padding for ensuring proper alignment
    let mut total_size: u64 = 0;

    for member in &bitfield_definition.members {
        total_size += match member.size {
            BitSize::Signed(size) => size,
            BitSize::Unsigned(size) => size
        };
    }

    let padding: BitfieldMember = BitfieldMember {
        identifier: String::from("padding"),
        size:       BitSize::Unsigned((bitfield_definition.backing_type.c_size() * 8) - total_size),
        index:      0, // Does not matter
        comment:    Some(String::from(" Padding to ensure proper alignment "))
    };

    let padding_name_size: u64 = match padding.size {
        BitSize::Signed(size) => size,
        BitSize::Unsigned(size) => size
    };

    // Calculate longest member name for spacing
    let mut longest_name: usize = match padding_name_size {
        0 => 0,
        _ => String::from("padding").len()
    };

    for member in &bitfield_definition.members {
        let member_name = pascal_to_snake_case(&member.identifier);
        if member_name.len() > longest_name {
            longest_name = member_name.len();
        }
    }

    // Disclaimer
    // ———————————

    header_file.add_line(String::from("// Disclaimer ! Run rune_bitfield_tester() function to check whether bitfields are behaving as intended"));

    // Little endian order
    // ————————————————————

    header_file.add_line(String::from("#if defined __LITTLE_ENDIAN__"));
    header_file.add_line(format!("typedef struct RUNIC_BITFIELD {0} {{", bitfield_name));

    // Comment
    if bitfield_definition.comment.is_some() {
        header_file.add_line(format!("/**{0}*/", bitfield_definition.comment.as_ref().unwrap()));
    }

    // Get little endian order
    for i in 0..bitfield_definition.members.len() as u64 {
        for member in &bitfield_definition.members {
            if member.index == i {
                little_endian_order.push(member.clone());
            }
        }
    }

    // Add padding - In the end for little endian
    little_endian_order.push(padding.clone());

    // Print bits
    for member in little_endian_order.iter().enumerate() {
        // Member comment
        if member.1.comment.is_some() {
            if member.0 != 0 {
                header_file.add_newline();
            }
            header_file.add_line(format!("    /**{0}*/", member.1.comment.as_ref().unwrap()));
        }

        let member_name = pascal_to_snake_case(&member.1.identifier);

        // Get bit size
        let bit_size: u64;
        let backing_string: String;

        match member.1.size {
            BitSize::Signed(size) => {
                backing_string = format!("{0} ", backing_type.1.to_c_type(c_standard)?);
                bit_size = size;
            },
            BitSize::Unsigned(size) => {
                backing_string = backing_type.0.to_c_type(c_standard)?;
                bit_size = size;
            }
        };

        header_file.add_line(format!("    {0} {1}{2} : {3};", backing_string, member_name, spaces(longest_name - member_name.len()), bit_size));
    }

    header_file.add_line(format!("}} {0}_t;", bitfield_name));

    // Big endian order
    // —————————————————

    header_file.add_line(String::from("#elif defined __BIG_ENDIAN__"));
    header_file.add_line(format!("typedef struct RUNIC_BITFIELD {0} {{", bitfield_name));

    // Comment
    if bitfield_definition.comment.is_some() {
        header_file.add_line(format!("/**{0}*/", bitfield_definition.comment.as_ref().unwrap()));
    }

    // Add padding - In the beginning for little endian
    big_endian_order.push(padding.clone());

    // Get big endian order
    for z in 0..bitfield_definition.members.len() as u64 {
        let i = bitfield_definition.members.len() as u64 - 1 - z;
        for member in &bitfield_definition.members {
            if member.index == i {
                big_endian_order.push(member.clone());
            }
        }
    }

    // Print bits
    for member in big_endian_order.iter().enumerate() {
        // Member comment
        if member.1.comment.is_some() {
            if member.0 != 0 {
                header_file.add_newline();
            }
            header_file.add_line(format!("    /**{0}*/", member.1.comment.as_ref().unwrap()));
        }

        let member_name: String = pascal_to_snake_case(&member.1.identifier);

        // Get bit size
        let bit_size: u64;
        let backing_string: String;

        match member.1.size {
            BitSize::Signed(size) => {
                backing_string = format!("{0} ", backing_type.1.to_c_type(c_standard)?);
                bit_size = size;
            },
            BitSize::Unsigned(size) => {
                backing_string = backing_type.0.to_c_type(c_standard)?;
                bit_size = size;
            }
        };

        header_file.add_line(format!("    {0} {1}{2} : {3};", backing_string, member_name, spaces(longest_name - member_name.len()), bit_size));
    }

    header_file.add_line(format!("}} {0}_t;", bitfield_name));

    // Error
    // ——————

    header_file.add_line(String::from("#else"));
    header_file.add_line(String::from("#error \"Only little and big endianness is supported by this Rune C implementation\""));
    header_file.add_line(String::from("#endif // __BYTE_ORDER__"));
    header_file.add_newline();

    // Initializer
    // ————————————

    header_file.add_line(format!("#define {0}_INIT 0", pascal_to_uppercase(&bitfield_definition.name)));
    header_file.add_newline();

    Ok(())
}

/// Outputs a define statement into the header file
fn output_define(header_file: &mut OutputFile, define: &DefineDefinition) {
    // Print comment if present
    match &define.comment {
        Some(comment) => header_file.add_line(format!("/**{0}*/", comment)),
        None => ()
    }

    let define_name: String = define.name.clone();

    let define_value: String = {
        // Check if the value has been redefined. If so, use the redefined value
        let value: &DefineValue = match &define.redefinition {
            Some(redefine) => &redefine.value,
            None => &define.value
        };

        match value {
            DefineValue::NoValue => String::from(""),
            DefineValue::NumericLiteral(value) => value.to_string()
        }
    };

    header_file.add_line(format!("#define {0} {1}", define_name, define_value));
}

/// Outputs an enum into the header file
fn output_enum(header_file: &mut OutputFile, configurations: &CConfigurations, enum_definition: &EnumDefinition) -> Result<(), CompilerError> {
    let c_standard = &configurations.compiler_configurations.c_standard;

    // Print comment if present
    match &enum_definition.comment {
        Some(comment) => header_file.add_line(format!("/**{0}*/", comment)),
        None => ()
    }

    let enum_name: String = pascal_to_snake_case(&enum_definition.name);

    let allow_backing_type: bool = configurations.compiler_configurations.c_standard.allows_enum_backing_type();
    let mut needs_backing_value: bool = !allow_backing_type;

    header_file.add_line(format!(
        "typedef enum RUNIC_ENUM {0}{1} {{",
        enum_name,
        match allow_backing_type {
            false => String::from(""),
            true => format!(": {0}", enum_definition.backing_type.to_c_type(c_standard)?)
        }
    ));

    let mut longest_member_name: usize = 0;

    // Get longest name for spacing calculations
    for i in 0..enum_definition.members.len() {
        if longest_member_name < pascal_to_uppercase(&enum_definition.members[i].identifier).len() {
            longest_member_name = pascal_to_uppercase(&enum_definition.members[i].identifier).len();
        }
    }

    let mut initializer_value: String = String::from("0");

    // Print all enum members
    for i in 0..enum_definition.members.len() {
        let enum_member = &enum_definition.members[i];

        // Member comment
        if enum_member.comment.is_some() {
            if i != 0 {
                header_file.add_newline();
            }
            header_file.add_line(format!("    /**{0}*/", enum_member.comment.as_ref().unwrap()));
        }

        let member_name: String = pascal_to_uppercase(&enum_member.identifier);

        let is_zero: bool = match enum_member.value {
            NumericLiteral::Boolean(value) => value == false,
            NumericLiteral::PositiveInteger(value, _) => value == 0,
            NumericLiteral::NegativeInteger(value, _) => value == 0,
            NumericLiteral::Float(value) => value == 0.0
        };

        if is_zero && (initializer_value == "0") {
            initializer_value = member_name.clone();
        }

        // Check if the value is large enough to trigger the desired backing type
        if needs_backing_value {
            if enum_member.value.requires_size() == enum_definition.backing_type.c_size() {
                needs_backing_value = false;
            }
        }

        let ending: String = match (i == enum_definition.members.len() - 1) && !needs_backing_value {
            false => String::from(","),
            true => String::from("")
        };

        header_file.add_line(format!(
            "    {0}{1} = {2}{3}",
            member_name,
            spaces(longest_member_name - member_name.len()),
            enum_member.value.to_string(),
            ending
        ));
    }

    if needs_backing_value {
        header_file.add_newline();
        header_file.add_line(format!(
            "    /** Value to coerce enum to minimum size of declared backing type {0} */",
            enum_definition.backing_type.to_c_type(c_standard)?
        ));
        header_file.add_line(format!(
            "    {0}_SIZE_RESERVE_VALUE = {1}",
            pascal_to_uppercase(&enum_definition.name),
            match enum_definition.backing_type.c_size() {
                0 => "0",
                1 => "0xFF",
                2 => "0xFFFF",
                4 => "0xFFFFFFFF",
                8 => "0xFFFFFFFFFFFFFFFF",
                _ => unreachable!("Invalid value returned from primitive_c_size()!")
            }
        ));
    }

    // Output enum definitions
    header_file.add_line(format!("}} {0}_t;", enum_name));
    header_file.add_newline();

    // Output enum initializer value
    header_file.add_line(format!("#define {0}_INIT {1}", pascal_to_uppercase(&enum_name), initializer_value));
    header_file.add_newline();

    Ok(())
}

/// Output a struct into the header file
fn output_struct(header_file: &mut OutputFile, configurations: &CConfigurations, struct_definition: &StructDefinition) -> Result<Vec<StructMember>, CompilerError> {
    let c_standard = &configurations.compiler_configurations.c_standard;

    // Print comment if present
    match &struct_definition.comment {
        Some(comment) => header_file.add_line(format!("/**{0}*/", comment)),
        None => ()
    }

    let struct_name: String = pascal_to_snake_case(&struct_definition.name);

    header_file.add_line(format!("typedef struct RUNIC_STRUCT {0} {{", struct_name));

    // Sorted list --> Then use sorted list instead of other one
    let sorted_member_list: Vec<StructMember> = struct_definition.sort_members()?;

    // >>> Spacing of struct members does not look good, and will thus be dropped <<<

    // Get type sizes for spacing reasons
    // let mut longest_type: usize = 0;
    //
    // for member in &sorted_member_list {
    //     if member.field_type.to_c_type().len() > longest_type {
    //         longest_type = member.field_type.to_c_type().len();
    //     }
    // }

    // >>> end <<<

    // Print all struct members
    for i in 0..sorted_member_list.len() {
        let struct_member: &StructMember = &sorted_member_list[i];

        // Member comment
        if struct_member.comment.is_some() {
            if i != 0 {
                header_file.add_newline();
            }
            header_file.add_line(format!("    /**{0}*/", struct_member.comment.as_ref().unwrap()));
        }

        let member_name: String = pascal_to_snake_case(&struct_member.identifier);
        let spacing: usize = 0; // longest_type - sorted_member_list[i].field_type.to_c_type().len();

        header_file.add_line(format!("    {0};", struct_member.data_type.create_c_variable(&member_name, spacing, c_standard)?));
    }

    header_file.add_line(format!("}} {0}_t;", struct_name));
    header_file.add_newline();

    header_file.add_line(format!("extern const rune_descriptor_t {0}_descriptor;", struct_name));
    header_file.add_newline();

    Ok(sorted_member_list)
}

fn output_struct_initializer(output_file: &mut OutputFile, configurations: &CConfigurations, struct_definition: &StructDefinition) -> Result<(), CompilerError> {
    let c_standard: &CStandard = &configurations.compiler_configurations.c_standard;

    let mut pre_equal_length: usize = 0;

    let sorted_member_list: Vec<StructMember> = struct_definition.sort_members()?;

    // Calculate spacing for aligning the '=' sign
    // ————————————————————————————————————————————

    for member in &sorted_member_list {
        if member.identifier.len() > pre_equal_length {
            pre_equal_length = member.identifier.len();
        }
    }

    // Calculate the space for aligning the '\' at the end
    // ————————————————————————————————————————————————————

    let initializer_string: String = format!(
        "#define {0}_INIT ({1}) {{{2}",
        pascal_to_uppercase(&struct_definition.name),
        format!("{0}_t", pascal_to_snake_case(&struct_definition.name)),
        spaces(0)
    );
    let mut pre_newline_length: usize = initializer_string.len();

    // Calculate spacing for after the newline
    for i in 0..sorted_member_list.len() {
        let member: &StructMember = &sorted_member_list[i];

        let is_last: bool = i != sorted_member_list.len() - 1;

        let pre_equal: usize = pre_equal_length - member.identifier.len();

        let comma = match is_last {
            true => ",",
            false => ""
        };

        let string: String = match c_standard.allows_designated_initializers() {
            true => format!(
                "    .{0}{1} = {2}{3} {4}\\",
                member.identifier,
                spaces(pre_equal),
                member.data_type.c_initializer(c_standard)?,
                comma,
                ""
            ),
            false => format!("    {0}{1} {2}\\", member.data_type.c_initializer(c_standard)?, comma, "")
        };

        // I don't know why the -2 is needed, but it does not work without it
        if string.len() - 2 > pre_newline_length {
            pre_newline_length = string.len() - 2;
        }
    }

    // 20 seems to be the number of fixed characters on the define string
    let define_size: usize = 20 + pascal_to_uppercase(&struct_definition.name).len() + pascal_to_snake_case(&struct_definition.name).len();

    output_file.add_line(format!(
        "#define {0}_INIT ({1}_t) {{ {2}\\",
        pascal_to_uppercase(&struct_definition.name),
        pascal_to_snake_case(&struct_definition.name),
        spaces(pre_newline_length - define_size)
    ));

    for i in 0..sorted_member_list.len() {
        let member: &StructMember = &sorted_member_list[i];

        let is_last: bool = i != sorted_member_list.len() - 1;
        let static_length: usize;
        let pre_equal: usize;
        let pre_newline;

        match c_standard.allows_designated_initializers() {
            true => {
                pre_equal = pre_equal_length - member.identifier.len();
                static_length = 9;
                pre_newline = pre_newline_length - pre_equal_length - member.data_type.c_initializer(c_standard)?.len() - static_length + (!is_last as usize);
            },
            false => {
                pre_equal = 0;
                static_length = 5;
                pre_newline = pre_newline_length - member.data_type.c_initializer(c_standard)?.len() - static_length + (!is_last as usize)
            }
        };

        let comma = match is_last {
            true => ",",
            false => ""
        };

        let initializer_string = match c_standard.allows_designated_initializers() {
            true => format!(
                "    .{0}{1} = {2}{3} {4}\\",
                member.identifier,
                spaces(pre_equal),
                member.data_type.c_initializer(c_standard)?,
                comma,
                spaces(pre_newline)
            ),
            false => format!("    {0}{1} {2}\\", member.data_type.c_initializer(c_standard)?, comma, spaces(pre_newline))
        };

        output_file.add_line(initializer_string);
    }
    output_file.add_line(format!("}}"));
    output_file.add_newline();

    output_file.add_line(format!(
        "#define {0}_DESCRIPTOR &{1}_descriptor",
        pascal_to_uppercase(&struct_definition.name),
        pascal_to_snake_case(&struct_definition.name)
    ));
    output_file.add_newline();

    Ok(())
}

pub fn output_header(file: &RuneFileDescription, configurations: &CConfigurations, output_path: &Path) -> Result<(), CompilerError> {
    // Print disclaimers. Requires C23 compliant compiler
    //
    // · Autogenerated code info
    //
    // · Compiler version (C23 compliant)
    //
    // GCC 13 or higher
    // CLang 8.0 or higher
    //
    // · Include & C++ guards
    //
    // · standard includes
    //
    // <stdbool.h>
    // <stdint.h>
    //
    // —————————————————————————————————————————————————

    let h_file_string: String = format!(
        "{0}{1}.rune.h",
        match file.relative_path.is_empty() {
            true => String::new(),
            false => format!("/{0}", file.relative_path)
        },
        file.name
    );

    let mut header_file: OutputFile = OutputFile::new(String::from(output_path.to_str().unwrap()), h_file_string);

    // Disclaimers
    // ————————————

    // ...

    // Start & C++ guards
    // ———————————————————

    header_file.add_line(format!("#ifndef {0}_RUNE_H", file.name.to_uppercase()));
    header_file.add_line(format!("#define {0}_RUNE_H", file.name.to_uppercase()));
    header_file.add_newline();

    header_file.add_line(format!("#ifdef __cplusplus"));
    header_file.add_line(format!("extern \"C\" {{"));
    header_file.add_line(format!("#endif /* __cplusplus */"));
    header_file.add_newline();

    // File inclusions
    // ————————————————

    // Standard library
    header_file.add_line(format!("#include <stdbool.h>"));
    header_file.add_line(format!("#include <stdint.h>"));
    header_file.add_newline();

    // Include Runic Definitions
    header_file.add_line(format!("#include \"rune.h\""));
    header_file.add_newline();

    if !file.definitions.includes.is_empty() {
        // Print out includes
        for include_definition in &file.definitions.includes {
            header_file.add_line(format!("#include \"{0}.rune.h\"", include_definition.file));
        }

        // Separation line
        header_file.add_newline();
    }

    // User defines
    // —————————————

    if !file.definitions.defines.is_empty() {
        for define in &file.definitions.defines {
            output_define(&mut header_file, &define);
        }
        header_file.add_newline();
    }

    // Enums
    // ——————

    // Print all enum definitions
    for enum_definition in &file.definitions.enums {
        output_enum(&mut header_file, configurations, &enum_definition)?;
    }

    // Bitfields
    // ——————————

    for bitfield_definition in &file.definitions.bitfields {
        output_bitfield(&mut header_file, configurations, &bitfield_definition)?;
    }

    // Structs
    // ————————

    // Print out structs
    for struct_definition in &file.definitions.structs {
        output_struct(&mut header_file, configurations, &struct_definition)?;

        // Add struct initializer
        output_struct_initializer(&mut header_file, configurations, &struct_definition)?
    }

    // End & C++ guards
    // —————————————————

    header_file.add_line(format!("#ifdef __cplusplus"));
    header_file.add_line(format!("}}"));
    header_file.add_line(format!("#endif /* __cplusplus */"));
    header_file.add_newline();

    header_file.add_line(format!("#endif /* {0}_RUNE_H */", file.name.to_uppercase()));

    // Output file
    // ————————————

    header_file.output_file()
}
