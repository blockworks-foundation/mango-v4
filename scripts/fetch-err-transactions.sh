#!/bin/env zsh

RPC_URL=$MB_CLUSTER_URL

REQUEST_BODY=$(cat <<- \END
{"jsonrpc": "2.0", "id": 1, "method": "getTransaction", "params": [
  "{}",
  {"commitment": "confirmed", "encoding": "json", "maxSupportedTransactionVersion": 0}
]}
END
)


mkdir -p err-txs/original
mkdir -p err-txs/bak
mkdir -p err-txs/processed

rm -rf err-txs/processed/*

# File containing last n number of error signatures, fetch txs
# cat ~/Downloads/err-tx-sigs | parallel -k -j8 "curl -sS -X POST $RPC_URL -H 'Content-type: application/json' -d'$REQUEST_BODY' > err-txs/original/{}"

# Ignore market makers
cp err-txs/original/* err-txs/bak/
grep -rl '4hXPGTmR6dKNNqjLYdfDRSrTaa1Wt2GZoZnQ9hAJEeev' err-txs/bak | xargs rm
grep -rl 'BLgb4NFwhpurMrGX5LQfb8D8dBpGSGtBqqew2Em8uyRT' err-txs/bak | xargs rm
grep -rl '2f4nvyfS47tL8XaMt1Nm8kiE6dPW2W5udoRSWZch1bK9' err-txs/bak | xargs rm


# Extract logs for easy viewing
find err-txs/bak/* | xargs -I % basename % | parallel "jq '.result.meta.logMessages' err-txs/bak/{} > err-txs/processed/{}"

# Ignore known errors
grep -rl 'out of order' err-txs/processed | xargs rm
grep -rl 'srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX failed: custom program error: 0x2a' err-txs/processed | xargs rm
grep -rl 'programs/dyson/src/instructions/swap_arb' err-txs/processed | xargs rm
grep -rl 'programs/dyson/src/instructions/swap_protected' err-txs/processed | xargs rm
grep -rl 'SlippageToleranceExceeded' err-txs/processed | xargs rm
grep -rl 'InvalidCalculation' err-txs/processed | xargs rm
grep -rl 'RaydiumSwapExactOutput' err-txs/processed | xargs rm
grep -rl 'RaydiumClmmSwapExactOutput' err-txs/processed | xargs rm
grep -rl 'SharedAccountsExactOutRoute' err-txs/processed | xargs rm
grep -rl 'OracleStale' err-txs/processed | xargs rm
grep -rl 'AccountCreate' err-txs/processed | xargs rm
grep -rl 'ProfitabilityMismatch' err-txs/processed | xargs rm
grep -rl 'token is in reduce only mode' err-txs/processed | xargs rm


find err-txs/processed/ -size 0 -print -delete
