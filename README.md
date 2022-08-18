### Development

- rust version 1.59.0 (9d1b2106e 2022-02-23)
- solana-cli 1.9.13
- anchor-cli 0.24.2
- npm 8.1.2
- node v16.13.1

Devnet deployment - m43thNJ58XCjL798ZSq6JGAG1BnWskhdq5or6kcnfsD

For testing latest program changes while developing,
just run below scripts in given order form any branch,
these set of scripts should more or less always work,
bump up GROUP_NUM if you unsure if previous GROUP_NUM has not been cleanly closed or setup with older program code

```
./release-to-devnet.sh
GROUP_NUM=4 yarn ts-node ts/client/src/scripts/devnet-admin.ts
GROUP_NUM=4 yarn ts-node ts/client/src/scripts/devnet-user.ts
GROUP_NUM=4 yarn ts-node ts/client/src/scripts/devnet-user-close-account.ts
GROUP_NUM=4 yarn ts-node ts/client/src/scripts/devnet-admin-close.ts
```
