mod c_standard;
mod c_utilities;
mod compile_error;
mod header;
#[macro_use]
mod output;
mod output_file;
mod parser;
mod runic_definitions;
mod source;

use std::{fs::create_dir, path::Path};

use clap::Parser;
use rune_parser::{RuneFileDescription, parser_rune_files};

use crate::{
    c_standard::CStandard,
    c_utilities::{CConfigurations, CompileConfigurations},
    compile_error::CompilerError,
    header::output_header,
    output::*,
    parser::output_parser,
    runic_definitions::output_runic_definitions,
    source::output_source
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path of folder where to find Rune files
    #[arg(long, short = 'i')]
    input_folder: String,

    /// Path of folder where to output source code
    #[arg(long, short = 'o')]
    output_folder: String,

    /// Whether to pack (remove padding) from outputted sources - Defaults to false
    #[arg(long, short = 'p', default_value = "false")]
    pack_data: bool,

    /// Whether to pack (remove padding) and size-optimize the outputted parsing metadata - Defaults to false
    #[arg(long, short = 'm', default_value = "false")]
    pack_metadata: bool,

    /// Whether to store all Rune data in a specific section. By default no section is declared
    #[arg(long, short = 'd')]
    data_section: Option<String>,

    /// Whether to avoid sorting struct field placement to optimize alignment - Defaults to false
    #[arg(long, short = 'u', default_value = "false")]
    unsorted: bool,

    /// Whether the program should avoid printing any output at all
    #[arg(long, short = 's', default_value = "false")]
    silent: bool,

    /// Specifies which C standard the output source should comply with - Defaults to C23
    #[arg(long, short = 'c', default_value = "C23")]
    c_standard: String
}

fn main() -> Result<(), CompilerError> {
    // Parse arguments
    // ————————————————

    let args: Args = Args::parse();

    // Disable print output if silent argument was passed
    if args.silent {
        enable_silent();
    };

    let c_standard: CStandard = match CStandard::from_string(&args.c_standard) {
        Err(_) => {
            error!("Invalid C Standard passed. Got {0}, and valid values are: {1}", args.c_standard, CStandard::valid_values());
            return Err(CompilerError::InvalidArgument);
        },
        Ok(value) => value
    };

    let input_path: &Path = Path::new(args.input_folder.as_str());
    let output_path: &Path = Path::new(args.output_folder.as_str());
    let configurations: CompileConfigurations = CompileConfigurations {
        c_standard:    c_standard,
        pack_data:     args.pack_data,
        pack_metadata: args.pack_metadata,
        section:       args.data_section,
        sort:          !args.unsorted
    };

    // Validate arguments
    // ———————————————————

    // If input folder does not exist, return an error
    if !input_path.exists() {
        error!("Input path invalid!");
        return Err(CompilerError::InvalidInputPath);
    }

    // If output folder does exist, create it
    if !output_path.is_dir() {
        match create_dir(output_path) {
            Err(error) => {
                error!("Cannot create directory {0:?}. Got error {1}", output_path, error);
                return Err(CompilerError::FileSystemError(error));
            },
            Ok(()) => ()
        }
    }

    let definitions_list: Vec<RuneFileDescription> = match parser_rune_files(input_path, true, false) {
        Ok(value) => value,
        Err(error) => {
            error!("Could not parser Rune files! Got error {0:?}", error);
            return Err(CompilerError::ParsingError(error));
        }
    };

    // Create source files
    // ————————————————————

    output_c_files(definitions_list, output_path, configurations)
}

pub fn output_c_files(file_descriptions: Vec<RuneFileDescription>, output_path: &Path, configurations: CompileConfigurations) -> Result<(), CompilerError> {
    let c_configurations: CConfigurations = CConfigurations::parse(&file_descriptions, &configurations);

    // Create runic definitions file
    info!("Outputting runic definitions");
    output_runic_definitions(&file_descriptions, &c_configurations, output_path)?;

    // Create source and header files matching the Rune files
    info!("Outputting headers and sources for:");
    for file in &file_descriptions {
        info!("    {0}{1}.rune", file.relative_path, file.file_name);

        // Create header file
        output_header(&file, &c_configurations, output_path)?;

        // Create source file
        output_source(&file, &c_configurations, output_path)?;
    }

    // Create parser
    info!("Outputting parser file");
    output_parser(&file_descriptions, &c_configurations, output_path)?;

    info!("Rune C compiler is done!");
    Ok(())
}
