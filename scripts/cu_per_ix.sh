#!/usr/bin/env bash

# gather logs from tests
cargo test-bpf --features enable-gpl -- test_perp_settle_pnl_with_fallback  > test.log 2<&1

# filter mango instructions and logging of consumed compute units
python3 parse_logs.py test.log \
  | sed 's/ Program 4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg,//g' \
  | sed 's/ Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA,//g' \
  | rg -v '^$' > cu_instruction.log 

#rm test.log
