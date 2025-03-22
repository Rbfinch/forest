use chrono::Local;
use quote::ToTokens;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use syn::visit::{self, Visit};
use syn::{spanned::Spanned, Expr, Pat, Type};
use toml::Value;

mod args;

// Structure to store information about variables
struct VarInfo {
    name: String,       // Variable name
    mutable: bool,      // Whether the variable is mutable
    file_path: PathBuf, // Path to the file where the variable is declared
    line_number: usize, // Line number of the declaration
    context: String,    // Line of code containing the declaration
    var_kind: String,   // Kind (how declared) of the variable
    var_type: String,   // The fundamental Rust type of the variable
}

// Function to format the type
fn format_type(ty: &Type) -> String {
    quote::quote!(#ty).to_string()
}

// Implementing Display trait for VarInfo to format the output
impl fmt::Display for VarInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({}): {} at {}:{} - kind: {}, type: {}",
            self.name,
            if self.mutable { "mutable" } else { "immutable" },
            self.context.trim(),
            self.file_path.display(),
            self.line_number,
            self.var_kind,
            self.var_type
        )
    }
}

// Structure to store analysis results
struct AnalysisResults {
    mutable_vars: Vec<VarInfo>,   // List of mutable variables
    immutable_vars: Vec<VarInfo>, // List of immutable variables
}

struct AnalysisMetadata {
    project_name: String,
    version: String,
    datetime: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    // Parse command-line arguments using the clap-based module
    let args = args::parse_args();

    // Get the current datetime
    let datetime = Local::now().to_string();
    println!("Analysis run at: {}", datetime);

    // Read the version from Cargo.toml
    let cargo_toml_path = Path::new(&args.project_dir).join("Cargo.toml");
    let cargo_toml_content = fs::read_to_string(cargo_toml_path)?;
    let cargo_toml: Value = toml::from_str(&cargo_toml_content)?;
    let version = cargo_toml["package"]["version"]
        .as_str()
        .unwrap_or("unknown");
    let project_name = cargo_toml["package"]["name"].as_str().unwrap_or("unknown");

    println!("Analyzing Rust project at: {}", args.project_dir);
    println!("Project version: {}", version);

    let metadata = AnalysisMetadata {
        project_name: project_name.to_string(),
        version: version.to_string(),
        datetime,
    };

    // Analyze the project directory
    let mut results = analyze_project(&args.project_dir)?;

    // Sort results if requested
    if args.sort {
        results.mutable_vars.sort_by(|a, b| a.name.cmp(&b.name));
        results.immutable_vars.sort_by(|a, b| a.name.cmp(&b.name));
    }

    println!("\n\x1b[1mSummary:\x1b[0m");
    println!("Found {} mutable variables", results.mutable_vars.len());
    println!("Found {} immutable variables", results.immutable_vars.len());

