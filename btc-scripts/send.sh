#!/bin/bash

# Check if both address and amount are provided
if [ $# -ne 2 ]; then
    echo "Usage: $0 <address> <amount_in_mbtc>"
    exit 1
fi

address=$1
mbtc_amount=$2

# Convert mBTC to BTC (force decimal point)
btc_amount=$(echo "if($mbtc_amount/1000 < 1) print 0; $mbtc_amount/1000" | bc -l)

# Send the transaction
bitcoin-cli -regtest sendtoaddress "$address" "$btc_amount"