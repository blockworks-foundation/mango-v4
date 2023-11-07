_work in progress_

## License

See the LICENSE file.

The majority of this repo is MIT licensed, but some parts needed for compiling
the solana program are under GPL.

All GPL code is gated behind the `enable-gpl` feature. If you use the `mango-v4`
crate as a dependency with the `client` or `cpi` features, you use only MIT
parts of it.

The intention is for you to be able to depend on the `mango-v4` crate for
building closed-source tools and integrations, including other solana programs
that call into the mango program.

But deriving a solana program with similar functionality to the mango program
from this codebase would require the changes and improvements to stay publicly
available under GPL.

## Development

See DEVELOPING.md and FAQ-DEV.md

### Dependencies

- rust version 1.69.0
- solana-cli 1.16.7
- anchor-cli 0.28.0
- npm 8.1.2
- node v16.13.1

### Deployments

- devnet: 4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg
- mainnet-beta: 4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg
- primary mango group on mainnet-beta: 78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX

### Release

For program deployment, see RELEASING.md.

Here are steps followed while performing a npm package release
note: the UI currently uses code directly from github, pointing to the ts-client branch

- use `yarn publish` to release a new package, ensure compatibility with program release to mainnet-beta
- fix the tag auto added by yarn to match our internal convention, see script `fix-npm-tag.sh`, tags should look like this e.g.`npm-v0.0.1`, note: the npm package version/tag should not necessarily match the latest program deployment