    // Output results
    match args.output_file {
        Some(ref file) => {
            output_results(&results, &metadata, file, &args.format)?;
            println!("Results written to: {}", file);
        }
        None => {
            // Print to console
            print_results(&results, &metadata);
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

// Function to analyze a single file with syn parser
fn analyze_file(
    file_path: &Path,
    mutable_vars: &mut Vec<VarInfo>,
    immutable_vars: &mut Vec<VarInfo>,
) -> io::Result<()> {
    let mut file = File::open(file_path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    // First, try to parse with syn
    match syn::parse_file(&content) {
        Ok(file_ast) => {
            // Parse using AST approach
            let mut visitor = VariableVisitor {
                file_path: file_path.to_path_buf(),
                lines: content.lines().collect(),
                mutable_vars,
                immutable_vars,
            };

            visitor.visit_file(&file_ast);
            Ok(())
        }
        Err(_) => {
            // Fallback to the manual approach if syn parsing fails
            analyze_file_manual_implementation(file_path, mutable_vars, immutable_vars, &content)
        }
    }
}

// Struct for collecting variables during AST traversal
struct VariableVisitor<'ast> {
    file_path: PathBuf,
    lines: Vec<&'ast str>,
    mutable_vars: &'ast mut Vec<VarInfo>,
    immutable_vars: &'ast mut Vec<VarInfo>,
}

impl<'ast> Visit<'ast> for VariableVisitor<'ast> {
    // Visit local variable declarations (let statements)
    fn visit_local(&mut self, local: &'ast syn::Local) {
        // Get the line number for this node
        let span = local.span();
        // Use a different approach to get line number - spans in proc_macro2 don't have line()
        let line_number = self.get_line_number(&span, &local.to_token_stream().to_string());

        // Get the context (full line of code)
        let context = if line_number <= self.lines.len() {
            self.lines[line_number - 1].to_string()
        } else {
            format!("Unknown context at line {}", line_number)
        };

        // Extract pattern (which contains variable names)
        if let Pat::Ident(pat_ident) = &local.pat {
            let name = pat_ident.ident.to_string();
            let mutable = pat_ident.mutability.is_some();

            // Extract type information
            let var_type = if let Some(init) = &local.init {
                let expr = &init.expr;
                // Try to infer from initialization expression
                infer_type_from_expr(expr)
            } else {
                "inferred".to_string()
            };

            let var_info = VarInfo {
                name,
                mutable,
                file_path: self.file_path.clone(),
                line_number,
                context,
                var_kind: "inferred from initialization".to_string(),
                var_type,
            };

            if mutable {
                self.mutable_vars.push(var_info);
            } else {
                self.immutable_vars.push(var_info);
            }
        } else if let Pat::Type(pat_type) = &local.pat {
            // Handle pattern with explicit type annotation
            self.extract_variables_from_pattern(
                &pat_type.pat,
                &Some(pat_type.ty.as_ref()),
                line_number,
                &context,
            );
        } else {
            // Handle other pattern types (destructuring, etc.)
            self.extract_variables_from_pattern(&local.pat, &None, line_number, &context);
        }

        // Continue traversing the AST
        visit::visit_local(self, local);
    }

    // Visit function parameters
    fn visit_fn_arg(&mut self, arg: &'ast syn::FnArg) {
        if let syn::FnArg::Typed(pat_type) = arg {
            let span = arg.span();
            // Fix span line retrieval
            let line_number = self.get_line_number(&span, &arg.to_token_stream().to_string());

            // Get the context
            let context = if line_number <= self.lines.len() {
                self.lines[line_number - 1].to_string()
            } else {
                format!("Unknown context at line {}", line_number)
            };

            // Extract mutable parameters
            if let Pat::Ident(pat_ident) = &*pat_type.pat {
                if pat_ident.mutability.is_some() {
                    let name = pat_ident.ident.to_string();
                    let var_type = format_type(&pat_type.ty);

                    self.mutable_vars.push(VarInfo {
                        name,
                        mutable: true,
                        file_path: self.file_path.clone(),
                        line_number,
                        context,
                        var_kind: format!("function parameter: {}", quote::quote!(#pat_type.ty)),
                        var_type,
                    });
                }
            }
        }

        visit::visit_fn_arg(self, arg);
    }

    // Visit for loops to catch "for mut x in ..." patterns
    fn visit_expr_for_loop(&mut self, for_loop: &'ast syn::ExprForLoop) {
        let span = for_loop.span();
        // Fix span line retrieval
        let line_number = self.get_line_number(&span, &for_loop.to_token_stream().to_string());

        // Get the context
        let context = if line_number <= self.lines.len() {
            self.lines[line_number - 1].to_string()
        } else {
            format!("Unknown context at line {}", line_number)
        };

        // Check if the loop variable is mutable
        if let Pat::Ident(pat_ident) = &*for_loop.pat {
            if pat_ident.mutability.is_some() {
                let name = pat_ident.ident.to_string();
                // Infer type from the iterator expression
                let var_type = infer_type_from_loop_expr(&for_loop.expr);

                self.mutable_vars.push(VarInfo {
                    name,
                    mutable: true,
                    file_path: self.file_path.clone(),
                    line_number,
                    context,
                    var_kind: "for loop variable".to_string(),
                    var_type,
                });
            }
        } else {
            // Handle other pattern types in for loops
            self.extract_variables_from_pattern(&for_loop.pat, &None, line_number, &context);
        }

        visit::visit_expr_for_loop(self, for_loop);
    }

    // Visit if-let and while-let expressions
    fn visit_expr_if(&mut self, if_expr: &'ast syn::ExprIf) {
        // Fix the source_text access and type issues
        if let (Some(if_let_str), Some(cond_str)) = (
            if_expr.if_token.span().source_text(),
            if_expr.cond.span().source_text(),
        ) {
            if if_let_str.starts_with("if let ") {
                let parts: Vec<&str> = cond_str.splitn(2, '=').collect();
                let (pat, expr) = if parts.len() == 2 {
                    (parts[0].trim(), parts[1].trim())
                } else {
                    (cond_str.as_str(), "")
                };

                // Fix span line retrieval
                let line_number =
                    self.get_line_number(&if_expr.span(), &if_expr.to_token_stream().to_string());

                // Get the context
                let context = if line_number <= self.lines.len() {
                    self.lines[line_number - 1].to_string()
                } else {
                    format!("Unknown context at line {}", line_number)
                };

                // Check for mutable patterns in if-let
                if pat.contains("mut ") {
                    // This is a simplified approach - ideally we'd parse the pattern properly
                    for part in pat.split_whitespace() {
                        if part.starts_with("mut") && part.len() > 3 {
                            let name = part[3..]
                                .trim_matches(|c: char| !c.is_alphanumeric() && c != '_')
                                .to_string();
                            if !name.is_empty() {
                                self.mutable_vars.push(VarInfo {
                                    name,
                                    mutable: true,
                                    file_path: self.file_path.clone(),
                                    line_number,
                                    context: context.clone(),
                                    var_kind: "if-let pattern".to_string(),
                                    var_type: infer_type_from_pattern_match(pat, expr),
                                });
                            }
                        }
                    }
                }
            }
        }

        visit::visit_expr_if(self, if_expr);
    }
}

// Improved helper methods for the visitor
impl VariableVisitor<'_> {
    // Add this new helper method to find line numbers
    fn get_line_number(&self, span: &proc_macro2::Span, token_str: &str) -> usize {
        // Try to find the token in the file content
        let content_str = token_str.trim();
        if !content_str.is_empty() {
            for (idx, line) in self.lines.iter().enumerate() {
                if line.contains(content_str) {
                    return idx + 1;
                }
            }
        }

        // Fallback: approximate by using span information when available
        if let Some(span_text) = span.source_text() {
            for (idx, line) in self.lines.iter().enumerate() {
                if line.contains(&span_text) {
                    return idx + 1;
                }
            }
        }

        // Last resort - use a default value
        1
    }

    fn extract_variables_from_pattern(
        &mut self,
        pat: &Pat,
        ty: &Option<&Type>,
        line_number: usize,
        context: &str,
    ) {
        match pat {
            Pat::Ident(pat_ident) => {
                let name = pat_ident.ident.to_string();
                let mutable = pat_ident.mutability.is_some();

                // Determine the type - either from explicit annotation or by inference
                let var_type = if let Some(ty) = ty {
                    format_type(ty)
                } else {
                    // Try to infer from context
                    infer_type_from_context(context)
                };

                let var_info = VarInfo {
                    name,
                    mutable,
                    file_path: self.file_path.clone(),
                    line_number,
                    context: context.to_string(),
                    var_kind: if ty.is_some() {
                        "explicitly typed pattern".to_string()
                    } else {
                        "pattern match".to_string()
                    },
                    var_type,
                };

                if mutable {
                    self.mutable_vars.push(var_info);
                } else {
                    self.immutable_vars.push(var_info);
                }
            }
            Pat::Tuple(tuple) => {
                // For tuple destructuring, try to extract element types
                for (i, elem) in tuple.elems.iter().enumerate() {
                    let elem_type = if let Some(Type::Tuple(tuple_type)) = ty {
                        tuple_type.elems.get(i)
                    } else {
                        None
                    };

                    self.extract_variables_from_pattern(elem, &elem_type, line_number, context);
                }
            }
            Pat::TupleStruct(tuple_struct) => {
                // For tuple struct patterns like Some(x), try to determine wrapped type
                let struct_name = tuple_struct
                    .path
                    .segments
                    .last()
                    .map(|seg| seg.ident.to_string())
                    .unwrap_or_default();

                // Handle special cases like Option and Result
                let elem_type_hint = match struct_name.as_str() {
                    "Some" => "optional value",
                    "Ok" => "success value",
                    "Err" => "error value",
                    _ => "",
                };

                for elem in &tuple_struct.elems {
                    // When destructuring, pass more specific type information
                    if let Pat::Ident(pat_ident) = elem {
                        let name = pat_ident.ident.to_string();
                        let mutable = pat_ident.mutability.is_some();

                        // Improve the type inference for known wrappers
                        let var_type = if !elem_type_hint.is_empty() {
                            elem_type_hint.to_string()
                        } else {
                            infer_type_from_context(context)
                        };

                        let var_info = VarInfo {
                            name,
                            mutable,
                            file_path: self.file_path.clone(),
                            line_number,
                            context: context.to_string(),
                            var_kind: format!("destructured from {}", struct_name),
                            var_type,
                        };

                        if mutable {
                            self.mutable_vars.push(var_info);
                        } else {
                            self.immutable_vars.push(var_info);
                        }
                    } else {
                        // For more complex nested patterns
                        self.extract_variables_from_pattern(elem, &None, line_number, context);
                    }
                }
            }
            Pat::Struct(struct_pat) => {
                // For struct patterns like Point { x, y }, try to link fields to their types
                let struct_name = struct_pat
                    .path
                    .segments
                    .last()
                    .map(|seg| seg.ident.to_string())
                    .unwrap_or_default();

                for field in &struct_pat.fields {
                    let field_name = field.member.to_token_stream().to_string();

                    if let Pat::Ident(pat_ident) = &*field.pat {
                        let name = pat_ident.ident.to_string();
                        let mutable = pat_ident.mutability.is_some();

                        // Try to infer field type based on struct and field name
                        let var_type = format!("field '{}' of {}", field_name, struct_name);

                        let var_info = VarInfo {
                            name,
                            mutable,
                            file_path: self.file_path.clone(),
                            line_number,
                            context: context.to_string(),
                            var_kind: format!("destructured from struct {}", struct_name),
                            var_type,
                        };

                        if mutable {
                            self.mutable_vars.push(var_info);
                        } else {
                            self.immutable_vars.push(var_info);
                        }
                    } else {
                        // For nested patterns
                        self.extract_variables_from_pattern(
                            &field.pat,
                            &None,
                            line_number,
                            context,
                        );
                    }
                }
            }
            Pat::Reference(ref_pat) => {
                // Process reference patterns like &x or &mut x
                // Pass along information that this is a reference type
                if let Pat::Ident(pat_ident) = &*ref_pat.pat {
                    let name = pat_ident.ident.to_string();
                    let mutable = pat_ident.mutability.is_some() || ref_pat.mutability.is_some();

                    let ref_type = if ref_pat.mutability.is_some() {
                        "mutable reference to"
                    } else {
                        "reference to"
                    };

                    // Try to determine what's being referenced
                    let base_type = infer_type_from_context(context);
                    let var_type = format!("{} {}", ref_type, base_type);

                    let var_info = VarInfo {
                        name,
                        mutable,
                        file_path: self.file_path.clone(),
                        line_number,
                        context: context.to_string(),
                        var_kind: "reference pattern".to_string(),
                        var_type,
                    };

                    if mutable {
                        self.mutable_vars.push(var_info);
                    } else {
                        self.immutable_vars.push(var_info);
                    }
                } else {
                    // For nested patterns within the reference
                    self.extract_variables_from_pattern(&ref_pat.pat, &None, line_number, context);
                }
            }
            Pat::Slice(slice_pat) => {
                // For slice patterns like [a, b, ..rest]
                for elem in &slice_pat.elems {
                    if let Pat::Ident(pat_ident) = elem {
                        let name = pat_ident.ident.to_string();
                        let mutable = pat_ident.mutability.is_some();

                        // Determine if this is a rest pattern (e.g., ..rest)
                        let is_rest = name.starts_with(".."); // Simplistic check

                        let var_type = if is_rest {
                            "remaining slice elements".to_string()
                        } else {
                            "slice element".to_string()
                        };

                        let var_info = VarInfo {
                            name,
                            mutable,
                            file_path: self.file_path.clone(),
                            line_number,
                            context: context.to_string(),
                            var_kind: "slice pattern".to_string(),
                            var_type,
                        };

                        if mutable {
                            self.mutable_vars.push(var_info);
                        } else {
                            self.immutable_vars.push(var_info);
                        }
                    } else {
                        // For nested patterns
                        self.extract_variables_from_pattern(elem, &None, line_number, context);
                    }
                }
            }
            Pat::Or(or_pat) => {
                // For or-patterns like `A | B`
                // Just process the first case for simplicity
                if !or_pat.cases.is_empty() {
                    self.extract_variables_from_pattern(&or_pat.cases[0], ty, line_number, context);
                }
            }
            Pat::Type(type_pat) => {
                // For patterns with explicit type annotations
                self.extract_variables_from_pattern(
                    &type_pat.pat,
                    &Some(&type_pat.ty),
                    line_number,
                    context,
                );
            }
            // Add other pattern types as needed
            _ => {}
        }
    }
}

// New function to infer types from surrounding context
fn infer_type_from_context(context: &str) -> String {
    // Extracting type from various contexts

    // Check for let destructuring with type hints
    if let Some(idx) = context.find("let") {
        // Look for type annotation after the pattern
        if let Some(type_start) = context[idx..].find(':') {
            let type_end = context[idx + type_start..]
                .find(|c| ";=".contains(c))
                .unwrap_or(context.len() - (idx + type_start));

            if type_start + 1 < type_end {
                let type_str = context[idx + type_start + 1..idx + type_start + type_end].trim();
                return extract_detailed_type(type_str);
            }
        }

        // If no explicit type, try to infer from right side of assignment
        if let Some(eq_idx) = context[idx..].find('=') {
            let rhs = context[idx + eq_idx + 1..].trim();

            // Check for vector or array destructuring
            if context[..idx].contains('[') {
                if rhs.contains("vec!") || rhs.contains("Vec::") {
                    // Try to extract element type from vec! macro or Vec::new()
                    if let Some(angle_start) = rhs.find('<') {
                        if let Some(angle_end) = rhs[angle_start..].find('>') {
                            let element_type = rhs[angle_start + 1..angle_start + angle_end].trim();
                            return format!(
                                "vector element of {}",
                                extract_detailed_type(element_type)
                            );
                        }
                    }
                    return "vector element".to_string();
                }
                return "array element".to_string();
            }

            // Check for common patterns in RHS
            if rhs.contains("Some(") {
                return "value inside Option".to_string();
            }
            if rhs.contains("Ok(") {
                return "success value from Result".to_string();
            }
            if rhs.contains("Err(") {
                return "error value from Result".to_string();
            }

            // More specific handling for common functions
            if rhs.contains(".iter()") {
                return "reference to collection element".to_string();
            }
            if rhs.contains(".iter_mut()") {
                return "mutable reference to collection element".to_string();
            }
            if rhs.contains(".into_iter()") {
                return "owned collection element".to_string();
            }
        }
    }

    // Check for function parameters
    if (context.contains("fn ") || context.contains("pub fn ")) && context.contains('(') {
        return "function parameter".to_string();
    }

    // Check for for loops
    if context.contains("for") && context.contains("in") {
        // Handle range-based iteration
        if context.contains("..") {
            return "integer from range".to_string();
        }

        // Look for iterating over collections
        if context.contains("iter()") {
            return "reference to collection element".to_string();
        }
        if context.contains("iter_mut()") {
            return "mutable reference to collection element".to_string();
        }
        if context.contains("into_iter()") {
            return "owned collection element".to_string();
        }

        return "iteration variable".to_string();
    }

    // Pattern matching in if let or match
    if context.contains("let Some(") {
        return "value inside Option".to_string();
    }
    if context.contains("let Ok(") {
        return "success value from Result".to_string();
    }
    if context.contains("let Err(") {
        return "error value from Result".to_string();
    }

    // Default case
    "inferred from context".to_string()
}

// Enhanced function to extract more detailed type information
fn extract_detailed_type(type_str: &str) -> String {
    let type_str = type_str.trim();

    // Handle empty or missing type
    if type_str.is_empty() || type_str == "inferred" {
        return "inferred".to_string();
    }

    // Handle references
    if type_str.starts_with('&') {
        let mutability = if type_str.starts_with("&mut ") {
            "mutable "
        } else {
            ""
        };
        let referenced_type =
            extract_detailed_type(type_str.trim_start_matches("&mut ").trim_start_matches('&'));
        return format!("{}reference to {}", mutability, referenced_type);
    }

    // Handle generics
    if let Some(generic_start) = type_str.find('<') {
        if let Some(generic_end) = type_str.rfind('>') {
            let base_type = type_str[..generic_start].trim();
            let generic_params = type_str[generic_start + 1..generic_end].trim();

            match base_type {
                "Vec" => format!("vector of {}", extract_detailed_type(generic_params)),
                "Option" => format!("optional {}", extract_detailed_type(generic_params)),
                "Result" => {
                    // Handle Result<T, E>
                    if let Some(comma_idx) = generic_params.find(',') {
                        let ok_type = extract_detailed_type(&generic_params[..comma_idx]);
                        let err_type = extract_detailed_type(&generic_params[comma_idx + 1..]);
                        format!("result with Ok({}) or Err({})", ok_type, err_type)
                    } else {
                        format!("result of {}", extract_detailed_type(generic_params))
                    }
                }
                "HashMap" | "BTreeMap" => {
                    // Handle maps with key-value pairs
                    if let Some(comma_idx) = generic_params.find(',') {
                        let key_type = extract_detailed_type(&generic_params[..comma_idx]);
                        let value_type = extract_detailed_type(&generic_params[comma_idx + 1..]);
                        format!("map from {} to {}", key_type, value_type)
                    } else {
                        "map".to_string()
                    }
                }
                "HashSet" | "BTreeSet" => {
                    format!("set of {}", extract_detailed_type(generic_params))
                }
                // For other generic types
                _ => format!("{}<{}>", base_type, generic_params),
            }
        } else {
            type_str.to_string()
        }
    }
    // Handle array types [T; N]
    else if type_str.starts_with('[') && type_str.contains(';') {
        let semicolon_idx = type_str.find(';').unwrap();
        let element_type = extract_detailed_type(&type_str[1..semicolon_idx]);
        let size = type_str[semicolon_idx + 1..].trim_end_matches(']');
        format!("array of {} with size {}", element_type, size)
    }
    // Handle tuple types (T1, T2, ...)
    else if type_str.starts_with('(') && type_str.ends_with(')') {
        let inner = &type_str[1..type_str.len() - 1];
        if inner.is_empty() {
            "unit type ()".to_string()
        } else {
            let components: Vec<String> = inner
                .split(',')
                .map(|s| extract_detailed_type(s.trim()))
                .collect();
            format!("tuple of ({})", components.join(", "))
        }
    }
    // Handle basic types
    else {
        match type_str {
            // Numeric types
            "i8" | "i16" | "i32" | "i64" | "i128" | "isize" => format!("integer ({})", type_str),
            "u8" | "u16" | "u32" | "u64" | "u128" | "usize" => {
                format!("unsigned integer ({})", type_str)
            }
            "f32" | "f64" => format!("floating-point ({})", type_str),

            // Other primitives
            "bool" => "boolean".to_string(),
            "char" => "character".to_string(),
            "String" => "owned string".to_string(),
            "str" => "string slice".to_string(),

            // Default to the type string itself
            _ => type_str.to_string(),
        }
    }
}

// Improved function to extract variable name and kind from a line of code

// New function to infer type from destructuring context
fn infer_destructuring_type<'a>(rhs: &'a str, pattern: &str) -> &'a str {
    // Try to infer the type based on the right-hand side of the assignment
    // and the structure of the pattern

    if rhs.starts_with("vec!") || rhs.contains("Vec::") {
        // Vector destructuring
        if pattern.starts_with("[") {
            return "vector element";
        }
    }

    if rhs.starts_with("[") {
        // Array destructuring
        if pattern.starts_with("[") {
            return "array element";
        }
    }

    if rhs.contains("Some(") {
        // Option destructuring
        if pattern.starts_with("Some(") {
            return "optional value";
        }
    }

    if rhs.contains("Ok(") || rhs.contains("Err(") {
        // Result destructuring
        if pattern.starts_with("Ok(") {
            return "success value";
        }
        if pattern.starts_with("Err(") {
            return "error value";
        }
    }

    // Tuple or struct destructuring
    if (pattern.starts_with("(") && rhs.contains("("))
        || (pattern.starts_with("{") && rhs.contains("{"))
    {
        return "tuple or struct field";
    }

    "destructured value"
}

// Function to infer type from an expression
fn infer_type_from_expr(expr: &Expr) -> String {
    match expr {
        Expr::Lit(lit_expr) => match &lit_expr.lit {
            syn::Lit::Str(_) => "string".to_string(),
            syn::Lit::ByteStr(_) => "byte string".to_string(),
            syn::Lit::Byte(_) => "byte".to_string(),
            syn::Lit::Char(_) => "character".to_string(),
            syn::Lit::Int(int_lit) => {
                // Fix suffix access - it returns &str directly, not Option<&str>
                let suffix = int_lit.suffix();
                if !suffix.is_empty() {
                    match suffix {
                        "i8" | "i16" | "i32" | "i64" | "i128" | "isize" => {
                            format!("integer ({})", suffix)
                        }
                        "u8" | "u16" | "u32" | "u64" | "u128" | "usize" => {
                            format!("unsigned integer ({})", suffix)
                        }
                        _ => "integer".to_string(),
                    }
                } else {
                    "integer".to_string()
                }
            }
            syn::Lit::Float(float_lit) => {
                // Fix suffix access for float literal
                let suffix = float_lit.suffix();
                match suffix {
                    "f32" => "floating-point (f32)".to_string(),
                    "f64" => "floating-point (f64)".to_string(),
                    _ => "floating-point".to_string(),
                }
            }
            syn::Lit::Bool(_) => "boolean".to_string(),
            _ => "literal".to_string(),
        },
        Expr::Array(_) => "array".to_string(),
        Expr::Call(call_expr) => {
            if let Expr::Path(path_expr) = &*call_expr.func {
                let path_string = quote::quote!(#path_expr).to_string();
                if path_string.ends_with("::new") {
                    let type_name = path_string.trim_end_matches("::new");
                    match type_name {
                        "Vec" => "vector".to_string(),
                        "String" => "string".to_string(),
                        "HashMap" => "hash map".to_string(),
                        "BTreeMap" => "tree map".to_string(),
                        _ => format!("{} instance", type_name),
                    }
                } else {
                    "function result".to_string()
                }
            } else {
                "function result".to_string()
            }
        }
        Expr::MethodCall(method_call) => {
            let method_name = method_call.method.to_string();
            match method_name.as_str() {
                "iter" => "iterator".to_string(),
                "iter_mut" => "mutable iterator".to_string(),
                "into_iter" => "owned iterator".to_string(),
                "collect" => "collection".to_string(),
                "map" => "mapped iterator".to_string(),
                "filter" => "filtered iterator".to_string(),
                "unwrap" => "unwrapped value".to_string(),
                "expect" => "unwrapped value".to_string(),
                "clone" => "cloned value".to_string(),
                "to_string" => "string".to_string(),
                _ => "method result".to_string(),
            }
        }
        Expr::Struct(struct_expr) => {
            let struct_name = if let Some(path) = &struct_expr.path.get_ident() {
                path.to_string()
            } else {
                quote::quote!(#struct_expr.path).to_string()
            };
            struct_name
        }
        Expr::Reference(ref_expr) => {
            let mutability = if ref_expr.mutability.is_some() {
                "mutable "
            } else {
                ""
            };
            format!("{}reference", mutability)
        }
        Expr::Binary(bin_expr) => match bin_expr.op {
            syn::BinOp::Add(_)
            | syn::BinOp::Sub(_)
            | syn::BinOp::Mul(_)
            | syn::BinOp::Div(_)
            | syn::BinOp::Rem(_) => "numeric".to_string(),

            syn::BinOp::And(_) | syn::BinOp::Or(_) => "boolean".to_string(),

            syn::BinOp::BitAnd(_)
            | syn::BinOp::BitOr(_)
            | syn::BinOp::BitXor(_)
            | syn::BinOp::Shl(_)
            | syn::BinOp::Shr(_) => "integer".to_string(),

            syn::BinOp::Eq(_)
            | syn::BinOp::Lt(_)
            | syn::BinOp::Le(_)
            | syn::BinOp::Ne(_)
            | syn::BinOp::Ge(_)
            | syn::BinOp::Gt(_) => "boolean".to_string(),

            _ => "expression result".to_string(),
        },
        Expr::Match(_) => "match result".to_string(),
        Expr::If(_) => "conditional result".to_string(),
        _ => "expression result".to_string(),
    }
}

// Function to infer type from a loop iterator expression
fn infer_type_from_loop_expr(expr: &Expr) -> String {
    match expr {
        Expr::Range(_) => "integer (range)".to_string(),
        Expr::MethodCall(method_call) => {
            let method_name = method_call.method.to_string();
            match method_name.as_str() {
                "iter" => "reference to collection element".to_string(),
                "iter_mut" => "mutable reference to collection element".to_string(),
                "into_iter" => "owned collection element".to_string(),
                _ => "collection element".to_string(),
            }
        }
        _ => "collection element".to_string(),
    }
}

// Function to infer type from pattern matching
fn infer_type_from_pattern_match(pattern: &str, _expr: &str) -> String {
    if pattern.contains("Some(") {
        "optional value content".to_string()
    } else if pattern.contains("Ok(") {
        "success result value".to_string()
    } else if pattern.contains("Err(") {
        "error result value".to_string()
    } else if pattern.contains("&") {
        "reference value".to_string()
    } else {
        "pattern matched value".to_string()
    }
}

// Fallback manual parser when syn parsing fails
fn analyze_file_manual_implementation(
    file_path: &Path,
    mutable_vars: &mut Vec<VarInfo>,
    immutable_vars: &mut Vec<VarInfo>,
    content: &str,
) -> io::Result<()> {
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
            if let Some((name, var_kind)) = extract_var_name_and_kind(line, idx + 8) {
                let rust_type = if var_kind != "inferred" {
                    infer_type_from_context(var_kind)
                } else {
                    // Try to infer type from initialization
                    infer_type_from_initialization(line)
                };

                mutable_vars.push(VarInfo {
                    name: name.to_string(),
                    mutable: true,
                    file_path: file_path.to_path_buf(),
                    line_number: i + 1,
                    context: line.to_string(),
                    var_kind: var_kind.to_string(),
                    var_type: rust_type,
                });
            }
        }
        // 2. Check for immutable let declarations
        else if let Some(idx) = line.find("let ") {
            // Make sure it's not actually "let mut"
            if !line[idx..].starts_with("let mut ") {
                if let Some((name, var_kind)) = extract_var_name_and_kind(line, idx + 4) {
                    let rust_type = if var_kind != "inferred" {
                        infer_type_from_context(var_kind)
                    } else {
                        // Try to infer type from initialization
                        infer_type_from_initialization(line)
                    };

                    immutable_vars.push(VarInfo {
                        name: name.to_string(),
                        mutable: false,
                        file_path: file_path.to_path_buf(),
                        line_number: i + 1,
                        context: line.to_string(),
                        var_kind: var_kind.to_string(),
                        var_type: rust_type,
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
                    var_kind: "inferred from loop".to_string(),
                    var_type: infer_type_from_loop(line),
                });
            }
        }

        // 4. Check for function parameters with mut
        if (line.contains("fn ") || line.contains("pub fn ")) && line.contains("mut ") {
            extract_mut_parameters(line, file_path, i + 1, mutable_vars);
        }

        // 5. Check for pattern matching with mut: "if let Some(mut x) =" or similar
        if (line.contains("if let ") || line.contains("while let ") || line.contains("match "))
            && line.contains("mut ")
        {
            extract_mut_patterns(line, file_path, i + 1, mutable_vars);
        }
    }

    Ok(())
}

// New function to extract variable name and kind from a line of code - improved
fn extract_var_name_and_kind(line: &str, start_idx: usize) -> Option<(&str, &str)> {
    let rest = &line[start_idx..];

    // Handle pattern matching with destructuring
    if rest.starts_with("(") || rest.starts_with("{") || rest.starts_with("[") {
        // More detailed extraction for destructuring patterns
        // Get first name in pattern
        let pattern_end = match rest.starts_with("(") {
            true => rest.find(')').unwrap_or(rest.len()),
            false if rest.starts_with("{") => rest.find('}').unwrap_or(rest.len()),
            false => rest.find(']').unwrap_or(rest.len()),
        };

        let pattern = &rest[0..pattern_end + 1];

        // Try to find variable names in the pattern
        let first_var = pattern
            .split(|c| "()[]{},".contains(c))
            .map(|s| s.trim())
            .find(|s| !s.is_empty() && !s.starts_with(".."))
            .unwrap_or("unknown");

        // Check for type annotation
        let type_str = if let Some(type_idx) = rest[pattern_end..].find(':') {
            let type_start = pattern_end + type_idx + 1;
            let type_end = rest[type_start..]
                .find(|c| ";=".contains(c))
                .unwrap_or(rest.len() - type_start);

            if type_start < type_end {
                rest[type_start..type_end].trim()
            } else {
                "complex pattern"
            }
        } else {
            // Try to infer from RHS if present
            if let Some(eq_idx) = rest.find('=') {
                let rhs = rest[eq_idx + 1..].trim();
                infer_destructuring_type(rhs, pattern)
            } else {
                "complex pattern"
            }
        };

        return Some((first_var, type_str));
    }

    // Standard variable name extraction for non-pattern declarations
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

    // kind extraction - handle both explicit and inferred kinds
    let var_kind = if let Some(kind_start) = rest.find(':') {
        let kind_end = rest[kind_start..]
            .find(|c| ";=".contains(c))
            .unwrap_or(rest.len() - kind_start);

        if kind_start + 1 >= kind_end + kind_start {
            "inferred"
        } else {
            rest[kind_start + 1..kind_start + kind_end].trim()
        }
    } else {
        "inferred"
    };

    Some((name, var_kind))
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

// New function to infer type from variable initialization
fn infer_type_from_initialization(line: &str) -> String {
    // Find the equals sign for initialization
    if let Some(eq_idx) = line.find('=') {
        let rhs = line[eq_idx + 1..].trim();

        // String literals
        if rhs.starts_with('"') {
            return "string".to_string();
        }

        // Character literals
        if rhs.starts_with('\'') && rhs.len() >= 3 {
            return "character".to_string();
        }

        // Numeric literals
        if rhs.chars().next().map_or(false, |c| c.is_ascii_digit()) {
            if rhs.contains('.') {
                return "floating-point".to_string();
            } else {
                return "integer".to_string();
            }
        }

        // Boolean literals
        if rhs == "true" || rhs == "false" {
            return "boolean".to_string();
        }

        // Array or vector literals
        if rhs.starts_with('[') {
            if rhs.contains("vec!") || rhs.contains("Vec::new") {
                return "vector".to_string();
            }
            return "array".to_string();
        }

        // Struct construction
        if rhs.contains("{") && !rhs.starts_with("if") && !rhs.starts_with("match") {
            // Try to get struct name
            let struct_name = rhs.split('{').next().unwrap_or("").trim();
            if !struct_name.is_empty() {
                return struct_name.to_string();
            }
            return "struct".to_string();
        }

        // Function/method calls
        if rhs.contains("(") && !rhs.starts_with("if") && !rhs.starts_with("match") {
            return "function result".to_string();
        }
    }

    "inferred".to_string()
}

// New function to infer type from loop context
fn infer_type_from_loop(line: &str) -> String {
    if line.contains("for") && line.contains("in") {
        // Look for common iterator patterns
        if line.contains(".iter()") {
            return "reference to collection element".to_string();
        }
        if line.contains(".iter_mut()") {
            return "mutable reference to collection element".to_string();
        }
        if line.contains(".into_iter()") {
            return "owned collection element".to_string();
        }
        if line.contains("..") {
            return "integer (range)".to_string();
        }
        // Generic case
        return "collection element".to_string();
    }

    "inferred from loop".to_string()
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

                // Extract kind if available
                let param_kind = if let Some(kind_idx) = params_part[param_name_start..].find(':') {
                    let kind_start = param_name_start + kind_idx + 1;
                    let kind_end = params_part[kind_start..]
                        .find(|c| ",)".contains(c))
                        .unwrap_or(params_part.len() - kind_start);
                    params_part[kind_start..kind_start + kind_end].trim()
                } else {
                    "inferred parameter"
                };

                // Extract the Rust type
                let rust_type = infer_type_from_context(param_kind);

                mutable_vars.push(VarInfo {
                    name: param_name.to_string(),
                    mutable: true,
                    file_path: file_path.to_path_buf(),
                    line_number,
                    context: line.to_string(),
                    var_kind: param_kind.to_string(),
                    var_type: rust_type,
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

            // Try to infer the type from pattern matching context
            let pattern_type = infer_type_from_pattern(line);

            mutable_vars.push(VarInfo {
                name: var_name.to_string(),
                mutable: true,
                file_path: file_path.to_path_buf(),
                line_number,
                context: line.to_string(),
                var_kind: "pattern matched".to_string(),
                var_type: pattern_type,
            });
        } else if !line[var_name_start..].is_empty() {
            // Handle case where the variable is at the end of the line
            let var_name = &line[var_name_start..];

            // Try to infer the type from pattern matching context
            let pattern_type = infer_type_from_pattern(line);

            mutable_vars.push(VarInfo {
                name: var_name.to_string(),
                mutable: true,
                file_path: file_path.to_path_buf(),
                line_number,
                context: line.to_string(),
                var_kind: "pattern matched".to_string(),
                var_type: pattern_type,
            });
        }

        // Move search index forward
        search_idx = absolute_idx + 4;
    }
}

// New function to infer type from pattern matching
fn infer_type_from_pattern(line: &str) -> String {
    // Look for common patterns
    if line.contains("Some(") {
        return "optional value content".to_string();
    }
    if line.contains("Ok(") {
        return "success result value".to_string();
    }
    if line.contains("Err(") {
        return "error result value".to_string();
    }
    if line.contains("if let") && line.contains("=") {
        // Try to infer from right side of equals
        if let Some(eq_idx) = line.find('=') {
            let rhs = line[eq_idx + 1..].trim();
            if !rhs.is_empty() {
                return format!(
                    "part of {}",
                    infer_type_from_initialization(&format!("let x = {}", rhs))
                );
            }
        }
    }

    "pattern matched value".to_string()
}

// Function to print analysis results to the console

fn print_results(results: &AnalysisResults, metadata: &AnalysisMetadata) {
    println!("\n\x1b[1mProject Information:\x1b[0m");
    println!("Project Name: {}", metadata.project_name);
    println!("Version: {}", metadata.version);
    println!("Analysis Run At: {}", metadata.datetime);

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
    metadata: &AnalysisMetadata,
    file: &str,
    format: &str,
) -> Result<(), Box<dyn Error>> {
    match format {
        "json" => output_json(results, metadata, file)?,
        "csv" => output_csv(results, metadata, file)?,
        "text" => output_text(results, metadata, file)?,
        _ => return Err("Invalid format".into()),
    }

    Ok(())
}

// Function to output results in JSON format
fn output_json(
    results: &AnalysisResults,
    metadata: &AnalysisMetadata,
    file: &str,
) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(file)?;

    // Convert to a serializable structure
    let mut output = HashMap::new();

    // Add metadata
    let metadata_map = serde_json::json!({
        "version": metadata.version,
        "project_name": metadata.project_name,
        "datetime": metadata.datetime,
    });
    output.insert("metadata", metadata_map);

    // Use the already sorted vectors from the results
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
                "kind".to_string(),
                serde_json::Value::String(v.var_kind.clone()),
            );
            map.insert(
                "type".to_string(),
                serde_json::Value::String(v.var_type.clone()),
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
                "kind".to_string(),
                serde_json::Value::String(v.var_kind.clone()),
            );
            map.insert(
                "type".to_string(),
                serde_json::Value::String(v.var_type.clone()),
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
fn output_csv(
    results: &AnalysisResults,
    metadata: &AnalysisMetadata,
    file: &str,
) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(file)?;

    // Write metadata
    writeln!(file, "Project Name,{}", metadata.project_name)?;
    writeln!(file, "Version,{}", metadata.version)?;
    writeln!(file, "Analysis Run At,{}", metadata.datetime)?;
    writeln!(file)?;

    // Write header
    writeln!(file, "mutability,name,file,line,context,kind,type")?;

    // Write mutable variables
    for var in &results.mutable_vars {
        writeln!(
            file,
            "mutable,\"{}\",\"{}\",{},\"{}\",\"{}\",\"{}\"",
            var.name,
            var.file_path.display(),
            var.line_number,
            var.context.trim().replace("\"", "\"\""),
            var.var_kind,
            var.var_type
        )?;
    }

    // Write immutable variables
    for var in &results.immutable_vars {
        writeln!(
            file,
            "immutable,\"{}\",\"{}\",{},\"{}\",\"{}\",\"{}\"",
            var.name,
            var.file_path.display(),
            var.line_number,
            var.context.trim().replace("\"", "\"\""),
            var.var_kind,
            var.var_type
        )?;
    }

    Ok(())
}

// Function to output results in text format
fn output_text(
    results: &AnalysisResults,
    metadata: &AnalysisMetadata,
    file: &str,
) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(file)?;

    writeln!(file, "Project Information")?;
    writeln!(file, "-------------------")?;
    writeln!(file, "Project Name: {}", metadata.project_name)?;
    writeln!(file, "Version: {}", metadata.version)?;
    writeln!(file, "Analysis Run At: {}", metadata.datetime)?;
    writeln!(file)?;

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
