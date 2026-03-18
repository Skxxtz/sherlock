#!/bin/bash
# Get current version name
read -p "Current version: " version

# Dir setup 
rm -rf ~/.tmp/sherlock-release/
mkdir -p ~/.tmp/sherlock-release/

# Build
cargo build --release --features wayland

# Strip and copy
strip target/release/sherlock-gpui
cp target/release/sherlock-gpui ~/.tmp/sherlock-release/sherlock
cp LICENSE ~/.tmp/sherlock-release/LICENSE

# Compress
cd ~/.tmp/sherlock-release/
tar -czf sherlock-v${version}-x86_64.tar.gz sherlock LICENSE

# Print checksum for PKGBUILD
echo ""
echo "sha256sum for PKGBUILD:"
sha256sum sherlock-v${version}-x86_64.tar.gz

echo ""
echo "Archive created at: ~/.tmp/sherlock-release/sherlock-v${version}-x86_64.tar.gz"
