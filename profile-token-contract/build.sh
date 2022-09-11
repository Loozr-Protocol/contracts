#!/bin/sh

echo ">> Building contract"

rustup target add wasm32-unknown-unknown
cargo build --all --target wasm32-unknown-unknown --release

echo ">> Copying .wasm file to 'res' directory..."
mkdir -p res
cp target/wasm32-unknown-unknown/release/loozr_creator_token.wasm ./res/