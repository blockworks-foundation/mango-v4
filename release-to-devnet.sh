#!/usr/bin/env bash

set -ex pipefail

WALLET_WITH_FUNDS=~/.config/solana/mango-mainnet-1.json
PROGRAM_ID=zF2vSz6V9g1YHGmfrzsY497NJzbRr84QUrPry4bLQ25

# build program, 
anchor build -- --features enable-gpl

# patch types, which we want in rust, but anchor client doesn't support
./idl-fixup.sh

# update types in ts client package
cp -v ./target/types/mango_v4.ts ./ts/client/src/mango_v4.ts

(cd ./ts/client && yarn tsc)

# publish program
solana --url $CLUSTER_URL program deploy --program-id $PROGRAM_ID  \
    -k $WALLET_WITH_FUNDS target/deploy/mango_v4.so --skip-fee-check

# publish idl
anchor idl upgrade --provider.cluster $CLUSTER_URL --provider.wallet $WALLET_WITH_FUNDS \
    --filepath target/idl/mango_v4_no_docs.json $PROGRAM_ID
