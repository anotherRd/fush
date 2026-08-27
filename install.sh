#!/bin/sh

set -e

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# build
echo "INSTALLING"
cargo build --release

# copy the binary file
sudo cp -r target/release /opt/fush
sudo chmod +x /opt/fush/fush
sudo ln -sf "/opt/fush/fush" /usr/local/bin/fush
echo "Successfully installed to /opt/fush/fush"