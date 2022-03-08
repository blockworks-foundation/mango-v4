#!/usr/bin/env bash

set -euo pipefail

anchor build
cp ../../target/deploy/margin_trade.so ../mango-v4/tests/fixtures/