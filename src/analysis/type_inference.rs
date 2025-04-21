// Copyright (c) 2025 Nicholas D. Crosbie
use ra_ap_base_db::SourceDatabase;
use ra_ap_hir_def::resolver::HasResolver;
use ra_ap_hir_ty::{InferenceResult, Ty};
use ra_ap_syntax::{ast, AstNode, SourceFile};
use std::collections::HashMap;
use syn::Type;

// Helper function to convert syn::Type to ra_ap_syntax::ast::Type for better type analysis
fn syn_to_ra_type(ty: &Type) -> Option<ast::Type> {
    let type_str = quote::quote!(#ty).to_string();
    let parsed = SourceFile::parse(&type_str);
    if let Ok(source_file) = parsed {
        source_file.syntax().descendants().find_map(ast::Type::cast)
    } else {
        None
    }
}

// Helper to get canonical type representation from rust-analyzer
fn get_canonical_type(type_name: &str) -> &str {
    match type_name {
        // Integer types
        "i8" | "i16" | "i32" | "i64" | "i128" | "isize" => "integer",
        "u8" | "u16" | "u32" | "u64" | "u128" | "usize" => "unsigned integer",

        // Floating point types
        "f32" | "f64" => "floating-point",

        // Other primitives
        "bool" => "boolean",
        "char" => "character",
        "String" => "string",
        "str" => "string slice",
        "Vec" => "vector",
        "HashMap" | "BTreeMap" | "Map" => "map",
        "HashSet" | "BTreeSet" | "Set" => "set",
        "Option" => "optional",
        "Result" => "result",

        // Default case
        _ => type_name,
    }
}

pub fn infer_type_from_context(type_str: &str) -> String {
    if type_str.contains("inferred") {
        type_str.to_string()
    } else {
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

pub fn infer_type_from_initialization(line: &str) -> String {
    // Implementation would go here
    "inferred from initialization".to_string()
}

pub fn infer_basic_type_from_context(line: &str) -> String {
    // Implementation would go here
    "inferred".to_string()
}

// Function to extract the basic Rust type
pub fn extract_basic_type(ty: &Type) -> String {
    // Try to use rust-analyzer for more accurate type information if possible
    if let Some(ra_type) = syn_to_ra_type(ty) {
        if let Some(type_name) = extract_type_name_from_ra(&ra_type) {
            return type_name;
        }
    }

    // Fall back to our existing implementation if rust-analyzer parsing fails
    match ty {
        Type::Path(path) => {
            if let Some(segment) = path.path.segments.last() {
                let type_name = segment.ident.to_string();
                match type_name.as_str() {
                    "Option" => {
                        if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                            if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                                let inner_type = extract_basic_type(inner_ty);
                                return format!("Option<{}>", inner_type);
                            }
                        }
                        "Option<T>".to_string()
                    }
                    "Result" => {
                        if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                            let mut types = args.args.iter().filter_map(|arg| {
                                if let syn::GenericArgument::Type(inner_ty) = arg {
                                    Some(extract_basic_type(inner_ty))
                                } else {
                                    None
                                }
                            });
                            let ok_type = types.next().unwrap_or("T".to_string());
                            let err_type = types.next().unwrap_or("E".to_string());
                            return format!("Result<{}, {}>", ok_type, err_type);
                        }
                        "Result<T, E>".to_string()
                    }
                    "Vec" => {
                        if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                            if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                                let inner_type = extract_basic_type(inner_ty);
                                return format!("Vec<{}>", inner_type);
                            }
                        }
                        "Vec<T>".to_string()
                    }
                    "HashMap" => {
                        if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                            let mut types = args.args.iter().filter_map(|arg| {
                                if let syn::GenericArgument::Type(inner_ty) = arg {
                                    Some(extract_basic_type(inner_ty))
                                } else {
                                    None
                                }
                            });
                            let key_type = types.next().unwrap_or("K".to_string());
                            let value_type = types.next().unwrap_or("V".to_string());
                            return format!("HashMap<{}, {}>", key_type, value_type);
                        }
                        "HashMap<K, V>".to_string()
                    }
                    // Add improved detection for standard library types
                    "Box" | "Rc" | "Arc" | "Cell" | "RefCell" | "Mutex" | "RwLock" => {
                        if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                            if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                                let inner_type = extract_basic_type(inner_ty);
                                return format!(
                                    "{}({})",
                                    get_canonical_type(&type_name),
                                    inner_type
                                );
                            }
                        }
                        format!("{}(T)", type_name)
                    }
                    // Smart detection for standard primitive types
                    s if is_primitive_type(s) => get_canonical_type(s).to_string(),
                    // Default to the type name itself
                    _ => get_canonical_type(&type_name).to_string(),
                }
            } else {
                "Unknown".to_string()
            }
        }
        Type::Reference(ref_type) => {
            let mutability = if ref_type.mutability.is_some() {
                "mut "
            } else {
                ""
            };
            let inner_type = extract_basic_type(&ref_type.elem);
            format!("&{}{}", mutability, inner_type)
        }
        Type::Array(array_type) => {
            let inner_type = extract_basic_type(&array_type.elem);
            format!("[{}; N]", inner_type)
        }
        Type::Tuple(tuple_type) => {
            if tuple_type.elems.is_empty() {
                "()".to_string() // Unit type
            } else {
                let types: Vec<String> = tuple_type.elems.iter().map(extract_basic_type).collect();
                format!("({})", types.join(", "))
            }
        }
        Type::Slice(slice_type) => {
            let inner_type = extract_basic_type(&slice_type.elem);
            format!("[{}]", inner_type)
        }
        Type::Ptr(ptr_type) => {
            let mutability = if ptr_type.mutability.is_some() {
                "mut "
            } else {
                ""
            };
            let inner_type = extract_basic_type(&ptr_type.elem);
            format!("*{}{}", mutability, inner_type)
        }
        Type::Never(_) => "never".to_string(),
        _ => "Unknown".to_string(),
    }
}

