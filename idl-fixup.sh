#!/bin/bash

# Anchor works purely on a token level and does not know that the index types
# are just type aliases for a primitive type. This hack replaces them with the
# primitive in the idl json and types ts file.
for pair_str in \
        "TokenIndex u16" \
        "Serum3MarketIndex u16" \
        "PerpMarketIndex u16" \
        "usize u64" \
        "NodeHandle u32" \
        ; do
    pair=( $pair_str );
    perl -0777 -pi -e "s/\{\s*\"defined\":\s*\"${pair[0]}\"\s*\}/\"${pair[1]}\"/g" \
        target/idl/mango_v4.json target/types/mango_v4.ts;
done
