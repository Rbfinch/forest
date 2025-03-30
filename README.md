# forest

_Explore and summarise Rust projects_

[![Crates.io](https://img.shields.io/crates/v/forest.svg)](https://crates.io/crates/forest)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

Having trouble seeing the forest for the trees? **forest** analyses Rust projects to summarise variable mutability and data structure usage. It provides insights about where variables and data structures are declared, used, and what their types are.

Multiple output formats are supported, however the JSON output is the most convenient to work with as it can be easily manipulated with a tool like **jq** or **nushell**, for example:

```nushell
open out.json | get mutable_variables | table --expand
```

## Installation

`cargo install forest`

## Usage

See [HELP](https://github.com/Rbfinch/forest/blob/main/HELP.md)

## Example output

See [out.json](https://github.com/Rbfinch/forest/blob/main/out.json)

## Update changes

see [CHANGELOG](https://github.com/Rbfinch/forest/blob/main/CHANGELOG.md)

## Contributing

see [CONTRIBUTING](https://github.com/Rbfinch/forest/blob/main/CONTRIBUTING.md)

## License

MIT
