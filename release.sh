#!/usr/bin/env bash

set -euo pipefail

# build program, TODO try removing --skip-lint
anchor build --skip-lint

# update types in ts client package
cp -v ./target/types/mango_v4.ts ./ts/mango_v4.ts
yarn clean && yarn build && cp package.json ./dist/

# if [[ -z "${PROVIDER_WALLET}" ]]; then
#   echo "Please provide path to a provider wallet keypair."
#   exit -1
# fi

# if [[ -z "${VERSION_MANUALLY_BUMPED}" ]]; then
#   echo "Please bump versions in package.json and in cargo.toml."
#   exit -1
# fi

# # update on chain program and IDL, atm used for testing/developing
# anchor deploy --provider.cluster devnet --provider.wallet ${PROVIDER_WALLET}
# anchor idl upgrade --provider.cluster devnet --provider.wallet ${PROVIDER_WALLET}\
#  --filepath target/idl/voter_stake_registry.json m43thNJ58XCjL798ZSq6JGAG1BnWskhdq5or6kcnfsD


# # publish the npm package
# yarn publish dist

# echo
# echo Remember to commit and push the version update as well as the changes
# echo to ts/mango_v4.tx.
# echo
