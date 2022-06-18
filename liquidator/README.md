# Liquidatable Accounts Feed

This implements a service that listens to Solana updates of Mango related accounts
and checks if they are liquidatable. If they are, it notifies connected clients
about it.

Its purpose is to hand potentially liquidatable accounts to a Mango liquidator
like https://github.com/blockworks-foundation/liquidator-v3.

It is a separate service because computing account health is around two orders of
magnitude faster this way.

## Architecture

Data flows into this service through
- Solana JSON RPC PubSub websocket streams, for slot and account updates
- Solana JSON RPC getProgramAccounts requests, for snapshots

The service models the current bank state for relevant accounts, checks their
health and sends interesting data back out to all clients that connected to its
websocket server.

All data resides in memory. The service does not write to disk.

## Running

Run `liquidatable-accounts-feed myconfig.toml`. The service is supposed to run
until aborted. Please report any panics or early exits as issues.

### Configuration

Check `example-config.toml`.

Note that you will need to configure an RPC server that allows websocket connections
as well as getProgramAccounts RPC calls.

## Output

Websocket messages look like this (without the comments):
```
{
  "jsonrpc": "2.0",
  // "candidate" is sent each time an account is looked at
  // "candidateStart" is sent the first time account health is below threshold
  // "candidateStop" is send when a candidate's health is above threshold again
  "method": "candidate",
  "params": {
    "account": "DopjuzaqPURVDy3DQhffGa1YZ9maMe5StGY1aXfJAymk",
    // the being_liquidated flag on the account
    "being_liquidated": false,
    // assets divided by liabilities; <1.0 means liquidatable
    "health_fraction": 1.0000339686978705,
    // weighted sum of assets
    "assets": 48741,
    // weighted sum of liabilities
    "liabilities": 48740
  }
}
```
