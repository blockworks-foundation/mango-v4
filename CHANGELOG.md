# Mango v4 Program Change Log

Update this for each mainnet deployment.

## not on mainnet
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
