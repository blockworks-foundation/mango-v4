# Mango v4 release steps

- Setup and info

  - $KEY as a path to a keypair (needs around 20 SOL for the buffer)
  - $RPC_URL as a url to an RPC node
  - 4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg is the address of the Mango v4 Program
  - FP4PxqHTVzeG2c6eZd7974F9WvKUSdBeduUK3rjYyvBw is the address of the Mango v4 Program Governance

- Assuming there's a release branch (like release/program-v0.22.0)
  with a completed audit and an updated changelog.

- Check out the release branch

- Make sure the version is bumped in programs/mango-v4/Cargo.toml

- Update the idl ./update-local-idl.sh and verify that there's no difference

- Run the tests to double check there are no failures

- Tag (`git tag program-v0.xy.z HEAD`) and push it (`git push <tag>`)

- Do a verifiable build

  Set GITHUB_SHA and GITHUB_REF_NAME to the release sha1 and tag name.

  anchor build --verifiable --docker-image backpackapp/build:v0.28.0 --solana-version 1.16.14 --env GITHUB_SHA --env GITHUB_REF_NAME -- --features enable-gpl

  (or wait for github to finish and create the release)

- Get the checksum of the verifiable build binary

  sha256sum target/verifiable/mango_v4.so

  to compare it with the one from github.

- Create the program buffer

  solana -k $KEY -u $RPC_URL program write-buffer target/verifiable/mango_v4.so

  Save the returned address as $PROGRAM_BUFFER

- Set new buffer authority

  solana -k $KEY -u $RPC_URL program set-buffer-authority --new-buffer-authority FP4PxqHTVzeG2c6eZd7974F9WvKUSdBeduUK3rjYyvBw $PROGRAM_BUFFER

- Create IDL buffer

  anchor idl write-buffer --provider.cluster $RPC_URL --provider.wallet $KEY --filepath target/idl/mango_v4_no_docs.json 4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg

  Save the returned address as $IDL_BUFFER

- Set IDL buffer authority

  anchor idl set-authority --provider.cluster $RPC_URL --provider.wallet $KEY --program-id 4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg --new-authority FP4PxqHTVzeG2c6eZd7974F9WvKUSdBeduUK3rjYyvBw $IDL_BUFFER

- Make a gist for the proposal description, ideally based on previous upgrade proposals

- Go to the DAO proposal website and make a proposal:
  - Upgrade program with the new buffer, set the spill address to the address of $KEY
  - Upgrade idl with the new buffer

- Bump the version to the next one, update idl and push
