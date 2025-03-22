use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process;

// Variable information structure
struct VarInfo {
    name: String,
    mutable: bool,
    file_path: PathBuf,
    line_number: usize,
    context: String,
    var_type: String, // New property to store the variable type
}

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

struct AnalysisResults {
    mutable_vars: Vec<VarInfo>,
    immutable_vars: Vec<VarInfo>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

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

    let results = analyze_project(project_dir)?;

    println!("\nAnalysis complete!");
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

fn analyze_project(dir: &str) -> Result<AnalysisResults, Box<dyn Error>> {
    let mut mutable_vars = Vec::new();
    let mut immutable_vars = Vec::new();

    visit_dirs(Path::new(dir), &mut mutable_vars, &mut immutable_vars)?;

    Ok(AnalysisResults {
        mutable_vars,
        immutable_vars,
    })
}

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

fn analyze_file(
    file_path: &Path,
    mutable_vars: &mut Vec<VarInfo>,
    immutable_vars: &mut Vec<VarInfo>,
) -> io::Result<()> {
    let mut file = File::open(file_path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        // Skip comments
        if line.trim().starts_with("//") || line.trim().starts_with("/*") {
            continue;
        }

        // Look for variable declarations
        // This is a simplified approach and might miss some cases

        // Check for let mut declarations
        if let Some(idx) = line.find("let mut ") {
            if let Some((name, var_type)) = extract_var_name_and_type(line, idx + 8) {
                mutable_vars.push(VarInfo {
                    name: name.to_string(),
                    mutable: true,
                    file_path: file_path.to_path_buf(),
                    line_number: i + 1,
                    context: line.to_string(),
                    var_type: var_type.to_string(), // Store the variable type
                });
            }
        }
        // Check for let (immutable) declarations
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
                        var_type: var_type.to_string(), // Store the variable type
                    });
                }
            }
        }
    }

    Ok(())
}

fn extract_var_name_and_type(line: &str, start_idx: usize) -> Option<(&str, &str)> {
    let rest = &line[start_idx..];
    let name_end = rest.find(|c: char| !c.is_alphanumeric() && c != '_');
    let name = match name_end {
        Some(end) if end > 0 => &rest[..end],
        None if !rest.is_empty() => rest,
        _ => return None,
    };

    let type_start = rest.find(':')?;
    let type_end = rest[type_start..]
        .find(|c| ";=".contains(c))
        .unwrap_or(rest.len());

    // Ensure type_start is less than type_end
    if type_start + 1 >= type_end {
        return None;
    }

    let var_type = rest[type_start + 1..type_end].trim();

    Some((name, var_type))
}

fn print_results(results: &AnalysisResults) {
    println!("\nMutable Variables:");
    for var in &results.mutable_vars {
        println!("  {}", var);
    }

    println!("\nImmutable Variables:");
    for var in &results.immutable_vars {
        println!("  {}", var);
    }
}

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
