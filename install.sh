#!/bin/bash

VERSION=v1.0.0-rc
# if VERSION is set in environment, use that
if [ -n "$VERSION" ]; then
  echo "Using VERSION from environment: $VERSION"
else
  VERSION=$(curl -s https://api.github.com/repos/riptide-org/client/releases/latest | grep tag_name | cut -d '"' -f 4)
fi

# store current directory
DIR="$(pwd)"

# create and move into a temporary directory
mkdir -p /tmp/riptide-install-dir
cd /tmp/riptide-install-dir

# check if dnf command exists
if command -v dnf &> /dev/null
then
    sudo dnf install -y curl openssl openssl-devel openssl1.1
    echo "dnf complete"
# check if apt command exists
elif command -v apt &> /dev/null
then
    sudo apt install -y curl openssl libssl-dev
    echo "apt complete"
else
    echo "Could not install packages no package manager found"
    exit 1
fi

# download latest release from repo
curl -sL https://github.com/riptide-org/client/releases/download/$VERSION/agent --output agent
curl -sL https://github.com/riptide-org/client/releases/download/$VERSION/cli --output cli
curl -sL https://github.com/riptide-org/client/releases/download/$VERSION/riptide.service --output riptide.service

# download the completion scripts for bash and zsh
curl -sL https://github.com/riptide-org/client/releases/download/$VERSION/riptide.bash --output riptide.bash
curl -sL https://github.com/riptide-org/client/releases/download/$VERSION/riptide.zsh --output riptide.zsh

# make folders for systemd
mkdir -p $HOME/.config/systemd/user
mkdir -p $HOME/.config/riptide
mkdir -p $HOME/.local/bin/

# move the service file to the user systemd directory
mv riptide.service $HOME/.config/systemd/user/riptide.service

# replace any $USER in the service file with the current user
sed -i "s/\$USER/$USER/g" $HOME/.config/systemd/user/riptide.service

# replace any $HOME in the service file with the current user's home directory
# making sure to escape the / in the path
NEW_HOME=$(echo $HOME | sed 's/\//\\\//g')
sed -i "s/\$HOME/$NEW_HOME/g" $HOME/.config/systemd/user/riptide.service

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