// Helper function to extract type name from rust-analyzer AST
fn extract_type_name_from_ra(ra_type: &ast::Type) -> Option<String> {
    match ra_type {
        ast::Type::PathType(path_type) => {
            if let Some(path) = path_type.path() {
                if let Some(segment) = path.segment() {
                    let type_name = segment.name_ref()?.to_string();
                    return Some(get_canonical_type(&type_name).to_string());
                }
            }
            None
        }
        ast::Type::ReferenceType(ref_type) => {
            let mut_text = if ref_type.mut_token().is_some() {
                "mut "
            } else {
                ""
            };
            if let Some(target_type) = ref_type.ty() {
                if let Some(inner_type) = extract_type_name_from_ra(&target_type) {
                    return Some(format!("&{}{}", mut_text, inner_type));
                }
            }
            None
        }
        ast::Type::ArrayType(array_type) => {
            if let Some(element_type) = array_type.ty() {
                if let Some(inner_type) = extract_type_name_from_ra(&element_type) {
                    return Some(format!("[{}; N]", inner_type));
                }
            }
            None
        }
        ast::Type::TupleType(tuple_type) => {
            let mut types = Vec::new();
            if let Some(fields) = tuple_type.fields() {
                for field in fields {
                    if let Some(inner_type) = extract_type_name_from_ra(&field) {
                        types.push(inner_type);
                    }
                }
                return Some(format!("({})", types.join(", ")));
            }
            None
        }
        ast::Type::SliceType(slice_type) => {
            if let Some(element_type) = slice_type.ty() {
                if let Some(inner_type) = extract_type_name_from_ra(&element_type) {
                    return Some(format!("[{}]", inner_type));
                }
            }
            None
        }
        ast::Type::PtrType(ptr_type) => {
            let mut_text = if ptr_type.mut_token().is_some() {
                "mut "
            } else {
                ""
            };
            if let Some(target_type) = ptr_type.ty() {
                if let Some(inner_type) = extract_type_name_from_ra(&target_type) {
                    return Some(format!("*{}{}", mut_text, inner_type));
                }
            }
            None
        }
        _ => None,
    }
}

// Helper function to check if a type is a primitive Rust type
fn is_primitive_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "i8" | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "f32"
            | "f64"
            | "bool"
            | "char"
            | "str"
            | "String"
    )
}
