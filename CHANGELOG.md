# Mango v4 Program Change Log

Update this for each mainnet deployment.

## not on mainnet

-

## mainnet

Aug 4, 2022 at 09:30:00 Central European Summer Time

ts/client changes

- rename getGroupForAdmin -> getGroupForCreator
- rename getInNativeUsdcUnits -> getEquivalentNativeUsdcPosition
- rename getMangoAccountForOwner-> getMangoAccountsForOwner
- getOrCreateMangoAccount and createMangoAccount take an explicit payer, previously it was just implicitly using the client provider's wallet
- upgraded anchor npm package to latest
- anchor is now a git submodule

new features

- many rust liquidator improvements
- mango account is now dynamically sized and is expandable, there is a new account_expand ix, default size of account is 8 token positions, and 2 serum3 and 2 perp positions, expanded account has 16 token positions and 8 serum3 and 8 perps for now
- group account - has a creator field which is set on creation and should never change, is used for pda derivation, has a new fast_listing_admin field for governance, and also has a group_edit ix to change both the admin keys
- group account - has a version field, version 0 which is used in the setup scripts for now, means serum3 and perp market registration is forbidden, and multiple banks are prevented from been added
- each account now has a reserved space of around 256 bytes
- flash loan ix 1 and 2 are removed, flash loan 3 has been renamed to just flash loan
- mint_info, serum3 markets and perp markets have a field called registration_time which is seconds from epoch, e.g. use case how freshly has the market been added, if it was recently added then liquidity might need some time to improve
- removed fields and commented out code for address lookup tables
- new ix to register tokens trustlessly
- insurance fund for trustless vs not trustful
- token registration ixs dont take a bank_num anymore, hardcoded to 0
- enforced a minimum maximum rate of 50% so that rates don't fall so low that they cannot recover

Jul 14, 2022 at 09:33:52 Central European Summer Time
https://explorer.solana.com/tx/vZ5hP1vGp37fgzBfG9nb4nfA5ZdmYgk8meq53YPR4ReFxrcTwBUxTYBQUgnfAnq9u5fH36S3QTfb9mVkBXt5A6C

- Account data was rearranged to put fields that are often used with gPA first
- The `CreateGroup` instruction now requires an `insurance_mint` account, which is
  used as the mint for the `insurance_vault` token account it creates. Pass the
  USDC mint address.
- The token with `token_index` zero is now required to be the `insurance_mint`.
  Trying to register a different token for index zero will now fail.
- New instruction: `LiqTokenBankruptcy` to resolve insurance fund payouts and
  socialized loss for bankrupt accounts.
- The `PerpCreateMarket` instruction no longer requires a `quote_token_index`
  argument. The USDC/insurance mint is always used as quote currency for perps.
- The `UpdateIndex` instruction now requires the `oracle` account to be passed
  for logging purposes.
- New instructions: `AccountEdit`, `TokenEdit`, `PerpEditMarket` for reconfiguring.
- The `delegate` field on `MangoAccount` is now used and many instructions can be
  called by the account delegate.
- `TokenUpdateIndexAndRate` now maintains dynamic optimal and max rates for token interest rates.

- Renamed instructions:
  - create/close_group -> group_create/close
  - create/edit/close_account -> account_create/edit/close
  - update_index -> token_update_index
  - create/set_stub_oracle -> stub_oracle_create/set
