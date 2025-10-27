use std::{
    fs::{File, create_dir, remove_file},
    io::Write,
    path::Path
};

use crate::{compile_error::CompilerError, output::*};

pub struct OutputFile {
    path:          String,
    name:          String,
    string_buffer: String
}

impl OutputFile {
    pub fn new(output_path: String, file_name: String) -> OutputFile {
        // Create string buffer
        let string_buffer: String = String::with_capacity(0x2000);

        OutputFile {
            path: output_path,
            name: match file_name.strip_prefix("/") {
                None => file_name,
                Some(stripped) => String::from(stripped)
            },
            string_buffer
        }
    }

    pub fn add_line(&mut self, string: String) {
        self.string_buffer.push_str(format!("{0}\n", string).as_str());
    }

    pub fn add_newline(&mut self) {
        self.string_buffer.push_str("\n");
    }

    fn create_folder(path: &Path) -> Result<(), CompilerError> {
        if path.exists() {
            // If path already exists, do nothing and return
            return Ok(());
        }

        match path.parent() {
            None => return Ok(()),
            Some(parent) => {
                OutputFile::create_folder(parent)?;

                match create_dir(path) {
                    Err(error) => {
                        error!("Could not create directory {0:?}. Got error {1}", path, error);
                        return Err(CompilerError::FileSystemError(error));
                    },
                    Ok(_) => Ok(())
                }
            }
        }
    }

    pub fn output_file(&self) -> Result<(), CompilerError> {
        let full_file_name: String = format!("{0}/{1}", self.path, self.name);

        let relative_file_path: &Path = Path::new(&self.name);

        let output_file_path: &Path = Path::new(&full_file_name);

        // Create parent folders if any
        if relative_file_path.parent().is_some() {
            // println!("Calling create folder on {0:?}", output_file_path);
            OutputFile::create_folder(output_file_path.parent().unwrap())?;
        }

        // Check if file already exists
        if output_file_path.exists() {
            match remove_file(output_file_path) {
                Err(error) => {
                    error!("Could not delete existing {0} file. Got error {1}", output_file_path.to_str().unwrap(), error);
                    return Err(CompilerError::FileSystemError(error));
                },
                Ok(_) => ()
            }
        }

        let mut output_file: File = match File::create(output_file_path) {
            Err(error) => {
                error!("Could not create output file \"{0}\". Got error {1}", output_file_path.to_str().unwrap(), error);
                return Err(CompilerError::FileSystemError(error));
            },
            Ok(file_result) => file_result
        };

        match output_file.write(self.string_buffer.as_bytes()) {
            Err(error) => {
                error!("Could not write to \"{0}\" file. Got error {1}", self.name, error);
                return Err(CompilerError::FileSystemError(error));
            },
            Ok(_) => match output_file.flush() {
                Err(error) => {
                    error!("Could not flush to \"{0}\" file. Got error {1}", self.name, error);
                    return Err(CompilerError::FileSystemError(error));
                },
                Ok(_) => Ok(())
            }
        }
    }
}
