# service-mango-fills

This module parses event queues and exposes individual fills on a websocket.

Public API: `https://api.mngo.cloud/fills/v1/`

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

Subscribe to markets

```
{
   "command": "subscribe"
   "marketIds": ["MARKET_PUBKEY"]
}
```

```
{
	"success": true,
	"message": "subscribed to market MARKET_PUBKEY"
}
```

Subscribe to account

```
{
   "command": "subscribe"
   "account": ["MANGO_ACCOUNT_PUBKEY"]
}
```

```
{
	"success": true,
	"message": "subscribed to account MANGO_ACCOUNT_PUBKEY"
}
```

Fill Event

```
{
	"event": {
		"eventType": "perp",
		"maker": "MAKER_MANGO_ACCOUNT_PUBKEY",
		"taker": "TAKER_MANGO_ACCOUNT_PUBKEY",
		"takerSide": "bid",
		"timestamp": "2023-04-06T13:00:00+00:00",
		"seqNum": 132420,
		"makerClientOrderId": 1680786677648,
		"takerClientOrderId": 1680786688080,
		"makerFee": -0.0003,
		"takerFee": 0.0006,
		"price": 20.72,
		"quantity": 0.45
	},
	"marketKey": "ESdnpnNLgTkBCZRuTJkZLi5wKEZ2z47SG3PJrhundSQ2",
	"marketName": "SOL-PERP",
	"status": "new",
	"slot": 186869253,
	"writeVersion": 662992260539
}
```

If the fill occurred on a fork, an event will be sent with the 'status' field set to 'revoke'.

## Setup

## Local

1. Prepare the connector configuration file.

   [Here is an example](service-mango-fills/conf/example-config.toml).

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
