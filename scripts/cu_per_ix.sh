#!/usr/bin/env bash

# gather logs from tests
cargo test-bpf --features enable-gpl -- test_health_compute_serum  > logs/test.log 2<&1

# filter mango instructions and logging of consumed compute units
python3 parse_logs.py logs/test.log \
  | sed 's/ Program [^,]*,//g' \
  | rg -v '^$' > logs/cu_instruction.log 

#rm test.log