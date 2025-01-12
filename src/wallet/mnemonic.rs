use anyhow::{anyhow, Result};

use bdk_wallet::{
    bip39::{Language, Mnemonic},
    keys::{bip39::WordCount, GeneratableKey, GeneratedKey},
    miniscript,
};

use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Serialize, Deserialize)]
struct EncryptedMnemonic {
    encrypted_data: Vec<u8>,
    nonce: Vec<u8>,
    salt: Vec<u8>,
}

pub struct MnemonicStorage {
    storage_path: PathBuf,
}

impl MnemonicStorage {
    pub fn new(storage_path: PathBuf) -> Self {
        Self { storage_path }
    }

    pub fn load_or_create_by_password(&self, password: &str) -> String {
        let mnemonic_words = match self.load_mnemonic(password) {
            Ok(mnemonic_words) => mnemonic_words,
            Err(_) => {
                println!("Creating new mnemonic");
                // Generate fresh mnemonic
                let mnemonic: GeneratedKey<_, miniscript::Segwitv0> =
                    Mnemonic::generate((WordCount::Words12, Language::English)).unwrap();
                let mnemonic_words = mnemonic.to_string();

                // Save the mnemonic
                self.save_mnemonic(&mnemonic_words, password).unwrap();
                println!("New wallet created and saved");

                self.load_mnemonic(password).unwrap()
            }
        };
        mnemonic_words
    }

    pub fn save_mnemonic(&self, mnemonic: &str, password: &str) -> Result<()> {
        // Generate a random salt
        let mut salt = [0u8; 32];
        OsRng.fill_bytes(&mut salt);

        // Derive encryption key from password
        let key = derive_key(password, &salt)?;

        // Generate a random nonce
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt the mnemonic
        let cipher = ChaCha20Poly1305::new_from_slice(&key)
            .map_err(|e| anyhow!("Invalid key length: {}", e))?;
        let encrypted_data = cipher
            .encrypt(nonce, mnemonic.as_bytes())
            .map_err(|e| anyhow!("Encryption failed: {}", e))?;

        let encrypted_mnemonic = EncryptedMnemonic {
            encrypted_data,
            nonce: nonce_bytes.to_vec(),
            salt: salt.to_vec(),
        };

        // Save to file
        let json = serde_json::to_string(&encrypted_mnemonic)?;
        fs::write(&self.storage_path, json)?;

        Ok(())
    }

    pub fn load_mnemonic(&self, password: &str) -> Result<String> {
        if !self.storage_path.exists() {
            return Err(anyhow!("Mnemonic file does not exist"));
        }

        let json = fs::read_to_string(&self.storage_path)?;
        let encrypted_mnemonic: EncryptedMnemonic = serde_json::from_str(&json)?;

        let key = derive_key(password, &encrypted_mnemonic.salt)?;
        let cipher = ChaCha20Poly1305::new_from_slice(&key)
            .map_err(|e| anyhow!("Invalid key length: {}", e))?;
        let nonce = Nonce::from_slice(&encrypted_mnemonic.nonce);

        let decrypted_data = cipher
            .decrypt(nonce, encrypted_mnemonic.encrypted_data.as_ref())
            .map_err(|e| anyhow!("Decryption failed: {}", e))?;

        String::from_utf8(decrypted_data).map_err(|e| anyhow!("Invalid UTF-8: {}", e))
    }
}

fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; 32]> {
    let mut key = [0u8; 32];
    argon2::Argon2::default()
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| anyhow!("Key derivation failed: {}", e))?;
    Ok(key)
}
