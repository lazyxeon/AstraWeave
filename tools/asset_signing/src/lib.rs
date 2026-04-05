#![forbid(unsafe_code)]
// Asset signing and verification (Phase 0): Ed25519 signature over SHA-256 hash of file contents
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey, SECRET_KEY_LENGTH};
use sha2::{Digest, Sha256};

pub mod keystore;
pub use keystore::KeyStore;
// Re-export VerifyingKey for validators.rs to use
pub use ed25519_dalek::VerifyingKey as VerifyKey;

fn hash_file(path: &str) -> Result<[u8; 32], String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let out = hasher.finalize();
    Ok(out.into())
}

#[allow(clippy::expect_used)] // INVARIANT: private_key length validated at function entry
pub fn sign_asset(path: &str, private_key: &[u8]) -> Result<Vec<u8>, String> {
    if private_key.len() != SECRET_KEY_LENGTH {
        return Err("private key must be 32 bytes (ed25519 seed)".into());
    }
    let hash = hash_file(path)?;
    let sk = SigningKey::from_bytes(private_key.try_into().expect("BUG: length checked above"));
    let sig: Signature = sk.sign(&hash);
    Ok(sig.to_bytes().to_vec())
}

#[allow(clippy::expect_used)] // INVARIANT: public_key/signature lengths validated at function entry
pub fn verify_asset(path: &str, public_key: &[u8], signature: &[u8]) -> Result<bool, String> {
    if public_key.len() != 32 || signature.len() != 64 {
        return Err("invalid key or signature size".into());
    }
    let hash = hash_file(path)?;
    let vk = VerifyingKey::from_bytes(public_key.try_into().expect("BUG: length checked above"))
        .map_err(|e| format!("vk: {e}"))?;
    let sig = Signature::from_bytes(signature.try_into().expect("BUG: length checked above"));
    Ok(vk.verify_strict(&hash, &sig).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::OsRng;
    #[test]
    fn roundtrip_sign_verify() {
        let tmp = "asset.tmp";
        std::fs::write(tmp, b"hello").unwrap();
        let sk = SigningKey::generate(&mut OsRng);
        let pk = VerifyingKey::from(&sk);
        let sig = sign_asset(tmp, &sk.to_bytes()).unwrap();
        assert!(verify_asset(tmp, pk.as_bytes(), &sig).unwrap());
        std::fs::remove_file(tmp).ok();
    }
}
