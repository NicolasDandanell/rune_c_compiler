use rune_parser::{
    scanner::NumericLiteral,
    types::{ArraySize, ArrayType, DefineValue, FieldIndex, FieldType, MessageDefinition, MessageField, UserDefinitionLink}
};

use crate::{
    c_configuration::{Architecture, CStandard, CompileConfigurations},
    c_utilities::{CArray, CPrimitive, CStructMember, pascal_to_snake_case, pascal_to_uppercase, spaces},
    compile_error::CompilerError,
    output::*
};

// Message field methods
// ——————————————————————

pub trait CMessageField {
    fn c_initializer(&self, c_standard: &CStandard) -> Result<String, CompilerError>;
    fn c_size(&self) -> Result<u64, CompilerError>;
    fn c_size_definition(&self, c_standard: &CStandard) -> Result<String, CompilerError>;
    fn create_c_variable(&self, spacing: usize, c_standard: &CStandard) -> Result<String, CompilerError>;
    fn index_empty(index: u64) -> Result<MessageField, CompilerError>;
}

impl CMessageField for MessageField {
    fn c_initializer(&self, c_standard: &CStandard) -> Result<String, CompilerError> {
        let string = match &self.data_type {
            FieldType::Primitive(primitive) => primitive.c_initializer(c_standard),
            FieldType::UserDefined(name, _) => format!("{0}_INIT", pascal_to_uppercase(name)),
            FieldType::Array(array) => array.c_initializer(c_standard)?,
            FieldType::Empty => {
                error!("Cannot initialize an empty field!");
                return Err(CompilerError::LogicError);
            }
        };
        Ok(string)
    }

    fn c_size_definition(&self, c_standard: &CStandard) -> Result<String, CompilerError> {
        let size_string: String = match &self.data_type {
            FieldType::Primitive(primitive) => format!("sizeof({0})", primitive.to_c_type(c_standard)?),
            FieldType::UserDefined(type_name, _) => format!("sizeof({0}_t)", pascal_to_snake_case(type_name)),
            FieldType::Array(array) => {
                let type_string: String = match &array.data_type {
                    ArrayType::Primitive(primitive) => format!("sizeof({0})", primitive.to_c_type(c_standard)?),
                    ArrayType::UserDefined(definition_name, _) => format!("sizeof({0}_t)", pascal_to_snake_case(definition_name))
                };

                format!("({0} * {1})", type_string, array.element_count)
            },
            FieldType::Empty => String::from("0")
        };
        Ok(size_string)
    }

    fn c_size(&self) -> Result<u64, CompilerError> {
        match &self.data_type {
            // Calculate Array size based on (field type * field size)
            FieldType::Array(array) => {
                // Get the array size first
                let array_size: u64 = match &array.element_count {
                    ArraySize::Integer(value, _) => *value,
                    ArraySize::UserDefinition(definition) => match &definition.value {
                        DefineValue::NumericLiteral(value) => match value {
                            NumericLiteral::PositiveInteger(value, _) => *value,
                            _ => {
                                error!("Got \"{0:?}\" array size definition of an invalid type!", self.identifier);
                                return Err(CompilerError::MalformedSource);
                            }
                        },
                        _ => {
                            error!("Got \"{0}\" array size definition of an invalid type!", self.identifier);
                            return Err(CompilerError::MalformedSource);
                        }
                    }
                };

                // Parse the byte size based on the array type
                let total_size: u64 = match &array.data_type {
                    ArrayType::Primitive(primitive) => primitive.c_size() * array_size,
                    ArrayType::UserDefined(definition_name, definition_link) => match &definition_link {
                        UserDefinitionLink::NoLink => {
                            error!("Could not find definition for type {0} while parsing C size. This should not happen!", definition_name);
                            return Err(CompilerError::MalformedSource);
                        },
                        UserDefinitionLink::BitfieldLink(bitfield_definition) => bitfield_definition.backing_type.c_size() * array_size,
                        UserDefinitionLink::EnumLink(enum_definition) => enum_definition.backing_type.c_size() * array_size,
                        UserDefinitionLink::MessageLink(_) => {
                            error!("Cannot have an array of messages! This should not happen!");
                            return Err(CompilerError::MalformedSource);
                        },
                        UserDefinitionLink::StructLink(struct_definition) => {
                            let mut struct_size: u64 = 0;

                            // Call this function recursively for each struct member to get size
                            for member in &struct_definition.members {
                                struct_size += member.c_size()?;
                            }

                            struct_size * array_size
                        }
                    }
                };

                Ok(total_size)
            },
            FieldType::Empty => Ok(0),
            FieldType::Primitive(primitive) => Ok(primitive.c_size()),
            FieldType::UserDefined(definition_name, definition_link) => match &definition_link {
                UserDefinitionLink::NoLink => {
                    error!("Found no definition link for item {0}!", definition_name);
                    Err(CompilerError::MalformedSource)
                },
                UserDefinitionLink::BitfieldLink(bitfield_definition) => Ok(bitfield_definition.backing_type.c_size()),
                UserDefinitionLink::EnumLink(enum_definition) => Ok(enum_definition.backing_type.c_size()),
                UserDefinitionLink::MessageLink(message_definition) => {
                    let mut total_size: u64 = 0;

                    for field in &message_definition.fields {
                        total_size += field.c_size()?;
                    }

                    Ok(total_size)
                },
                UserDefinitionLink::StructLink(struct_definition) => {
                    let mut struct_size: u64 = 0;

                    // Call this function recursively for each struct member to get size
                    for member in &struct_definition.members {
                        struct_size += member.c_size()?;
                    }

                    Ok(struct_size)
                }
            }
        }
    }

