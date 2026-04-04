use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::fs;
use std::path::Path;

use crate::error::{Error, Result};

/// Length of AES-256 key in bytes.
const KEY_LEN: usize = 32;

/// Length of GCM nonce (IV) in bytes.
const NONCE_LEN: usize = 12;

/// Information about how an object was encrypted, stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionInfo {
    /// Encryption mode: "SSE-S3" (server-side with master key) or "SSE-C" (customer-provided key).
    pub mode: String,
    /// Base64-encoded initialization vector (nonce).
    pub iv: String,
    /// HMAC of the original data key (base64) — used to verify the correct key is provided.
    /// For SSE-S3 this is derived from the master key; for SSE-C it's derived from the customer key.
    pub key_hmac: String,
}

/// Manages the server-side master encryption key.
pub struct MasterKeyManager {
    /// The raw 256-bit master key.
    master_key: [u8; KEY_LEN],
}

impl MasterKeyManager {
    /// Load the master key from `data/config/master.key`, generating a new one if absent.
    pub fn load_or_create(data_dir: &Path) -> Result<Self> {
        let config_dir = data_dir.join("config");
        fs::create_dir_all(&config_dir).map_err(Error::IoError)?;

        let key_path = config_dir.join("master.key");
        let master_key = if key_path.exists() {
            let bytes = fs::read(&key_path).map_err(Error::IoError)?;
            if bytes.len() != KEY_LEN {
                return Err(Error::InternalError(
                    "Master key file has wrong length".to_string(),
                ));
            }
            let mut arr = [0u8; KEY_LEN];
            arr.copy_from_slice(&bytes);
            arr
        } else {
            let mut arr = [0u8; KEY_LEN];
            rand::thread_rng().fill_bytes(&mut arr);
            // Write with restricted permissions (owner-only on unix)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::write(&key_path, arr).map_err(Error::IoError)?;
                fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600))
                    .map_err(Error::IoError)?;
            }
            #[cfg(not(unix))]
            {
                fs::write(&key_path, arr).map_err(Error::IoError)?;
            }
            // arr was moved into fs::write, but it's Copy, so we need to regenerate or keep a copy
            // Actually [u8; 32] is Copy, so arr is still valid after the move
            arr
        };

        Ok(Self { master_key })
    }

    /// Encrypt data using the master key (SSE-S3 mode).
    pub fn encrypt_sse_s3(&self, plaintext: &[u8]) -> Result<(Vec<u8>, EncryptionInfo)> {
        encrypt_data(plaintext, &self.master_key, "SSE-S3")
    }

    /// Encrypt data using a customer-provided key (SSE-C mode).
    pub fn encrypt_sse_c(
        &self,
        plaintext: &[u8],
        customer_key: &[u8; KEY_LEN],
    ) -> Result<(Vec<u8>, EncryptionInfo)> {
        encrypt_data(plaintext, customer_key, "SSE-C")
    }

    /// Decrypt data using the master key (SSE-S3 mode).
    pub fn decrypt_sse_s3(&self, ciphertext: &[u8], info: &EncryptionInfo) -> Result<Vec<u8>> {
        verify_key_hmac(&self.master_key, &info.key_hmac)?;
        decrypt_data(ciphertext, &self.master_key, &info.iv)
    }

    /// Decrypt data using a customer-provided key (SSE-C mode).
    pub fn decrypt_sse_c(
        &self,
        ciphertext: &[u8],
        info: &EncryptionInfo,
        customer_key: &[u8; KEY_LEN],
    ) -> Result<Vec<u8>> {
        verify_key_hmac(customer_key, &info.key_hmac)?;
        decrypt_data(ciphertext, customer_key, &info.iv)
    }

    /// Get a copy of the master key bytes (for pre-signed URL signing).
    pub fn get_key_bytes(&self) -> [u8; KEY_LEN] {
        self.master_key
    }
}

