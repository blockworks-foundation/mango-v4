_work in progress_

## Development

See DEVELOPING.md

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
- primary mango group on mainnet-beta: 78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX

### Release

Here are steps followed while performing a program deployment to mainnet-beta

- review diff of last deployed tag to mainnet-beta, e.g. https://github.com/blockworks-foundation/mango-v4/compare/program-v0.4.0..dev, pay special attention to account layout changes, backward compatibility of newly introduced account fields, etc.
- deploy to mainnet-beta
- update changelog with deploy timestamp and tx
- add a git tag e.g. `program-v0.0.1`, should match the version the program has
- reset `main` to currently deployed tag
- notify other contributors for bringing in changes from new release by merging `main` into their branch, e.g. `ts-client` and `deploy-mm`
- notify other contributors for appropriately handling offchain services e.g. scrapers, market makers, etc.
- bump program version in `Cargo.toml` on dev branch for next release

Here are steps followed while performing a npm package release
note: the UI currently uses code directly from github, pointing to the ts-client branch

- use `yarn publish` to release a new package, ensure compatibility with program release to mainnet-beta
- fix the tag auto added by yarn to match our internal convention, see script `fix-npm-tag.sh`, tags should look like this e.g.`npm-v0.0.1`, note: the npm package version/tag should not necessarily match the latest program deployment
