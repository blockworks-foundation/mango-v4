For deploying to mainnet,

If there are no breaking changes, simply running `release-to-mainnet.sh` should suffice.

- close accounts for recovering rent sol + allow creating them with new layout in future with same PDA address

  - close mango account `ts/client/src/scripts/mb-example1-close-account.ts`,
    note: for whatever reason if withdrawing tokens, or closing positions, etc. fails, just comment out everything else and just run the script with
    `await client.closeMangoAccount(group, mangoAccount);`
  - close all things setup by admin `ts/client/src/scripts/mb-example1-admin-close.ts`
    note: the admin does get dust token balances, so no mainnet tokens are lost

- merge dev to main, and then `release-to-mainnet.sh` to deploy latest code to mainnet
- setup group and banks, etc. using `ts/client/src/scripts/mb-example1-admin.ts`
- update ids json `ts/client/src/scripts/mb-example1-ids-json.ts`
- create mango account and deposit some tokens `ts/client/src/scripts/mb-example1-ids-json.ts`

tldr;

```
yarn ts-node ts/client/src/scripts/mb-example1-close-account.ts
yarn ts-node ts/client/src/scripts/mb-example1-admin-close.ts
./release-to-mainnet.sh
yarn ts-node ts/client/src/scripts/mb-example1-admin.ts
yarn ts-node ts/client/src/scripts/mb-example1-ids-json.ts
yarn ts-node ts/client/src/scripts/mb-example1-ids-json.ts
```

TODO:
- consolidate devnet+mainnet-beta scripts into single scripts
- consolidate release scripts
