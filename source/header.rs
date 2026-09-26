use std::path::Path;

use rune_parser::{
    ArrayType,
    scanner::NumericLiteral,
    types::{
        BitSize, BitfieldDefinition, BitfieldMember, DefineDefinition, DefineValue, Definitions, EnumDefinition, FieldType, MemberType, MessageDefinition, MessageField, Primitive, StructDefinition,
        StructMember
    }
};

use crate::{
    RuneFileDescription,
    c_configuration::CConfigurations,
    c_utilities::{
        CMessageDefinition, CMessageField, CNumericValue, CPrimitive, CStandard, CStructDefinition, CStructMember, comment, documentation_comment, pascal_to_snake_case, pascal_to_uppercase, spaces
    },
    compile_error::CompilerError,
    output::*,
    output_file::OutputFile
};

/// Which C standard headers are needed for a particular header
struct HeaderNeeds {
    /// Whether the <stdbool.h> header is needed
    pub boolean: bool,

    /// Whether the <stdint.h> header is needed
    pub integer: bool
}

impl HeaderNeeds {
    pub fn new() -> HeaderNeeds {
        HeaderNeeds { boolean: false, integer: false }
    }

    pub fn any(&self) -> bool {
        self.boolean || self.integer
    }

    fn update(&mut self, primitive: &Primitive, c_standard: &CStandard) {
        // Need <stdbool.h> if any booleans are used and if < C23
        // Need <stdint.h> if any int is declared (most likely)

        match primitive {
            Primitive::Bool => match c_standard {
                // Booleans are a keyword from C23 onwards
                CStandard::C23 => (),

                // Boolean was introduced in C99
                _ if c_standard.allows_boolean() => self.boolean = true,

                // Pre-C99 there is no boolean header to include
                _ => ()
            },
            // Fixed integer types were introduced in C99
            _ if primitive.is_int() == true => {
                if c_standard.allows_integer_types() {
                    self.integer = true
                }
            },
            _ => ()
        }
    }

    pub fn parse(definitions: &Definitions, c_standard: &CStandard) -> HeaderNeeds {
        let mut needs: HeaderNeeds = HeaderNeeds::new();

        for bitfield in &definitions.bitfields {
            needs.update(&bitfield.backing_type, c_standard);
        }

        for enumeration in &definitions.enums {
            needs.update(&enumeration.backing_type, c_standard);
        }

        for message in &definitions.messages {
            for field in &message.fields {
                match &field.data_type {
                    FieldType::Array(array) => match &array.data_type {
                        ArrayType::Primitive(primitive) => needs.update(&primitive, c_standard),
                        ArrayType::UserDefined(_, _) => ()
                    },
                    FieldType::Primitive(primitive) => needs.update(&primitive, c_standard),
                    FieldType::UserDefined(_, _) => (),
                    FieldType::Empty => ()
                }
            }
        }

        for structure in &definitions.structs {
            for member in &structure.members {
                match &member.data_type {
                    MemberType::Array(array) => match &array.data_type {
                        ArrayType::Primitive(primitive) => needs.update(&primitive, c_standard),
                        ArrayType::UserDefined(_, _) => ()
                    },
                    MemberType::Primitive(primitive) => needs.update(&primitive, c_standard),
                    MemberType::UserDefined(_, _) => ()
                };
            }
        }

        needs
    }
}

