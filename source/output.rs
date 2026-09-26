static mut QUIET: bool = false;
static mut DEBUG: bool = false;

pub fn enable_quiet() {
    unsafe {
        QUIET = true;
    }
}

pub fn enable_debug() {
    unsafe {
        DEBUG = true;
    }
}

pub fn is_quiet() -> bool {
    unsafe { QUIET }
}

pub fn is_debugging() -> bool {
    unsafe { DEBUG }
}

// Reset  - "\u{001B}[0m"
// Black  - "\u{001B}[0;30m"
// Red    - "\u{001B}[0;31m"
// Green  - "\u{001B}[0;32m"
// Yellow - "\u{001B}[0;33m"
// Blue   - "\u{001B}[0;34m"
// Purple - "\u{001B}[0;35m"
// Cyan   - "\u{001B}[0;36m"
// White  - "\u{001B}[0;37m"

#[macro_export]
macro_rules! debug {
    ($($value: expr), *) => {
        if !is_quiet() && is_debugging() {
            print!("\u{001B}[0;32m");
            print!($($value),*);
            println!("\u{001B}[0m");
        }
    };
}

#[macro_export]
macro_rules! info {
    ($($value: expr), *) => {
        if !is_quiet() {
            println!($($value),*);
        }
    };
}

#[macro_export]
macro_rules! warning {
    ($($value: expr), *) => {
        if !is_quiet() {
            print!("\u{001B}[0;33m");
            print!($($value),*);
            println!("\u{001B}[0m");
        }
    };
}

#[macro_export]
macro_rules! error {
    ($($value: expr), *) => {
        if !is_quiet() {
            eprint!("\u{001B}[0;31m");
            eprint!($($value),*);
            eprintln!("\u{001B}[0m");
        }
    };
}