    fn create_c_variable(&self, spacing: usize, c_standard: &CStandard) -> Result<String, CompilerError> {
        match &self.data_type {
            FieldType::Primitive(primitive) => primitive.create_c_variable(&self.identifier, spacing, c_standard),
            FieldType::UserDefined(string, _) => Ok(format!("{0}_t {1}{2}", pascal_to_snake_case(string), spaces(spacing), &self.identifier)),
            FieldType::Array(array) => array.create_c_variable(&self.identifier, spacing, c_standard),
            FieldType::Empty => {
                error!("Cannot create an empty field!");
                Err(CompilerError::LogicError)
            }
        }
    }

    fn index_empty(index: u64) -> Result<MessageField, CompilerError> {
        // Check if value is positive and within the legal values (0 to and including 31)
        let field_index = match index {
            // Legal values
            0..32 => FieldIndex::Numeric(index),
            // Higher than legal values
            32.. => {
                error!("Field index cannot have a value higher than 31!");
                return Err(CompilerError::LogicError);
            }
        };

        Ok(MessageField {
            identifier: String::from("(empty)"),
            data_type:  FieldType::Empty,
            index:      field_index,
            comment:    None
        })
    }
}

// Struct definition methods
// ——————————————————————————

pub trait CMessageDefinition {
    fn estimate_size(&self, configurations: &CompileConfigurations) -> Result<u64, CompilerError>;
    fn index_sort_fields(&self) -> Result<Vec<MessageField>, CompilerError>;
    fn size_sort_fields(&self, configurations: &CompileConfigurations) -> Result<Vec<MessageField>, CompilerError>;
}

#[derive(Clone, Debug)]
struct SizedMessageField {
    field: MessageField,
    size:  u64
}

impl SizedMessageField {
    fn new(field: &MessageField, size: u64) -> SizedMessageField {
        SizedMessageField { field: field.clone(), size }
    }
}

/// Sort the non-aligned members based on the architecture
fn sort_non_aligned(non_aligned: &mut Vec<SizedMessageField>, configurations: &CompileConfigurations) {
    // Try to fit small non-aligned members in spaces between the bigger members
    // ——————————————————————————————————————————————————————————————————————————

    let sorting_value: u64 = configurations.architecture.byte_size() as u64;

    let mut large_values: Vec<SizedMessageField> = Vec::with_capacity(0x20);
    let mut small_values: Vec<SizedMessageField> = Vec::with_capacity(0x20);

    // Sort all values into large and small items
    for member in &*non_aligned {
        if member.size > sorting_value {
            large_values.push(member.clone());
        } else {
            small_values.push(member.clone());
        }
    }

    // Clear old list
    non_aligned.clear();

    for large in large_values {
        non_aligned.push(large.clone());

        let leftover_bytes: u64 = sorting_value - (large.size % sorting_value);
        let mut best_found_index: isize = -1;
        let mut best_found_size: u64 = sorting_value;

        debug!(
            "    Handling large unaligned field {0} with index {1}, size {2}, and leftover {3}",
            large.field.identifier,
            large.field.index.value(),
            large.size,
            leftover_bytes
        );

        // Try to find a value that fits perfectly. If none found, take the one that fits best
        for (list_index, small) in small_values.iter().enumerate() {
            if (small.size <= leftover_bytes) && (leftover_bytes - small.size < best_found_size) {
                debug!("        Found new best in {0} with a size {1}", small.field.identifier, small.size);
                best_found_size = leftover_bytes - small.size;
                best_found_index = list_index as isize;
            }
        }

        // What to do if no values match ???
        if best_found_index < 0 {
            continue;
        } else {
            non_aligned.push(small_values[best_found_index as usize].clone());
            small_values.remove(best_found_index as usize);
        }
    }

    for remaining_small_value in small_values {
        non_aligned.push(remaining_small_value);
    }
}

