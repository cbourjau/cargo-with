cargo-with
==========
`cargo-with` is a cargo-subcommand making it easy to run the build artifacts produced by `cargo run`, `cargo build` or `cargo bench` through other tools such as `gdb`, `strace`, `valgrind`, `rr`, or whatever else you may come up with.

[![Build Status](https://travis-ci.org/cbourjau/cargo-with.svg)](https://travis-ci.org/cbourjau/cargo-with)
[![crates.io](https://img.shields.io/crates/v/cargo-with.svg)](https://crates.io/crates/cargo-with)


Installation
-----------
Install with the usual `cargo install` magic:
```shell
cargo install cargo-with
```
Usage
-----
The core idea of `cargo-with` is to fit well into your development workflow using `cargo <subcommand>`.
All you have to do is add `with <some-command> -- ` in front of your usual `cargo` commands. `cargo-with` will then try it's best to identify the created artifact and run it with your command.

E.g. in order to run your binary through `gdb` do:

```shell
cargo with gdb -- run
```

This will firstly build the binary using `cargo build`, and then run `gdb {bin} {args}`, where `{bin}` is the path to the produced artifact and `{args}` is the arguments provided to cargo after the last `--` (in this case none).


### Moving arguments around

Instead of implicitly appending the artifact path and arguments to the provided command, you could also use placeholders to tell `cargo-with` where to place them. This can be done by using `{bin}` and `{args}` in the provided command.

```
cargo with "echo {args} {bin}" -- run -- --argument1 --argument2
```

I the above command, `{bin}` will be replaced by the path to the built artifact while `{args}` will be replaced by `--argument1 --argument2`.

### Disambiguating multiple binaries

There are often mulitiple candiate artifacts when cargo builds your project, especially when building tests. Therefore `cargo-with` may in some situations need more information to select your preferred candidate. This is done via explicitly specificing to cargo which artifact to build through the use of `--bin <name-of-binary>`, `--example <name-of-example>`, `--lib <name-of-lib>`* or `--test <name-of-unit-test>`*.

```
cargo with "gdb --args {bin} {args}" -- test --bin my-app
cargo with "gdb --args {bin} {args}" -- test --lib my-library
cargo with "gdb --args {bin} {args}" -- test --test my-unit-test
cargo with "gdb --args {bin} {args}" -- test --example my-example
```

*Only avaliable when using `cargo test`

### Examining tests

Cargo will run tests in parallel, hence running `cargo with gdb -- test --lib my-library` is probably not what you want. You can examine a single test by giving the name of the test function to cargo; `cargo with gdb -- test --lib my-library my_test_function_name`.

Note about `cargo run`
----------------------
In the case of `cargo run` `cargo-with` does replace `run` with `build` implicitly in order to avoid execution of
the binary after compilation.

Future of this crate
--------------------
There are currently [open issues](https://github.com/rust-lang/cargo/issues/3670) upstream in cargo which might make this crate redundant in the future.

Contributors
------------
This crate would not be what it is today without the many contributions by [@barskern](https://github.com/barskern)!
