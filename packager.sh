#!/bin/bash

# Get current version name
read -p "Current version: " version

# Dir setup 
rm -rf ~/.tmp/sherlock-release/
mkdir -p ~/.tmp/sherlock-release/

# Build
cargo build --release

# Strip and copy
strip target/release/sherlock
cp target/release/sherlock ~/.tmp/sherlock-release/sherlock
cp LICENSE ~/.tmp/sherlock-release/LICENSE

# Compress
cd ~/.tmp/sherlock-release/
tar -czf sherlock-v${version}-bin-linux-x86_64.tar.gz sherlock LICENSE

