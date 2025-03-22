use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process;

// Structure to store information about variables
struct VarInfo {
    name: String,       // Variable name
    mutable: bool,      // Whether the variable is mutable
    file_path: PathBuf, // Path to the file where the variable is declared
    line_number: usize, // Line number of the declaration
    context: String,    // Line of code containing the declaration
    var_type: String,   // Type of the variable
}

// Implementing Display trait for VarInfo to format the output
impl fmt::Display for VarInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({}): {} at {}:{} - type: {}",
            self.name,
            if self.mutable { "mutable" } else { "immutable" },
            self.context.trim(),
            self.file_path.display(),
            self.line_number,
            self.var_type // Display the variable type
        )
    }
}

// Structure to store analysis results
struct AnalysisResults {
    mutable_vars: Vec<VarInfo>,   // List of mutable variables
    immutable_vars: Vec<VarInfo>, // List of immutable variables
}

fn main() -> Result<(), Box<dyn Error>> {
    // Collect command-line arguments
    let args: Vec<String> = env::args().collect();

    // Check if the required arguments are provided
    if args.len() < 2 {
        eprintln!(
            "Usage: {} <project_directory> [--output <file>] [--format json|csv|text]",
            args[0]
        );
        process::exit(1);
    }

    let project_dir = &args[1];
    let mut output_file = None;
    let mut format = "text";

    // Parse optional arguments
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--output" => {
                if i + 1 < args.len() {
                    output_file = Some(&args[i + 1]);
                    i += 2;
                } else {
                    eprintln!("Error: --output requires a filename");
                    process::exit(1);
                }
            }
            "--format" => {
                if i + 1 < args.len() {
                    format = &args[i + 1];
                    if !["json", "csv", "text"].contains(&format) {
                        eprintln!("Error: format must be one of: json, csv, text");
                        process::exit(1);
                    }
                    i += 2;
                } else {
                    eprintln!("Error: --format requires a value (json, csv, or text)");
                    process::exit(1);
                }
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                process::exit(1);
            }
        }
    }

    println!("Analyzing Rust project at: {}", project_dir);

    // Analyze the project directory
    let results = analyze_project(project_dir)?;

    println!("\n\x1b[1mSummary:\x1b[0m");
    println!("Found {} mutable variables", results.mutable_vars.len());
    println!("Found {} immutable variables", results.immutable_vars.len());

    // Output results
    match output_file {
        Some(file) => {
            output_results(&results, file, format)?;
            println!("Results written to: {}", file);
        }
        None => {
            // Print to console
            print_results(&results);
        }
    }

    Ok(())
}

// Function to analyze the project directory
fn analyze_project(dir: &str) -> Result<AnalysisResults, Box<dyn Error>> {
    let mut mutable_vars = Vec::new();
    let mut immutable_vars = Vec::new();

    // Recursively visit directories and analyze files
    visit_dirs(Path::new(dir), &mut mutable_vars, &mut immutable_vars)?;

    Ok(AnalysisResults {
        mutable_vars,
        immutable_vars,
    })
}

// Function to visit directories and analyze files
fn visit_dirs(
    dir: &Path,
    mutable_vars: &mut Vec<VarInfo>,
    immutable_vars: &mut Vec<VarInfo>,
) -> io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Skip target directory, which contains build artifacts
                if path.file_name().unwrap_or_default() != "target" {
                    visit_dirs(&path, mutable_vars, immutable_vars)?;
                }
            } else if let Some(extension) = path.extension() {
                if extension == "rs" {
                    analyze_file(&path, mutable_vars, immutable_vars)?;
                }
            }
        }
    }
    Ok(())
}

