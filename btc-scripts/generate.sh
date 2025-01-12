#!/bin/bash

if [ "$#" -ne 2 ]; then
    echo "Usage: $0 <bitcoin_address> <block_to_mine>"
    echo "Example: $0 bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh 100"
    exit 1
fi

address=$1
block_to_mine=$2

# Mine the specified number of blocks
bitcoin-cli -regtest generatetoaddress "$block_to_mine" "$address"