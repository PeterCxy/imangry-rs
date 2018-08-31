#!/bin/bash

rm -rf out
cargo clean --package=backend
cargo build --package=backend --release
mkdir out
cp ./target/release/backend out/
cp -R static out/