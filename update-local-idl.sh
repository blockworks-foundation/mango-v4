#!/usr/bin/env bash

set -e pipefail

anchor build --skip-lint
./idl-fixup.sh
cp -v ./target/types/mango_v4.ts ./ts/mango_v4.ts
tsc