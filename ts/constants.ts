import { PublicKey } from '@solana/web3.js';

export const DEVNET_GROUP = 'EjDBeQkKQ1y68ki4YskWjSZc4v5hJ44KWH64uvMNubsg';

export const DEVNET_MINTS = new Map([
  ['USDC', '8FRFC6MoGGkMFQwngccyu69VnYbzykGeez7ignHVAFSN'],
  ['BTC', '3UNBZ6o52WTWwjac2kPUb4FyodhU1vFkRJheu1Sh2TvU'],
]);

export const DEVNET_MINTS_REVERSE = Array.from(DEVNET_MINTS.entries()).reduce(
  function (map, obj) {
    map[obj[1]] = obj[0];
    return map;
  },
  {},
);

export const DEVNET_ORACLES = new Map([
  ['BTC', 'HovQMDrbAgAYPCmHVSrezcSmkMtXSSUsLDFANExrZh2J'],
]);

export const DEVNET_SERUM3_MARKETS = new Map([
  ['BTC/USDC', 'DW83EpHFywBxCHmyARxwj3nzxJd7MUdSeznmrdzZKNZB'],
]);

export const DEVNET_SERUM3_MARKETS_REVERSE = Array.from(
  DEVNET_SERUM3_MARKETS.entries(),
).reduce(function (map, obj) {
  map[obj[1]] = obj[0];
  return map;
}, {});

export const DEVNET_SERUM3_PROGRAM_ID = new PublicKey(
  'DESVgJVGajEgKGXhb6XmqDHGz3VjdgP7rEVESBgxmroY',
);
