#!/bin/bash

# WALLET_WITH_FUNDS=~/.config/solana/mango-devnet.json
# PROGRAM_ID=zF2vSz6V9g1YHGmfrzsY497NJzbRr84QUrPry4bLQ25

anchor build -- --features enable-gpl
./idl-fixup.sh
RUST_BACKTRACE=full anchor test --skip-build
