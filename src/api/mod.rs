use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{pubsub::ChainEvent, BitServWallet};

// Response types
#[derive(Serialize)]
pub struct BalanceResponse {
    confirmed: u64,
    unconfirmed: u64,
}

#[derive(Serialize)]
pub struct AddressResponse {
    success: bool,
    address: String,
    error: Option<String>,
}

#[derive(Serialize)]
pub struct AddressesResponse {
    addresses: Vec<(String, bool)>,
}

#[derive(Serialize)]
pub struct AddressesWithBalanceResponse {
    addresses: Vec<(String, f64)>,
}

// Handler functions
async fn get_balance(
    State(wallet): State<Arc<BitServWallet>>,
) -> (StatusCode, Json<BalanceResponse>) {
    println!("Getting balance");
    let balance = wallet.balance();
    (
        StatusCode::OK,
        Json(BalanceResponse {
            confirmed: balance.confirmed.to_sat(),
            unconfirmed: balance.total().to_sat(),
        }),
    )
}

async fn get_receiving_address_by_index(
    Path(index): Path<i32>,
    State(wallet): State<Arc<BitServWallet>>,
) -> (StatusCode, Json<AddressResponse>) {
    println!("Getting address by index");
    let address = wallet.get_receiving_address_by_index(index as u32);
    (
        StatusCode::OK,
        Json(AddressResponse {
            success: true,
            address,
            error: None,
        }),
    )
}

async fn get_new_address(
    State(wallet): State<Arc<BitServWallet>>,
) -> (StatusCode, Json<AddressResponse>) {
    println!("Getting new address");
    let response = match wallet.reveal_next_address() {
        Ok(address) => (
            StatusCode::OK,
            Json(AddressResponse {
                success: true,
                address,
                error: None,
            }),
        ),
        Err(_e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AddressResponse {
                success: false,
                address: String::new(),
                error: Some(String::from("Error getting new address")),
            }),
        ),
    };
    response
}

async fn get_all_addresses(
    State(wallet): State<Arc<BitServWallet>>,
) -> (StatusCode, Json<AddressesResponse>) {
    println!("Getting all addresses");
    let addresses = wallet.get_all_addresses();
    (StatusCode::OK, Json(AddressesResponse { addresses }))
}

async fn get_addresses_with_balance(
    State(wallet): State<Arc<BitServWallet>>,
) -> (StatusCode, Json<AddressesWithBalanceResponse>) {
    println!("Getting addresses with balance");
    let addresses = wallet.get_addresses_with_balance();
    (
        StatusCode::OK,
        Json(AddressesWithBalanceResponse { addresses }),
    )
}

async fn test_pub_deposits(
    State(wallet): State<Arc<BitServWallet>>,
    Json(tx): Json<TestPubTxRequest>,
) -> (StatusCode, Json<TestPubTxResponse>) {
    println!("Testing public transaction");
    let chain_event = ChainEvent::NewDeposits { deposits: tx.txs };

    println!("Publishing event: {:?}", chain_event);

    let response = match wallet.publish_chainevent(chain_event) {
        Ok(_) => (
            StatusCode::OK,
            Json(TestPubTxResponse {
                success: true,
                error: None,
            }),
        ),
        Err(_e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(TestPubTxResponse {
                success: false,
                error: Some(String::from("Error testing public transaction")),
            }),
        ),
    };
    response
}

// Create router
pub fn create_router(wallet: Arc<BitServWallet>) -> Router {
    Router::new()
        .route("/balance", get(get_balance))
        .route("/address/:index", get(get_receiving_address_by_index))
        .route("/new-address", get(get_new_address))
        .route("/addresses", get(get_all_addresses))
        .route("/addresses/balances", get(get_addresses_with_balance))
        .route("/test/pub-deposits", post(test_pub_deposits))
        .with_state(wallet)
}

#[derive(Deserialize)]
struct TestPubTxRequest {
    txs: Vec<(String, u64, String)>,
}

#[derive(Serialize)]
struct TestPubTxResponse {
    success: bool,
    error: Option<String>,
}