/// Outputs a bitfield definition into the header file
fn output_bitfield(header_file: &mut OutputFile, configurations: &CConfigurations, bitfield_definition: &BitfieldDefinition) -> Result<(), CompilerError> {
    let c_standard: &CStandard = &configurations.compiler_configurations.c_standard;
    let strict: bool = configurations.compiler_configurations.strict;

    // Print comment if present
    if let Some(comment) = &bitfield_definition.comment {
        header_file.add_line(&documentation_comment(comment, 0, c_standard))
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

    // Start bitfield definition
    header_file.add_line(&format!("typedef struct {0}{1} {{", configurations.attributes.bitfield_attributes, bitfield_name));

    // Little endian order
    // ————————————————————

    header_file.add_line(&format!("#if {0}", c_standard.little_endian_check(strict)?));

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
            header_file.add_line(&documentation_comment(member.1.comment.as_ref().unwrap(), 4, c_standard));
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

        header_file.add_line(&format!("    {0} {1}{2} : {3};", backing_string, member_name, spaces(longest_name - member_name.len()), bit_size));
    }

    // Big endian order
    // —————————————————

    header_file.add_line(&format!("#elif {0}", c_standard.big_endian_check(strict)?));

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
            header_file.add_line(&documentation_comment(member.1.comment.as_ref().unwrap(), 4, c_standard));
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

        header_file.add_line(&format!("    {0} {1}{2} : {3};", backing_string, member_name, spaces(longest_name - member_name.len()), bit_size));
    }

    // Error
    // ——————

    header_file.add_line(&String::from("#else"));
    header_file.add_line(&String::from("#error \"Only little and big endianness is supported by this Rune C implementation\""));
    header_file.add_line(&String::from("#endif"));

    // End bitfield definition
    header_file.add_line(&format!("}} {0}_t;", bitfield_name));
    header_file.add_newline();

    // Static Assertions
    // ——————————————————

    // Special compile time warning if declaring a 64 bit bitfield in a 32 bit platform
    if (bitfield_definition.backing_type == Primitive::I64) || (bitfield_definition.backing_type == Primitive::U64) {
        // If size of (void*) == 4. This should work as bitfields require at least C11, and UINTPTR_MAX and UINT32_MAX were introduced in C99
        header_file.add_line(&String::from("#if UINTPTR_MAX == UINT32_MAX"));
        header_file.add_line(&String::from(
            "#warning \"Compiling a 64 bit bitfield in a 32 bit platform! Rune cannot guarantee the bitfield will compile as intended\""
        ));
        header_file.add_line(&String::from("#endif"));
    }

    // Assert overall expected struct size
    header_file.add_line(&documentation_comment(&"Size check to verify bitfield structure", 0, c_standard));
    header_file.add_line(&c_standard.static_assertion(
        &format!("sizeof({0}_t) == {1}", bitfield_name, bitfield_definition.backing_type.c_size()),
        &mut format!("{0}_t did not have the expected size of {1}!", bitfield_name, bitfield_definition.backing_type.c_size())
    )?);
    header_file.add_newline();

    // Assert the position and width of every struct member
    //  · Not possible sadly... Might not be needed though if full padding is present, as an overall size check would catch any issues

    // static constexpr raw_value
    // static const

    // Initializer
    // ————————————

    header_file.add_line(&format!("#define {0}_INIT {{ 0 }}", pascal_to_uppercase(&bitfield_definition.name)));
    header_file.add_newline();

    Ok(())
}

