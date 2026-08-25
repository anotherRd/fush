#!/bin/sh

set -e

# build
echo "INSTALLING"
cargo build --release

# copy the binary file
sudo mkdir -p "/usr/local/bin/"
sudo cp target/release/fush "/usr/local/bin/fush"
sudo chmod 755 "/usr/local/bin/fush"
echo "Successfully installed to /usr/local/bin/fush"