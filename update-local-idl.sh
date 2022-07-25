#!/usr/bin/env bash

set -e pipefail

ANCHOR_BRANCH=v0.25.0-mangov4
ANCHOR_FORK=$(cd ../anchor && git rev-parse --abbrev-ref HEAD)
if [ "$ANCHOR_FORK" != "$ANCHOR_BRANCH" ]; then
  echo "Check out anchor fork at git@github.com:blockworks-foundation/anchor.git, and switch to branch $ANCHOR_BRANCH!"
  exit 1;
fi

# TODO fix need for --skip-lint
# build program, 
cargo run --manifest-path ../anchor/cli/Cargo.toml build --skip-lint

# patch types, which we want in rust, but anchor client doesn't support
./idl-fixup.sh

# update types in ts client package
cp -v ./target/types/mango_v4.ts ./ts/client/src/mango_v4.ts

(cd ./ts/client && yarn tsc)
