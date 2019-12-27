#!/bin/bash
# FIXME: rustup is the most convenient way to install rust 
# but it doesn't check signatures of downloaded packages :/
# https://github.com/rust-lang-nursery/rustup.rs#security

# must run as regular user, not root.
if [[ "$EUID" -eq 0 ]]
  then echo "Please don't run as root"
  exit
fi

RUST_VER=${RUST_VER:-nightly-2019-12-05}
if hash rustup 2>/dev/null; then # rustup is installed
  rustup toolchain install $RUST_VER
  rustup default $RUST_VER
else # rustup is not installed  
  curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain $RUST_VER
fi 

# make rust environment available on next login 
if ! grep "source ~/.cargo/env" ~/.bashrc >/dev/null; then 
  echo "source ~/.cargo/env" >> ~/.bashrc
fi
# make rust environment available for commands below 
source ~/.cargo/env

# required for c2rust-refactor tests
# rustup run $RUST_VER cargo install --force rustfmt
rustup component add rustfmt-preview

# Make rustup directory world-writable so other test users can install new rust
# versions
chmod -R a+w ~/.rustup
