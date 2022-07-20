# Mango v4 Program Change Log

Update this for each mainnet deployment.

## not on mainnet

-

## mainnet

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
