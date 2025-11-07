use std::fmt::{Display, Formatter};

use crate::{compile_error::CompilerError, output::*};

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum CStandard {
    // C90 is an alias for C89
    C89 = 0,
    C95 = 1,
    C99 = 2,
    C11 = 3,
    C17 = 4,
    C23 = 5
}

impl CStandard {
    pub fn from_string(string: &str) -> Result<CStandard, CompilerError> {
        match string {
            "c89" | "C89" | "c90" | "C90" => Ok(CStandard::C89),
            "c95" | "C95" => Ok(CStandard::C95),
            "c99" | "C99" => Ok(CStandard::C99),
            "c11" | "C11" => Ok(CStandard::C11),
            "c17" | "C17" => Ok(CStandard::C17),
            "c23" | "C23" => Ok(CStandard::C23),
            _ => {
                error!("Invalid C Standard passed. Got {0}, and valid values are: {1}", string, CStandard::valid_values());
                Err(CompilerError::InvalidArgument)
            }
        }
    }

    fn valid_values() -> String {
        String::from("C89/C90, C95, C99, C11, C17, C23")
    }

    // C99
    // ————

    pub fn allows_boolean(&self) -> bool {
        *self >= CStandard::C99
    }

    pub fn allows_designated_initializers(&self) -> bool {
        *self >= CStandard::C99
    }

    pub fn allows_flexible_array_members(&self) -> bool {
        *self >= CStandard::C99
    }

    pub fn allows_inline(&self) -> bool {
        *self >= CStandard::C99
    }

    pub fn allows_integer_types(&self) -> bool {
        *self >= CStandard::C99
    }

    // C23
    // ————

    pub fn allows_enum_backing_type(&self) -> bool {
        *self >= CStandard::C23
    }
}

impl Display for CStandard {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CStandard::C89 => write!(formatter, "C89"),
            CStandard::C95 => write!(formatter, "C95"),
            CStandard::C99 => write!(formatter, "C99"),
            CStandard::C11 => write!(formatter, "C11"),
            CStandard::C17 => write!(formatter, "C17"),
            CStandard::C23 => write!(formatter, "C23")
        }
    }
}
