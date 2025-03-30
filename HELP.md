# Command-Line Help for `forest`

This document contains the help content for the `forest` command-line program.

**Command Overview:**

* [`forest`↴](#forest)

## `forest`

Explore and summarise Rust projects

**Usage:** `forest [OPTIONS] <project_dir>`

Copyright (c) 2025 Nicholas D. Crosbie

###### **Arguments:**

* `<PROJECT_DIR>` — The directory containing the Rust project to analyse

###### **Options:**

* `--output <FILE>` — Write results to the specified file instead of stdout
* `--format <FORMAT>` — Output format (json, csv, or text)

  Default value: `text`

  Possible values: `json`, `csv`, `text`

* `-s`, `--sort` — Sort variable names alphabetically
* `--tree` — Generate a tree-like representation of the project's structure
* `--markdown-help` — Generate a markdown version of the help text

<hr/>

<small><i>
    This document was generated automatically by
    <a href="https://crates.io/crates/clap-markdown"><code>clap-markdown</code></a>.
</i></small>
