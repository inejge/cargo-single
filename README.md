# cargo-single

To write a relatively simple Rust program, which can fit in a single source
file but does need a couple of external dependencies, one must use Cargo to
create a project for the program. Cargo's defaults and tools like `cargo-edit`
help, but it's still some amount of ceremony and increased friction. This tool
lets one list the dependencies in the comments at the top of the source file,
and use that list and the file name to automatically generate the project
directory, which is then transparently used to check, build or run the program.

## Installation

You must have Rust and Cargo installed and working. Run:

```sh
cargo install cargo-single
```

See the [Cargo documentation](https://doc.rust-lang.org/cargo/) to learn how
`cargo install` works and how to set up your system to find the installed
binaries.

## Example

Create the source file for your program; as an example, save the following
as `random.rs`.

```rust
// rand = "0.7"

use rand::Rng;

fn main() {
    println!("{}", rand::thread_rng().gen_range(1, 11));
}
```

List the dependencies as comments at the top of the file. Each dependency line
must start with the string `// ` from the leftmost column, and continue in the
format used in the `[dependencies]` section of `Cargo.toml`. End the list of
dependencies with a blank line.

You can set the version of your program by including a pseudo-dependency named
__self__ in the list. The format of that dependency line is rigid: from the start
of the line, `// self = `, followed by the version string in double quotes,
followed by a newline without any intervening characters.

To build and execute the program, run:

```sh
cargo single run random.rs
```

## Usage

The tool is invoked through Cargo, with the syntax:

```sh
cargo single <command> [<option> ...] {<source-file>|<source-dir>} [<arguments>]
```

_Command_ is one of: __build__, __check__, __fmt__, __refresh__, or __run__. __Refresh__
will re-read the source file and update the dependencies in `Cargo.toml`, while
the remaining four are regular Cargo sub-commands which will be passed to Cargo.

_Options_ are a subset of options accepted by Cargo subcommands. The ones recognized by
`cargo-single` are:

* __+toolchain__: Name of a toolchain which will be used for building.

* __--release__: Build in release mode.

* __--target *target*__: Use the specified target for building.

* __--no-quiet__: Don't pass `--quiet` to Cargo.

Either the name of the source file, with the `.rs` extension, or of the project directory,
which has the same name without the extension, must be given to identify the program.

The remaining arguments, if any, will be passed to the program if it's executed.

## License

Licensed under either of:

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE)), or
 * MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
