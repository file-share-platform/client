# Riptide Client

![Test](https://github.com/riptide-org/client/actions/workflows/precommit.yml/badge.svg)
[![codecov](https://codecov.io/gh/riptide-org/client/branch/master/graph/badge.svg?token=T91ZY5HCE1)](https://codecov.io/gh/riptide-org/client)

## Overview

This repo contains client side applications for the riptide file transfer application. The client side is made up of two main components the Agent and the Cli. The Agent is a background process
that runs on the client machine and is responsible for managing the file transfer process. The Cli is a command line interface that is used to interact with the Agent, add and remove shares.

## Installation and Use

### Fedora 36
<!-- TODO -->

### Ubuntu 22.04
<!-- TODO -->

### OSX (MacOS)

Support for MacOS should be doable, but will require implementation of the macos systemctl equivalent for the agent.

### Windows 10/11

Support for windows 10/11 has not yet been implemented or tested, and will take significant work to setup.

## Development

### Fedora

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
    cargo install cargo-tarpaulin #test coverage

    # Install pre-commit hooks
    pre-commit install
```

## Contributing

All contributions are warmly welcomed, and will be licensed under MIT unless otherwise specified.
