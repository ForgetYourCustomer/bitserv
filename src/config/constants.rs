use config::{Config, ConfigError};
use lazy_static::lazy_static;
use serde::Deserialize;
use std::env;

#[derive(Debug, Clone, Copy)]
pub enum Environment {
    Test,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Test => "test",
            Environment::Production => "prod",
        }
    }

    pub fn from_env() -> Self {
        match env::var("ENV")
            .unwrap_or_else(|_| String::from("test"))
            .as_str()
        {
            "prod" => Environment::Production,
            _ => Environment::Test,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub environment: String,
    pub port: u16,
    pub publisher_bind_address: String,
    pub btcd_url: String,
    pub btcd_username: String,
    pub btcd_password: String,
    pub wallet_pw: String,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let environment = Environment::from_env();

        // Load .env file
        dotenv::from_filename(format!(".env.{}", environment.as_str()))
            .expect("Failed to load .env file");

        // Create new config
        let config = Config::builder()
            // Start with default Settings
            .set_default("environment", environment.as_str())?
            // Add in settings from environment variables (with a prefix of APP)
            // E.g. `APP_DEBUG=1 ./target/app` would set the `debug` key
            .add_source(config::Environment::default())
            .build()?;

        // Deserialize configuration
        config.try_deserialize()
    }

    pub fn environment(&self) -> Environment {
        match self.environment.as_str() {
            "prod" => Environment::Production,
            _ => Environment::Test,
        }
    }
}

// Create a lazy static instance of Settings
lazy_static! {
    pub static ref SETTINGS: Settings = Settings::new().expect("Failed to load settings");
}

// Constants can be accessed through these functions
pub fn environment() -> Environment {
    SETTINGS.environment()
}

pub fn btcd_url() -> &'static str {
    &SETTINGS.btcd_url
}

pub fn btcd_username() -> &'static str {
    &SETTINGS.btcd_username
}

pub fn btcd_password() -> &'static str {
    &SETTINGS.btcd_password
}

pub fn wallet_pw() -> &'static str {
    &SETTINGS.wallet_pw
}

pub fn port() -> u16 {
    SETTINGS.port
}

pub fn publisher_bind_address() -> &'static str {
    &SETTINGS.publisher_bind_address
}