/// Encrypt `plaintext` with the given key using AES-256-GCM.
fn encrypt_data(
    plaintext: &[u8],
    key: &[u8; KEY_LEN],
    mode: &str,
) -> Result<(Vec<u8>, EncryptionInfo)> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|_| Error::InternalError("Failed to create AES cipher".to_string()))?;

    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| Error::InternalError("Encryption failed".to_string()))?;

    let iv_b64 = BASE64.encode(nonce_bytes);
    let key_hmac = compute_key_hmac(key);

    Ok((
        ciphertext,
        EncryptionInfo {
            mode: mode.to_string(),
            iv: iv_b64,
            key_hmac,
        },
    ))
}

/// Decrypt `ciphertext` with the given key using AES-256-GCM.
fn decrypt_data(ciphertext: &[u8], key: &[u8; KEY_LEN], iv_b64: &str) -> Result<Vec<u8>> {
    let nonce_bytes = BASE64
        .decode(iv_b64)
        .map_err(|_| Error::InternalError("Failed to decode IV".to_string()))?;

    if nonce_bytes.len() != NONCE_LEN {
        return Err(Error::InternalError("Invalid IV length".to_string()));
    }

    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|_| Error::InternalError("Failed to create AES cipher".to_string()))?;

    let nonce = Nonce::from_slice(&nonce_bytes);
    cipher.decrypt(nonce, ciphertext).map_err(|_| {
        Error::DecryptionError("Decryption failed — wrong key or corrupted data".to_string())
    })
}

/// Compute HMAC-SHA256 of the key itself (for key verification without storing the key).
fn compute_key_hmac(key: &[u8; KEY_LEN]) -> String {
    use hmac::{Hmac, Mac};
    type HmacSha256 = Hmac<Sha256>;

    let mut mac =
        <HmacSha256 as Mac>::new_from_slice(b"objstor-key-verification").expect("HMAC key is valid");
    mac.update(key);
    BASE64.encode(mac.finalize().into_bytes())
}

/// Verify that a provided key matches the stored HMAC.
fn verify_key_hmac(key: &[u8; KEY_LEN], stored_hmac: &str) -> Result<()> {
    let computed = compute_key_hmac(key);
    if computed == stored_hmac {
        Ok(())
    } else {
        Err(Error::DecryptionError(
            "Key verification failed — wrong encryption key".to_string(),
        ))
    }
}

/// Decode a base64-encoded SSE-C customer key from the HTTP header.
pub fn decode_sse_c_key(key_b64: &str) -> Result<[u8; KEY_LEN]> {
    let bytes = BASE64
        .decode(key_b64)
        .map_err(|_| Error::InvalidHeaderValue("Invalid base64 SSE-C key".to_string()))?;
    if bytes.len() != KEY_LEN {
        return Err(Error::InvalidHeaderValue(format!(
            "SSE-C key must be {} bytes, got {}",
            KEY_LEN,
            bytes.len()
        )));
    }
    let mut arr = [0u8; KEY_LEN];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = [0x42u8; 32];
        let plaintext = b"hello, world!";
        let (ciphertext, info) = encrypt_data(plaintext, &key, "SSE-S3").unwrap();
        assert_ne!(ciphertext, plaintext.to_vec());
        let decrypted = decrypt_data(&ciphertext, &key, &info.iv).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_wrong_key_fails() {
        let key = [0x42u8; 32];
        let wrong_key = [0x43u8; 32];
        let plaintext = b"secret";
        let (ciphertext, info) = encrypt_data(plaintext, &key, "SSE-S3").unwrap();
        // Decryption with wrong key should fail
        assert!(decrypt_data(&ciphertext, &wrong_key, &info.iv).is_err());
    }

    #[test]
    fn test_key_hmac_verification() {
        let key = [0xABu8; 32];
        let hmac = compute_key_hmac(&key);
        assert!(verify_key_hmac(&key, &hmac).is_ok());
        let wrong_key = [0xCDu8; 32];
        assert!(verify_key_hmac(&wrong_key, &hmac).is_err());
    }
}
