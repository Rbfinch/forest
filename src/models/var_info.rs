// Copyright (c) 2025 Nicholas D. Crosbie
pub mod extractor;
pub mod type_inference;
pub mod visitor;

pub use extractor::*;
pub use type_inference::*;
pub use visitor::*;

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct VarInfo {
    pub name: String,
    pub mutable: bool,
    pub file_path: PathBuf,
    pub line_number: usize,
    pub context: String,
    pub var_kind: String,
    pub var_type: String,
    pub basic_type: String,
    pub scope: String,
}

impl VarInfo {
    pub fn new(
        name: String,
        mutable: bool,
        file_path: PathBuf,
        line_number: usize,
        context: String,
        var_kind: String,
        var_type: String,
        basic_type: String,
    ) -> Self {
        Self {
            name,
            mutable,
            file_path,
            line_number,
            context,
            var_kind,
            var_type,
            basic_type,
            scope: String::new(),
        }
    }

    // Updated method to generate a VSCode-compatible link to the source with proper absolute path
    pub fn vscode_link(&self) -> String {
        // Convert to absolute path if it's not already
        let absolute_path = if self.file_path.is_absolute() {
            self.file_path.clone()
        } else {
            // Try to get the absolute path by using canonical path
            match std::fs::canonicalize(&self.file_path) {
                Ok(path) => path,
                Err(_) => {
                    // Fallback: try joining with the current directory
                    if let Ok(current_dir) = std::env::current_dir() {
                        current_dir.join(&self.file_path)
                    } else {
                        self.file_path.clone() // Last resort: use as-is
                    }
                }
            }
        };

        // Format the link with proper URI encoding
        // vscode://file/<absolute_path>:<line_number>
        format!(
            "vscode://file/{}:{}",
            absolute_path.display().to_string().replace("\\", "/"),
            self.line_number
        )
    }
}
