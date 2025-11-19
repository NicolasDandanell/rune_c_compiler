use rune_parser::scanner::NumericLiteral;

// String helper functions
// ————————————————————————

/// Output the amount of ' ' spaces
pub fn spaces(amount: usize) -> String {
    let mut spaces = String::with_capacity(0x40);

    for _ in 0..amount {
        spaces.push(' ');
    }

    spaces
}

/// Convert NamedVariable to named_variable
pub fn pascal_to_snake_case(pascal: &str) -> String {
    let mut snake: String = String::with_capacity(0x40);

    for i in 0..pascal.len() {
        let letter: char = pascal.chars().nth(i).unwrap();

        if i != 0 && letter.is_ascii_uppercase() {
            snake.push('_');
        }

        snake.push(letter.to_ascii_lowercase());
    }

    snake
}

/// Convert NamedVariable to NAMED_VARIABLE
pub fn pascal_to_uppercase(pascal: &str) -> String {
    let mut uppecase: String = String::with_capacity(0x40);

    for i in 0..pascal.len() {
        let letter: char = pascal.chars().nth(i).unwrap();

        if i != 0 && letter.is_ascii_uppercase() {
            uppecase.push('_');
        }

        uppecase.push(letter.to_ascii_uppercase());
    }

    uppecase
}

// Numeric value helper functions
// ———————————————————————————————

pub trait CNumericValue {
    fn requires_size(&self) -> u64;
}

impl CNumericValue for NumericLiteral {
    fn requires_size(&self) -> u64 {
        let leading_zeroes = match self {
            NumericLiteral::Boolean(_) => return 1,
            NumericLiteral::PositiveInteger(value, _) => value.leading_zeros() / 8,
            NumericLiteral::NegativeInteger(value, _) => value.leading_zeros() / 8,
            NumericLiteral::Float(value) => value.to_bits().leading_zeros() / 8
        };

        match leading_zeroes {
            0..4 => 8,
            4..6 => 4,
            6..7 => 2,
            7.. => 1
        }
    }
}
