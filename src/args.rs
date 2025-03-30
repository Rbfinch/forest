// Copyright (c) 2025 Nicholas D. Crosbie
use clap::{Arg, ArgAction, Command};

pub struct Args {
    pub project_dir: String,
    pub output_file: Option<String>,
    pub format: String,
    pub sort: bool,
    pub tree: bool,
    pub markdown_help: bool,
}

// Add this new function that returns the Command definition
pub fn command() -> Command {
    Command::new("forest")
        .about("Explore and summarise Rust projects")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .after_help("Copyright (c) 2025 Nicholas D. Crosbie")
        .arg(
            Arg::new("project_dir")
                .help("The directory containing the Rust project to analyse")
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
        .arg(
            Arg::new("tree")
                .long("tree")
                .help("Generate a tree-like representation of the project's structure")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("markdown_help")
                .long("markdown-help")
                .help("Generate a markdown version of the help text")
                .action(ArgAction::SetTrue),
        )
}

pub fn parse_args() -> Args {
    let matches = command().get_matches();

    Args {
        project_dir: matches.get_one::<String>("project_dir").unwrap().clone(),
        output_file: matches.get_one::<String>("output").cloned(),
        format: matches.get_one::<String>("format").unwrap().clone(),
        sort: matches.get_flag("sort"),
        tree: matches.get_flag("tree"),
        markdown_help: matches.get_flag("markdown_help"),
    }
}
