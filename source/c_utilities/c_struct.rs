use rune_parser::types::{MemberType, StructDefinition, StructMember, UserDefinitionLink};

use crate::{
    c_configuration::CStandard,
    c_utilities::{CArray, CPrimitive, pascal_to_snake_case, pascal_to_uppercase, spaces},
    compile_error::CompilerError,
    output::*
};

// Struct member methods
// ——————————————————————

pub trait CStructMember {
    fn c_initializer(&self, c_standard: &CStandard) -> Result<String, CompilerError>;
    fn c_size(&self) -> Result<u64, CompilerError>;
    fn create_c_variable(&self, spacing: usize, c_standard: &CStandard) -> Result<String, CompilerError>;
}

impl CStructMember for StructMember {
    fn c_initializer(&self, c_standard: &CStandard) -> Result<String, CompilerError> {
        let string = match &self.data_type {
            MemberType::Primitive(primitive) => primitive.c_initializer(c_standard),
            MemberType::UserDefined(name, _) => format!("{0}_INIT", pascal_to_uppercase(name)),
            MemberType::Array(array) => array.c_initializer(c_standard)?
        };
        Ok(string)
    }

    fn c_size(&self) -> Result<u64, CompilerError> {
        match &self.data_type {
            MemberType::Array(array) => array.c_size(),
            MemberType::Primitive(primitive) => Ok(primitive.c_size()),
            MemberType::UserDefined(_, definition_link) => match definition_link {
                UserDefinitionLink::BitfieldLink(bitfield_definition) => Ok(bitfield_definition.backing_type.c_size()),
                UserDefinitionLink::EnumLink(enum_definition) => Ok(enum_definition.backing_type.c_size()),
                UserDefinitionLink::StructLink(struct_definition) => Ok(struct_definition.c_size()?),
                _ => {
                    error!("Invalid definition link {0:?} found while parsing c_size() of struct member {1}", definition_link, self.identifier);
                    Err(CompilerError::MalformedSource)
                }
            }
        }
    }

    fn create_c_variable(&self, spacing: usize, c_standard: &CStandard) -> Result<String, CompilerError> {
        match &self.data_type {
            MemberType::Primitive(primitive) => primitive.create_c_variable(&self.identifier, spacing, c_standard),
            MemberType::UserDefined(string, _) => Ok(format!("{0}_t {1}{2}", pascal_to_snake_case(string), spaces(spacing), &self.identifier)),
            MemberType::Array(array) => array.create_c_variable(&self.identifier, spacing, c_standard)
        }
    }
}

// Struct definition methods
// ——————————————————————————

pub trait CStructDefinition {
    fn c_size(&self) -> Result<u64, CompilerError>;
    fn index_sort_members(&self) -> Result<Vec<StructMember>, CompilerError>;
}

impl CStructDefinition for StructDefinition {
    fn c_size(&self) -> Result<u64, CompilerError> {
        let mut total_size: u64 = 0;

        // Struct is packed, so there is no padding
        for member in &self.members {
            total_size += member.c_size()?;
        }

        Ok(total_size)
    }

    fn index_sort_members(&self) -> Result<Vec<StructMember>, CompilerError> {
        let mut largest_index: u64 = 0;

        for member in &self.members {
            if member.index > largest_index {
                largest_index = member.index;
            }
        }

        // Get size range of the indexes
        let members_to_sort: u64 = largest_index + 1;

        let mut sorted_members: Vec<StructMember> = Vec::with_capacity(members_to_sort as usize);

        for i in 0..members_to_sort {
            for member in &self.members {
                if member.index == i {
                    sorted_members.push(member.clone());
                }
            }
        }

        Ok(sorted_members)
    }
}