impl CMessageDefinition for MessageDefinition {
    fn estimate_size(&self, configurations: &CompileConfigurations) -> Result<u64, CompilerError> {
        // println!("Estimating size of {0}", struct_definition.name);

        let struct_list: Vec<MessageField> = match configurations.sort {
            true => self.size_sort_fields(configurations)?,
            false => self.fields.clone()
        };

        // Calculate padding
        let mut total_size: u64 = 0;

        for member in &struct_list {
            // Assume 8 byte alignment target for items > 4 bytes for worst case scenario
            let member_alignment_size: u64 = match member.c_size()? {
                // Members with a size 0 can be skipped
                0 => continue,
                1 => 1,
                2 => 2,
                3..=4 => 4,
                5.. => match configurations.architecture {
                    Architecture::_32Bit => 4,
                    Architecture::_64Bit => 8
                }
            };

            // Estimate padding if packing disabled, and member does not align to the architecture specified width
            if !configurations.pack_data && (total_size % member_alignment_size) != 0 {
                // Add padding
                let padding: u64 = member_alignment_size - (total_size % member_alignment_size);
                total_size += padding;
            }

            total_size += member.c_size()?;
        }

        Ok(total_size)
    }

    /// Sort the fields declarations of a message based on their index, adding empty fields in skipped fields
    fn index_sort_fields(&self) -> Result<Vec<MessageField>, CompilerError> {
        let mut largest_index: u64 = 0;

        for field in &self.fields {
            if field.index.value() > largest_index {
                largest_index = field.index.value();
            }
        }

        // Get size range of the indexes
        let fields_to_sort: u64 = largest_index + 1;

        let mut sorted_members: Vec<MessageField> = Vec::with_capacity(fields_to_sort as usize);

        for i in 0..fields_to_sort {
            let mut sorted_member: MessageField = MessageField::index_empty(i)?;

            for member in &self.fields {
                if member.index.value() == i {
                    sorted_member = member.clone();
                }
            }

            sorted_members.push(sorted_member);
        }

        Ok(sorted_members)
    }

    /// Sort the field declarations of a message based on their size alignment to reduce eventual padding
    fn size_sort_fields(&self, configurations: &CompileConfigurations) -> Result<Vec<MessageField>, CompilerError> {
        let mut full_list: Vec<MessageField> = Vec::with_capacity(0x20);

        let mut aligned_8: Vec<SizedMessageField> = Vec::with_capacity(0x20);
        let mut aligned_4: Vec<SizedMessageField> = Vec::with_capacity(0x20);
        let mut aligned_2: Vec<SizedMessageField> = Vec::with_capacity(0x20);
        let mut aligned_1: Vec<SizedMessageField> = Vec::with_capacity(0x20);

        // Attempt to maintain index order wherever it makes sense
        for field in &self.fields {
            let size: u64 = field.c_size()?;

            // Zero-size fields are discarded
            if size == 0 {
                warning!("Member {0} of struct {1} had size 0.", field.identifier, self.name);
                continue;
            }

            // Align by 8 only if platform is 64 bit. If building for a 32 bit platform sorting by 8 is pointless
            if size % 8 == 0 && configurations.architecture == Architecture::_64Bit {
                // First 8 aligned
                aligned_8.push(SizedMessageField::new(field, size));
            } else if field.c_size()? % 4 == 0 {
                // First 4 aligned
                aligned_4.push(SizedMessageField::new(field, size));
            } else if field.c_size()? % 2 == 0 {
                // First 2 aligned
                aligned_2.push(SizedMessageField::new(field, size));
            } else {
                // Lastly non aligned
                aligned_1.push(SizedMessageField::new(field, size));
            }
        }

        // Sort the non-aligned fields to allow efficient packing
        sort_non_aligned(&mut aligned_1, configurations);

        // Append all field elements into the full sorted list
        full_list.append(&mut aligned_8.into_iter().map(|sized_member| sized_member.field).collect());
        full_list.append(&mut aligned_4.into_iter().map(|sized_member| sized_member.field).collect());
        full_list.append(&mut aligned_2.into_iter().map(|sized_member| sized_member.field).collect());
        full_list.append(&mut aligned_1.into_iter().map(|sized_member| sized_member.field).collect());

        Ok(full_list)
    }
}
