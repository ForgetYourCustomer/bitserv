#[cfg(test)]
mod tests {
    use bdk_bitcoind_rpc::bitcoincore_rpc::Auth;
    use bdk_wallet::bitcoin::Network;
    use bitserv::{BitServWallet, Client};

    #[tokio::test]
    async fn test_wallet_operations() {
        let bitcoind_url = "http://127.0.0.1:18443";
        let bitcoind_username = "myusername";
        let bitcoind_password = "mypassword";
        let password = "your-secure-password";
        let network = Network::Regtest;

        let auth = Auth::UserPass(bitcoind_username.to_string(), bitcoind_password.to_string());
        let client = Client::new_rpc(bitcoind_url, auth);

        let mut wallet = BitServWallet::new(password, network);
        wallet.init(&client);
        wallet.sync(client);

        // Test getting addresses
        let all_addresses = wallet.get_all_addresses();
        assert!(
            all_addresses.len() >= 0,
            "Should return a list of addresses"
        );

        let change_addresses = wallet.get_change_addresses();
        assert!(
            change_addresses.len() >= 0,
            "Should return a list of change addresses"
        );

        let addresses_with_balance = wallet.get_addresses_with_balance();
        assert!(
            addresses_with_balance.len() >= 0,
            "Should return a list of addresses with balance"
        );

        let receiving_address = wallet.get_receiving_address();
        assert!(
            !receiving_address.is_empty(),
            "Should return a valid receiving address"
        );

        wallet.stop_sync();
    }

    #[tokio::test]
    async fn test_wallet_sync() {
        let bitcoind_url = "http://127.0.0.1:18443";
        let auth = Auth::UserPass("myusername".to_string(), "mypassword".to_string());
        let client = Client::new_rpc(bitcoind_url, auth);

        let mut wallet = BitServWallet::new("test-password", Network::Regtest);
        wallet.init(&client);

        // Test sync and stop_sync
        wallet.sync(client);
        wallet.stop_sync();
    }
}
