#!/bin/bash

# WALLET_WITH_FUNDS=~/.config/solana/mango-devnet.json
# PROGRAM_ID=4MTevvuuqC2sZtckP6RPeLZcqV3JyqoF3N5fswzc3NmT

anchor build -- --features enable-gpl
./idl-fixup.sh
RUST_BACKTRACE=full anchor test --skip-build