// Function to analyze a single file
fn analyze_file(
    file_path: &Path,
    mutable_vars: &mut Vec<VarInfo>,
    immutable_vars: &mut Vec<VarInfo>,
) -> io::Result<()> {
    let mut file = File::open(file_path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let lines: Vec<&str> = content.lines().collect();

    // Track if we're in a multiline comment
    let mut in_multiline_comment = false;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Handle comments
        if trimmed.starts_with("//") {
            continue;
        }

        // Handle multiline comments
        if trimmed.contains("/*") && !trimmed.contains("*/") {
            in_multiline_comment = true;
            continue;
        }

        if in_multiline_comment {
            if trimmed.contains("*/") {
                in_multiline_comment = false;
            }
            continue;
        }

        // Skip empty lines
        if trimmed.is_empty() {
            continue;
        }

        // Enhanced pattern matching for variable declarations

        // 1. Check for let mut declarations (standard case)
        if let Some(idx) = line.find("let mut ") {
            if let Some((name, var_type)) = extract_var_name_and_type(line, idx + 8) {
                mutable_vars.push(VarInfo {
                    name: name.to_string(),
                    mutable: true,
                    file_path: file_path.to_path_buf(),
                    line_number: i + 1,
                    context: line.to_string(),
                    var_type: var_type.to_string(),
                });
            }
        }
        // 2. Check for immutable let declarations
        else if let Some(idx) = line.find("let ") {
            // Make sure it's not actually "let mut"
            if !line[idx..].starts_with("let mut ") {
                if let Some((name, var_type)) = extract_var_name_and_type(line, idx + 4) {
                    immutable_vars.push(VarInfo {
                        name: name.to_string(),
                        mutable: false,
                        file_path: file_path.to_path_buf(),
                        line_number: i + 1,
                        context: line.to_string(),
                        var_type: var_type.to_string(),
                    });
                }
            }
        }

        // 3. Check for for loops with mut pattern: "for mut x in"
        if let Some(idx) = line.find("for mut ") {
            if let Some((name, _)) = extract_name_from_for_loop(line, idx + 8) {
                mutable_vars.push(VarInfo {
                    name: name.to_string(),
                    mutable: true,
                    file_path: file_path.to_path_buf(),
                    line_number: i + 1,
                    context: line.to_string(),
                    var_type: "inferred from loop".to_string(),
                });
            }
        }

        // 4. Check for function parameters with mut
        if (line.contains("fn ") || line.contains("pub fn ")) && line.contains("mut ") {
            extract_mut_parameters(line, file_path, i + 1, mutable_vars);
        }

        // 5. Check for pattern matching with mut: "if let Some(mut x) =" or similar
        if line.contains("if let ") || line.contains("while let ") || line.contains("match ") {
            if line.contains("mut ") {
                extract_mut_patterns(line, file_path, i + 1, mutable_vars);
            }
        }
    }

    Ok(())
}

// Function to extract variable name and type from a line of code - improved
fn extract_var_name_and_type(line: &str, start_idx: usize) -> Option<(&str, &str)> {
    let rest = &line[start_idx..];

    // Handle pattern matching with destructuring
    if rest.starts_with("(") || rest.starts_with("{") {
        // Complex pattern - simplified extraction
        return Some((
            rest.split_whitespace().next().unwrap_or("unknown"),
            "complex pattern",
        ));
    }

    // Handle array or tuple destructuring
    if rest.contains('[') || rest.contains('(') {
        return Some((
            rest.split_whitespace().next().unwrap_or("unknown"),
            "destructured",
        ));
    }

    // Standard variable name extraction
    let mut name_end = rest.find(|c: char| !c.is_alphanumeric() && c != '_');

    // If we can't find a valid end, check for string end
    if name_end.is_none() && !rest.is_empty() {
        name_end = Some(rest.len());
    }

    let name = match name_end {
        Some(end) if end > 0 => &rest[..end],
        None if !rest.is_empty() => rest,
        _ => return None,
    };

    // Type extraction - handle both explicit and inferred types
    let var_type = if let Some(type_start) = rest.find(':') {
        let type_end = rest[type_start..]
            .find(|c| ";=".contains(c))
            .unwrap_or(rest.len() - type_start);

        if type_start + 1 >= type_end + type_start {
            "inferred"
        } else {
            rest[type_start + 1..type_start + type_end].trim()
        }
    } else {
        "inferred"
    };

    Some((name, var_type))
}

