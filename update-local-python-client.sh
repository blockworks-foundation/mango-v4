#!/usr/bin/env bash

set -e pipefail

cp target/idl/mango_v4.json py/src/
anchorpy client-gen ./target/idl/mango_v4.json  ./py/src --program-id m43thNJ58XCjL798ZSq6JGAG1BnWskhdq5or6kcnfsD
