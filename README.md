# zkSync zksolc Compiler Version Manager

This is a fork of the awesome work done in https://github.com/alloy-rs/svm-rs/. 

This repo provides a cross-platform support for managing zkSync's zksolc compiler versions.

## Install

```sh
cargo install --locked --git https://github.com/dutterbutter/zsvm-rs
```

## Usage

```sh
zksolc version manager

Usage: zksvm <COMMAND>

Commands:
  help     Print this message or the help of the given subcommand(s)
  install  Install zksolc versions
  list     List all zksolc versions
  remove   Remove a zksolc version, or "all" to remove all versions
  use      Set a zksolc version as the global default

Options:
  -h, --help     Print help
  -V, --version  Print version
```