// New function to extract mutable variable names from for loops
fn extract_name_from_for_loop(line: &str, start_idx: usize) -> Option<(&str, &str)> {
    let rest = &line[start_idx..];
    let name_end = rest.find(|c: char| !c.is_alphanumeric() && c != '_');

    let name = match name_end {
        Some(end) if end > 0 => &rest[..end],
        None if !rest.is_empty() => rest,
        _ => return None,
    };

    Some((name, "inferred from loop"))
}

// New function to extract mutable parameters from function signatures
fn extract_mut_parameters(
    line: &str,
    file_path: &Path,
    line_number: usize,
    mutable_vars: &mut Vec<VarInfo>,
) {
    // Look for "mut " patterns after the opening parenthesis
    if let Some(params_start) = line.find('(') {
        let params_part = &line[params_start..];

        // Find all occurrences of "mut " in the parameters section
        let mut search_idx = 0;
        while let Some(idx) = params_part[search_idx..].find("mut ") {
            let absolute_idx = search_idx + idx;
            let param_name_start = absolute_idx + 4; // Skip "mut "

            // Extract parameter name until next special character
            if let Some(end_idx) =
                params_part[param_name_start..].find(|c: char| !c.is_alphanumeric() && c != '_')
            {
                let param_name = &params_part[param_name_start..param_name_start + end_idx];

                // Extract type if available
                let param_type = if let Some(type_idx) = params_part[param_name_start..].find(':') {
                    let type_start = param_name_start + type_idx + 1;
                    let type_end = params_part[type_start..]
                        .find(|c| ",)".contains(c))
                        .unwrap_or(params_part.len() - type_start);
                    params_part[type_start..type_start + type_end].trim()
                } else {
                    "inferred parameter"
                };

                mutable_vars.push(VarInfo {
                    name: param_name.to_string(),
                    mutable: true,
                    file_path: file_path.to_path_buf(),
                    line_number,
                    context: line.to_string(),
                    var_type: param_type.to_string(),
                });
            }

            // Move search index forward
            search_idx = absolute_idx + 4;
        }
    }
}

// New function to extract mutable variables from pattern matching
fn extract_mut_patterns(
    line: &str,
    file_path: &Path,
    line_number: usize,
    mutable_vars: &mut Vec<VarInfo>,
) {
    // Look for patterns like "Some(mut x)" or "{mut y}"
    let mut search_idx = 0;
    while let Some(idx) = line[search_idx..].find("mut ") {
        let absolute_idx = search_idx + idx;
        let var_name_start = absolute_idx + 4; // Skip "mut "

        // Extract variable name until next special character
        if let Some(end_idx) =
            line[var_name_start..].find(|c: char| !c.is_alphanumeric() && c != '_')
        {
            let var_name = &line[var_name_start..var_name_start + end_idx];

            mutable_vars.push(VarInfo {
                name: var_name.to_string(),
                mutable: true,
                file_path: file_path.to_path_buf(),
                line_number,
                context: line.to_string(),
                var_type: "pattern matched".to_string(),
            });
        } else if !line[var_name_start..].is_empty() {
            // Handle case where the variable is at the end of the line
            let var_name = &line[var_name_start..];

            mutable_vars.push(VarInfo {
                name: var_name.to_string(),
                mutable: true,
                file_path: file_path.to_path_buf(),
                line_number,
                context: line.to_string(),
                var_type: "pattern matched".to_string(),
            });
        }

        // Move search index forward
        search_idx = absolute_idx + 4;
    }
}

// Function to print analysis results to the console
fn print_results(results: &AnalysisResults) {
    println!("\n\x1b[1mMutable Variables:\x1b[0m");
    for var in &results.mutable_vars {
        println!("  {}", var);
    }

    println!("\n\x1b[1mImmutable Variables:\x1b[0m");
    for var in &results.immutable_vars {
        println!("  {}", var);
    }
}

