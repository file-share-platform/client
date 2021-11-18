<!-- TODO: Add badges here -->
# Riptide Client
# Overview
This repo contains client side applications for the riptide file transfer application. The clientside is made up of two main components the Agent and the Cli discussed below.

## Agent

<!-- TODO -->

## Cli

<!-- TODO -->

# Installation and Use

## Fedora 34/35
<!-- TODO -->
**Not Yet Completed**

## Ubuntu 20.04
<!-- TODO -->
**Not Yet Completed**

## OSX (MacOS)
<!-- TODO -->
**Not Yet Completed**

# Development

## Fedora

### Dependencies
```sh
    # Install Deps
    sudo dnf groupinstall @development-tools @development-libraries
    sudo dnf install libsqlite3x-devel

    # Install Rust
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

    # Install Docker
    # Follow here: https://docs.docker.com/engine/install/fedora/

    # Install Dependencies
    cargo install diesel_cli --no-default-features --features sqlite
    cargo install rustfmt #for automatic code formatting
    cargo install clippy #collection of useful lints
    cargo install cargo-tarpaulin #test coverage
```

### Development
```sh
# Spawn a postgres backing db
docker run --name fsp-data -p 5432:5432 -e POSTGRES_PASSWORD=postgres -d postgres -N 500

# Test the cli
cargo test -- --test-threads 4
```

# Contributing

## Code Guidelines
**Please write tests** if we have good test coverage we can avoid any bugs down the line.

Outside of this we use standard Rust formatting for code. This will be enforced through use of [clippy](https://github.com/rust-lang/rust-clippy) and [rustfmt](https://github.com/rust-lang/rustfmt).

## Commit Guidelines
In all commits, please try to follow the [convention for commits](https://www.conventionalcommits.org/en/v1.0.0/#specification).

Ideally aim to push every commit you make, rather than accumulating a large number of commits before pushing, this helps to keep everyone on the same
codebase when collaborating.

The exception for this is that you should not commit non-compiling code to the main branch. Open a new branch and
commit to that instead.

## Use of Pull Requests
Outside of exceptional cases, please always push commits to a new branch and then generate a pull request with your new feature. Automated actions will attempt to validate that your code does not break anything existing, but this doesn't guarantee your code works. Please write tests!