#!/bin/bash

# WALLET_WITH_FUNDS=~/.config/solana/mango-devnet.json
# PROGRAM_ID=m43thNJ58XCjL798ZSq6JGAG1BnWskhdq5or6kcnfsD

cargo run -p anchor-cli -- build
./idl-fixup.sh
RUST_BACKTRACE=full cargo run -p anchor-cli -- test --skip-build