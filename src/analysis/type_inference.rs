// Copyright (c) 2025 Nicholas D. Crosbie
use syn::Type;

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
    match ty {
        Type::Path(path) => {
            if let Some(segment) = path.path.segments.last() {
                let type_name = segment.ident.to_string();
                match type_name.as_str() {
                    "Option" => {
                        if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                            if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                                return format!("Option<{}>", extract_basic_type(inner_ty));
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
                                return format!("Vec<{}>", extract_basic_type(inner_ty));
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
                    _ => type_name,
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
            format!("&{}{}", mutability, extract_basic_type(&ref_type.elem))
        }
        Type::Array(array_type) => {
            format!("[{}; N]", extract_basic_type(&array_type.elem))
        }
        Type::Tuple(tuple_type) => {
            let types: Vec<String> = tuple_type.elems.iter().map(extract_basic_type).collect();
            format!("({})", types.join(", "))
        }
        _ => "Unknown".to_string(),
    }
}
