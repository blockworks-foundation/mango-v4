# Mango v4 Program Change Log

Update this for each program release and mainnet deployment.

## not on mainnet

## mainnet

### v0.21.2, 2024-1-

- Allow fast-listing of Openbook v1 markets (#839, #841)

### v0.21.1, 2024-1-

- Prevent withdraw operations from bringing token utilization over 100%.
- Prevent extreme interest rates for tokens with borrows but near zero deposits.

### v0.21.0, 2023-12-13

Deployment: Dec 13, 2023 at 09:02:46 Central European Standard Time, https://explorer.solana.com/tx/47BBFEugtHYciK5jHVzVtXawc7oyXKzX8o5V4ERXX3Cb7AZqmr8w6uDrFPRpyJRDccRWtuno8g2micqaFoSLC1EL

- Introduce deposit limits (#806)

  The DAO can now configure hard deposit limits per token. They can be used in
  conjunction with the previous soft limits to restrict how much of a token can
  be on the platform providing collateral weight.

- Improve OpenBook order tracking and price bands (#805)

  In order for hard deposit limits to work, OpenBook orders need to be tracked
  and potentially restricted. The DAO can now configure a band around the oracle
  price and new bids and asks that don't fall within this band will be rejected.

- SerumPlaceOrderV2 breaking change (#805)

  A new instruction for placing orders on OpenBook markets is introduced. The
  old instruction should be disabled shortly after release.

- Changing token maint weights over time (#780)

  The DAO can now trigger a gradual change in token maint weights. This allows
  it to make maint weights less favorable without potentially causing many
  liquidations at the same time.

- Changed perp settlement incentives (#771)

  The incentives were too high when the user account was close to liquidation.
  The DAO had previously reduced the percentage amount as a mitigation.

  With this change:
  - low-health settlement incentives are capped at 2x the flat fee, removing
    unlimited percentual incentive fees entirely
  - incentives are only paid if at least 1% of position value is settled,
    avoiding the incentive to settle accounts with large positions very frequently

- More configurable token interest rate curve (#755)

  The scaling factor and target utilization are now stored separately, giving the
  DAO more flexibility for configuration.

- Delegates can now deposit even when a new token position needs to be created (#775)
- TokenRegister: Add argument for insurance (#782)
- Close zero token positions when user asks to withdraw everything (#793)
- Fix default parameters for fast listing tokens (#804)
- Disable TokenAddBank instruction, which was unused (#803)
- Significantly reduce program heap use (#787, #785)
- Reduce compute use of OpenBook health computations (#750)

### v0.20.0, 2023-11-8

Deployment: Nov 8, 2023 at 10:44:24 Central European Standard Time, https://explorer.solana.com/tx/4LM5NJAa71tjjKT4a7MXVVsautU1DNvszbXp2ufeps9gMrksRh9pURRiacoyCEgW9gdBYJb1W3TL6o7dzDcUVmVH

- Token conditional swaps: Add two auction mechanisms (#717)

  The trigger orders that are used to implement stop loss and take profit orders
  currently require users to set a fixed premium - an incentive for the order
  triggerer. Two new types of trigger orders were added:

  - Premium auctions: After starting, the premium offered to triggerers gradually
    increases from zero. This way users are less likely to overpay on premium,
    but execution will be delayed until the premium is sufficiently high.
  - Linear auctions: A simple auction where users configure start and end for
    both time and price. The offered trigger price changes linearly with time
    instead of being tied to the oracle price.

- Account shrinking and migration (#692)

  The AccountExpand instruction can now shrink accounts. This allows users to
  change the trade-off between token positions, perp positions and OpenBook open
  order slots now. It will be particularly useful when the OpenBook v2 integration
  arrives.

  This also adds a AccountSizeMigration instruction to permissionlessly shrink
  existing accounts where safe while migrating them to the v3 account layout.

- Drop HealthCache from IDL and disable ComputeAccountData instruction (#723)

  Both were not intended as public API and are only used in tests as an old
  way of retrieving account information.

- Token withdraw: Deactivate zero positions when withdrawing zero (#736)

  Previously an "active but zero" token position would not be closed by a
  withdraw-all style instruction.

- Update dependencies to Anchor v0.28.0 and Solana v1.16.14 (#718)
- Flash loan: Introduce specialized FlashLoanSwapBegin to save tx bytes (#744)
- Flash loan: Whitelist Jupiter v6 program for delegates (#737)
- Token deposit: Require a valid oracle when opening a new token position (#722)
- Fix computing maximum allowed amount when swapping zero asset-weight tokens (#699)
- Fix too-strict validation of max rate on token edit (#734)

### v0.19.1, 2023-9-16

Deployment: Sep 16, 2023 at 11:20:20 Central European Summer Time, https://explorer.solana.com/tx/K9BJ1uDBH6Xe8erhS6C8Rmz6k6V1cKJ8z6wNmf4DV2aF5Woin4H5xXKj1ypTNDSTccNvcsAUTHStoai3k2hYY5E

- Fix a health overestimation with OpenBook open orders

  When bids or asks crossed the oracle price, the serum3 health would be
  overestimated before.

  Now we track an account's max bid and min ask in each market and use that
  as a worst-case price. The tracking isn't perfect for technical reasons
  (compute cost, no notifications on fill) but produces an upper bound on
  bids (lower bound on asks) that is sufficient to make health not
  overestimate.

### v0.19.0, 2023-9-7

Deployment: Sep 7, 2023 at 13:10:08 Central European Summer Time, https://explorer.solana.com/tx/3xcQWmAinBjFF4QgUCS7v5KxS7CjUQMJmENBHMyMMoeNCdKpLQL6fJXcKRRDmzW4ajPUywgPxBzMoYJn9c8CteEP

- Token deposits and withdraws: Allow full withdraw or full borrow repays
  even when the oracle is stale (#646, #675)

  Stale oracles are a problem for Mango because the risk engine can then no
  longer safely determine if a user action is safe or not. Before, a stale oracle
  would completely block interactions with an account until the oracle got
  updated again.

  This change allows some actions even while an oracle is stale:

  - Users with deposits in a token with a stale oracle can now withdraw tokens
    as long as their account health provided by tokens with non-stale oracles
    remains positive.
  - Users with borrows of a token with a stale oracle can now repay the borrows
    (unless they were being liquidated at the time).

  These actions can be used to unblock an account by removing the offending token
  from its balance sheet.

- Expiring delegate: Accounts can now have a short-term delegate (#663)

  This might allow users to temporarily delegate to an in-memory key, so they
  can trade without having to re-approve every transaction on their wallet.

- Flash loan: Start allowing Mango instructions after flash_loan_end (#681)

  Liquidators may be interested in performing actions in the same transaction
  as a flash loan swap.

- Flash loan: The DAO can now charge a deposit fee (#660, #693)

  The DAO can now configure a fee on deposits that happen in flash loans. This
  could be used to apply a fee to flash loan swaps.

  Previously flash loans that did not increase the user's token balance and did
  not borrow tokens were free.

- Stop loss: Respect net borrow limits and change low-health completion (#677)
- Stop loss: Store helpful UI fields (#654, #667)
- Stop loss: Fees are configured by-token instead of globally (#659)
- Stop loss: Avoid expensive health cache for expired orders (#682)
- Account creation: Add account_create_v2 instruction (#680, #685)
- Account resizing: Lower maximums due to tx account limit (#686, #688, #689)
- Account resizing: Fix denial of service if account has too many lamports (#694)
- Token register: Revamp API for simpler use from governance (#665)
- Token register untrusted: Adjust default oracle staleness (#678)
- Fix typo in name of admin_token_withdraw_fees instruction (#655)
- Flash loan: Better errors for missing banks (#639)
- OpenBook v2 integration: First draft of instructions (#628)

### v0.18.0, 2023-7-28

Deployment: Jul 28, 2023 at 08:29:46 Central European Summer Time, https://explorer.solana.com/tx/TaPcQ8dUDyFEaqprasGVEeG3x4Z2nMT7jY9tr2G8KVVf3kvDUQv8TRTjzDirasx3YkyYq3PmQcmcMbCcHsAnUNT

- Introduce limit and stop loss orders for arbitrary spot pairs (#604, #634)

  Allow users to request that a swap between two spot tokens should be executed
  once the price crosses a threshold. Independent of OpenBook markets.

- Improve behavior when listing tokens or markets with upcoming oracles (#620)

  When we listed RNDR before the oracle started publishing a price, there
  was an issue where the stable price got initialized to 0. Now, the stable
  price is only initialized the first time a valid oracle value is read.

- Deprecate Serum3SettleFunds (#606)

  Use the Serum3SettleFundsV2 instruction introduced in v0.8.0.

- Perp FillEventLog: Include amount of closed pnl (#624)
- Pyth: Fix reading most recent valid price (#631)
- Introduce mechanism for moving collected fees to DAO (#644)

### v0.17.1, 2023-7-6

Deployment: Jul 6, 2023 at 20:26:34 Central European Summer Time, https://explorer.solana.com/tx/4kiVtR1G3xNh8bTP4FetfG7rjPjLThFjrQNzMMs2TfQHnw7Ezp6JX4rboQbGrJsfZDd6zaMuEa1ZTxahRwPPb9JR

- Remove extra Pyth oracle status check added in v0.17.0

  The Pyth oracle status also reverts to Unknown if not enough publishers have
  reported in a 25 slot window. So checking for the "Trading" status means an
  implicit staleness limit of 25 slots.

  This staleness limit is much more strict than the ones configured on the
  oracles currently used by Mango and caused occasional transaction failures.

### v0.17.0, 2023-7-3

Deployment: Jul 3, 2023 at 09:46:14 Central European Summer Time, https://explorer.solana.com/tx/4G6b1uihopkHqp968sq3RYacYHn5ND8mMmeNd1RfswTCmiqeappTN2747JTvswVXxs7oqgfU6M3VKPGVRFPGJYuL

- Configurable perp market settle token (#550)

  This changes perp market margining to no longer assume all pnl is in USD
  while settlement is in USDC. Instead, a configurable settle token is used for
  pnl and settlement, defaulting to USDC.

  There is no difference while the USDC price is forced to $1 and its init and liab
  weights are 1. But with this patch, it becomes possible to change that.

  For now it is not recommended to use a token other than USDC or USDT (or
  another USD targeting stable token) for perp settlement.

  The patch also updates all instructions dealing with the insurance vault
  to be aware that the insurance fund is not in USD but in USDC and apply the
  USDC price before payouts. To do this, the previous
  PerpLiqNegativePnlOrBankruptcy was replaced by a new
  PerpLiqNegativePnlOrBankruptcyV2 instruction.

- Allow reduce-only actions when init health is low (#592)

  Previously when init health was negative, the program only allowed actions that
  increased init health. Now it also accepts actions that keep init health the
  same.

  This is helpful for users because they now can place reducing limit orders on
  the spot or perp orderbooks while their account has low health.

- Whitelist PerpPlaceOrderV2 and PerpPlaceOrderPeggedV2 for HealthRegions (#597)
- Improve logging of loans (#599, #603)
- Pyth oracle status is checked (#607)
- Fixes to the inactive fee buyback feature (#608)
- Fix token force close to respect the reduce-only flag (#613)
- Improve docs (#590, #594)
- Use workspace dependencies (#588)

### v0.16.0, 2023-5-19

Deployment: May 19, 2023 at 15:35:12 Central European Summer Time, https://explorer.solana.com/tx/22fEcghPGgAnYCZkfjTxTeKQwX5rzWSx3c5CV9TikJmaAKWCpubCZYBx5ZJJPeNG1xWUPWMw3ooDhFBRYCR3tKYU

- New event: PerpTakerTradeLog immediately logs your trade execution (#579, #584)

  Previously you had to look at the logs from FillEvent processing to determine
  how much was taken and what the fees were. The new PerpTakerTradeLog event
  is emitted during PerpPlaceOrder and simplifies that.

- Perp self-trade options (#533)

  There are new PerpPlaceOrderV2 and PerpPlaceOrderPeggedV2 instructions that take
  an argument that controls self-trade behavior, similar to OpenBook.

  The old instructions still exist with nearly unchanged behavior: They default
  to DecrementTake, which means being allowed to match against your own orders.
  But now you don't pay fees if you do so.

- Update anchor to v0.27.0 (#582)

  Mango used to depend on a fork of anchor. Now all patches are upstreamed and
  we have upgraded to the unmodified upstream version of v0.27.0.

### v0.15.0, 2023-5-11

Deployment: May 11, 2023 at 09:29:12 Central European Summer Time, https://explorer.solana.com/tx/3h6KFxLEAvifNGDBNcQrWdc6cRkpHTzFzL8VradfAXBYNfScrLJzDxm52N4RNmS9dmE84zDuwbErQ75RcxDcihY3

- Change TokenRegisterTrustless instruction to disable borrows by default (#567)

  The instruction is intended to use very conservative defaults for listing
  tokens. It now lists new tokens with zero asset weights and without allowing
  borrowing, which should leave oracle staleness and potential bugs as the main
  risks of listing new tokens.

- OpenBook place order instruction: Respect reduce-only flags on the base and
  quote bank (#569)

  This way the DAO can potentially leave related OpenBook markets open when it
  marks a token as reduce-only.

- FlashLoan: Whitelist the ComputeBudget program when called by delegates (#572)

  For convenience. When constructing a flash loan instruction for a delegated
  account, users no longer need to take care to remove compute budget
  instructions from the flash loan scope.

- Perp Order Matching: Exit when no lots can be filled due to the quote limit (#576)

  Previously it would keep looping unnecessarily.

- Improve error message for incorrect number of accounts in FixedAccountRetriever (#566)
- Add oracle confidence and type information to perp update funding logs (#568)

### v0.14.0, 2023-4-29

Deployment: Apr 29, 2023 at 11:58:43 Central European Summer Time, https://explorer.solana.com/tx/2iaLQTT6PqFjFQr94j5g2iUhDT9v6CJk5rNC9mY7cY7BfRjn6pWixnUF5Wv2qAAUq4hmEvM7WyajDxQjq6QbufSk

- Force-closing of perp positions (#525)

  When a perp markets is set to "force-close" by the DAO, anyone can close open
  perp orders and positions on the market. This allows the DAO to wind down perp
  markets if needed.

- Force-closing of OpenBook market use via Mango (#551)

  When an OpenBook market's Mango integration is set to "force-close" by the DAO,
  anyone can close open orders on that market that were placed via Mango.
  This allows the DAO to wind down interactions with an OpenBook market.

- Fix exception for the Jupiter program in flash loan (#552)

  Account delegates cannot execute generic flash loans, but were supposed to be
  able to use whitelisted Jupiter programs during a flash loan. The bug that
  prevented the exception from working was fixed.

- Allow the DAO to withdraw from the insurance fund token account (#561)
- Fix a bug with settle limit accounting when liqors take over positive pnl (#562)
- Improve logging on force-close instructions (#555)
- Fix perp order seqnum logging (#556)
- Fix build when using mango-v4 code with the "no-entrypoint" feature (#558)

### v0.13.0, 2023-4-18

Deployment: Apr 18, 2023 at 17:33:15 Central European Summer Time, https://explorer.solana.com/tx/4WWVHCAheTRBhzyXUjsV1Kqfn8LdnkupiVbK4qaPNqby8P5vv7hY6HS3rHHL9bMu1RGdCZvqsd2MHjdawLYQ6Pxi

- Add explicit token account checks to FlashLoan (#542)

  It looks like the reported security issue was not exploitable, but the guards
  that prevented it were too incidental. This change adds explicit checks,
  improving safety and readability.

  It adds the FlashLoanEndV2 instruction, replacing FlashLoanEnd.

- Don't incentivize using asset tokens with high liquidation fee during liquidation (#536)

  Previously liqors received the sum of the liquidation fee of the asset and liab token,
  which meant liqors would preferably liquidate with high-liq-fee tokens.

  After this change only the liab token's liq fee is used. That's sensible because
  the fee is about giving liqors some margin to work with when settling the
  liability they took on.

- Force-closing of tokens (#518)

  Mango already has the concept of switching tokens into a "reduce-only" mode where
  deposits and borrows are only allowed to decrease.

  The new "force-close" mode is even stricter: When it's enabled, liqors can reduce
  an account's borrows of the force-closed token even if the account is healthy.

  The goal is to have a way of winding down the platform's and users' exposure to
  a token if necessary. Only a DAO vote can change a token's state to force-close.

- Improve perp trade logging (#535)

  PerpUpdateFunding now logs the oracle update slot to allow traders to better
  evaluate oracle peg orders.

  Perp fills now create a fill log that contains the fill event seqnum. That allows
  relating the transaction with the order matching to the transaction that processed
  the fill event for the trade.

- IxGate: Fix check for re-enabling instructions (#540)

  The security admin was not supposed to be able to enable instructions, but a bug
  allowed it. With this fix, only the group admin (DAO) can enable instructions.

### v0.12.0, 2023-4-17

Deployment: Apr 17, 2023 at 15:49:33 Central European Summer Time, https://explorer.solana.com/tx/2PbaCRMGgpGiysxk5y8x3TdFRZbGEAKZdyAzEQhAMXfCxS4bPN96YZ4Pp6hHfp17fd7RYUd13t4vtjpaFb4ccYRm

- Emit perp fees settled on update_funding (#530)

  Required to have a full picture of total perp market fees.

- Net borrow limit: Separate out tracking from checking (#534)

  That way it's easier to be specific about where the limit should be checked.

### v0.11.0, 2023-4-4

Deployment: Apr 4, 2023 at 21:43:18 Central European Summer Time, https://explorer.solana.com/tx/5Z36iV6VhAfmxwZubQduV1hNyUyyB9AyjovAwNrWLb5cdAqGm4F3NGmz6V8VpHT6yUwCEDxm2hWMrdJXNkZ8RSPR

- Limit funding and interest accrual during downtimes (#529)

  Previously, if the funding or interest updating instruction wassn't
  called for a long time (like for a solana downtime or the security
  council halting the program), the next update would apply funding or
  interest for the whole time interval since the last update.

  This could lead to a bad downtime situation becoming worse. Instead,
  limit the maximum funding and interest time interval to one hour.

- Update default interest parameters in token_register_trustless (#523)

  This brings them in line with the recent interest rate changes for >50%
  utilization.

- Perp: Fix logging of funding rate in update funding and deactivate pos (#528)

### v0.10.0, 2023-4-3

Deployment: Apr 3, 2023 at 20:10:26 Central European Summer Time, https://explorer.solana.com/tx/3Rvv7hxqYQ7mPXE7jopzq1RAAoEwPi1pRPY7EubzEiZih8zMVhTMe1AsuYNJq3gwpM8BVVC3CXkAWcsFdd7SE6zC

- HealthRegion: Explicitly whitelist allowed instructions (#508)

  The security council had disabled the HealthRegion instructions after the audit
  found a vulnerability. The issue has been resolved by restricting which other
  instructions may be called in a health region. That way it's still usable to
  save compute units, but its attack surface is significantly reduced.

- Use insurance fund token oracle for bankruptcies (#503)

  This is in preparation for using an oracle for the USDC price instead of fixing
  its value to $1. The insurance fund is in USDC, so the oracle price needs to
  be taken into account once a real oracle is provided.

- Fee buyback: Use the USDC oracle (#504)
- Perp settle fees: Return early instead of error on failure (#526)
- Net borrow limits: Fixed accounting of deposits (#513)
- Better logging in IxGateSet instruction
- Sanity check token_index in TokenRegister instruction
- Allow using all available bytes for bank and market names

### v0.9.0, 2023-3-16

Deployment: Mar 16, 2023 at 11:07:30 Central European Standard Time, https://explorer.solana.com/tx/2hVqFQhxC9BGzDvH7y9bWChrMRvzsBGMPcMepHLBamK4vKJMJG48Fv8ZB54b46qErH1aGRy9YVhFnVnpaKgnoP3c

- Downgrade the "fixed" dependency to v1.11.0 (#500)

  The dependency had a regression. This downgrades to the previous version that
  had been in use with Mango v3, while backporting the safety improvements done
  for release v0.8.0.

- Improvements to perp position docstrings (#497)

### v0.8.0, 2023-3-11

Deployment: Mar 11, 2023 at 08:06:22 Central European Standard Time, https://explorer.solana.com/tx/61CbcyDaCV1DKHEGxkfNfx9nCUfsH3RgUU7mivTjtqbHJ3YVPX6vNAzn91CZYRsjohVc5LdcZCZtteDKrCiKjYEi

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
