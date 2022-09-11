#!/bin/sh

./build.sh

echo ">> Deploying contract..."
near deploy --wasmFile ./res/loozr_creator_token.wasm --accountId $1  --initFunction "new_default_meta" --initArgs "{\"owner_id\": \"$1\"}"