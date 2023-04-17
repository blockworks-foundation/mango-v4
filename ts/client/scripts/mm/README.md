This directory contains a sample market maker (`market-maker.ts`) in typescript, which can be run using ts-node.

The environment variables required are

- `MANGO_ACCOUNT_PK` - public key of the mango account
- `KEYPAIR` - private key of the owner of the mango account
- `MB_CLUSTER_URL` - RPC cluster url

Notes:
- Quoting is based off of kraken
- see default.json for quoting rules

Future:
- Hedging perp positions on mango-v4 spot
- Observing fills and reacting earlier
- Quoting off of binance