#!/bin/bash

# Environment:
# - RPC_URL
# - ACCOUNT
#
# Writes random files.
# Does filtering that may be inappropriate.

# Get tx sigs for an account
#
# If there are more than 1000, use --before <txsig>
solana -u $RPC_URL transaction-history \
  --limit 10 \
  $ACCOUNT \
  | head -n-2 > tx-sigs

# fetch tx (expensive!)
#
# Note: `solana transaction-history --show-transactions` exists, but is _way_ slower
# because it fetches only one at a time.
REQUEST_BODY=$(cat <<- \END
{"jsonrpc": "2.0", "id": 1, "method": "getTransaction", "params": [
  "{}",
  {"commitment": "confirmed", "encoding": "json", "maxSupportedTransactionVersion": 0}
]}
END
)
cat tx-sigs | parallel -k -j8 "curl -sS -X POST $RPC_URL -H 'Content-type: application/json' -d'$REQUEST_BODY'" > tx-data

# filter
JQ_CMD=$(cat <<- \END
.result
| select(
  all(.meta.logMessages[];
    (
      contains("Program log: Liqee is not liquidatable")
      or contains("PerpSettlePnl")
      or contains("PerpConsumeEvents")
    ) | not
  )
)
| {
  date:.blockTime | todate,
  sig: .transaction.signatures[0],
  msg: .meta.logMessages,
}
END
)
cat tx-data | jq "$JQ_CMD" > tx-out
