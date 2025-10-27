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
    pub fn from_string(string: &String) -> Result<CStandard, ()> {
        match string.as_str() {
            "c89" | "C89" | "c90" | "C90" => Ok(CStandard::C89),
            "c95" | "C95" => Ok(CStandard::C95),
            "c99" | "C99" => Ok(CStandard::C99),
            "c11" | "C11" => Ok(CStandard::C11),
            "c17" | "C17" => Ok(CStandard::C17),
            "c23" | "C23" => Ok(CStandard::C23),
            _ => Err(())
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            CStandard::C89 => String::from("C89"),
            CStandard::C95 => String::from("C95"),
            CStandard::C99 => String::from("C99"),
            CStandard::C11 => String::from("C11"),
            CStandard::C17 => String::from("C17"),
            CStandard::C23 => String::from("C23")
        }
    }

    pub fn valid_values() -> String {
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
