#!/bin/sh

set -e

# test
echo "RUNNING TEST"
cargo run -- test
rm -rf target

# build
echo "INSTALLING"
cargo build --release

# copy the binary file
sudo mkdir -p "/usr/local/bin/"
sudo cp target/release/fush "/usr/local/bin/fush"
sudo chmod 755 "/usr/local/bin/fush"
rm -rf target
echo "Successfully installed to /usr/local/bin/fush"