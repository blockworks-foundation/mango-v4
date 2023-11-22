while true;
do
cargo run --bin liquidator --release -- --liqor-mango-account 43dEtfUoL1dN9v4JPFB5KSkHHFF4bswGn6mUJ9dpEAix --liqor-owner liquidator.json --parallel-rpc-requests 100 --prioritization-micro-lamports 500000 --rpc-url http://202.8.8.12:8899 --telemetry false --min-health-ratio 20 --check-interval-ms 20 --rebalance false --take-tcs false;
sleep 10;
done
