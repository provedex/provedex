use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use ed25519_dalek::{Signature, SigningKey, VerifyingKey, SECRET_KEY_LENGTH};
use rand::rngs::OsRng;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum KeyError {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("home directory not found")]
    HomeNotFound,
    #[error("invalid key length: expected {SECRET_KEY_LENGTH}, got {0}")]
    InvalidLength(usize),
    #[error("invalid hex: {0}")]
    Hex(#[from] hex::FromHexError),
    #[error("signature verification failed")]
    BadSignature,
}

/// Ed25519 signing keypair. Public key is derived from the secret on load.
#[derive(Debug)]
pub struct SigningKeypair {
    signing: SigningKey,
}

impl SigningKeypair {
    pub fn generate() -> Self {
        Self {
            signing: SigningKey::generate(&mut OsRng),
        }
    }

    /// Load the keypair at `path`; if absent, generate one and persist it.
    /// File holds the raw 32-byte secret; permission narrowed to 0600 on unix.
    pub fn load_or_create<P: AsRef<Path>>(path: P) -> Result<Self, KeyError> {
        let path = path.as_ref();
        if path.exists() {
            return Self::load(path);
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let kp = Self::generate();
        kp.save(path)?;
        Ok(kp)
    }

    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, KeyError> {
        let bytes = fs::read(path.as_ref())?;
        if bytes.len() != SECRET_KEY_LENGTH {
            return Err(KeyError::InvalidLength(bytes.len()));
        }
        let mut secret = [0u8; SECRET_KEY_LENGTH];
        secret.copy_from_slice(&bytes);
        Ok(Self {
            signing: SigningKey::from_bytes(&secret),
        })
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), KeyError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, self.signing.to_bytes())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(path, perms)?;
        }
        Ok(())
    }

    pub fn sign(&self, message: &[u8]) -> Signature {
        use ed25519_dalek::Signer;
        self.signing.sign(message)
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing.verifying_key()
    }

    pub fn pubkey_hex(&self) -> String {
        hex::encode(self.signing.verifying_key().to_bytes())
    }
}

impl Clone for SigningKeypair {
    fn clone(&self) -> Self {
        // Reconstruct from the raw secret bytes; cheaper and version-proof
        // versus relying on a derived Clone in the dalek type.
        Self {
            signing: SigningKey::from_bytes(&self.signing.to_bytes()),
        }
    }
}

pub fn verify_signature(
    pubkey_hex: &str,
    message: &[u8],
    signature_hex: &str,
) -> Result<(), KeyError> {
    use ed25519_dalek::Verifier;
    let pk_bytes = hex::decode(pubkey_hex)?;
    if pk_bytes.len() != 32 {
        return Err(KeyError::InvalidLength(pk_bytes.len()));
    }
    let mut pk_array = [0u8; 32];
    pk_array.copy_from_slice(&pk_bytes);
    let pk = VerifyingKey::from_bytes(&pk_array).map_err(|_| KeyError::BadSignature)?;

    let sig_bytes = hex::decode(signature_hex)?;
    if sig_bytes.len() != Signature::BYTE_SIZE {
        return Err(KeyError::InvalidLength(sig_bytes.len()));
    }
    let mut sig_array = [0u8; Signature::BYTE_SIZE];
    sig_array.copy_from_slice(&sig_bytes);
    let sig = Signature::from_bytes(&sig_array);

    pk.verify(message, &sig).map_err(|_| KeyError::BadSignature)
}

/// Default Provedex data root, e.g. `~/.provedex/`.
pub fn default_data_dir() -> Result<PathBuf, KeyError> {
    let home = dirs::home_dir().ok_or(KeyError::HomeNotFound)?;
    Ok(home.join(".provedex"))
}

pub fn default_key_path() -> Result<PathBuf, KeyError> {
    Ok(default_data_dir()?.join("keys").join("ed25519.key"))
}

pub fn default_ledger_path() -> Result<PathBuf, KeyError> {
    Ok(default_data_dir()?.join("ledger.ndjson"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn sign_verify_roundtrip() {
        let kp = SigningKeypair::generate();
        let msg = b"hello provedex";
        let sig = kp.sign(msg);
        let sig_hex = hex::encode(sig.to_bytes());
        assert!(verify_signature(&kp.pubkey_hex(), msg, &sig_hex).is_ok());
    }

    #[test]
    fn signature_fails_on_modified_message() {
        let kp = SigningKeypair::generate();
        let sig = kp.sign(b"original");
        let sig_hex = hex::encode(sig.to_bytes());
        assert!(verify_signature(&kp.pubkey_hex(), b"tampered", &sig_hex).is_err());
    }

    #[test]
    fn save_load_preserves_identity() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("k.key");
        let kp = SigningKeypair::generate();
        let pk_hex = kp.pubkey_hex();
        kp.save(&path).unwrap();
        let loaded = SigningKeypair::load(&path).unwrap();
        assert_eq!(loaded.pubkey_hex(), pk_hex);
    }

    #[test]
    fn load_or_create_generates_and_reuses() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nested").join("k.key");
        let first = SigningKeypair::load_or_create(&path).unwrap();
        let second = SigningKeypair::load_or_create(&path).unwrap();
        assert_eq!(first.pubkey_hex(), second.pubkey_hex());
    }

    #[test]
    fn clone_preserves_identity() {
        let kp = SigningKeypair::generate();
        let cloned = kp.clone();
        assert_eq!(kp.pubkey_hex(), cloned.pubkey_hex());
        let msg = b"same key signs the same";
        let sig = hex::encode(cloned.sign(msg).to_bytes());
        assert!(verify_signature(&kp.pubkey_hex(), msg, &sig).is_ok());
    }
}
