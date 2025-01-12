# In a shell dedicated to the bitcoin daemon 
bitcoind -regtest -daemon

# In a new shell dedicated to the bitcoin-cli
bitcoin-cli -regtest getblockchaininfo

bitcoin-cli -regtest createwallet mywallet
bitcoin-cli -regtest loadwallet mywallet
bitcoin-cli -regtest getnewaddress

# Mine 101 blocks
bitcoin-cli -regtest generatetoaddress 101 <address>

# Send to address
bitcoin-cli -regtest sendtoaddress <address> <amount>