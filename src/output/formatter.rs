// Copyright (c) 2025 Nicholas D. Crosbie
pub mod extractor;
pub mod type_inference;
pub mod visitor;

pub use extractor::*;
pub use type_inference::*;
pub use visitor::*;

use crate::models::{data_structureInfo, VarInfo};
use chrono::Local;
use std::io::Write;
use std::path::Path;

pub trait OutputFormatter {
    fn format_analysis_results(
        &self,
        mutable_vars: &[VarInfo],
        immutable_vars: &[VarInfo],
        data_structures: &[data_structureInfo],
        project_path: &Path,
    ) -> String;
}

pub struct ConsoleFormatter;

impl OutputFormatter for ConsoleFormatter {
    fn format_analysis_results(
        &self,
        mutable_vars: &[VarInfo],
        immutable_vars: &[VarInfo],
        data_structures: &[data_structureInfo],
        project_path: &Path,
    ) -> String {
        let mut output = String::new();

        // Add timestamp
        let now = Local::now();
        output.push_str(&format!(
            "Analysis completed at: {}\n",
            now.format("%Y-%m-%d %H:%M:%S")
        ));

        // Project info
        output.push_str(&format!("Project path: {}\n\n", project_path.display()));

        // Mutable variables
        output.push_str(&format!(
            "Found {} mutable variables:\n",
            mutable_vars.len()
        ));
        for var in mutable_vars {
            output.push_str(&format!(
                "  {} ({}): {}:{} - {}\n",
                var.name,
                var.var_type,
                var.file_path.display(),
                var.line_number,
                var.context.trim()
            ));
        }

        // Immutable variables
        output.push_str(&format!(
            "\nFound {} immutable variables:\n",
            immutable_vars.len()
        ));
        for var in immutable_vars {
            output.push_str(&format!(
                "  {} ({}): {}:{} - {}\n",
                var.name,
                var.var_type,
                var.file_path.display(),
                var.line_number,
                var.context.trim()
            ));
        }

        // data_structures
        output.push_str(&format!(
            "\nFound {} data_structures:\n",
            data_structures.len()
        ));
        for data_structure in data_structures {
            output.push_str(&format!("  {}\n", data_structure));
        }

        output
    }
}

pub struct HtmlFormatter;
