#!/usr/bin/env bash

set -e pipefail

WALLET_WITH_FUNDS=~/.config/solana/mango-mainnet.json
PROGRAM_ID=5V2zCYCQkm4sZc3WctiwQEAzvfAiFxyjbwCvzQnmtmkM

# TODO fix need for --skip-lint
# build program, 
anchor build --skip-lint

# patch types, which we want in rust, but anchor client doesn't support
./idl-fixup.sh

# update types in ts client package
cp -v ./target/types/mango_v4.ts ./ts/client/src/mango_v4.ts

(cd ./ts/client && yarn tsc)

if [[ -z "${NO_DEPLOY}" ]]; then
    # publish program
    solana --url $CLUSTER_URL program deploy --program-id $PROGRAM_ID  \
        -k $WALLET_WITH_FUNDS target/deploy/mango_v4.so

    # publish idl
    anchor idl upgrade --provider.cluster $CLUSTER_URL --provider.wallet $WALLET_WITH_FUNDS \
        --filepath target/idl/mango_v4.json $PROGRAM_ID
else
    echo "Skipping deployment..."
fi


# build npm package
(cd ./ts/client && yarn tsc)
