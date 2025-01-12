use std::sync::Arc;

use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;

use crate::BitServWallet;

// Response types
#[derive(Serialize)]
pub struct BalanceResponse {
    confirmed: u64,
    unconfirmed: u64,
}

#[derive(Serialize)]
pub struct AddressResponse {
    address: String,
}

#[derive(Serialize)]
pub struct AddressesResponse {
    addresses: Vec<(String, bool)>,
}

#[derive(Serialize)]
pub struct AddressBalancesResponse {
    addresses: Vec<(String, f64)>,
}

// Handler functions
async fn get_balance(State(wallet): State<Arc<BitServWallet>>) -> Json<BalanceResponse> {
    let balance = wallet.balance();
    Json(BalanceResponse {
        confirmed: balance.confirmed.to_sat(),
        unconfirmed: balance.total().to_sat(),
    })
}

async fn get_receiving_address(State(wallet): State<Arc<BitServWallet>>) -> Json<AddressResponse> {
    let address = wallet.get_receiving_address();
    Json(AddressResponse { address })
}

async fn get_all_addresses(State(wallet): State<Arc<BitServWallet>>) -> Json<AddressesResponse> {
    let addresses = wallet.get_all_addresses();
    Json(AddressesResponse { addresses })
}

async fn get_addresses_with_balance(
    State(wallet): State<Arc<BitServWallet>>,
) -> Json<AddressBalancesResponse> {
    let addresses = wallet.get_addresses_with_balance();
    Json(AddressBalancesResponse { addresses })
}

// Create router
pub fn create_router(wallet: Arc<BitServWallet>) -> Router {
    Router::new()
        .route("/balance", get(get_balance))
        .route("/address", get(get_receiving_address))
        .route("/addresses", get(get_all_addresses))
        .route("/addresses/balances", get(get_addresses_with_balance))
        .with_state(wallet)
}
