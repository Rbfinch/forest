// Copyright (c) 2025 Nicholas D. Crosbie
pub mod extractor;
pub mod type_inference;
pub mod visitor;

pub use extractor::*;
pub use type_inference::*;
pub use visitor::*;

use std::error::Error;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use toml::Value;

pub fn read_file_to_string(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}

pub fn find_cargo_toml(start_dir: &Path) -> Option<PathBuf> {
    let cargo_path = start_dir.join("Cargo.toml");
    if cargo_path.exists() {
        return Some(cargo_path);
    }

    let parent = start_dir.parent()?;
    find_cargo_toml(parent)
}

pub fn parse_cargo_toml(path: &Path) -> Result<Value, Box<dyn Error>> {
    let content = read_file_to_string(path)?;
    let value = content.parse::<Value>()?;
    Ok(value)
}

pub fn find_rust_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut rust_files = Vec::new();

    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Skip hidden directories and target directory
                if let Some(dir_name) = path.file_name() {
                    let dir_name = dir_name.to_string_lossy();
                    if !dir_name.starts_with('.') && dir_name != "target" {
                        let mut subdir_files = find_rust_files(&path)?;
                        rust_files.append(&mut subdir_files);
                    }
                }
            } else if let Some(extension) = path.extension() {
                if extension == "rs" {
                    rust_files.push(path);
                }
            }
        }
    }

    Ok(rust_files)
}
