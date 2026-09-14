//! AES-256-GCM encryption-at-rest for sensitive local data
//! (clipboard contents, recovered text).
//!
//! The 256-bit data key is generated once and stored in the OS credential
//! store — Windows Credential Manager / macOS Keychain / libsecret — via the
//! `keyring` crate. Ciphertext blobs in SQLite are `nonce || ciphertext`
//! and are useless without the vault entry.

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    AeadCore, Aes256Gcm, Key,
};
use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use rand::RngCore;

const KEY_SERVICE: &str = "app.smarthub.desktop";
const KEY_ACCOUNT: &str = "smarthub-data-key-v1";

#[derive(Clone)]
pub struct Crypto {
    cipher: Aes256Gcm,
}

impl Crypto {
    /// Load (or lazily create) the data key from the OS credential manager.
    pub fn load_or_create() -> Result<Self> {
        let entry = keyring::Entry::new(KEY_SERVICE, KEY_ACCOUNT)
            .context("open OS credential store")?;

        let key_bytes: [u8; 32] = match entry.get_password() {
            Ok(b64) => {
                let raw = B64.decode(b64.trim()).context("decode vaulted data key")?;
                raw.try_into()
                    .map_err(|_| anyhow!("corrupted data key in OS vault (wrong length)"))?
            }
            Err(_) => {
                // First run — generate and vault a fresh key.
                let mut key = [0u8; 32];
                OsRng.fill_bytes(&mut key);
                entry
                    .set_password(&B64.encode(key))
                    .context("store data key in OS vault")?;
                key
            }
        };

        Ok(Self {
            cipher: Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes)),
        })
    }

    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96-bit unique nonce
        let ct = self
            .cipher
            .encrypt(&nonce, plaintext)
            .map_err(|_| anyhow!("encryption failed"))?;
        let mut out = Vec::with_capacity(nonce.len() + ct.len());
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ct);
        Ok(out)
    }

    pub fn decrypt(&self, blob: &[u8]) -> Result<Vec<u8>> {
        if blob.len() < 12 {
            return Err(anyhow!("ciphertext too short"));
        }
        let (nonce, ct) = blob.split_at(12);
        self.cipher
            .decrypt(nonce.into(), ct)
            .map_err(|_| anyhow!("decryption failed (key changed?)"))
    }

    pub fn encrypt_str(&self, s: &str) -> Result<Vec<u8>> {
        self.encrypt(s.as_bytes())
    }

    pub fn decrypt_str(&self, blob: &[u8]) -> Result<String> {
        Ok(String::from_utf8(self.decrypt(blob)?)?)
    }
}
