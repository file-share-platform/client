#!/bin/bash

VERSION=v1.0.0-rc

# store current directory
DIR="$(pwd)"

# create and move into a temporary directory
mkdir -p /tmp/riptide-install-dir
cd /tmp/riptide-install-dir

# download latest release from repo
curl -L https://github.com/riptide-org/client/releases/download/$VERSION/agent --output agent
curl -L https://github.com/riptide-org/client/releases/download/$VERSION/cli --output cli
curl -L https://github.com/riptide-org/client/releases/download/$VERSION/riptide.service --output cli

# download the completion scripts for bash and zsh
curl -L https://github.com/riptide-org/client/releases/download/$VERSION/riptide.bash --output riptide.bash
curl -L https://github.com/riptide-org/client/releases/download/$VERSION/riptide.zsh --output riptide.zsh

# make folders for systemd
mkdir -p $HOME/.config/systemd/user
mkdir -p $HOME/.config/riptide

# move the service file to the user systemd directory
mv riptide.service $HOME/.config/systemd/user/riptide.service

# replace any $USER in the service file with the current user
sed -i "s/\$USER/$USER/g" $HOME/.config/systemd/user/riptide.service

# replace any $HOME in the service file with the current user's home directory
sed -i "s/\$HOME/$HOME/g" $HOME/.config/systemd/user/riptide.service

# move binary to the user installation (no sudo required)
mv cli $HOME/.local/bin/riptide
mv agent $HOME/.local/bin/riptide-agent

# make executable
chmod +x $HOME/.local/bin/riptide
chmod +x $HOME/.local/bin/riptide-agent

# start the service
systemctl start --user riptide.service
systemctl enable --user riptide.service

# if the user has .oh-my-zsh installed, install the completion script
if [ -d ~/.oh-my-zsh ]; then
    mkdir -p ~/.oh-my-zsh/completions/
    mv riptide.zsh ~/.oh-my-zsh/completions/_riptide
fi

# install the completion script for bash
mkdir -p ~/.bash_completion.d/
mv riptide.bash ~/.bash_completion.d/riptide

# move back to original directory
cd "$DIR"

# remove temporary directory
rm -rf /tmp/riptide-install-dir

echo "riptide installed successfully, try running riptide --help"
