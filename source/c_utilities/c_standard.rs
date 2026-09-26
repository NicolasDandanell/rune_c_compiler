use core::fmt::{Display, Formatter};

use crate::{CompilerError, output::is_quiet};

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

    pub fn null_string(&self) -> &str {
        match *self >= CStandard::C23 {
            true => &"nullptr",
            false => &"NULL"
        }
    }

    pub fn bool_string(&self) -> &str {
        match *self >= CStandard::C99 {
            true => &"bool",
            false => &"char"
        }
    }

    pub fn true_string(&self) -> &str {
        match *self >= CStandard::C99 {
            true => &"true",
            false => &"1"
        }
    }

    pub fn false_string(&self) -> &str {
        match *self >= CStandard::C99 {
            true => &"false",
            false => &"0"
        }
    }

    pub fn little_endian_check(&self, strict: bool) -> Result<String, CompilerError> {
        match *self {
            CStandard::C23 => Ok(String::from("__STDC_ENDIAN_NATIVE__ == __STDC_ENDIAN_LITTLE__")),
            _ => match strict {
                false => Ok(String::from("__BYTE_ORDER__ == __ORDER_LITTLE_ENDIAN__")),
                true => Err(CompilerError::SourceAndCStandardMismatch)
            }
        }
    }

    pub fn big_endian_check(&self, strict: bool) -> Result<String, CompilerError> {
        match *self {
            CStandard::C23 => Ok(String::from("__STDC_ENDIAN_NATIVE__ == __STDC_ENDIAN_BIG__")),
            _ => match strict {
                false => Ok(String::from("__BYTE_ORDER__ == __ORDER_BIG_ENDIAN__")),
                true => Err(CompilerError::SourceAndCStandardMismatch)
            }
        }
    }

    pub fn static_assertion(&self, condition: &str, message: &mut str) -> Result<String, CompilerError> {
        match *self {
            CStandard::C11 => Ok(format!("_Static_assert({0}, \"{1}\");", condition, message)),
            CStandard::C23 => Ok(format!("static_assert({0}, \"{1})\");", condition, message)),
            _ => {
                // Build static assertion string, like so: (maybe without macro)
                //
                // #define STATIC_ASSERT(COND, MSG) typedef char static_assertion_ ## MSG [(COND) ? 1 : -1]
                //
                // STATIC_ASSERT(sizeof(potato_t) == 4, potato);
                //
                // typedef uint8_t static_assertion_XXX_size_verification [(COND) ? 1 : -1];

                let pre_11_message: String = message.replace(' ', &"_").replace('!', &"");

                let valid_message: bool = pre_11_message.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');

                if !valid_message {
                    error!("An invalid message was received in a pre-C11 static assertion. This is most likely an error in the rune compiler itself...");
                    info!("Message was: {0}", pre_11_message);
                    return Err(CompilerError::MalformedSource);
                }

                Ok(format!("typedef uint8_t static_assertion_{0}[({1}) ? 1 : -1];", &pre_11_message, condition))
            }
        }
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

    // C11
    // ————

    pub fn allows_static_assertions(&self) -> bool {
        *self >= CStandard::C11
    }

    // C23
    // ————

    pub fn allows_enum_backing_type(&self) -> bool {
        *self >= CStandard::C23
    }

    pub fn allows_nullptr(&self) -> bool {
        *self >= CStandard::C23
    }

    pub fn allows_binary_literals(&self) -> bool {
        *self >= CStandard::C23
    }

    pub fn has_builtin_boolean(&self) -> bool {
        *self >= CStandard::C23
    }

    pub fn allows_endianness_check(&self) -> bool {
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
