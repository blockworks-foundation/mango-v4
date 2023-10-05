# service-mango-orderbook

This module parses bookside accounts and exposes L2 and L3 data and updates on a websocket

Public API: `https://api.mngo.cloud/orderbook/v1/`

## API Reference

Get a list of markets

```
{
   "command": "getMarkets"
}
```

```
{
    "ESdnpnNLgTkBCZRuTJkZLi5wKEZ2z47SG3PJrhundSQ2": "SOL-PERP",
    "HwhVGkfsSQ9JSQeQYu2CbkRCLvsh3qRZxG6m4oMVwZpN": "BTC-PERP",
    "Fgh9JSZ2qfSjCw9RPJ85W2xbihsp2muLvfRztzoVR7f1": "ETH-PERP",
}
```

### L2 Data 

Subscribe to L2 updates

```
{
   "command": "subscribe",
   "marketId": "MARKET_PUBKEY",
   "subscriptionType": "level",
}
```

```
{
    "success": true,
    "message": "subscribed to level updates for MARKET_PUBKEY"
}
```

L2 Checkpoint - Sent upon initial subscription

```
{
    "market": "ESdnpnNLgTkBCZRuTJkZLi5wKEZ2z47SG3PJrhundSQ2",
    "bids":
        [22.17, 8.86],
        [22.15, 88.59],
    ],
    "asks": [
        [22.19, 9.17],
        [22.21, 91.7],
    ],
    "slot": 190826373,
    "write_version": 688377208758
}
```

L2 Update - Sent per side

```
{
    "market": "ESdnpnNLgTkBCZRuTJkZLi5wKEZ2z47SG3PJrhundSQ2",
    "bids":          // or asks
        [22.18, 6],  // new level added
        [22.17, 1],  // level changed
        [22.15, 0],  // level removed
    ],
    "slot": 190826375,
    "write_version": 688377208759
}
```
### L3 Data 

Subscribe to L3 updates
:warning: If the subscribed market is a perp market, `ownerPubkey` corresponds to a `mangoAccount`, if the subscribed market is a spot market, `ownerPubkey` corresponds to an open orders account.

```
{
   "command": "subscribe",
   "marketId": "MARKET_PUBKEY",
   "subscriptionType": "book",
}
```

```
{
    "success": true,
    "message": "subscribed to book updates for MARKET_PUBKEY"
}
```

L3 Checkpoint - Sent upon initial subscription

```
{
    "market": "ESdnpnNLgTkBCZRuTJkZLi5wKEZ2z47SG3PJrhundSQ2",
    "bids": [
        {
          "price": 20.81,
          "quantity": 1.3,
          "ownerPubkey": "F1SZxEDxxCSLVjEBbMEjDYqajWRJQRCZBwPQnmcVvTLV"
        },
        {
          "price": 20.81,
          "quantity": 62.22,
          "ownerPubkey": "BGYWnqfaauCeebFQXEfYuDCktiVG8pqpprrsD4qfqL53"
        },
        {
          "price": 20.8,
          "quantity": 8,
          "ownerPubkey": "CtHuPg2ctVVV7nqmvVEcMtcWyJAgtZw9YcNHFQidjPgF"
        }
    ],
    "asks": [
        {
          "price": 20.94,
          "quantity": 62.22,
          "ownerPubkey": "BGYWnqfaauCeebFQXEfYuDCktiVG8pqpprrsD4qfqL53"
        },
        {
          "price": 20.95,
          "quantity": 1.3,
          "ownerPubkey": "F1SZxEDxxCSLVjEBbMEjDYqajWRJQRCZBwPQnmcVvTLV"
        },
        {
          "price": 21.31,
          "quantity": 30,
          "ownerPubkey": "5gHsqmFsMaguM3HMyEmnME4NMQKj6NrJWUGv6VKnc2Hk"
        }
    ],
    "slot": 190826373,
    "write_version": 688377208758
}
```

L3 Update - Sent per side

```
{
    "market": "ESdnpnNLgTkBCZRuTJkZLi5wKEZ2z47SG3PJrhundSQ2",
    "side": "ask",
    "additions": [
        {
          "price": 20.92,
          "quantity": 61.93,
          "ownerPubkey": "BGYWnqfaauCeebFQXEfYuDCktiVG8pqpprrsD4qfqL53"
        }
      ],
      "removals": [
        {
          "price": 20.92,
          "quantity": 61.910000000000004,
          "ownerPubkey": "BGYWnqfaauCeebFQXEfYuDCktiVG8pqpprrsD4qfqL53"
        }
      ],
      "slot": 197077534,
      "write_version": 727782187614
}
```


## Setup

## Local

1. Prepare the connector configuration file.

   [Here is an example](service-mango-orderbook/conf/example-config.toml).

   - `bind_ws_addr` is the listen port for the websocket clients
   - `rpc_ws_url` is unused and can stay empty.
   - `connection_string` for your `grpc_sources` must point to the gRPC server
     address configured for the plugin.
   - `rpc_http_url` must point to the JSON-RPC URL.
   - `program_id` must match what is configured for the gRPC plugin

2. Start the service binary.

   Pass the path to the config file as the first argument. It logs to stdout. It
   should be restarted on exit.

3. Monitor the logs

   `WARN` messages can be recovered from. `ERROR` messages need attention. The
   logs are very spammy changing the default log level is recommended when you
   dont want to analyze performance of the service.

## fly.io

