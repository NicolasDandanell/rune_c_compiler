use rune_parser::scanner::NumericLiteral;

use crate::c_utilities::CStandard;

// String helper functions
// ————————————————————————

/// TODO: Fix multiline comments

pub fn documentation_comment(comment: &str, offset: usize, c_standard: &CStandard) -> String {
    match comment.contains('\n') {
        // Single line comment
        false => match *c_standard >= CStandard::C99 {
            true => format!("{0}/// {1}", spaces(offset), comment.trim_start()),
            false => format!("{0}/** {1} */", spaces(offset), comment.trim_start())
        },
        // Multi line comment
        true => {
            let mut lines: Vec<&str> = comment.split('\n').collect();

            let mut final_string: String = String::with_capacity(comment.len() * 2);

            if *c_standard < CStandard::C99 {
                final_string.push_str(&format!("{0}/**\n", spaces(offset)));
            }

            let mut previous_had_text = false;

            // Purge first line if they are empty
            if lines.first().unwrap().len() == 0 {
                lines.remove(0);
            }

            // Purge last line if they are empty
            if lines.last().unwrap().len() == 0 {
                lines.pop();
            }

            for line in lines {
                if !previous_had_text && !line.contains(char::is_alphanumeric) {
                    continue;
                }

                // Check if the line contains any alphanumeric characters
                previous_had_text = line.contains(char::is_alphanumeric);

                match *c_standard >= CStandard::C99 {
                    true => final_string.push_str(&format!("{0}/// {1}\n", spaces(offset), line.trim_start())),
                    false => final_string.push_str(&format!("{0} * {1}\n", spaces(offset), line.trim_start()))
                }
            }

            if *c_standard < CStandard::C99 {
                final_string.push_str(&format!("{0} */", spaces(offset)));
            }

            // Trim any trailing newline
            if final_string.ends_with('\n') {
                final_string.pop();
            }

            final_string
        }
    }
}

pub fn comment(comment: &str, offset: usize, c_standard: &CStandard) -> String {
    match *c_standard >= CStandard::C99 {
        true => format!("{0}// {1}", spaces(offset), comment.trim_start()),
        false => format!("{0} /* {1} */", spaces(offset), comment.trim_start())
    }
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

    SPACES[0..amount as usize].iter().collect()
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
