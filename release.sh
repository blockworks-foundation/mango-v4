#!/usr/bin/env bash

set -euo pipefail

if [[ -z "${PROVIDER_WALLET}" ]]; then
  echo "Please provide path to a provider wallet keypair."
  exit -1
fi

if [[ -z "${VERSION_MANUALLY_BUMPED}" ]]; then
  echo "Please bump versions in package.json and in cargo.toml."
  exit -1
fi

# build program
anchor build

# update on chain program and IDL, atm used for testing/developing
anchor deploy --provider.cluster devnet --provider.wallet ${PROVIDER_WALLET}
anchor idl upgrade --provider.cluster devnet --provider.wallet ${PROVIDER_WALLET}\
 --filepath target/idl/voter_stake_registry.json Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS

# update types in npm package and publish the npm package
cp cp ./target/types/mango_v4.ts ./src/mango_v4.ts
yarn clean && yarn build && cp package.json ./dist/
# yarn publish dist # TODO: should this package replace mango-v3-client?

echo
echo Remember to commit and push the version update as well as the changes
echo to src/mango_v4.tx.
echo
