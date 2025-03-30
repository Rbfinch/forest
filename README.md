# forest - Explore a Rust Project

Having trouble seeing the forest for the trees?

This tool analyzes Rust projects to summarize variable mutability and data structure usage.

It provides insights about where variables and data structures are declared, used, and what their types are.

>[!NOTE]
**forest**'s JSON output is best viewed using a tool like **jq** or **nushell**, for example:

```nushell
open out.json | get mutable_variables | table --expand
```

# Command-Line Help for `forest`

This document contains the help content for the `forest` command-line program.

**Command Overview:**

* [`forest`↴](#forest)

## `forest`

Generate a summaries of Rust projects

**Usage:** `forest [OPTIONS] <project_dir>`

Copyright (c) 2025 Nicholas D. Crosbie

###### **Arguments:**

* `<PROJECT_DIR>` — The directory containing the Rust project to analyze

###### **Options:**

* `--output <FILE>` — Write results to the specified file instead of stdout
* `--format <FORMAT>` — Output format (json, csv, or text)

  Default value: `text`

  Possible values: `json`, `csv`, `text`

* `-s`, `--sort` — Sort variable names alphabetically
* `--tree` — Generate a tree-like representation of the project's structure
* `--markdown-help` — Generate a markdown version of the help text
