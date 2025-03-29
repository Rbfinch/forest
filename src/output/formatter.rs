use crate::models::{ContainerInfo, VarInfo};
use chrono::Local;
use std::io::Write;
use std::path::Path;

pub trait OutputFormatter {
    fn format_analysis_results(
        &self,
        mutable_vars: &[VarInfo],
        immutable_vars: &[VarInfo],
        containers: &[ContainerInfo],
        project_path: &Path,
    ) -> String;
}

pub struct ConsoleFormatter;

impl OutputFormatter for ConsoleFormatter {
    fn format_analysis_results(
        &self,
        mutable_vars: &[VarInfo],
        immutable_vars: &[VarInfo],
        containers: &[ContainerInfo],
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

        // Containers
        output.push_str(&format!("\nFound {} containers:\n", containers.len()));
        for container in containers {
            output.push_str(&format!("  {}\n", container));
        }

        output
    }
}

pub struct HtmlFormatter;
