# Mango v4 Program Change Log

Update this for each program release and mainnet deployment.

## not on mainnet

### v0.8.0, 2023-3-

Deployment:

- Introduce a new "fee buyback" feature. (#464, #478, #479, #481, #485, #489)

  If enabled, users who paid perp or openbook fees can optionally perform a MNGO
  to USDC swap at a favorable price to pay fees in MNGO instead.

  For example, if a user had paid 100 USDC in fees, they could use the new
  instruction to pay $90 worth of MNGO instead and get back their 100 USDC
  (concrete numbers and enabling this feature are up to DAO vote).

- The security admin is now allowed to reduce the init asset weight of tokens
  and perp markets to zero. (#482)

  This allows the security council to disable new borrows against a token or
  perp market in emergency situations. The primary usecase is a scenario where
  an oracle no longer tracks the real value of an asset. (like when the soBTC
  price depegged: Mango would have used the BTC oracle for it if it had been live)

- Introduce a new `Serum3SettleFundsV2` instruction (#484)

  OpenBook has a UI fee that bots don't have to pay. Previously, Mango claimed
  this OpenBook fee component for itself. The new instruction allows bots to
  skip paying the OpenBook UI fee when trading through Mango.

  The idea is to avoid penalizing market maker bots for trading on OpenBook
  through a Mango account.

- Make reduce-only behavior more intuitive for PerpPlaceOrder instruction (#483)
- Allow the group admin to edit the token and perp names stored on-chain (#488)
- Allow the group admin to call the `TokenRegisterTrustless` instruction (#477)
- Vendor `fixed` crate to enable general overflow checking in release mode (#476)

## mainnet

### v0.7.0, 2023-2-22

Deployment: Feb 22, 2023 at 14:45:12 Central European Standard Time, https://explorer.solana.com/tx/2KjMd2GLggSTJGSBQ3T96KK8Pj8XEXSDad65b8AN9gtCo6XdWmaFtewUJbPFvXK8WnKgdTxUNJjftpbtRJNEVhDg

- Security admin can now set OpenBook markets to reduce-only (#472)
- PlacePerpOrder: Improved logging when reduce-only is set (#468)
- PerpSettlePnl: Grant the low-health settle fee even if the settled amount is
  below the flat fee threshold (#458)
- OpenBook: Take referrer rebate as Mango fee (#469)

### v0.6.0, 2023-2-14

Deployment: Feb 14, 2023 at 16:06:03 Central European Standard Time, https://explorer.solana.com/tx/4vpjuiESQZn5t6XErHeSX76dCng4P4KPrr5pMGuYv9LhA3EcLgTw1bYxg8aRmBt1rfJCTqqYLws1cr4EvnrrETue

- Client: Increase search iteration limit
- Update Serum dependency to most recent openbook version (#437)
- Enable release-move overflow checks (#438)
- Remove cleanup testing instruction PerpZeroOut (#430)
- Liquidation: Fix amount limits by introducing a new "LiquidationEnd" health type (#440, #447)
- Fix amount logging in token deposit (#446)
- Restrict what the security admin can do (#452)
- Fix bug in perp cancel all so it doesn't error on filled/expired orders (#453)

### v0.5.0, 2023-2-2

Deployment: Feb 2, 2023 at 10:51:02 Central European Standard Time, https://explorer.solana.com/tx/eVGLcy3y8Vi9sMDKQbKdRKZa6dpjTjdP5HyDFXXQFqAaS1CXCg2QnFC1hgE8F8unWfgpmXC8PvmuRMhmQEE1YzK

- Log old and new values in edit instructions (#418)
- PerpPlaceOrder returns order id (#417)
- Prevent setting the group admin to the default address (#423)
- Allow security admin to disable individual instructions (#419)
- Rename pnl_asset_weight to overall_asset_weight (#427)
- Significant changes to perp liquidation instructions (#424)
- Reorganize perp fill events to save bytes and have client order ids (#426)
- Add market index to serum3 events (#429)

### v0.4.0, 2023-1-24

Deployment: Jan 24, 2023 at 10:21:59 Central European Standard Time, https://explorer.solana.com/tx/3C5vSUrC2xJhAeaDjRMuhE1Gnbj72gDKPRibpFk2gP2afoaFquY8GgUeBwhNoP25QtPvTJG3NZmZBoHoSgvrEWGH

- Perp instruction constraint fixes (#399)
- Documentation and cleanup from perp code audit (#400, #401, #406, #410, #412)
- Perp: Don't generate fill events with zero quantity (#404)
- Perp: add testing instruction to fix inconsistency from deleted accounts/markets (#413)
- Add program token deposit limit (#415)
- Allow security admin to set markets to reduce only or reduce init asset weight (#394)

### v0.3.0, 2023-1-17

Deployment: Jan 17, 2023 at 14:57:12 Central European Standard Time, https://explorer.solana.com/tx/5uGKvLwcBjPkUAKtFGqKdwm6pHXFaMGkF44P8rhJrRbmTGwgKShkSoVvLvqDNvJYa4iMftiQgZW7gG9tQaXjmrEk

- Add perp market pnl asset weights, replacing the "trusted market" flag (#391)
- Add tracking of realized PnL over a position's lifetime to perp positions (#392)
- Fix oracle staleness detection for pyth oracles (#393)

### v0.2.0, 2023-1-13

Deployment: Jan 13, 2023 at 11:31:05 Central European Standard Time, https://explorer.solana.com/tx/4yGRUk6QwntvC4umECDPDZJNcbevSJ1fdZi75Mz9rGa9SHKzUtjMF3V5FCTkzBZqAETQTccqv63BYw6yX8JNxiur

- Add an optional security authority with the ability to halt a group or
  temporarily freeze user accounts.
- Extend perp pnl settle limits to apply to realized pnl
- Rename perp_liq_bankruptcy to perp_liq_quote_and_bankruptcy and extend it to
  cover taking over the liqee's negative pnl while the settle limits and perp
  settle health allow it.
- Perp bankruptcy is now allowed when settling is impossible, even when there are
  spot assets remaining.

### Jan 5, 2023 Central European Standard Time

- Change max staleness slots from -1 to 600 for trustless token registering

### Jan 4, 2023 Central European Standard Time

- Reduce only mode for tokens, and perp markets
- Perp settlement applies no loan origination fee

### Dec 16, 2022 at 16:40 Central European Standard Time

### Oct 8, 2022 at 14:38:31 Central European Summer Time
https://explorer.solana.com/tx/3m8EDohkgwJZyiwpGXztBWARWQVxyhnSNDVuH467D7FPS2wxJerr79HhdhDEed5hpConHgGsKHvxtW1HJP6GixX9

### Oct 8, 2022 at 14:38:31 Central European Summer Time
https://explorer.solana.com/tx/3m8EDohkgwJZyiwpGXztBWARWQVxyhnSNDVuH467D7FPS2wxJerr79HhdhDEed5hpConHgGsKHvxtW1HJP6GixX9

- New ix `TokenDepositIntoExisting`

### Sep 1, 2022 at 10:24:35 Central European Summer Time
https://explorer.solana.com/tx/3NnX13A3QwsREKKKo3iYR4jqgoongpCjdhhXuJ3y5iP6FwfPcNieVop623tpgPbyreC7m7KtphwdWdoHYE5YC394

- Add HealthRegionBegin, -End instructions
- Add explicit "oracle" account argument for TokenDeposit and TokenWithdraw instructions

### Aug 20, 2022 at 19:58:29 Central European Summer Time
https://explorer.solana.com/tx/3R4frko1AekQKJmmQ5T6k3mdXF9uZVHTR7oocdspTPsc82xX7qrbgnG61r28UdhCxsjMxtQHgBqMc37FSvoHQfCN

- loan fee logging for off-chain services

### Aug 18, 2022 at 17:17:40 Central European Summer Time
https://explorer.solana.com/tx/4Xnyswcwx98y6khw8ptNVmdhQZwJjuNy2BvmQg2pJayoThFiw8kmS2ecRAg5cg2DncvW3NQgn2vtP8mCUtv6Q1yB

- liq_token_bankruptcy: removed liab_token_index argument
- flash_loan: both begin and end instructions now require the group to be passed as the final trailing remaining account
- flash_loan: the end instruction now requires passing a FlashLoanType, so logging can distinguish swaps from other uses
- ts client changes
  Class Group
  banksMap is now private
  there are now getFirstBankByMint, getMintDecimals, getFirstBankByTokenIndex

  Class MangoAccount
  How to navigate

  - if a function is returning a I80F48, then usually the return value is in native quote or native token, unless specified
  - if a function is returning a number, then usually the return value is in ui token, unless specified
  - functions try to be explicit by having native or ui in the name to better reflect the value
  - some values might appear unexpected large or small, usually the doc contains a "note"

  getMaxSourceForTokenSwap takes sourceMintPk and targetMintPk instead of sourceTokenName and targetTokenName
  simHealthRatioWithTokenPositionChanges takes mintPk instead of tokenName
  getEquivalentNativeUsdcPosition -> getEquivalentUsdcPosition
  getEquivalentNativeTokenPosition -> getEquivalentTokenPosition
  getNative -> getTokenBalance
  getNativeDeposits -> getTokenDeposits
  getNativeBorrows -> getTokenBorrows
  getUi -> getTokenBalanceUi
  deposits -> getTokenDepositsUi
  borrows -> getTokenBorrowsUi
  getAssetsVal -> getAssetsValue
  getLiabsVal-> getLiabsValue

  Class TokenPosition
  ui -> balanceUi
  uiDeposits -> depositsUi
  uiBorrows -> borrowsUi

  Class MangoClient
  Constructor doesnt take groupName anymore, it optionally takes idsSource with the correct default already set
  tokenDeposit now takes mintPk instead of tokenName
  tokenDepositNative now takes mintPk instead of tokenName
  tokenWithdraw -- same as above --
  tokenWithdrawNative -- same as above --
  marginTrade takes inputMintPk and outputMintPk instead of inputToken and outputToken
  marginTrade takes flashLoanType as an argument

### Aug 8, 2022 at 18:56:04 Central European Summer Time
https://explorer.solana.com/tx/yjZggRTrcDNquMkftNvBKLv77Dk4xp5yQPYXgN3qvBHTBWWJVhLPGHxqpGwosmEq3j8byHZMa13oxLLerBWUdgW

- improved logging for off chain services
- `AccountCreate` ix takes explicit input for sizes of various features

### Aug 4, 2022 at 09:30:00 Central European Summer Time

ts/client changes

- Renamed `getGroupForAdmin` to `getGroupForCreator`.
- Renamd `getInNativeUsdcUnits` to `getEquivalentNativeUsdcPosition`.
- Renamd `getMangoAccountForOwner` to `getMangoAccountsForOwner`.
- `getOrCreateMangoAccount` and c`reateMangoAccount` take an explicit payer, previously it was just implicitly using the client provider's wallet
- Upgraded anchor npm package to latest.
- Anchor is now a git submodule.

New features

- Many rust liquidator improvements.
- MangoAccount is now dynamically sized and is expandable, there is a new `AccountExpand` ix, default size of account is 8 token positions, and 2 serum3 and 2 perp positions, expanded account has 16 token positions and 8 serum3 and 8 perps for now.
- Group account - has a `creator` field which is set on creation and should never change, is used for pda derivation, has a new `fast_listing_admin` field for governance, and also has a `GroupEdit` ix to change both the admin keys.
- Group account - has a `version` field, version 0 which is used in the setup scripts for now, means serum3 and perp market registration is forbidden, and multiple banks are prevented from been added.
- Each account now has a reserved space of around 256 bytes.
- flash_loan ix 1 and 2 are removed, flash loan 3 has been renamed to just `FlashLoan`.
- `MintInfo`, `Serum3Market`, and `PerpMarket` have a field called `registration_time` which is seconds from epoch, e.g. use case how freshly has the market been added, if it was recently added then liquidity might need some time to improve
- Removed fields and commented out code for address lookup tables.
- New `TokenRegisterTrustless` ix to register tokens trustlessly.
- Insurance fund is now disabled for trustless tokens.
- `TokenRegistration` and `TokenRegisterTrustless` ixs dont take a bank_num anymore, hardcoded to 0.
- Enforced a minimum maximum rate of 50% so that rates don't fall so low that they cannot recover.

### Jul 14, 2022 at 09:33:52 Central European Summer Time
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
