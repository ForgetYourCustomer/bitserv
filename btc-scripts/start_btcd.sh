#!/bin/bash

# Check if --no-daemon argument is passed
DAEMON_MODE="-daemon"
if [ "$1" == "--no-daemon" ]; then
    DAEMON_MODE=""
fi

# Start the bitcoin daemon with the appropriate mode
bitcoind -regtest $DAEMON_MODE

# Only wait and show messages if running in daemon mode
if [ -n "$DAEMON_MODE" ]; then
    # Wait for the daemon to start
    sleep 3

    # Create the wallet if it doesn't exist, otherwise load it
    bitcoin-cli -regtest createwallet "regtest_wallet" 2>/dev/null || bitcoin-cli -regtest loadwallet "regtest_wallet" 2>/dev/null

    echo "Bitcoin daemon started and wallet loaded"
fi