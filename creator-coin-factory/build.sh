#!/bin/sh

echo ">> Building contract"

rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release

echo ">> Copying .wasm file to 'res' directory..."
mkdir -p res
cp target/wasm32-unknown-unknown/release/creator_coin_factory.wasm ./res/