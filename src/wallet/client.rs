pub enum Client {
    Rpc(bdk_bitcoind_rpc::bitcoincore_rpc::Client),
    Electrum(bdk_electrum::electrum_client::Client),
}

impl Client {
    pub fn new_rpc(url: &str, auth: bdk_bitcoind_rpc::bitcoincore_rpc::Auth) -> Self {
        let client = bdk_bitcoind_rpc::bitcoincore_rpc::Client::new(url, auth).unwrap();
        Client::Rpc(client)
    }
    pub fn new_electrum(url: &str) -> Self {
        let client = bdk_electrum::electrum_client::Client::new(url).unwrap();
        Client::Electrum(client)
    }
}
