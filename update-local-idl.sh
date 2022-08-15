#!/usr/bin/env bash

set -e pipefail

# build program, 
cargo run -p anchor-cli -- build

# patch types, which we want in rust, but anchor client doesn't support
./idl-fixup.sh

# update types in ts client package
cp -v ./target/types/mango_v4.ts ./ts/client/src/mango_v4.ts

(cd ./ts/client && yarn tsc)
