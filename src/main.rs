use anyhow::Result;
use bdk_bitcoind_rpc::bitcoincore_rpc::Auth;
use bdk_wallet::bitcoin::Network;
use log::info;
use std::sync::Arc;

use bitserv::{BitServWallet, Client};

mod api;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the logger
    env_logger::init();
    info!("Starting BitServ wallet application...");

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

    // Create Arc for sharing wallet between threads
    let wallet = Arc::new(wallet);

    // Create router
    let app = api::create_router(wallet);

    // Run server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    println!("Server running on http://127.0.0.1:3000");
    axum::serve(listener, app).await?;

    Ok(())
}
