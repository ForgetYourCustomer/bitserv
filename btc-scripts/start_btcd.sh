#!/bin/bash

# In a shell dedicated to the bitcoin daemon 
bitcoind -regtest -daemon

# Wait for the daemon to start
sleep 3

# Create the wallet if it doesn't exist, otherwise load it
bitcoin-cli -regtest createwallet "regtest_wallet" 2>/dev/null || bitcoin-cli -regtest loadwallet "regtest_wallet" 2>/dev/null

echo "Bitcoin daemon started and wallet loaded"