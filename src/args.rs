use clap::{Arg, ArgAction, Command};

pub struct Args {
    pub project_dir: String,
    pub output_file: Option<String>,
    pub format: String,
    pub sort: bool,
}

pub fn parse_args() -> Args {
    let matches = Command::new("forest")
        .about("Analyzes Rust code for variable usage patterns")
        .arg(
            Arg::new("project_dir")
                .help("The directory containing the Rust project to analyze")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .help("Write results to the specified file instead of stdout")
                .value_name("FILE"),
        )
        .arg(
            Arg::new("format")
                .long("format")
                .help("Output format (json, csv, or text)")
                .value_name("FORMAT")
                .value_parser(["json", "csv", "text"])
                .default_value("text"),
        )
        .arg(
            Arg::new("sort")
                .short('s')
                .long("sort")
                .help("Sort variable names alphabetically")
                .action(ArgAction::SetTrue),
        )
        .get_matches();

    Args {
        project_dir: matches.get_one::<String>("project_dir").unwrap().clone(),
        output_file: matches.get_one::<String>("output").cloned(),
        format: matches.get_one::<String>("format").unwrap().clone(),
        sort: matches.get_flag("sort"),
    }
}
