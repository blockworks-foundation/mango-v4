### Set environment variables

```
CLUSTER=devnet
CLUSTER_URL=https://mango.devnet.rpcpool.com/<token>
PAYER_KEYPAIR=~/.config/solana/mb-liqtest.json
# Adjust this to a free group
GROUP_NUM=200
```

### Get devnet SOL

The scripts need a lot of SOL for mint, market, group and account creation.
There's ample available, best to ask around.

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

Since devnet doesn't have any jupiter, run with

```
JUPITER_VERSION=mock
TCS_MODE=borrow-buy
REBALANCE=false
```

### Settle and close all open mango accounts

At any point, to reset by closing all accounts:

```
yarn ts-node ts/client/scripts/liqtest/liqtest-settle-and-close-all.ts
```
