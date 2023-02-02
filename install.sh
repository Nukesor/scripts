#!/usr/bin/env bash
# Some best practice settings for bash scripts
# set -e: automatically exits on any failing command
# set -u: exits if there are any unset variables
# set -o pipefail: automatically exits, if any command in a pipe fails (normally only the last is counted)
set -euo pipefail

cp hooks/* .git/hooks

# Get absolute path this script's directory
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

BIN_FOLDER="$HOME/.bin"
# Install all bash scripts
mkdir -p "$BIN_FOLDER"

echo "Deploying shell scripts"
for file in $DIR/shell/* ; do
    file_name=$(basename $file)
    if [ ! -L "$HOME/.bin/${file_name}" ]; then
        ln -s $file $BIN_FOLDER/$file_name
    fi
done

rustup update stable

# Rust scripts
echo "Installing rust scripts"
cargo install --path $DIR
