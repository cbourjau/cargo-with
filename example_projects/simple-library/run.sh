#!/bin/bash
set -e

cargo with echo -- test

cargo with "echo {bin}" -- test

# Filter tests
cargo with "echo {bin}" -- test it_works
cargo with "echo {bin}" -- test it_works -- myargs

cargo with "echo {bin} {args}" -- test -- myargs

# Some examples which should fail
! cargo with "echo {bin}"
cargo with "echo {bin}" -- not-a-cargo-subcommand || echo "asdfsadfadf"
# This is not a binary and we should still fail if we try to run it
! cargo with echo -- run
