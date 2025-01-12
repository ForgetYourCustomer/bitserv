use std::{
    sync::{mpsc::Sender, Arc, Mutex},
    thread::sleep,
    time::Duration,
};

use bdk_bitcoind_rpc::{bitcoincore_rpc::RpcApi, Emitter};
use bdk_wallet::{
    bip39::Mnemonic,
    bitcoin::{self, Network, Transaction},
    chain::CheckPoint,
    keys::{DerivableKey, ExtendedKey},
    rusqlite::{self, Connection},
    template::Bip84,
    Balance, KeychainKind, PersistedWallet, Wallet,
};

use super::{client::Client, mnemonic::MnemonicStorage, paths::WalletPaths};

pub struct BitServWallet {
    bdk_wallet: Arc<Mutex<PersistedWallet<Connection>>>,
    // client: Option<Client>,
    // address: String,
    // balance: f64,
    // transactions: Vec<Transaction>,
    // initialized: bool,
    is_syncing: bool,
    stop_sync_tx: Option<Sender<()>>,
}

impl BitServWallet {
    pub fn balance(&self) -> Balance {
        let wallet = self.bdk_wallet.lock().unwrap();
        wallet.balance()
    }

    pub fn new(password: &str, network: Network) -> Self {
        let paths = WalletPaths::from_password(password);
        let mnemonic_storage = MnemonicStorage::new(paths.mnemonic_path);
        let mnemonic_words = mnemonic_storage.load_or_create_by_password(password);

        let mnemonic = Mnemonic::parse(&mnemonic_words).unwrap();

        let xkey: ExtendedKey = mnemonic.into_extended_key().unwrap();
        // Get xprv from the extended key
        let xprv = xkey.into_xprv(network).unwrap();

        let mut conn = rusqlite::Connection::open(paths.wallet_path).unwrap();

        let wallet_opt = Wallet::load()
            .descriptor(
                KeychainKind::External,
                Some(Bip84(xprv, KeychainKind::External)),
            )
            .descriptor(
                KeychainKind::Internal,
                Some(Bip84(xprv, KeychainKind::Internal)),
            )
            .extract_keys() // only needed if using private key descriptors
            .check_network(network)
            .load_wallet(&mut conn)
            .unwrap();

        let wallet = match wallet_opt {
            Some(wallet) => {
                println!("Loaded wallet from database");
                wallet
            }
            None => {
                println!("Creating new wallet");
                let wallet = Wallet::create(
                    Bip84(xprv, KeychainKind::External),
                    Bip84(xprv, KeychainKind::Internal),
                )
                .network(network)
                .create_wallet(&mut conn)
                .unwrap();
                wallet
            }
        };

        Self {
            bdk_wallet: Arc::new(Mutex::new(wallet)),
            is_syncing: false,
            stop_sync_tx: None,
            // client: None,
            // initialized: false,
            // address: String::from(""),
            // balance: 0.0,
            // transactions: Vec::new(),
        }
    }

    pub fn init(&mut self, client: &Client) {
        match client {
            Client::Rpc(rpc_client) => {
                let blockchain_info = match rpc_client.get_blockchain_info() {
                    Ok(blockchain_info) => blockchain_info,
                    Err(e) => panic!("Error getting blockchain info: {}", e),
                };
                println!(
                    "\nConnected to Bitcoin Core RPC.\nChain: {}\nLatest block: {} at height {}\n",
                    blockchain_info.chain, blockchain_info.best_block_hash, blockchain_info.blocks,
                );

                let mut wallet_lock = self.bdk_wallet.lock().unwrap();

                let wallet_tip: CheckPoint = wallet_lock.latest_checkpoint();
                println!(
                    "Current wallet tip is: {} at height {}",
                    &wallet_tip.hash(),
                    &wallet_tip.height()
                );

                let mut emitter = Emitter::new(rpc_client, wallet_tip.clone(), wallet_tip.height());

                println!("Syncing blocks...");
                while let Some(block) = emitter.next_block().unwrap() {
                    print!("{} ", block.block_height());
                    wallet_lock
                        .apply_block_connected_to(
                            &block.block,
                            block.block_height(),
                            block.connected_to(),
                        )
                        .unwrap();
                }

                let mempool_emissions: Vec<(Transaction, u64)> = emitter.mempool().unwrap();
                wallet_lock.apply_unconfirmed_txs(mempool_emissions);
                let balance: Balance = wallet_lock.balance();
                println!("Wallet balance after syncing: {}", balance.total());
            }
            Client::Electrum(_electrum) => {
                println!("Not supported");
            }
        }
    }

