static mut SILENT: bool = false;

pub fn enable_silent() {
    unsafe {
        SILENT = true;
    }
}

pub fn is_silent() -> bool {
    unsafe { SILENT }
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
macro_rules! info {
    ($($value: expr), *) => {
        if !is_silent(){
            println!($($value),*);
        }
    };
}

#[macro_export]
macro_rules! warning {
    ($($value: expr), *) => {
        if !is_silent(){
            print!("\u{001B}[0;33m");
            print!($($value),*);
            println!("\u{001B}[0m");
        }
    };
}

#[macro_export]
macro_rules! error {
    ($($value: expr), *) => {
        if !is_silent(){
            eprint!("\u{001B}[0;31m");
            eprint!($($value),*);
            eprintln!("\u{001B}[0m");
        }
    };
}
