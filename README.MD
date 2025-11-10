# Rune to C Compiler

This program was written to compile _.rune_ files into C source files. It allows the user to specify whether the packages and respective metadata should be packed, as well as which linker section the metadata should be placed in. This is still in active development, and many changes and features are yet to come.

## Dependencies
Currently this program needs to be compiled from source to be used by the main Rune library (not published yet), but binaries are planned to be released in the future to avoid a dependence on Rust to run the program in the main Rune library.

## Usage
To use the Rune C compiler, simply specify the following parameters:

* __--input_folder (-i) <path_to_input_folder>__ -
Path to folder where the _.rune_ files are located. The compiler will automatically search subfolders recursively for more _.rune_ files. Can be passed multiple times if rune files are spread over multiple different directories.

* __--output_folder (-o) <path_to_input_folder>__ - Path where to output the generated C source files. They will keep the same folder structure as where they were found within the input folder.

### Optional Arguments

* __--pack_data (-p)__ - Optional argument to pack generated data structures. By default they are not packed.

* __--pack_metadata (-m)__ - Optional argument to pack generated parsing metadata structures. By default they are not packed.

* __--data_section (-d) <linker_section>__ - Optional argument to place all generated parsing data into a specific linker section.

* __--unsorted (-u)__ Optional argument to avoid optimizing field elements in the structs for better alignment. By default they are optimized.

* __--silent (-s)__ Optional argument to specify whether the program should avoid printing any info output at all. By default info output is printed

* __--c_standard (-s)__ Optional argument which specifies which C standard the output source should comply with. By default it compiles to C23 standard

* __--debug__ Optional argument which enables the output of debug messages. Used mostly for bug-finding and developing on the compiler.
