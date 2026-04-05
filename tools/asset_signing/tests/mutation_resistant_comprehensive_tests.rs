//! Mutation-resistant comprehensive tests for asset_signing.
//!
//! Tests sign_asset/verify_asset with temp files (no keyring needed).

use asset_signing::*;
use base64::Engine;

// ═══════════════════════════════════════════════════════════════════════════
// sign_asset / verify_asset roundtrip
// ═══════════════════════════════════════════════════════════════════════════

fn make_ed25519_keypair() -> ([u8; 32], [u8; 32]) {
    use ed25519_dalek::SigningKey;
    // Deterministic seed for reproducible tests
    let seed: [u8; 32] = [
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
        26, 27, 28, 29, 30, 31, 32,
    ];
    let signing = SigningKey::from_bytes(&seed);
    let verifying = signing.verifying_key();
    (seed, verifying.to_bytes())
}

#[test]
fn sign_verify_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test_asset.bin");
    std::fs::write(&path, b"hello world asset data").unwrap();

    let (private_key, public_key) = make_ed25519_keypair();
    let sig = sign_asset(path.to_str().unwrap(), &private_key).unwrap();
    assert_eq!(sig.len(), 64, "Ed25519 signature should be 64 bytes");

    let valid = verify_asset(path.to_str().unwrap(), &public_key, &sig).unwrap();
    assert!(valid, "signature should verify for correct data");
}

#[test]
fn verify_wrong_data_fails() {
    let dir = tempfile::tempdir().unwrap();
    let path1 = dir.path().join("original.bin");
    let path2 = dir.path().join("tampered.bin");
    std::fs::write(&path1, b"original data").unwrap();
    std::fs::write(&path2, b"tampered data").unwrap();

    let (private_key, public_key) = make_ed25519_keypair();
    let sig = sign_asset(path1.to_str().unwrap(), &private_key).unwrap();

    // Verify against tampered file should fail
    let valid = verify_asset(path2.to_str().unwrap(), &public_key, &sig).unwrap();
    assert!(!valid, "signature should not verify for tampered data");
}

#[test]
fn sign_nonexistent_file_fails() {
    let (private_key, _) = make_ed25519_keypair();
    let result = sign_asset("C:/nonexistent/path/asset.bin", &private_key);
    assert!(result.is_err());
}

#[test]
fn verify_nonexistent_file_fails() {
    let (_, public_key) = make_ed25519_keypair();
    let sig = vec![0u8; 64];
    let result = verify_asset("C:/nonexistent/path/asset.bin", &public_key, &sig);
    assert!(result.is_err());
}

#[test]
fn sign_empty_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.bin");
    std::fs::write(&path, b"").unwrap();

    let (private_key, public_key) = make_ed25519_keypair();
    let sig = sign_asset(path.to_str().unwrap(), &private_key).unwrap();
    assert_eq!(sig.len(), 64);

    let valid = verify_asset(path.to_str().unwrap(), &public_key, &sig).unwrap();
    assert!(valid);
}

#[test]
fn sign_large_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large.bin");
    let data = vec![0xAB; 100_000];
    std::fs::write(&path, &data).unwrap();

    let (private_key, public_key) = make_ed25519_keypair();
    let sig = sign_asset(path.to_str().unwrap(), &private_key).unwrap();
    assert_eq!(sig.len(), 64);

    let valid = verify_asset(path.to_str().unwrap(), &public_key, &sig).unwrap();
    assert!(valid);
}

#[test]
fn sign_wrong_key_size_fails() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.bin");
    std::fs::write(&path, b"data").unwrap();

    let result = sign_asset(path.to_str().unwrap(), &[0u8; 16]); // Wrong size
    assert!(result.is_err());
}

#[test]
fn verify_wrong_sig_size_fails() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.bin");
    std::fs::write(&path, b"data").unwrap();

    let (_, public_key) = make_ed25519_keypair();
    let result = verify_asset(path.to_str().unwrap(), &public_key, &[0u8; 32]); // Wrong sig size
    assert!(result.is_err());
}

#[test]
fn verify_wrong_public_key_fails() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.bin");
    std::fs::write(&path, b"secret data").unwrap();

    let (private_key, _) = make_ed25519_keypair();
    let sig = sign_asset(path.to_str().unwrap(), &private_key).unwrap();

    // Different key pair
    let other_seed: [u8; 32] = [
        99, 98, 97, 96, 95, 94, 93, 92, 91, 90, 89, 88, 87, 86, 85, 84, 83, 82, 81, 80, 79, 78, 77,
        76, 75, 74, 73, 72, 71, 70, 69, 68,
    ];
    let other_signing = ed25519_dalek::SigningKey::from_bytes(&other_seed);
    let other_pub = other_signing.verifying_key().to_bytes();

    let valid = verify_asset(path.to_str().unwrap(), &other_pub, &sig).unwrap();
    assert!(!valid, "wrong public key should fail verification");
}

#[test]
fn sign_deterministic() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("det.bin");
    std::fs::write(&path, b"deterministic test").unwrap();

    let (private_key, _) = make_ed25519_keypair();
    let sig1 = sign_asset(path.to_str().unwrap(), &private_key).unwrap();
    let sig2 = sign_asset(path.to_str().unwrap(), &private_key).unwrap();
    assert_eq!(
        sig1, sig2,
        "same file + same key → same signature (Ed25519 is deterministic)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// KeyStore::verify (static method — no keyring needed)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn keystore_verify_detects_invalid_signature() {
    let (seed, _) = make_ed25519_keypair();
    let signing = ed25519_dalek::SigningKey::from_bytes(&seed);
    let verify_key = signing.verifying_key();

    // Base64-encoded garbage signature (64 bytes of zeros)
    let bad_sig = base64::engine::general_purpose::STANDARD.encode([0u8; 64]);
    let result = KeyStore::verify(&verify_key, "test manifest", &bad_sig);
    if let Ok(valid) = result {
        assert!(!valid, "garbage sig should not verify");
    }
    // Err is also acceptable — some implementations error on bad sigs
}

// ═══════════════════════════════════════════════════════════════════════════
// KeyStore::export_public_key / import_public_key
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn export_import_public_key_roundtrip() {
    let (seed, _) = make_ed25519_keypair();
    let signing = ed25519_dalek::SigningKey::from_bytes(&seed);
    let original_key = signing.verifying_key();

    let dir = tempfile::tempdir().unwrap();
    let key_path = dir.path().join("pubkey.pem");

    // Write PEM manually (same format as KeyStore::export_public_key)
    let key_b64 = base64::engine::general_purpose::STANDARD.encode(original_key.as_bytes());
    let pem = format!("-----BEGIN PUBLIC KEY-----\n{key_b64}\n-----END PUBLIC KEY-----\n");
    std::fs::write(&key_path, pem).unwrap();

    let imported = KeyStore::import_public_key(key_path.to_str().unwrap()).unwrap();
    assert_eq!(imported.as_bytes(), original_key.as_bytes());
}
