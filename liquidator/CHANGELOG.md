# v0.2.0

- Improve snapshotting behavior for Serum OpenOrders account.

  This now uses multiple getMultipleAccounts requests instead of a single
  getProgramAccounts request, which finishes more quickly and puts far less load
  on the RPC node.

  This requires two new configuration settings: `parallel_rpc_requests` and
  `get_multiple_accounts_count`. See `example-config.toml`.

- Use jemalloc, improving long-term memory footprint

# v0.1.0

Initial release.
