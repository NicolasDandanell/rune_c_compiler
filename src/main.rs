mod header;
mod parser;
mod runic_definitions;
mod source;
mod utilities;

use clap::Parser;
use crate::{ utilities::CConfigurations, header::output_header, parser::output_parser, runic_definitions::output_runic_definitions, source::output_source };
use rune_parser::{ parser_rune_files, RuneFileDescription };
use std::{ fs::create_dir, path::Path };

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path of folder where to find Rune files
    #[arg(long, short = 'i')]
    rune_folder: String,

    /// Path of folder where to output source code
    #[arg(long, short = 'o')]
    output_folder: String,

    /// Whether to pack (remove padding) from outputted sources - Defaults to false
    #[arg(long, short = 'p', default_value = "false")]
    pack: bool,

    /// Whether to store all Rune data in a specific section. By default no section is declared
    #[arg(long, short = 'd')]
    data_section: Option<String>,

    /// Whether to sort struct field placement to optimize alignment - Defaults to true
    #[arg(long, short = 's', default_value = "true")]
    sort: bool
}

#[derive(Debug, Clone)]
pub struct CompileConfigurations {
    /// Whether or not to pack data structures
    pack: bool,

    /// Whether to declare all rune data in a specific section - Default to None
    section: Option<String>,

    /// Whether to size sort structs to optimize packing - Defaults to true
    sort: bool
}

fn main() -> Result<(), usize> {

    // Parse arguments
    // ————————————————

    let args: Args = Args::parse();

    let input_path: &Path                     = Path::new(args.rune_folder.as_str());
    let output_path: &Path                    = Path::new(args.output_folder.as_str());
    let configurations: CompileConfigurations = CompileConfigurations {
        pack:    args.pack,
        section: args.data_section,
        sort:    args.sort
    };

    // Validate arguments
    // ———————————————————

    // If output folder does exist, create it
    if !output_path.is_dir() {
        match create_dir(output_path) {
            Err(error) => panic!("Cannot create directory {0:?}. Got error {1}", output_path, error),
            Ok(()) => ()
        }
    }

    let definitions_list = match parser_rune_files(input_path) {
        Ok(value) => value,
        Err(error)         => panic!("Could not parser Rune files! Got error {0:?}", error)
    };

    // Create source files
    // ————————————————————

    output_c_files(definitions_list, output_path, configurations);

    Ok(())
}

pub fn output_c_files(file_descriptions: Vec<RuneFileDescription>, output_path: &Path, configurations: CompileConfigurations) {

    let c_configurations: CConfigurations = CConfigurations::parse(&file_descriptions, &configurations);

    // Create runic definitions file
    println!("Outputting runic definitions");
    output_runic_definitions(&file_descriptions, &c_configurations, output_path);

    // Create source and header files matching the Rune files
    println!("Outputting headers and sources for:");
    for file in &file_descriptions {
        println!("    {0}.rune", file.file_name);

        // Create header file
        output_header(&file, output_path);

        // Create source file
        output_source(&file, output_path);
    }

    // Create parser
    println!("Outputting parser file");
    output_parser(&file_descriptions, output_path);

    println!("Done!");
}
