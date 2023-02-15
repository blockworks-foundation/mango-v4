#!/bin/bash

# WALLET_WITH_FUNDS=~/.config/solana/mango-devnet.json
# PROGRAM_ID=4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg

cargo run -p anchor-cli -- build -- --features enable-gpl
./idl-fixup.sh
RUST_BACKTRACE=full cargo run -p anchor-cli -- test --skip-build