    pub fn sync(&mut self, client: Client) {
        // Perform a regular sync
        println!("Syncing blocks...");
        match client {
            Client::Rpc(rpc_client) => {
                let wallet_tip: CheckPoint = self.bdk_wallet.lock().unwrap().latest_checkpoint();

                let wallet_ref = self.bdk_wallet.clone();

                let (tx, rx) = std::sync::mpsc::channel::<()>();

                self.stop_sync_tx = Some(tx);

                let _ = std::thread::spawn(move || loop {
                    let mut emitter =
                        Emitter::new(&rpc_client, wallet_tip.clone(), wallet_tip.height());
                    while let Some(block) = emitter.next_block().unwrap() {
                        print!("{} ", block.block_height());
                        wallet_ref
                            .lock()
                            .unwrap()
                            .apply_block_connected_to(
                                &block.block,
                                block.block_height(),
                                block.connected_to(),
                            )
                            .unwrap();
                    }
                    if rx.try_recv().is_ok() {
                        println!("Received stop signal, ending sync");
                        break;
                    }
                    sleep(Duration::from_secs(5));
                });
            }
            Client::Electrum(_electrum) => {
                println!("Not supported");
            }
        }
    }

    pub fn stop_sync(&mut self) {
        if let Some(tx) = self.stop_sync_tx.take() {
            let _ = tx.send(());
        }
        self.is_syncing = false;
    }

    pub fn get_receiving_address(&self) -> String {
        let mut wallet = self.bdk_wallet.lock().unwrap();
        let address = wallet.reveal_next_address(KeychainKind::External);
        address.to_string()
    }

    /// Get all addresses (both external and change) that have been used in UTXOs
    /// Returns a vector of tuples containing (address, is_change)
    pub fn get_all_addresses(&self) -> Vec<(String, bool)> {
        let wallet = self.bdk_wallet.lock().unwrap();
        let mut addresses = std::collections::HashSet::new();

        // Get addresses from all transactions (both spent and unspent)
        for wallet_tx in wallet.transactions() {
            for output in wallet_tx.tx_node.tx.output.clone() {
                if let Some((keychain, _)) = wallet.derivation_of_spk(output.script_pubkey.clone())
                {
                    if let Ok(address) =
                        bitcoin::Address::from_script(&output.script_pubkey, wallet.network())
                    {
                        addresses.insert((address.to_string(), keychain == KeychainKind::Internal));
                    }
                }
            }
        }

        // Convert to sorted vector
        let mut result: Vec<_> = addresses.into_iter().collect();
        result.sort_by(|a, b| a.0.cmp(&b.0));

        result
    }

    /// Get only change addresses (internal addresses) that have been used in UTXOs
    pub fn get_change_addresses(&self) -> Vec<String> {
        let wallet = self.bdk_wallet.lock().unwrap();
        let mut addresses = std::collections::HashSet::new();

        // Get addresses from all transactions (both spent and unspent)
        for wallet_tx in wallet.transactions() {
            for output in wallet_tx.tx_node.tx.output.clone() {
                if let Some((keychain, _)) = wallet.derivation_of_spk(output.script_pubkey.clone())
                {
                    if keychain == KeychainKind::Internal {
                        if let Ok(address) =
                            bitcoin::Address::from_script(&output.script_pubkey, wallet.network())
                        {
                            addresses.insert(address.to_string());
                        }
                    }
                }
            }
        }

        // Convert to sorted vector
        let mut result: Vec<_> = addresses.into_iter().collect();
        result.sort();

        result
    }

    /// Get all external addresses that have a balance, along with their balances in BTC
    pub fn get_addresses_with_balance(&self) -> Vec<(String, f64)> {
        let wallet = self.bdk_wallet.lock().unwrap();

        // Group UTXOs by address and sum their values
        let mut balance_map = std::collections::HashMap::new();

        // Get only unspent UTXOs
        for utxo in wallet.list_unspent() {
            if let Some((keychain, _)) = wallet.derivation_of_spk(utxo.txout.script_pubkey.clone())
            {
                if keychain == KeychainKind::External {
                    if let Ok(address) =
                        bitcoin::Address::from_script(&utxo.txout.script_pubkey, wallet.network())
                    {
                        let amount = utxo.txout.value.to_btc();
                        *balance_map.entry(address.to_string()).or_insert(0.0) += amount;
                    }
                }
            }
        }

        // Convert to sorted vector
        let mut result: Vec<_> = balance_map.into_iter().collect();
        result.sort_by(|a, b| a.0.cmp(&b.0));

        result
    }
}

fn create_or_load_rpc_wallet(
    client: &bdk_bitcoind_rpc::bitcoincore_rpc::Client,
    wallet_name: &str,
) {
    match client.create_wallet(wallet_name, None, None, None, None) {
        Ok(_) => println!("Created new wallet: {}", wallet_name),
        Err(e) => {
            if !e.to_string().contains("Database already exists") {
                println!("Wallet already created");
            }
        }
    }
    // Load the wallet
    match client.load_wallet(wallet_name) {
        Ok(_) => println!("Loaded wallet: {}", wallet_name),
        Err(e) => {
            if !e.to_string().contains("Wallet already loaded") {
                println!("Wallet already loaded");
            }
        }
    }
}
