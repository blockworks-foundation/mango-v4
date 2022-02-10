# Mango V4

Created: January 13, 2022 10:15 PM
Last Edited Time: February 7, 2022 5:27 PM

# New Solana Features

1. address maps
2. expandable accounts
3. lower tx fees (? confirm with Solana devs)

# Features

1. mark price that takes into account order book and basis (This is important for term futures. The mark price for a term future should be the current oracle price + moving average of basis (where basis is oracle price - orderbook mid))
2. no more PriceCache, no more nodebanks
3. arbitrary number of tokens and markets, so more MAX_PAIRS
    1. find a way to limit risk → then we can add any token and not worry about contamination. perhaps limit deposits and borrows. perhaps attach 
    2. limiting deposits might also require separating collateral and lend deposits
4. Mango Slice
    1. needs a manipulation resistant mark price (?)
5. margin trading across all dexes
6. Raydium style perp liquidity provision (?)
7. Liquidations are auctions (?)
8. More efficient logging
9. ids.json lives on chain
10. mobile app, apple might not be very friendly with defi apps, react-native might be the choice of framework to go with, microwavedcola might have some contacts who could do some contracting work
11. support multiple quote currencies
12. daffy: interest rate pegged to an interest rate oracle provided by switchboard, the optimal rate would be pegged to oracle. We can still override it but it makes the lending markets more efficient. For example, the optimal rate of USDC is like 10% or something which causes massive spread in borrow and lend tates right now when market is not really willing to pay 10%
13. multi legged interest rate