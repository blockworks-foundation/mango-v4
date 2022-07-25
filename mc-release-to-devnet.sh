#!/usr/bin/env bash

set -e pipefail

ANCHOR_BRANCH=v0.25.0-mangov4
ANCHOR_FORK=$(cd ../anchor && git rev-parse --abbrev-ref HEAD)
if [ "$ANCHOR_FORK" != "$ANCHOR_BRANCH" ]; then
  echo "Check out anchor fork at git@github.com:blockworks-foundation/anchor.git, and switch to branch $ANCHOR_BRANCH!"
  exit 1;
fi

# rg m43thNJ58XCjL798ZSq6JGAG1BnWskhdq5or6kcnfsD -l | xargs -I % sed -i '' 's/m43thNJ58XCjL798ZSq6JGAG1BnWskhdq5or6kcnfsD/5V2zCYCQkm4sZc3WctiwQEAzvfAiFxyjbwCvzQnmtmkM/g' %;

WALLET_WITH_FUNDS=~/.config/solana/mango-devnet.json
PROGRAM_ID=5V2zCYCQkm4sZc3WctiwQEAzvfAiFxyjbwCvzQnmtmkM

# TODO fix need for --skip-lint
# build program, 
cargo run --manifest-path ../anchor/cli/Cargo.toml build --skip-lint

# patch types, which we want in rust, but anchor client doesn't support
./idl-fixup.sh

# update types in ts client package
cp -v ./target/types/mango_v4.ts ./ts/client/src/mango_v4.ts

if [[ -z "${NO_DEPLOY}" ]]; then
    # publish program
    solana --url https://mango.devnet.rpcpool.com program deploy --program-id 5V2zCYCQkm4sZc3WctiwQEAzvfAiFxyjbwCvzQnmtmkM -k ~/.config/solana/mango-devnet.json target/deploy/mango_v4.so

    # publish idl
    cargo run --manifest-path ../anchor/cli/Cargo.toml idl upgrade --provider.cluster https://mango.devnet.rpcpool.com --provider.wallet $WALLET_WITH_FUNDS --filepath target/idl/mango_v4.json $PROGRAM_ID
else
    echo "Skipping deployment..."
fi


# build npm package
(cd ./ts/client && tsc)
