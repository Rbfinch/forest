// Copyright (c) 2025 Nicholas D. Crosbie
pub mod extractor;
pub mod type_inference;
pub mod visitor;

pub use extractor::*;
pub use type_inference::*;
pub use visitor::*;

use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ContainerInfo {
    pub name: String,
    pub container_type: String,
    pub file_path: PathBuf,
    pub line_number: usize,
}

impl fmt::Display for ContainerInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({}): at {}:{}",
            self.name,
            self.container_type,
            self.file_path.display(),
            self.line_number
        )
    }
}
