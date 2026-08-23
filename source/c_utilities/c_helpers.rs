use std::ops::Index;

use rune_parser::scanner::NumericLiteral;

// String helper functions
// ————————————————————————

pub fn documentation_comment(comment: &str, offset: usize) -> String {
    let initial_space: &str = match comment.chars().nth(0) {
        Some(char) => match char {
            ' ' => "",
            _   => " "
        },
        None => " "
    };

    format!("{0}///{1}{2}", spaces(offset), initial_space, comment)
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

/// Output the amount of ' ' spaces
pub fn spaces(amount: usize) -> String {
    const SPACES: [char; 0x100] = [' '; 0x100];

    SPACES[0 .. amount as usize].iter().collect()
}

// Numeric value helper functions
// ———————————————————————————————

pub trait CNumericValue {
    fn requires_size(&self) -> u64;
}

impl CNumericValue for NumericLiteral {
    fn requires_size(&self) -> u64 {
        let leading_zeroes = match self {
            NumericLiteral::AsciiChar(_) => 1,
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
