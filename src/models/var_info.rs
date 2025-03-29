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
}
