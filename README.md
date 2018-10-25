cargo-with
==========
`cargo-with` is a cargo-subcommand making it easy to run the build artifacts produced by `cargo run` or `cargo build`
through other tools such as `gdb`, `strace`, `valgrind`, `rr`, or whatever else you may come up with.

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
The core idea of `cargo-with` is to fit well into your development workflow using `cargo run` and `cargo test`.
All you have to do is add `with <some_binary> -- ` in front of your usual `cargo` commands.
For example, in order to run your tests through `gdb` do:
```shell
cargo with gdb -- test
```

However, this would run all your tests in multiple threads, you probably want to filter on your tests.
More complicated calling signatures can be accommodated by using `{bin}` and `{args}` placeholders in the binary string:

```shell
cargo with "gdb --args {bin} {args}" -- tests -- the_name_of_my_test
```

If `{bin}` or `{args}` are not provided they are automatically appended to the end of the command.

Note about `cargo run`
----------------------
In the case of `cargo run` `cargo-with` does replace `run` with `build` implicitly in order to avoid execution of
the binary after compilation.

Future of this crate
--------------------
There are currently [open issues](https://github.com/rust-lang/cargo/issues/3670) upstream in cargo which might make this crate redundant in the future.
