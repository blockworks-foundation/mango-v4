#!/usr/bin/env bash

set -ex pipefail

WALLET_WITH_FUNDS=~/.config/solana/mango-devnet.json
PROGRAM_ID=4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg

# build program, 
cargo run -p anchor-cli -- build

# patch types, which we want in rust, but anchor client doesn't support
./idl-fixup.sh

# update types in ts client package
cp -v ./target/types/mango_v4.ts ./ts/client/src/mango_v4.ts

(cd ./ts/client && yarn tsc)

# publish program
solana --url https://mango.devnet.rpcpool.com program deploy --program-id $PROGRAM_ID  \
    -k $WALLET_WITH_FUNDS target/deploy/mango_v4.so --skip-fee-check

# # publish idl
cargo run -p anchor-cli -- idl upgrade --provider.cluster https://mango.devnet.rpcpool.com --provider.wallet $WALLET_WITH_FUNDS \
    --filepath target/idl/mango_v4_no_docs.json $PROGRAM_ID


# build npm package
(cd ./ts/client && tsc)
