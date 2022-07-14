For deploying to mainnet,

if there are breaking changes

- close accounts for recovering rent sol + allow creating them with new layout in future with same PDA address
  ** close mango account `ts/client/src/scripts/mb-example1-close-account.ts`,
  note: for whatever reason if withdrawing tokens, or closing positions, etc. fails, just comment out everything else and just run the script with
  `await client.closeMangoAccount(group, mangoAccount);`
  ** close all things setup by admin `ts/client/src/scripts/mb-example1-admin-close.ts`

- merge dev to main, and then `release-to-mainnet.sh` to deploy latest code to mainnet
- setup group and banks, etc. using `ts/client/src/scripts/mb-example1-admin.ts`
- `ts/client/src/scripts/mb-example1-ids-json.ts`