/// Outputs a define statement into the header file
fn output_define(header_file: &mut OutputFile, define: &DefineDefinition, c_standard: &CStandard) {
    // Print comment if present
    if let Some(comment) = &define.comment {
        header_file.add_line(&documentation_comment(comment, 0, c_standard))
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

    header_file.add_line(&format!("#define {0} {1}", define_name, define_value));
}

/// Outputs an enum into the header file
fn output_enum(header_file: &mut OutputFile, configurations: &CConfigurations, enum_definition: &EnumDefinition) -> Result<(), CompilerError> {
    let c_standard = &configurations.compiler_configurations.c_standard;

    // Print comment if present
    if let Some(comment) = &enum_definition.comment {
        header_file.add_line(&documentation_comment(comment, 0, c_standard))
    }

    let enum_name: String = pascal_to_snake_case(&enum_definition.name);

    let allow_backing_type: bool = configurations.compiler_configurations.c_standard.allows_enum_backing_type();
    let mut needs_backing_value: bool = !allow_backing_type;

    header_file.add_line(&format!(
        "typedef enum {0}{1}{2} {{",
        configurations.attributes.enum_attributes,
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
            header_file.add_line(&documentation_comment(enum_member.comment.as_ref().unwrap(), 4, c_standard));
        }

        let member_name: String = pascal_to_uppercase(&enum_member.identifier);

        let is_zero: bool = match enum_member.value {
            NumericLiteral::AsciiChar(value) => value as u8 == 0,
            NumericLiteral::Boolean(value) => !value,
            NumericLiteral::PositiveInteger(value, _) => value == 0,
            NumericLiteral::NegativeInteger(value, _) => value == 0,
            NumericLiteral::Float(value) => value == 0.0
        };

        if is_zero && (initializer_value == "0") {
            initializer_value = member_name.clone();
        }

        // Check if the value is large enough to trigger the desired backing type
        if needs_backing_value && enum_member.value.requires_size() == enum_definition.backing_type.c_size() {
            needs_backing_value = false;
        }

        let ending: String = match (i == enum_definition.members.len() - 1) && !needs_backing_value {
            false => String::from(","),
            true => String::from("")
        };

        header_file.add_line(&format!("    {0}{1} = {2}{3}", member_name, spaces(longest_member_name - member_name.len()), enum_member.value, ending));
    }

    if needs_backing_value {
        header_file.add_newline();
        header_file.add_line(&documentation_comment(
            &format!("Value to coerce enum to minimum size of declared backing type {0}", enum_definition.backing_type.to_c_type(c_standard)?),
            4,
            c_standard
        ));
        header_file.add_line(&format!(
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
    header_file.add_line(&format!("}} {0}_t;", enum_name));
    header_file.add_newline();

    // Output enum initializer value
    header_file.add_line(&format!("#define {0}_INIT {1}", pascal_to_uppercase(&enum_name), initializer_value));
    header_file.add_newline();

    Ok(())
}

/// Output a message into the header file
fn output_message(header_file: &mut OutputFile, configurations: &CConfigurations, struct_definition: &MessageDefinition) -> Result<Vec<MessageField>, CompilerError> {
    let c_standard = &configurations.compiler_configurations.c_standard;

    // Print comment if present
    if let Some(comment) = &struct_definition.comment {
        header_file.add_line(&documentation_comment(comment, 0, c_standard))
    }

    let message_name: String = pascal_to_snake_case(&struct_definition.name);

    header_file.add_line(&format!("typedef struct {0}{1} {{", configurations.attributes.message_attributes, message_name));

    // Sorted field list --> Then use sorted list instead of the one in the definition
    let sorted_field_list: Vec<MessageField> = struct_definition.size_sort_fields(&configurations.compiler_configurations)?;

    // >>> Spacing of message fields does not look good, and will thus be dropped <<<

    // Get type sizes for spacing reasons
    // let mut longest_type: usize = 0;
    //
    // for field in &sorted_field_list {
    //     if field.field_type.to_c_type().len() > longest_type {
    //         longest_type = field.field_type.to_c_type().len();
    //     }
    // }

    // >>> end <<<

    let mut is_first: bool = true;

    // Print all message fields
    for field in &sorted_field_list {
        // Field comment
        if field.comment.is_some() {
            if !is_first {
                header_file.add_newline();
            }
            header_file.add_line(&documentation_comment(field.comment.as_ref().unwrap(), 4, c_standard));
        }

        let spacing: usize = 0;

        header_file.add_line(&format!("    {0};", field.create_c_variable(spacing, c_standard)?));

        is_first = false;
    }

    header_file.add_line(&format!("}} {0}_t;", message_name));
    header_file.add_newline();

    header_file.add_line(&format!("extern const rune_descriptor_t {0}_descriptor;", message_name));
    header_file.add_newline();

    Ok(sorted_field_list)
}

fn output_message_metadata(output_file: &mut OutputFile, configurations: &CConfigurations, message_definition: &MessageDefinition) -> Result<(), CompilerError> {
    let c_standard: &CStandard = &configurations.compiler_configurations.c_standard;

    let mut pre_equal_length: usize = 0;

    let size_sorted_field_list: Vec<MessageField> = message_definition.size_sort_fields(&configurations.compiler_configurations)?;

    // Calculate spacing for aligning the '=' sign
    // ————————————————————————————————————————————

    for field in &size_sorted_field_list {
        if field.identifier.len() > pre_equal_length {
            pre_equal_length = field.identifier.len();
        }
    }

    // Calculate the space for aligning the '\' at the end
    // ————————————————————————————————————————————————————

    let initializer_string: String = format!(
        "#define {0}_INIT ({1}_t) {{{2}",
        pascal_to_uppercase(&message_definition.name),
        pascal_to_snake_case(&message_definition.name),
        spaces(0)
    );

    let mut pre_newline_length: usize = initializer_string.len();

    // Calculate spacing for after the newline
    for i in 0..size_sorted_field_list.len() {
        let field: &MessageField = &size_sorted_field_list[i];

        let is_last: bool = i != size_sorted_field_list.len() - 1;

        let pre_equal: usize = pre_equal_length - field.identifier.len();

        let comma = match is_last {
            true => ",",
            false => ""
        };

        let string: String = match c_standard.allows_designated_initializers() {
            true => format!("    .{0}{1} = {2}{3} {4}\\", field.identifier, spaces(pre_equal), field.c_initializer(c_standard)?, comma, ""),
            false => format!("    {0}{1} {2}\\", field.c_initializer(c_standard)?, comma, "")
        };

        // I don't know why the -2 is needed, but it does not work without it
        if string.len() - 2 > pre_newline_length {
            pre_newline_length = string.len() - 2;
        }
    }

    // 20 seems to be the number of fixed characters on the define string
    let define_size: usize = 20 + pascal_to_uppercase(&message_definition.name).len() + pascal_to_snake_case(&message_definition.name).len();

    output_file.add_line(&documentation_comment("Initializes all values of the message, and subsequent sub-messages to 0", 0, c_standard));
    output_file.add_line(&format!(
        "#define {0}_INIT ({1}_t) {{ {2}\\",
        pascal_to_uppercase(&message_definition.name),
        pascal_to_snake_case(&message_definition.name),
        spaces(pre_newline_length - define_size)
    ));

    for i in 0..size_sorted_field_list.len() {
        let field: &MessageField = &size_sorted_field_list[i];

        let is_last: bool = i != size_sorted_field_list.len() - 1;
        let static_length: usize;
        let pre_equal: usize;
        let pre_newline;

        match c_standard.allows_designated_initializers() {
            true => {
                pre_equal = pre_equal_length - field.identifier.len();
                static_length = 9;
                pre_newline = pre_newline_length - pre_equal_length - field.c_initializer(c_standard)?.len() - static_length + (!is_last as usize);
            },
            false => {
                pre_equal = 0;
                static_length = 5;
                pre_newline = pre_newline_length - field.c_initializer(c_standard)?.len() - static_length + (!is_last as usize)
            }
        };

        let comma = match is_last {
            true => ",",
            false => ""
        };

        let initializer_string = match c_standard.allows_designated_initializers() {
            true => format!(
                "    .{0}{1} = {2}{3} {4}\\",
                field.identifier,
                spaces(pre_equal),
                field.c_initializer(c_standard)?,
                comma,
                spaces(pre_newline)
            ),
            false => format!("    {0}{1} {2}\\", field.c_initializer(c_standard)?, comma, spaces(pre_newline))
        };

        output_file.add_line(&initializer_string);
    }
    output_file.add_line(&"}".to_string());
    output_file.add_newline();

    output_file.add_line(&documentation_comment(
        "Describes the message and all its fields, as is passed to encoding and decoding functions to indicate how to parse the message",
        0,
        c_standard
    ));
    output_file.add_line(&format!(
        "#define {0}_DESCRIPTOR &{1}_descriptor",
        pascal_to_uppercase(&message_definition.name),
        pascal_to_snake_case(&message_definition.name)
    ));
    output_file.add_newline();

    // Estimate encoded sizes
    // ———————————————————————

    let optimal_encoded_size: u64 = match message_definition.optimal_full_encoded_size() {
        Err(error) => return Err(CompilerError::ParsingError(error)),
        Ok(value) => value
    };

    let pessimal_encoded_size_option: Option<u64> = match message_definition.pessimal_encoded_size() {
        Err(error) => return Err(CompilerError::ParsingError(error)),
        Ok(value) => value
    };

    output_file.add_line(&documentation_comment(
        "Most efficient encoded size of this message while retaining all data bytes. Rune encoders should generally aim for this as the maximum encoded size",
        0,
        c_standard
    ));
    output_file.add_line(&format!("#define {0}_OPTIMAL_ENCODED_SIZE {1}", pascal_to_uppercase(&message_definition.name), optimal_encoded_size));
    output_file.add_newline();
    if let Some(pessimal_encoded_size) = pessimal_encoded_size_option {
        output_file.add_line(&documentation_comment(
            "Most inefficient encoded size possible for this message. This should be used to allocate buffers when for the decoder if the encoder implementation is not known",
            0,
            c_standard
        ));
        output_file.add_line(&format!("#define {0}_PESSIMAL_ENCODED_SIZE {1}", pascal_to_uppercase(&message_definition.name), pessimal_encoded_size));
        output_file.add_newline();
    } else {
        output_file.add_line(&documentation_comment(
            "A skipped message field in here or in a sub-message means that a pessimal encoded case cannot be calculated. This is because the size of the of the skipped field cannot be known",
            0,
            c_standard
        ));
        output_file.add_newline();
    }

    Ok(())
}

fn output_struct(header_file: &mut OutputFile, configurations: &CConfigurations, struct_definition: &StructDefinition) -> Result<Vec<StructMember>, CompilerError> {
    let c_standard: &CStandard = &configurations.compiler_configurations.c_standard;

    // Print comment if present
    if let Some(comment) = &struct_definition.comment {
        header_file.add_line(&documentation_comment(comment, 0, c_standard))
    }

    let struct_name: String = pascal_to_snake_case(&struct_definition.name);

    header_file.add_line(&format!("typedef struct {0}{1} {{", configurations.attributes.struct_attributes, struct_name));

    // Sorted member list --> Then use sorted list instead of the one in the definition
    let sorted_member_list: Vec<StructMember> = struct_definition.index_sort_members()?;

    let mut is_first: bool = true;

    // Print all struct members
    for member in &sorted_member_list {
        // Member comment
        if member.comment.is_some() {
            if !is_first {
                header_file.add_newline();
            }
            header_file.add_line(&documentation_comment(member.comment.as_ref().unwrap(), 4, c_standard));
        }

        let spacing: usize = 0;

        header_file.add_line(&format!("    {0};", member.create_c_variable(spacing, c_standard)?));

        is_first = false;
    }

    header_file.add_line(&format!("}} {0}_t;", struct_name));
    header_file.add_newline();

    // Statically assert that the struct has the desired size
    header_file.add_line(&comment("Struct size check", 0, c_standard));
    header_file.add_line(&struct_definition.size_assertion(c_standard)?);
    header_file.add_newline();

    Ok(sorted_member_list)
}

fn output_struct_metadata(output_file: &mut OutputFile, configurations: &CConfigurations, struct_definition: &StructDefinition) -> Result<(), CompilerError> {
    let c_standard: &CStandard = &configurations.compiler_configurations.c_standard;

    let mut pre_equal_length: usize = 0;

    let index_sorted_member_list: Vec<StructMember> = struct_definition.index_sort_members()?;

    // Calculate spacing for aligning the '=' sign
    // ————————————————————————————————————————————

    for field in &index_sorted_member_list {
        if field.identifier.len() > pre_equal_length {
            pre_equal_length = field.identifier.len();
        }
    }

    // Calculate the space for aligning the '\' at the end
    // ————————————————————————————————————————————————————

    let initializer_string: String = format!(
        "#define {0}_INIT ({1}_t) {{{2}",
        pascal_to_uppercase(&struct_definition.name),
        pascal_to_snake_case(&struct_definition.name),
        spaces(0)
    );

    let mut pre_newline_length: usize = initializer_string.len();

    // Calculate spacing for after the newline
    for i in 0..index_sorted_member_list.len() {
        let member: &StructMember = &index_sorted_member_list[i];

        let is_last: bool = i != index_sorted_member_list.len() - 1;

        let pre_equal: usize = pre_equal_length - member.identifier.len();

        let comma = match is_last {
            true => ",",
            false => ""
        };

        let string: String = match c_standard.allows_designated_initializers() {
            true => format!("    .{0}{1} = {2}{3} {4}\\", member.identifier, spaces(pre_equal), member.c_initializer(c_standard)?, comma, ""),
            false => format!("    {0}{1} {2}\\", member.c_initializer(c_standard)?, comma, "")
        };

        // I don't know why the -2 is needed, but it does not work without it
        if string.len() - 2 > pre_newline_length {
            pre_newline_length = string.len() - 2;
        }
    }

    // 20 seems to be the number of fixed characters on the define string
    let define_size: usize = 20 + pascal_to_uppercase(&struct_definition.name).len() + pascal_to_snake_case(&struct_definition.name).len();

    output_file.add_line(&documentation_comment("Initializes all values of the message, and subsequent sub-messages to 0", 0, &c_standard));
    output_file.add_line(&format!(
        "#define {0}_INIT ({1}_t) {{ {2}\\",
        pascal_to_uppercase(&struct_definition.name),
        pascal_to_snake_case(&struct_definition.name),
        spaces(pre_newline_length - define_size)
    ));

    for i in 0..index_sorted_member_list.len() {
        let member: &StructMember = &index_sorted_member_list[i];

        let is_last: bool = i != index_sorted_member_list.len() - 1;
        let static_length: usize;
        let pre_equal: usize;
        let pre_newline;

        match c_standard.allows_designated_initializers() {
            true => {
                pre_equal = pre_equal_length - member.identifier.len();
                static_length = 9;
                pre_newline = pre_newline_length - pre_equal_length - member.c_initializer(c_standard)?.len() - static_length + (!is_last as usize);
            },
            false => {
                pre_equal = 0;
                static_length = 5;
                pre_newline = pre_newline_length - member.c_initializer(c_standard)?.len() - static_length + (!is_last as usize)
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
                member.c_initializer(c_standard)?,
                comma,
                spaces(pre_newline)
            ),
            false => format!("    {0}{1} {2}\\", member.c_initializer(c_standard)?, comma, spaces(pre_newline))
        };

        output_file.add_line(&initializer_string);
    }
    output_file.add_line(&"}".to_string());
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
    // · Include & C++ guards
    //
    // · standard includes
    //
    // —————————————————————————————————————————————————

    let c_standard: &CStandard = &configurations.compiler_configurations.c_standard;

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

    header_file.add_line(&format!("#ifndef {0}_RUNE_H", file.name.to_uppercase()));
    header_file.add_line(&format!("#define {0}_RUNE_H", file.name.to_uppercase()));
    header_file.add_newline();

    header_file.add_line(&"#ifdef __cplusplus".to_string());
    header_file.add_line(&"extern \"C\" {".to_string());
    header_file.add_line(&"#endif // __cplusplus".to_string());
    header_file.add_newline();

    // File inclusions
    // ————————————————

    // Parse the needed C standard headers
    let standard_headers: HeaderNeeds = HeaderNeeds::parse(&file.definitions, c_standard);

    // Add standard headers if any is needed
    if standard_headers.any() {
        if standard_headers.boolean {
            header_file.add_line(&String::from("#include <stdbool.h>"));
        }

        if standard_headers.integer {
            header_file.add_line(&String::from("#include <stdint.h>"));
        }

        header_file.add_newline();
    }

    // Include Runic Definitions
    header_file.add_line(&"#include \"rune.h\"".to_string());
    header_file.add_newline();

    if !file.definitions.includes.is_empty() {
        // Print out includes
        for include_definition in &file.definitions.includes {
            header_file.add_line(&format!("#include \"{0}.rune.h\"", include_definition.file));
        }

        // Separation line
        header_file.add_newline();
    }

    // User defines
    // —————————————

    if !file.definitions.defines.is_empty() {
        for define in &file.definitions.defines {
            output_define(&mut header_file, define, c_standard);
        }
        header_file.add_newline();
    }

    // Enums
    // ——————

    // Print all enum definitions
    for enum_definition in &file.definitions.enums {
        output_enum(&mut header_file, configurations, enum_definition)?;
    }

    // Bitfields
    // ——————————

    // Check that we can either use C23 or GNU extensions, otherwise we cannot output bitfields
    if !file.definitions.bitfields.is_empty() {
        let standard: &CStandard = &configurations.compiler_configurations.c_standard;
        let strict: bool = configurations.compiler_configurations.strict;

        // C23 + strict = OK
        // C11 + GNU (endianness check) = OK
        // Older = Not ok, even with GNU as we need static assertions

        // If the C standard used is older than C23, and we have the 'strict' flag set, then we cannot support bitfields, as we require endianness check
        if !standard.allows_endianness_check() && strict {
            error!(
                "Cannot currently guarantee bitfield order before C23 standard if using 'strict' flag due to lack of endianness checks! Thus they are not allowed if using {0} with a 'strict' flag",
                standard.to_string()
            );
            return Err(CompilerError::SourceAndCStandardMismatch);
        }

        // Given the previous check guarantees that we either have C23 or GNU extensions.
        // If the guarantee is that we have GNU extensions, then we need to check if we have at least C11 otherwise we cannot do static assertions

        if !standard.allows_static_assertions() {
            error!(
                "Cannot currently guarantee bitfield size before C11 standard due to lack of static assertions! Thus they are not allowed if using {0}",
                standard.to_string()
            );
            return Err(CompilerError::SourceAndCStandardMismatch);
        }

        for bitfield_definition in &file.definitions.bitfields {
            output_bitfield(&mut header_file, configurations, bitfield_definition)?;
        }
    }

    // Structs
    // ————————

    // TO-DO: Implement!!!

    // Print out structs
    for struct_definition in &file.definitions.structs {
        output_struct(&mut header_file, configurations, struct_definition)?;

        // Add struct metadata (initializer, descriptor, encoded sizes)
        output_struct_metadata(&mut header_file, configurations, struct_definition)?
    }

    // Messages
    // —————————

    // Print out messages
    for message_definition in &file.definitions.messages {
        output_message(&mut header_file, configurations, message_definition)?;

        // Add struct metadata (initializer, descriptor, encoded sizes)
        output_message_metadata(&mut header_file, configurations, message_definition)?
    }

    // End & C++ guards
    // —————————————————

    header_file.add_line(&"#ifdef __cplusplus".to_string());
    header_file.add_line(&"}".to_string());
    header_file.add_line(&"#endif // __cplusplus".to_string());
    header_file.add_newline();

    header_file.add_line(&format!("#endif // {0}_RUNE_H", file.name.to_uppercase()));

    // Output file
    // ————————————

    header_file.output_file()
}
