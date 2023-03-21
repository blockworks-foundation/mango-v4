This repo has two main branches:

- `devnet`: bleeding edge, may be unstable, could be incompatible with deployed program
- `main`: stable, currently running on the `mainnet-beta` cluster

## Setup Environment

### .env Config file:

A `.env` file can be used to configure the liquidator setup. See `.env.example` for a example.

The environment variables required are

- `LIQOR_MANGO_ACCOUNT` - public key of the mango account
- `LIQOR_OWNER` - private key of the owner of the mango account
- `RPC_URL` - RPC cluster url
- `SERUM_PROGRAM` - the Openbook program Id the mango group is configured with e.g. primary mango group is configured with "srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX"

```shell
cargo run --bin liquidator
```

There is also a dockerfile `Dockerfile.liquidator` available in case one wants to run this in a containerized environment.
