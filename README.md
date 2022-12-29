## Development

### Dependencies

- rust version 1.65.0
- solana-cli 1.14.9
- npm 8.1.2
- node v16.13.1

### Submodules

After cloning this repo you'll need to init and update its git submodules.
Consider setting the git option `submodule.recurse=true`.

### Deployments

- devnet: 4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg
- mainnet-beta: 4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg

### Notes

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
