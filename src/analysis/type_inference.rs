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
            // Extract the last segment as the base type
            if let Some(segment) = path
                .path
                .segments
                .last()
                .map(|segment| segment.ident.to_string())
            {
                // Check for primitive types
                match segment.as_str() {
                    "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32"
                    | "u64" | "u128" | "usize" | "f32" | "f64" | "bool" | "char" => {
                        segment.to_string()
                    }
                    "String" => "String".to_string(),
                    "Option" => {
                        // Implementation for Option type
                        "Option".to_string()
                    }
                    _ => segment.to_string(),
                }
            } else {
                "unknown".to_string()
            }
        }
        _ => "unknown".to_string(),
    }
}
