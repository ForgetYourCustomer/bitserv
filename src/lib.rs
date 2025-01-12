mod wallet;
pub mod api;

// Re-export the main types that users of our library will need
pub use wallet::bitserv::BitServWallet;
pub use wallet::client::Client;
