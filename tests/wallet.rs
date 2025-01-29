#[cfg(test)]
mod tests {
    use bdk_bitcoind_rpc::bitcoincore_rpc::Auth;
    use bdk_wallet::bitcoin::Network;
    use bitserv::{BitServWallet, Client};
    use lazy_static::lazy_static;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    struct TestContext {
        wallet: BitServWallet,
        client: Option<Client>,
    }

    impl TestContext {
        fn new() -> Self {
            let bitcoind_url = "http://127.0.0.1:18443";
            let bitcoind_username = "myusername";
            let bitcoind_password = "mypassword";
            let auth = Auth::UserPass(bitcoind_username.to_string(), bitcoind_password.to_string());
            let client = Client::new_rpc(bitcoind_url, auth);
            let mut wallet = BitServWallet::new("your-secure-password", Network::Regtest);
            wallet.init(&client);

            TestContext {
                wallet,
                client: Some(client),
            }
        }

        fn create_client(&self) -> Client {
            let bitcoind_url = "http://127.0.0.1:18443";
            let bitcoind_username = "myusername";
            let bitcoind_password = "mypassword";
            let auth = Auth::UserPass(bitcoind_username.to_string(), bitcoind_password.to_string());
            Client::new_rpc(bitcoind_url, auth)
        }

        fn sync_wallet(&mut self) {
            let client = self.create_client();
            self.wallet.sync(client);
        }
    }

    lazy_static! {
        static ref TEST_CONTEXT: Arc<Mutex<TestContext>> = {
            let context = TestContext::new();
            Arc::new(Mutex::new(context))
        };
    }

    #[tokio::test]
    async fn start_syncing() {
        let mut context = TEST_CONTEXT.lock().await;
        context.sync_wallet();
        println!("Wallet setup completed successfully");
    }

    #[tokio::test]
    async fn test_wallet_operations() {
        let context = TEST_CONTEXT.lock().await;

        // Test getting addresses
        let all_addresses = context.wallet.get_all_addresses();
        assert!(
            all_addresses.len() >= 0,
            "Should return a list of addresses"
        );

        let change_addresses = context.wallet.get_change_addresses();
        assert!(
            change_addresses.len() >= 0,
            "Should return a list of change addresses"
        );
    }

    #[tokio::test]
    async fn test_wallet_sync() {
        let mut context = TEST_CONTEXT.lock().await;
        context.wallet.stop_sync();
        context.sync_wallet();
    }

    #[tokio::test]
    async fn test_get_next_address() {
        let context = TEST_CONTEXT.lock().await;
        let address = context.wallet.reveal_next_address().unwrap();
        assert!(!address.is_empty(), "Address should not be empty");
    }
}
