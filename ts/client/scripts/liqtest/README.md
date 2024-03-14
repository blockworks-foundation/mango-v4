### Set environment variables

```
CLUSTER=devnet
CLUSTER_URL=https://mango.devnet.rpcpool.com/<token>
PAYER_KEYPAIR=~/.config/solana/mb-liqtest.json
# Adjust this to a free group
GROUP_NUM=200
```

### Create tokens and markets

This is one-time setup:
```
yarn ts-node ts/client/scripts/liqtest/liqtest-create-tokens-and-markets.ts
```
It'll emit some MINTS=... and SERUM_MARKETS=.. env vars, set those, all further
commands will use them.

### Make a group

```
yarn ts-node ts/client/scripts/liqtest/liqtest-create-group.ts
```

Groups can be reused a lot, but sometimes closing them may be necessary
```
yarn ts-node ts/client/scripts/liqtest/liqtest-close-group.ts
```

Preferably close all mango accounts first.

### Create candidate mango accounts

```
yarn ts-node ts/client/scripts/liqtest/liqtest-make-candidates.ts
```

This creates a bunch of to-be-liquidated accounts as well as a LIQOR account.

### Liquidate

Run the liquidator on the group with the liqor account.

### Settle and close all open mango accounts

At any point, to reset by closing all accounts:
```
yarn ts-node ts/client/scripts/liqtest/liqtest-settle-and-close-all.ts
```
