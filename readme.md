# Riptide Client

![Test](https://github.com/riptide-org/client/actions/workflows/precommit.yml/badge.svg)
[![codecov](https://codecov.io/gh/riptide-org/client/branch/master/graph/badge.svg?token=T91ZY5HCE1)](https://codecov.io/gh/riptide-org/client)

## Overview

This repo contains client side applications for the riptide file transfer application. The client side is made up of two main components the Agent and the Cli. The Agent is a background process
that runs on the client machine and is responsible for managing the file transfer process. The Cli is a command line interface that is used to interact with the Agent, add and remove shares.

## Usage

```bash
> riptide --help

riptide 1.0.0
Fast and easy file sharing over the internet, through a simple cli.

USAGE:
    riptide [OPTIONS] [file]

ARGS:
    <file>    Name of the file to share

OPTIONS:
    -h, --help            Print help information
    -l, --list            List all currently shared files
    -r, --remove <ID>     Remove the file share indicated by this id by index or id
        --reset-config    Reset the config file to default
    -t, --time <HOURS>    Set how many hours to share the file for [default: 24]
    -V, --version         Print version information

```

## Installation

Installation can be done via a simple bash script.

```bash
curl https://raw.githubusercontent.com/riptide-org/client/main/install.sh | bash
```

## Development

### Fedora

```sh
    # Install Deps
    sudo dnf groupinstall @development-tools @development-libraries
    sudo dnf install libsqlite3x-devel libxcb-devel

    # Install Rust
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

    # Install Docker
    # Follow here: https://docs.docker.com/engine/install/fedora/

    # Install Dependencies
    cargo install diesel_cli --no-default-features --features sqlite
    cargo install cargo-tarpaulin #test coverage

    # Install pre-commit hooks
    pre-commit install
```

## Contributing

All contributions are warmly welcomed, and will be licensed under MIT unless otherwise specified.
