use std::{
    hash::Hash,
    sync::{mpsc::Sender, Arc, Mutex},
    thread::sleep,
    time::Duration,
};

use anyhow::Result;
use bdk_bitcoind_rpc::{bitcoincore_rpc::RpcApi, Emitter};
use bdk_wallet::{
    bip39::{Language, Mnemonic},
    bitcoin::{self, Network, Transaction},
    chain::CheckPoint,
    keys::{DerivableKey, ExtendedKey},
    rusqlite::Connection,
    template::Bip84,
    Balance, KeychainKind, PersistedWallet, Wallet,
};

use super::{client::Client, mnemonic::MnemonicStorage, paths::WalletPaths};
use crate::{
    config,
    pubsub::{ChainEvent, Publisher},
};

pub struct BitServWallet {
    bdk_wallet: Arc<Mutex<PersistedWallet<Connection>>>,
    conn: Arc<Mutex<Connection>>,
    is_syncing: bool,
    stop_sync_tx: Option<Sender<()>>,
    publisher: Arc<Mutex<Publisher>>,
}

impl BitServWallet {
    pub fn balance(&self) -> Balance {
        let wallet = self.bdk_wallet.lock().unwrap();
        wallet.balance()
    }

    pub fn new(password: &str, network: Network) -> Self {
        let paths = WalletPaths::from_password(password);
        let mnemonic_storage = MnemonicStorage::new(paths.mnemonic_path);

        let mnemonic = mnemonic_storage.load_or_create_by_password(password);

        let xkey: ExtendedKey = Mnemonic::parse_in(Language::English, mnemonic)
            .unwrap()
            .into_extended_key()
            .unwrap();

        let mut conn = Connection::open(paths.wallet_path).unwrap();
        let xprv = xkey.into_xprv(network).unwrap();
        let bdk_wallet = Wallet::load()
            .descriptor(
                KeychainKind::External,
                Some(Bip84(xprv, KeychainKind::External)),
            )
            .descriptor(
                KeychainKind::Internal,
                Some(Bip84(xprv, KeychainKind::Internal)),
            )
            .extract_keys()
            .check_network(network)
            .load_wallet(&mut conn)
            .unwrap()
            .unwrap();

        let publisher_bind_address = config::publisher_bind_address();

        let publisher = Arc::new(Mutex::new(Publisher::new(publisher_bind_address).unwrap()));

        Self {
            bdk_wallet: Arc::new(Mutex::new(bdk_wallet)),
            conn: Arc::new(Mutex::new(conn)),
            is_syncing: false,
            stop_sync_tx: None,
            publisher,
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

                println!("InitialSyncing blocks...");
                while let Some(block) = emitter.next_block().unwrap() {
                    println!("Syncing block number:{} ", block.block_height());
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
                wallet_lock.persist(&mut self.conn.lock().unwrap()).unwrap();
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
                let wallet_ref = self.bdk_wallet.clone();
                let conn_ref = self.conn.clone();
                let publisher = self.publisher.clone();
                let (tx, rx) = std::sync::mpsc::channel::<()>();

                self.stop_sync_tx = Some(tx);

                let _ = std::thread::spawn(move || loop {
                    let wallet_tip: CheckPoint = wallet_ref.lock().unwrap().latest_checkpoint();

                    let mut emitter =
                        Emitter::new(&rpc_client, wallet_tip.clone(), wallet_tip.height());
                    while let Some(block_event) = emitter.next_block().unwrap() {
                        print!("Processing block {} ", block_event.block_height());

                        let mut tx_details = Vec::<(String, u64, String)>::new();

                        // First pass: collect transaction details
                        {
                            let wallet = wallet_ref.lock().unwrap();
                            block_event.block.txdata.iter().for_each(|tx| {
                                tx.output.iter().for_each(|out| {
                                    if wallet.is_mine(out.script_pubkey.clone()) {
                                        if let Ok(address) = bitcoin::Address::from_script(
                                            &out.script_pubkey,
                                            wallet.network(),
                                        ) {
                                            let amount = out.value.to_sat();
                                            tx_details.push((
                                                address.to_string(),
                                                amount,
                                                tx.compute_txid().to_string(),
                                            ));
                                        }
                                    }
                                });
                            });
                        } // wallet lock released here

                        // Second pass: apply block
                        {
                            let mut wallet = wallet_ref.lock().unwrap();
                            wallet
                                .apply_block_connected_to(
                                    &block_event.block,
                                    block_event.block_height(),
                                    block_event.connected_to(),
                                )
                                .unwrap();
                            wallet.persist(&mut conn_ref.lock().unwrap()).unwrap();
                            println!("Block applied to wallet");
                        } // wallet lock released here

                        // Only publish if we found any deposits
                        if !tx_details.is_empty() {
                            if let Err(e) =
                                publisher.lock().unwrap().publish(ChainEvent::NewDeposits {
                                    deposits: tx_details,
                                })
                            {
                                eprintln!("Failed to publish block details: {}", e);
                            }
                        }

                        println!("✓"); // Visual indicator of block completion
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

    pub fn reveal_next_address(&self) -> Result<String> {
        let mut wallet = self.bdk_wallet.lock().unwrap();
        let mut conn = self.conn.lock().unwrap();
        let address = wallet.reveal_next_address(KeychainKind::External);

        match wallet.persist(&mut conn) {
            Ok(_) => Ok(address.to_string()),
            Err(e) => {
                println!("Error persisting wallet: {}", e);
                Err(e.into())
            }
        }
    }

    pub fn get_receiving_address_by_index(&self, index: u32) -> String {
        let wallet = self.bdk_wallet.lock().unwrap();
        let address = wallet.peek_address(KeychainKind::External, index);
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

    pub fn publish_chainevent(&self, event: ChainEvent) -> Result<()> {
        let publisher = self.publisher.lock().unwrap();
        publisher.publish(event)
    }
}