// Function to output analysis results to a file
fn output_results(
    results: &AnalysisResults,
    file: &str,
    format: &str,
) -> Result<(), Box<dyn Error>> {
    match format {
        "json" => output_json(results, file)?,
        "csv" => output_csv(results, file)?,
        "text" => output_text(results, file)?,
        _ => return Err("Invalid format".into()),
    }

    Ok(())
}

// Function to output results in JSON format
fn output_json(results: &AnalysisResults, file: &str) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(file)?;

    // Convert to a serializable structure
    let mut output = HashMap::new();

    let mut_vars: Vec<serde_json::Value> = results
        .mutable_vars
        .iter()
        .map(|v| {
            let mut map = serde_json::Map::new();
            map.insert(
                "name".to_string(),
                serde_json::Value::String(v.name.clone()),
            );
            map.insert(
                "file".to_string(),
                serde_json::Value::String(v.file_path.display().to_string()),
            );
            map.insert(
                "line".to_string(),
                serde_json::Value::Number(serde_json::Number::from(v.line_number)),
            );
            map.insert(
                "context".to_string(),
                serde_json::Value::String(v.context.trim().to_string()),
            );
            map.insert(
                "type".to_string(),
                serde_json::Value::String(v.var_type.clone()), // Include the variable type
            );
            serde_json::Value::Object(map)
        })
        .collect();

    let immut_vars: Vec<serde_json::Value> = results
        .immutable_vars
        .iter()
        .map(|v| {
            let mut map = serde_json::Map::new();
            map.insert(
                "name".to_string(),
                serde_json::Value::String(v.name.clone()),
            );
            map.insert(
                "file".to_string(),
                serde_json::Value::String(v.file_path.display().to_string()),
            );
            map.insert(
                "line".to_string(),
                serde_json::Value::Number(serde_json::Number::from(v.line_number)),
            );
            map.insert(
                "context".to_string(),
                serde_json::Value::String(v.context.trim().to_string()),
            );
            map.insert(
                "type".to_string(),
                serde_json::Value::String(v.var_type.clone()), // Include the variable type
            );
            serde_json::Value::Object(map)
        })
        .collect();

    output.insert("mutable_variables", serde_json::Value::Array(mut_vars));
    output.insert("immutable_variables", serde_json::Value::Array(immut_vars));

    let json = serde_json::to_string_pretty(&output)?;
    file.write_all(json.as_bytes())?;

    Ok(())
}

// Function to output results in CSV format
fn output_csv(results: &AnalysisResults, file: &str) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(file)?;

    // Write header
    writeln!(file, "mutability,name,file,line,context,type")?; // Include the type in the header

    // Write mutable variables
    for var in &results.mutable_vars {
        writeln!(
            file,
            "mutable,\"{}\",\"{}\",{},\"{}\",\"{}\"", // Include the type in the CSV
            var.name,
            var.file_path.display(),
            var.line_number,
            var.context.trim().replace("\"", "\"\""),
            var.var_type // Include the variable type
        )?;
    }

    // Write immutable variables
    for var in &results.immutable_vars {
        writeln!(
            file,
            "immutable,\"{}\",\"{}\",{},\"{}\",\"{}\"", // Include the type in the CSV
            var.name,
            var.file_path.display(),
            var.line_number,
            var.context.trim().replace("\"", "\"\""),
            var.var_type // Include the variable type
        )?;
    }

    Ok(())
}

// Function to output results in text format
fn output_text(results: &AnalysisResults, file: &str) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(file)?;

    writeln!(file, "Mutable Variables ({})", results.mutable_vars.len())?;
    writeln!(file, "-------------------")?;
    for var in &results.mutable_vars {
        writeln!(file, "{}", var)?;
    }

    writeln!(
        file,
        "\nImmutable Variables ({})",
        results.immutable_vars.len()
    )?;
    writeln!(file, "---------------------")?;
    for var in &results.immutable_vars {
        writeln!(file, "{}", var)?;
    }

    Ok(())
}
