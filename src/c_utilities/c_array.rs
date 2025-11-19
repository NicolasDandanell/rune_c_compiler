use rune_parser::types::{Array, ArrayType, Primitive, UserDefinitionLink};

use crate::{
    CStandard, CompilerError,
    c_utilities::{CPrimitive, CStructDefinition, pascal_to_snake_case, pascal_to_uppercase, spaces},
    output::*
};

// Array
// ——————

pub trait CArray {
    fn c_initializer(&self, c_standard: &CStandard) -> Result<String, CompilerError>;
    fn c_size(&self) -> Result<u64, CompilerError>;
    fn create_c_variable(&self, name: &str, spacing: usize, c_standard: &CStandard) -> Result<String, CompilerError>;
}

impl CArray for Array {
    fn c_initializer(&self, c_standard: &CStandard) -> Result<String, CompilerError> {
        Ok(format!(
            "{{ {0} }}",
            match &self.data_type {
                // Special 128 bit case
                ArrayType::Primitive(primitive) if *primitive == Primitive::I128 || *primitive == Primitive::U128 => {
                    String::from("0")
                },
                ArrayType::Primitive(primitive) => primitive.c_initializer(c_standard),
                ArrayType::UserDefined(definition_name, _) => format!("{0}_INIT", pascal_to_uppercase(definition_name))
            }
        ))
    }

    fn c_size(&self) -> Result<u64, CompilerError> {
        match self.element_count.value() {
            Ok(value) => Ok(self.data_type.c_size()? * value),
            Err(error) => {
                error!("");
                Err(CompilerError::ParsingError(error))
            }
        }
    }

    fn create_c_variable(&self, name: &str, spacing: usize, c_standard: &CStandard) -> Result<String, CompilerError> {
        Ok(format!("{0} {1}{2}[{3}]", self.data_type.to_c_type(c_standard)?, spaces(spacing), name, self.element_count))
    }
}

// Array Type
// ———————————

pub trait CArrayType {
    fn c_size(&self) -> Result<u64, CompilerError>;
    fn to_c_type(&self, c_standard: &CStandard) -> Result<String, CompilerError>;
}

impl CArrayType for ArrayType {
    fn c_size(&self) -> Result<u64, CompilerError> {
        match self {
            ArrayType::Primitive(primitive) => Ok(primitive.c_size()),
            ArrayType::UserDefined(_, definition_link) => match definition_link {
                UserDefinitionLink::BitfieldLink(bitfield_definition) => Ok(bitfield_definition.backing_type.c_size()),
                UserDefinitionLink::EnumLink(enum_definition) => Ok(enum_definition.backing_type.c_size()),
                UserDefinitionLink::StructLink(struct_definition) => Ok(struct_definition.c_size()?),
                _ => {
                    error!("Invalid definition link {0:?} found while parsing c_size() of array type {1:?}", definition_link, self);
                    Err(CompilerError::MalformedSource)
                }
            }
        }
    }

    fn to_c_type(&self, c_standard: &CStandard) -> Result<String, CompilerError> {
        match self {
            ArrayType::Primitive(primitive) => primitive.to_c_type(c_standard),
            ArrayType::UserDefined(definition, _) => Ok(format!("{0}_t", pascal_to_snake_case(definition)))
        }
    }
}
