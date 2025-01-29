use anyhow::Result;
use bdk_bitcoind_rpc::bitcoincore_rpc::Auth;
use bdk_wallet::bitcoin::Network;
use log::info;
use std::sync::Arc;

use bitserv::{api::create_router, config, BitServWallet, Client};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the logger
    env_logger::init();
    info!("Starting BitServ wallet application...");

    let port = config::port();
    let bitcoind_url = config::btcd_url();
    let bitcoind_username = config::btcd_username();
    let bitcoind_password = config::btcd_password();
    let password = config::wallet_pw();
    let network = Network::Regtest;

    let auth = Auth::UserPass(bitcoind_username.to_string(), bitcoind_password.to_string());
    let client = Client::new_rpc(bitcoind_url, auth);

    let mut wallet = BitServWallet::new(password, network);
    wallet.init(&client);
    wallet.sync(client);

    // Create Arc for sharing wallet between threads
    let wallet = Arc::new(wallet);

    // Create router
    let app = create_router(wallet);

    let bind_address = format!("127.0.0.1:{}", port);

    // Run server
    let listener = tokio::net::TcpListener::bind(&bind_address).await?;

    println!("Server running on {}", &bind_address);
    axum::serve(listener, app).await?;

    Ok(())
}
