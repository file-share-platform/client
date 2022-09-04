# Riptide Client

![Test](https://github.com/riptide-org/client/actions/workflows/precommit.yml/badge.svg)
[![codecov](https://codecov.io/gh/riptide-org/client/branch/master/graph/badge.svg?token=T91ZY5HCE1)](https://codecov.io/gh/riptide-org/client)

## Overview

This repo contains client side applications for the riptide file transfer application. The client side is made up of two main components the Agent and the Cli. The Agent is a background process
that runs on the client machine and is responsible for managing the file transfer process. The Cli is a command line interface that is used to interact with the Agent, add and remove shares.

## Usage

```bash
riptide --help
```

## Installation

### Fedora 36

```bash
mkdir /opt/riptide

export VERSION=v1.0.0-rc

# collect executables for 1.0 release
wget https://github.com/riptide-org/client/releases/download/${VERSION}/agent -O /opt/riptide/agent
wget https://github.com/riptide-org/client/releases/download/${VERSION}/cli -O /usr/local/bin/riptide

# set permissoins for executables
chmod +x /opt/riptide/agent
chmod +x /usr/local/bin/riptide

# create user and group
groupadd riptide
useradd --system --shell /usr/sbin/nologin --home /opt/riptide -g riptide riptide

# create systemd service
wget https://github.com/riptide-org/client/releases/download/${VERSION}/riptide.service -O /etc/systemd/system/riptide.service
systemctl daemon-reload
systemctl enable riptide
systemctl start riptide

# OPTIONAL: install shell completion files
wget https://github.com/riptide-org/client/releases/download/${VERSION}/shell_completions.zip
unzip shell_completions.zip

#BASH
mv shell_completions/riptide.bash /etc/bash_completion.d/riptide

#ZSH
mv shell_completions/riptide /usr/share/zsh/site-functions/riptide

#FISH
mv shell_completions/riptide.fish /usr/share/fish/vendor_completions.d/riptide.fish
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
