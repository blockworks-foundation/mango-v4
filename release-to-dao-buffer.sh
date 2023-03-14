#!/usr/bin/env bash

set -ex pipefail

WALLET_WITH_FUNDS=~/.config/solana/mango-mainnet-1.json
PROGRAM_ID=4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg

# build program
cargo run -p anchor-cli -- \
    build --verifiable --solana-version 1.14.13 -- \
    --features enable-gpl

# publish the buffer
solana --url $MB_CLUSTER_URL -k $WALLET_WITH_FUNDS \
    program write-buffer \
    --buffer-authority FP4PxqHTVzeG2c6eZd7974F9WvKUSdBeduUK3rjYyvBw \
    target/verifiable/mango_v4.so
