#!/usr/bin/env bash

set -e pipefail

# rg 4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg -l | xargs -I % sed -i '' 's/4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg/5V2zCYCQkm4sZc3WctiwQEAzvfAiFxyjbwCvzQnmtmkM/g' %;

WALLET_WITH_FUNDS=~/.config/solana/mango-devnet.json
PROGRAM_ID=5V2zCYCQkm4sZc3WctiwQEAzvfAiFxyjbwCvzQnmtmkM

# TODO fix need for --skip-lint
# build program, 
cargo run -p anchor-cli -- build --skip-lint

# patch types, which we want in rust, but anchor client doesn't support
./idl-fixup.sh

# update types in ts client package
cp -v ./target/types/mango_v4.ts ./ts/client/src/mango_v4.ts

if [[ -z "${NO_DEPLOY}" ]]; then
    # publish program
    solana --url https://mango.devnet.rpcpool.com program deploy --program-id 5V2zCYCQkm4sZc3WctiwQEAzvfAiFxyjbwCvzQnmtmkM -k ~/.config/solana/mango-devnet.json target/deploy/mango_v4.so

    # publish idl
    cargo run -p anchor-cli -- idl upgrade --provider.cluster https://mango.devnet.rpcpool.com --provider.wallet $WALLET_WITH_FUNDS --filepath target/idl/mango_v4.json $PROGRAM_ID
else
    echo "Skipping deployment..."
fi


# build npm package
(cd ./ts/client && tsc)
