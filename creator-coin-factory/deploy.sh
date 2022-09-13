#!/bin/sh

./build.sh

echo ">> Deploying contract..."
near deploy --wasmFile ./res/creator_coin_factory.wasm --accountId $1