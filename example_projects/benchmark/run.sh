#!/bin/bash
set -e

cargo +nightly with "echo {bin}" -- bench
