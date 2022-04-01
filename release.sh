#!/usr/bin/env bash

set -euo pipefail

WALLET_WITH_FUNDS=~/.config/solana/mango-devnet.json
PROGRAM_ID=m43thNJ58XCjL798ZSq6JGAG1BnWskhdq5or6kcnfsD

# TODO fix need for --skip-lint
# build program, 
anchor build --skip-lint

# patch types, which we want in rust, but anchor client doesn't support
./idl-fixup.sh

# update types in ts client package
cp -v ./target/types/mango_v4.ts ./ts/mango_v4.ts

# publish program
solana --url https://mango.devnet.rpcpool.com program deploy --program-id $PROGRAM_ID  \
    -k $WALLET_WITH_FUNDS target/deploy/mango_v4.so

# publish idl
anchor idl upgrade --provider.cluster https://mango.devnet.rpcpool.com --provider.wallet $WALLET_WITH_FUNDS \
    --filepath target/idl/mango_v4.json $PROGRAM_ID

# build npm package
# yarn clean && yarn build && cp package.json ./dist/

# publish the npm package
# yarn publish dist

# echo
# echo Remember to commit and push the version update as well as the changes
# echo to ts/mango_v4.tx.
# echo
