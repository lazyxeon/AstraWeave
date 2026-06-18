# AstraWeave Asset Signing and Key Management - Design Document

## Executive Summary

This document provides a design for upgrading AstraWeave's asset signing system from ephemeral Ed25519 keys to a more robust solution. The design draws from industry standards including Sigstore, The Update Framework (TUF), Docker Content Trust, and NPM provenance to create a secure, verifiable, and operationally sound asset signing infrastructure.

**Current State**: Ephemeral Ed25519 keys generated per-run (tools/aw_asset_cli/src/main.rs:476-487), no verification workflow
**Target State**: Key hierarchy with persistent keys, verification at runtime/build-time, and transparency logging

---

## 1. Industry Standards Analysis

### 1.1 Sigstore (Cosign + Rekor)

**Overview**: CNCF's modern approach to artifact signing with keyless signing and transparency logs.

**Key Concepts**:
- **Cosign**: Signs and verifies container images and other artifacts
- **Rekor**: Append-only transparency log (immutable audit trail)
- **Keyless signing**: Uses OIDC identity tokens (GitHub Actions, etc.) instead of long-lived keys
- **Hardware token support**: YubiKey, FIDO2, PKCS11 integration

**Strengths**:
- Modern, actively developed (2025 standards)
- Post-quantum cryptography roadmap
- Excellent for CI/CD workflows
- Strong non-repudiation via transparency logs
- No key storage burden for keyless mode

**Weaknesses**:
- Requires internet connectivity for transparency log
- OIDC dependency for keyless mode
- Complexity for offline/air-gapped environments

**Applicability to AstraWeave**: 
- Excellent for CI/CD pipeline signing
- Rekor transparency log provides public audit trail
- May be overkill for single-developer/small team scenarios
- Strong candidate for production releases

### 1.2 The Update Framework (TUF)

**Overview**: Secure software update framework with multi-tiered key hierarchy.

**Key Architecture**:
```
Root Keys (offline, long-lived)
├── Targets Keys (sign asset metadata)
├── Snapshot Keys (consistency guarantees)
└── Timestamp Keys (freshness guarantees)
```

**Key Rotation Strategy**:
- Root keys: Rarely rotated, stored offline, require m-of-n threshold
- Targets keys: Rotated periodically, used for actual signing
- Automatic rollover prevents key compromise escalation

**Strengths**:
- Battle-tested (used by Docker Notary, Python PyPI, automotive OTA)
- Robust key rotation and revocation
- Handles offline signing workflows
- Role separation limits blast radius of key compromise
- Delegations allow hierarchical trust (e.g., per-environment, per-developer)

**Weaknesses**:
- Complex metadata schema
- Requires infrastructure to manage multiple key types
- Learning curve for operators

**Applicability to AstraWeave**:
- Perfect for multi-developer teams
- Excellent operational security model
- Supports offline signing (important for sensitive keys)
- Can delegate signing authority per asset type/environment

### 1.3 NPM Package Signing (Provenance)

**Overview**: NPM's 2024-2025 rollout of provenance attestations using SLSA framework.

**Features**:
- **Provenance attestation**: Links package to source repo + build environment
- **Publish attestation**: Signed by registry when published by authorized user
- **Verification**: `npm audit signatures` validates chain of trust

**Strengths**:
- Transparent to users (automatic generation in GitHub Actions)
- Low friction for developers
- Strong supply chain security guarantees

**Weaknesses**:
- GitHub-centric (assumes GitHub Actions)
- Requires registry infrastructure

**Applicability to AstraWeave**:
- Excellent model for asset pipeline transparency
- Could generate provenance for each bake/cook operation
- Links built assets back to source files + tool versions

### 1.4 Docker Content Trust (Notary)

**Overview**: TUF-based signing for container images.

**Key Management**:
- Root key: Generated once, stored offline
- Targets key: Per-repository, used for signing images
- Snapshot/Timestamp keys: Managed by Notary server
- Delegation keys: Optional, for multi-signer workflows

**Strengths**:
- Proven at scale (Docker Hub)
- Clear separation of online/offline keys
- Hardware token support (PKCS11)
- Delegation model allows team workflows

**Weaknesses**:
- Requires Notary server for online keys
- Complex key ceremony for initial setup

**Applicability to AstraWeave**:
- Strong model for key hierarchy
- Delegation keys could map to developer identities
- Server requirement may be too heavy for initial implementation

### 1.5 Linux Package Signing (APT/RPM)

**Overview**: Traditional PGP-based signing with GPG keys.

**Model**:
- Root CA signs package maintainer keys
- Maintainers sign packages with their keys
- Users have root CA public key pre-installed

**Strengths**:
- Simple, well-understood
- Widely deployed tooling
- Works offline

**Weaknesses**:
- GPG complexity and footguns
- No automatic key rotation
- Web of trust model doesn't scale well

**Applicability to AstraWeave**:
- Too heavyweight (GPG dependencies)
- Modern alternatives (age, minisign) better for new systems
- Key escrow and rotation are painful

---

## 2. Current Implementation Analysis

### 2.1 Existing Code

**File**: `tools/aw_asset_cli/src/main.rs:476-487`
```rust
fn sign_manifest(manifest: &[ManifestEntry]) -> Result<SignedManifest> {
    let rng = SystemRandom::new();
    let pkcs8_bytes = Ed25519KeyPair::generate_pkcs8(&rng)
        .map_err(|_| anyhow::anyhow!("Failed to generate key"))?;
    let key_pair = Ed25519KeyPair::from_pkcs8(pkcs8_bytes.as_ref())
        .map_err(|_| anyhow::anyhow!("Invalid key"))?;
    let manifest_json = serde_json::to_string(manifest)?;
    let signature = key_pair.sign(manifest_json.as_bytes());
    Ok(SignedManifest {
        entries: manifest.to_vec(),
        signature: base64::engine::general_purpose::STANDARD.encode(signature.as_ref()),
    })
}
```

**File**: `tools/asset_signing/src/lib.rs`
```rust
pub fn sign_asset(path: &str, private_key: &[u8]) -> Result<Vec<u8>, String> {
    let hash = hash_file(path)?;  // SHA-256
    let sk = SigningKey::from_bytes(private_key.try_into().unwrap());
    let sig: Signature = sk.sign(&hash);
    Ok(sig.to_bytes().to_vec())
}

pub fn verify_asset(path: &str, public_key: &[u8], signature: &[u8]) -> Result<bool, String> {
    let hash = hash_file(path)?;
    let vk = VerifyingKey::from_bytes(public_key.try_into().unwrap())?;
    let sig = Signature::from_bytes(signature.try_into().unwrap());
    Ok(vk.verify_strict(&hash, &sig).is_ok())
}
```

### 2.2 Security Issues

1. **Ephemeral Keys**: New key generated each run, no way to verify signatures later
2. **No Public Key Distribution**: Verifiers don't know what public key to trust
3. **No Key Rotation**: Cannot rotate compromised keys
4. **No Revocation**: Cannot mark signatures as invalid if key is compromised
5. **No Verification Workflow**: `verify_asset` exists but not integrated into asset loading
6. **No Key Storage Security**: Private keys not protected (if they were persistent)
7. **No Audit Trail**: No record of who signed what, when

### 2.3 Strengths to Preserve

1. **Ed25519 Choice**: Modern, fast, secure signature algorithm
2. **SHA-256 Hashing**: Industry standard, resistant to collisions
3. **Clean API**: `sign_asset` / `verify_asset` are simple and correct
4. **Dual Implementation**: Ring (main.rs) and ed25519-dalek (lib.rs) for flexibility

---

## 3. Recommended Architecture

### 3.1 Key Hierarchy (TUF-Inspired)

```
┌─────────────────────────────────────────────────────────────┐
│                    Root of Trust                            │
│  root.key (Ed25519, offline, encrypted, 10-year validity)   │
│  - Signs: targets keys, revocation lists                    │
│  - Storage: Hardware token OR encrypted file + passphrase   │
│  - Backup: Split across 2-3 locations (Shamir secret share?)│
└─────────────────────────────────────────────────────────────┘
                          │
          ┌───────────────┴───────────────┬─────────────────┐
          ▼                               ▼                 ▼
┌──────────────────────┐    ┌──────────────────────┐   ┌────────────────────┐
│  Targets Key (Dev)   │    │ Targets Key (CI/CD)  │   │ Targets Key (Prod) │
│  dev.targets.key     │    │ ci.targets.key       │   │ prod.targets.key   │
│  - Signs: dev assets │    │ - Signs: CI builds   │   │ - Signs: releases  │
│  - 1-year validity   │    │ - 90-day validity    │   │ - 1-year validity  │
│  - OS keyring        │    │ - GitHub Secrets     │   │ - HSM or YubiKey   │
└──────────────────────┘    └──────────────────────┘   └────────────────────┘
```

**Key Roles**:
- **Root Key**: Offline, long-lived, signs other keys
- **Targets Keys**: Environment-specific (dev/staging/prod), sign actual assets
- **Optional**: Timestamp/snapshot keys if implementing full TUF

**Validity Periods**:
- Root: 10 years (rarely rotated)
- Production targets: 1 year (annual rotation ceremony)
- CI/CD targets: 90 days (quarterly rotation via automation)
- Developer targets: 1 year (or per-project)

### 3.2 Key Storage Options

#### Option A: OS Keyring (Recommended for Developers)

**Crate**: `keyring-rs` (https://crates.io/crates/keyring)

**Platforms**:
- Windows: Windows Credential Manager
- macOS: Keychain
- Linux: Secret Service API (GNOME Keyring, KWallet)

**Pros**:
- Native OS integration
- Password manager UX (unlock once per session)
- Backed up with OS backups
- No plaintext keys on disk

**Cons**:
- Requires user interaction on first use
- Platform-specific quirks

**Implementation**:
```rust
use keyring::{Entry, Error};

fn store_signing_key(env: &str, key: &[u8]) -> Result<(), Error> {
    let entry = Entry::new("AstraWeave", &format!("signing_key_{}", env))?;
    entry.set_password(&base64::encode(key))?;
    Ok(())
}

fn load_signing_key(env: &str) -> Result<Vec<u8>, Error> {
    let entry = Entry::new("AstraWeave", &format!("signing_key_{}", env))?;
    let encoded = entry.get_password()?;
    Ok(base64::decode(&encoded).unwrap())
}
```

#### Option B: Hardware Security Module (HSM)

**Use Case**: Production releases, high-value assets

**Options**:
- **YubiKey 5**: FIDO2 + PIV, ~$50, PKCS11 interface
- **AWS CloudHSM**: FIPS 140-2 Level 3, $1/hour + $1.16/key/month
- **Azure Key Vault**: HSM-backed keys, $1/key/month
- **GCP Cloud KMS**: HSM-backed, $0.06/10k operations

**YubiKey Example** (PKCS11):
```rust
// Using pkcs11 crate (wraps PKCS#11 C API)
use pkcs11::Ctx;

fn sign_with_yubikey(data: &[u8]) -> Result<Vec<u8>> {
    let ctx = Ctx::new_and_initialize("/usr/local/lib/libykcs11.so")?;
    let slots = ctx.get_slot_list(true)?;
    let slot = slots[0];
    // ... PKCS11 ceremony to load key, sign ...
}
```

**Pros**:
- Keys cannot be extracted
- Tamper-evident hardware
- Compliance-friendly (FIPS 140-2)

**Cons**:
- Cost (cloud HSMs) or friction (YubiKey ceremony)
- Requires PKCS11 interface

#### Option C: Encrypted File (age)

**Use Case**: CI/CD secrets, air-gapped environments

**Tool**: `age` (modern, simple encryption; https://age-encryption.org/)

**Workflow**:
1. Generate age keypair: `age-keygen -o key.txt`
2. Encrypt signing key: `age -r <pubkey> -o signing.key.age signing.key`
3. Decrypt in CI: `age -d -i ~/.age/key.txt signing.key.age`

**Pros**:
- Simple, no GPG complexity
- Good for automation (non-interactive)
- Works offline

**Cons**:
- Requires age binary
- Passphrase management still needed

**Rust Integration**:
```rust
// Use age crate or shell out to age binary
use std::process::Command;

fn decrypt_key(encrypted_path: &str, identity_path: &str) -> Result<Vec<u8>> {
    let output = Command::new("age")
        .args(&["-d", "-i", identity_path, encrypted_path])
        .output()?;
    Ok(output.stdout)
}
```

#### Option D: Cloud KMS (AWS/Azure/GCP)

**Use Case**: Cloud-native deployments, team workflows

**Example**: AWS KMS
```rust
use aws_sdk_kms as kms;

async fn sign_with_kms(data: &[u8], key_id: &str) -> Result<Vec<u8>> {
    let config = aws_config::load_from_env().await;
    let client = kms::Client::new(&config);
    
    let resp = client
        .sign()
        .key_id(key_id)
        .message(aws_sdk_kms::types::Blob::new(data))
        .signing_algorithm(kms::types::SigningAlgorithmSpec::EcdsaSha256)
        .send()
        .await?;
    
    Ok(resp.signature.unwrap().into_inner())
}
```

**Pros**:
- Managed infrastructure
- Automatic backups and HA
- Audit logs (CloudTrail, etc.)
- IAM-based access control

**Cons**:
- Vendor lock-in
- Cost at scale
- Latency (network calls)

### 3.3 Recommended Approach (Tiered)

**Tier 1: Local Development**
- Storage: OS Keyring via `keyring-rs`
- Key Type: Developer-specific targets key
- Validity: 1 year
- Workflow: Generate on first use, auto-loaded thereafter

**Tier 2: CI/CD**
- Storage: GitHub Secrets (encrypted) + age decryption
- Key Type: CI targets key
- Validity: 90 days
- Workflow: Encrypted key in repo, decrypted via GitHub Actions secret

**Tier 3: Production Releases**
- Storage: YubiKey (small teams) OR Cloud HSM (large teams)
- Key Type: Production targets key
- Validity: 1 year
- Workflow: Manual ceremony (YubiKey) OR automated (HSM API)

**Root Key** (all tiers):
- Storage: Encrypted file (age) + Shamir secret sharing (split across 3 maintainers)
- Usage: Only for key rotation and revocation
- Ceremony: Annual or on-demand (key compromise)

---

## 4. Key Rotation Strategy

### 4.1 Root Key Rotation

**Trigger**: 10-year expiry OR key compromise

**Process**:
1. Generate new root key (offline, air-gapped machine)
2. Sign new root key with old root key (establishes chain)
3. Sign all targets keys with new root key
4. Publish root key rotation notice (transparency log or signed announcement)
5. Update embedded root public key in next engine release
6. Grace period: 1 year (both old and new root keys trusted)
7. After grace period: Old root key fully retired

**Implementation**:
```rust
struct RootKeyRotation {
    old_root_pubkey: [u8; 32],
    new_root_pubkey: [u8; 32],
    rotation_date: DateTime<Utc>,
    grace_period_end: DateTime<Utc>,
    signature_over_new: Vec<u8>, // Signed by old root key
}
```

### 4.2 Targets Key Rotation

**Trigger**: Scheduled (90-365 days) OR key compromise

**Process**:
1. Generate new targets key
2. Sign new targets key with root key
3. Publish new targets key certificate
4. Update signing workflow to use new key
5. Grace period: 30 days (both keys valid)
6. After grace period: Old targets key revoked

**Automation** (CI/CD):
```yaml
# .github/workflows/rotate-signing-key.yml
name: Rotate CI Signing Key
on:
  schedule:
    - cron: '0 0 1 */3 *'  # Every 3 months
jobs:
  rotate:
    runs-on: ubuntu-latest
    steps:
      - name: Generate new targets key
        run: aw_asset_cli keygen --role ci-targets --output ci.targets.new.key
      - name: Sign with root key
        run: aw_asset_cli sign-key --root-key ${{ secrets.ROOT_KEY_ENCRYPTED }} --targets-key ci.targets.new.key
      - name: Update GitHub secret
        run: gh secret set CI_SIGNING_KEY < ci.targets.new.key
```

### 4.3 Emergency Revocation

**Scenario**: Private key leaked/compromised

**Immediate Actions**:
1. Publish revocation certificate (signed by root key)
2. Update revocation list (CRL)
3. Notify users via security advisory
4. Rotate to new key immediately (no grace period)

**Revocation Certificate Format**:
```rust
struct RevocationCertificate {
    revoked_pubkey: [u8; 32],
    reason: RevocationReason,
    revocation_date: DateTime<Utc>,
    signature: Vec<u8>, // Signed by root key
}

enum RevocationReason {
    KeyCompromise,
    SupersededByNewKey,
    CessationOfOperation,
}
```

---

## 5. Public Key Distribution

### 5.1 Embedded in Binary (Recommended)

**Approach**: Compile root public key into engine binary

**Implementation**:
```rust
// astraweave-asset/src/signing.rs
pub const ROOT_PUBKEY: [u8; 32] = [
    0x1a, 0x2b, 0x3c, /* ... root public key bytes ... */
];

pub const TRUSTED_TARGETS_KEYS: &[[u8; 32]] = &[
    [ /* dev key */ ],
    [ /* ci key */ ],
    [ /* prod key */ ],
];
```

**Pros**:
- Always available (no network dependency)
- Tamper-evident (if binary is signed)
- Simple, no distribution infrastructure

**Cons**:
- Key rotation requires engine update
- Separate builds for different key sets (dev/prod)

**Mitigation**: Grace period for key rotation (old + new keys trusted for 1 year)

### 5.2 Separate Keyring File

**Approach**: Distribute `trusted_keys.toml` alongside assets

**Format**:
```toml
[root]
pubkey = "1a2b3c...root key hex..."
valid_from = "2025-01-01T00:00:00Z"
valid_until = "2035-01-01T00:00:00Z"

[[targets]]
role = "production"
pubkey = "4d5e6f...prod targets key hex..."
valid_from = "2025-01-01T00:00:00Z"
valid_until = "2026-01-01T00:00:00Z"
signed_by = "root"

[[targets]]
role = "ci"
pubkey = "7a8b9c...ci targets key hex..."
valid_from = "2025-01-01T00:00:00Z"
valid_until = "2025-04-01T00:00:00Z"
signed_by = "root"
```

**Pros**:
- Key rotation without engine rebuild
- Environment-specific keyrings (dev vs prod)

**Cons**:
- Keyring itself must be trusted (bootstrap problem)
- Users might ignore/skip updates

**Solution**: Sign keyring file with root key; verify on load

### 5.3 Online Keyserver (Future)

**Approach**: HTTP API for key lookup (like PGP keyservers)

**Endpoints**:
- `GET /keys/root` → Latest root public key
- `GET /keys/targets/{role}` → Targets key for specific role
- `GET /revocations` → CRL (Certificate Revocation List)

**Pros**:
- Always up-to-date keys
- Centralized revocation

**Cons**:
- Requires internet connectivity
- Single point of failure
- Privacy concerns (key lookup leaks asset usage)

**Security**: Pin root key in binary; keyserver provides targets keys signed by root

### 5.4 Transparency Log (Rekor-like)

**Approach**: Public append-only log of all signing events

**Benefits**:
- Immutable audit trail
- Public accountability
- Detect mis-issuance or compromise
- Non-repudiation (signer cannot deny)

**Implementation Options**:
- **Hosted Rekor**: Use Sigstore's public instance (https://rekor.sigstore.dev)
- **Self-hosted Rekor**: Run own instance for privacy
- **Custom transparency log**: Simpler append-only log (SQLite + Merkle tree)

**Workflow**:
1. Sign asset → generate signature
2. Submit signature + metadata to transparency log
3. Log returns inclusion proof (Merkle tree path)
4. Include inclusion proof in asset manifest
5. Verifier checks: (a) signature valid, (b) signature in log

**Privacy Considerations**:
- Public log reveals all asset names/hashes → Use hash(hash(asset)) in log for anonymity
- Private log requires trusted operator

**Recommended**: Start without transparency log; add later for public releases

---

## 6. Verification Workflow

### 6.1 Runtime Verification (Engine)

**When**: Asset loading during gameplay

**Requirements**:
- Low latency (< 1ms per asset)
- No network calls (offline gameplay)
- Fail-safe defaults (reject unsigned assets in production)

**Implementation**:
```rust
// astraweave-asset/src/loader.rs
use crate::signing::{verify_asset, ROOT_PUBKEY, TRUSTED_TARGETS_KEYS};

pub struct AssetLoader {
    verify_signatures: bool, // true in release builds
    trusted_keys: Vec<[u8; 32]>,
}

impl AssetLoader {
    pub fn load_asset(&self, path: &Path) -> Result<Asset> {
        let asset_data = fs::read(path)?;
        
        if self.verify_signatures {
            let sig_path = path.with_extension("sig");
            let signature = fs::read(sig_path)?;
            
            // Try each trusted key until one succeeds
            let verified = self.trusted_keys.iter().any(|pubkey| {
                verify_asset_bytes(&asset_data, pubkey, &signature).unwrap_or(false)
            });
            
            if !verified {
                return Err(AssetError::InvalidSignature(path.to_owned()));
            }
        }
        
        // Parse asset...
    }
}

// Optimized for batch verification
pub fn verify_manifest(manifest: &SignedManifest, pubkey: &[u8; 32]) -> Result<bool> {
    let manifest_json = serde_json::to_string(&manifest.entries)?;
    let sig = base64::decode(&manifest.signature)?;
    
    let vk = VerifyingKey::from_bytes(pubkey)?;
    let sig = Signature::from_bytes(&sig.try_into().unwrap());
    Ok(vk.verify_strict(manifest_json.as_bytes(), &sig).is_ok())
}
```

**Performance**: Ed25519 verification is ~50,000 ops/sec on modern CPU; negligible overhead

### 6.2 Build-Time Verification (CI/CD)

**When**: Asset pipeline validation before deployment

**Requirements**:
- Comprehensive (verify all assets)
- Detailed errors (which asset failed, why)
- Block deployment on failure

**Implementation**:
```bash
# .github/workflows/build.yml
- name: Verify signed assets
  run: |
    aw_asset_cli verify-all \
      --manifest cooked_assets/manifest.json \
      --pubkey ${{ secrets.ROOT_PUBKEY }} \
      --strict
```

**CLI Tool**:
```rust
// tools/aw_asset_cli/src/main.rs
fn verify_all_command(manifest_path: &Path, pubkey: &str, strict: bool) -> Result<()> {
    let manifest: SignedManifest = serde_json::from_str(&fs::read_to_string(manifest_path)?)?;
    let pubkey_bytes = hex::decode(pubkey)?;
    
    if !verify_manifest(&manifest, &pubkey_bytes.try_into().unwrap())? {
        bail!("Manifest signature verification failed");
    }
    
    println!("✓ Manifest signature valid");
    
    let mut failed = Vec::new();
    for entry in &manifest.entries {
        let path = Path::new(&entry.output_path);
        let sig_path = path.with_extension("sig");
        
        if !sig_path.exists() {
            if strict {
                failed.push(format!("{}: missing signature", entry.output_path));
            }
            continue;
        }
        
        let signature = fs::read(sig_path)?;
        if !verify_asset(path.to_str().unwrap(), &pubkey_bytes, &signature)? {
            failed.push(format!("{}: invalid signature", entry.output_path));
        }
    }
    
    if !failed.is_empty() {
        bail!("Verification failed:\n{}", failed.join("\n"));
    }
    
    println!("✓ All {} assets verified", manifest.entries.len());
    Ok(())
}
```

### 6.3 Developer Tools Verification

**Use Case**: `aw_asset_cli` validates assets before commit

**Command**:
```bash
aw_asset_cli validate-assets \
  --path assets/ \
  --config aw_pipeline.toml \
  --verify-signatures \
  --format json
```

**Output** (JSON):
```json
{
  "total_assets": 42,
  "verified": 40,
  "unsigned": 1,
  "invalid_signatures": 1,
  "errors": [
    { "asset": "textures/missing.png", "error": "signature file not found" },
    { "asset": "models/corrupt.glb", "error": "signature verification failed" }
  ]
}
```

---

## 7. Signing Workflow (Offline + CI/CD)

### 7.1 Developer Workflow (Local Signing)

**Scenario**: Developer bakes assets on local machine

**Setup** (once):
```bash
# Generate developer targets key
aw_asset_cli keygen --role dev --output ~/.astraweave/dev.targets.key

# Store in OS keyring
aw_asset_cli keyring-store --role dev --keyfile ~/.astraweave/dev.targets.key

# Shred the plaintext file
shred -u ~/.astraweave/dev.targets.key

# Get public key for root signing
aw_asset_cli pubkey --role dev > dev.pubkey
# Send dev.pubkey to maintainer for root key signature
```

**Daily Usage**:
```bash
# Bake texture (auto-signs with dev key from keyring)
aw_asset_cli bake-texture input.png --output baked/

# Cook full pipeline (signs manifest)
aw_asset_cli cook aw_pipeline.toml

# Verify before commit
aw_asset_cli verify-all --manifest cooked_assets/manifest.json
```

**Under the Hood**:
```rust
fn sign_with_stored_key(role: &str, data: &[u8]) -> Result<Vec<u8>> {
    // Load from OS keyring
    let entry = Entry::new("AstraWeave", &format!("signing_key_{}", role))?;
    let key_b64 = entry.get_password()?;
    let key_bytes = base64::decode(&key_b64)?;
    
    // Sign
    let sk = SigningKey::from_bytes(&key_bytes.try_into().unwrap());
    let sig = sk.sign(data);
    Ok(sig.to_bytes().to_vec())
}
```

### 7.2 CI/CD Workflow (GitHub Actions)

**Setup** (in repository):
```bash
# Generate CI targets key (offline, secure machine)
aw_asset_cli keygen --role ci --output ci.targets.key

# Encrypt with age (for repo storage)
age -r github:<org>/<repo> -o ci.targets.key.age ci.targets.key

# Add age private key to GitHub Secrets: AGE_SECRET_KEY
# Commit ci.targets.key.age to repo
git add ci.targets.key.age && git commit -m "Add CI signing key"
```

**Workflow** (`.github/workflows/build-assets.yml`):
```yaml
name: Build and Sign Assets

on:
  push:
    branches: [main]
    paths: ['assets/**']

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install age
        run: |
          wget https://github.com/FiloSottile/age/releases/download/v1.1.1/age-v1.1.1-linux-amd64.tar.gz
          tar xzf age-v1.1.1-linux-amd64.tar.gz
          sudo mv age/age /usr/local/bin/
      
      - name: Decrypt signing key
        env:
          AGE_SECRET_KEY: ${{ secrets.AGE_SECRET_KEY }}
        run: |
          echo "$AGE_SECRET_KEY" > /tmp/age_key.txt
          age -d -i /tmp/age_key.txt ci.targets.key.age > /tmp/ci.signing.key
          shred -u /tmp/age_key.txt
      
      - name: Build Rust tools
        run: cargo build --release -p aw_asset_cli
      
      - name: Cook assets
        run: |
          ./target/release/aw_asset_cli cook aw_pipeline.toml \
            --sign-with /tmp/ci.signing.key
      
      - name: Verify signed assets
        run: |
          ./target/release/aw_asset_cli verify-all \
            --manifest cooked_assets/manifest.json \
            --pubkey $(cat ci.targets.pubkey)
      
      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: signed-assets
          path: cooked_assets/
      
      - name: Cleanup
        if: always()
        run: shred -u /tmp/ci.signing.key
```

### 7.3 Production Workflow (Air-Gapped Signing)

**Scenario**: Official release signing with production key on offline machine

**Setup**:
- Air-gapped machine (never connected to network)
- YubiKey with production targets key
- USB transfer of unsigned assets → signed assets

**Process**:
1. Build assets on CI (unsigned)
2. Download unsigned artifacts to USB
3. Transfer USB to air-gapped machine
4. Insert YubiKey
5. Run signing ceremony:
   ```bash
   aw_asset_cli sign-release \
     --manifest /mnt/usb/unsigned_manifest.json \
     --output /mnt/usb/signed_manifest.json \
     --yubikey \
     --slot 9c  # PIV signing slot
   ```
6. YubiKey prompts for PIN/touch
7. Transfer signed assets back via USB
8. Upload to distribution server

**YubiKey Integration** (PKCS11):
```rust
// tools/aw_asset_cli/src/yubikey.rs
use cryptoki::{context::Pkcs11, session::Session, types::mechanism::Mechanism};

pub fn sign_with_yubikey(data: &[u8], slot: u8) -> Result<Vec<u8>> {
    let ctx = Pkcs11::new("/usr/local/lib/libykcs11.so")?;
    ctx.initialize()?;
    
    let slots = ctx.get_slots()?;
    let slot = slots.first().ok_or("No YubiKey found")?;
    
    let session = ctx.open_session(*slot, true)?;
    session.login(cryptoki::types::session::UserType::User, Some("123456"))?; // PIN
    
    let key_handle = find_signing_key(&session)?;
    let signature = session.sign(&Mechanism::Ecdsa, key_handle, data)?;
    
    session.logout()?;
    Ok(signature)
}
```

---

## 8. Migration Path from Current System

### Phase 1: Add Persistent Keys (Weeks 1-2)

**Goal**: Stop generating ephemeral keys; use persistent developer keys

**Tasks**:
1. Implement `keyring-rs` integration
2. Add `aw_asset_cli keygen` command
3. Add `aw_asset_cli keyring-store` command
4. Modify `sign_manifest()` to load key from keyring (fallback to env var)
5. Document developer setup

**Testing**:
```bash
aw_asset_cli keygen --role dev
aw_asset_cli cook aw_pipeline.toml  # Should use stored key
aw_asset_cli cook aw_pipeline.toml  # Should use SAME key (verify signature matches)
```

**Code Changes**:
```rust
// tools/aw_asset_cli/src/main.rs
fn sign_manifest(manifest: &[ManifestEntry], role: Option<&str>) -> Result<SignedManifest> {
    let key_bytes = if let Some(role) = role {
        load_key_from_keyring(role)?
    } else if let Ok(key_hex) = env::var("ASTRAWEAVE_SIGNING_KEY") {
        hex::decode(key_hex)?
    } else {
        // Fallback: warn and generate ephemeral (backwards compat)
        eprintln!("WARNING: No signing key found, generating ephemeral key");
        eprintln!("Run 'aw_asset_cli keygen' to create persistent key");
        generate_ephemeral_key()?
    };
    
    sign_manifest_with_key(manifest, &key_bytes)
}
```

### Phase 2: Implement Verification (Weeks 3-4)

**Goal**: Engine can verify signed assets at runtime

**Tasks**:
1. Embed root public key in `astraweave-asset` crate
2. Implement `AssetLoader::verify_signatures` flag
3. Add `.sig` file generation to asset cooking
4. Add `#[cfg(debug_assertions)]` to skip verification in dev builds
5. Add `verify-all` CLI command for CI

**Testing**:
```rust
#[test]
fn test_runtime_verification() {
    let loader = AssetLoader::new(true); // Enable verification
    
    // Should succeed with valid signature
    let asset = loader.load_asset("test.png").unwrap();
    
    // Should fail with tampered signature
    fs::write("test.png.sig", b"invalid").unwrap();
    assert!(loader.load_asset("test.png").is_err());
}
```

### Phase 3: Key Hierarchy (Weeks 5-6)

**Goal**: Separate root key from targets keys

**Tasks**:
1. Generate root key (offline ceremony)
2. Sign developer/CI targets keys with root key
3. Implement key certificate format
4. Update verification to check certificate chain: asset signature → targets key → root key
5. Document root key backup procedure (Shamir splitting)

**Certificate Format**:
```rust
struct KeyCertificate {
    subject_pubkey: [u8; 32],
    subject_role: String, // "dev", "ci", "production"
    issuer_pubkey: [u8; 32], // Root key
    valid_from: DateTime<Utc>,
    valid_until: DateTime<Utc>,
    signature: Vec<u8>, // Signed by issuer
}
```

### Phase 4: CI/CD Integration (Weeks 7-8)

**Goal**: Automated signing in GitHub Actions

**Tasks**:
1. Create CI targets key
2. Encrypt with age
3. Set up GitHub Actions workflow
4. Test full pipeline: commit → build → sign → verify → deploy
5. Add security scanning (check for leaked keys)

**Security Audit**:
```bash
# Scan repo for accidentally committed keys
gitleaks detect --source . --verbose

# Check GitHub Actions for secret exposure
gh secret list
```

### Phase 5: Production Hardening (Weeks 9-12)

**Goal**: Signing with YubiKey/HSM

**Tasks**:
1. Acquire YubiKey or provision cloud HSM
2. Implement PKCS11 integration for YubiKey
3. Document air-gapped signing ceremony
4. Set up key rotation automation (cron job)
5. Implement revocation checking
6. Create incident response plan (key compromise)

**Deliverables**:
- [ ] Signing ceremony runbook
- [ ] Key rotation automation scripts
- [ ] Emergency revocation procedure
- [ ] User-facing documentation (how to verify assets)

### Phase 6: Transparency Log (Optional, Weeks 13-16)

**Goal**: Public audit trail of all signatures

**Tasks**:
1. Evaluate Rekor vs custom implementation
2. Set up transparency log infrastructure
3. Integrate log submission into signing workflow
4. Add log verification to asset loading
5. Public dashboard (https://transparency.astraweave.dev)

---

## 9. Security Properties

### 9.1 Achieved Properties

**Non-Repudiation**:
- Signatures prove who signed an asset (via key certificates)
- Transparency log provides immutable audit trail
- Cannot deny signing if private key was controlled

**Integrity**:
- SHA-256 hash prevents tampering
- Ed25519 signature detects any modification
- Verification failures prevent loading corrupted assets

**Authenticity**:
- Only holders of private key can sign
- Root key hierarchy establishes chain of trust
- Users verify against known root public key

**Confidentiality** (of keys):
- Private keys never leave secure storage (keyring/HSM/YubiKey)
- Encrypted at rest (OS keyring, age encryption)
- Never transmitted over network

**Forward Secrecy**:
- Key rotation limits blast radius of compromise
- Old signatures remain valid during grace period
- New assets cannot be signed with revoked keys

**Auditability**:
- Every signature includes timestamp, signer identity (via key role)
- Transparency log (optional) provides public record
- Revocation list shows compromised keys

### 9.2 Threat Model

**Protected Against**:
- ✅ Asset tampering (modified textures, models, code)
- ✅ Supply chain attacks (malicious asset injection)
- ✅ Insider threats (unauthorized asset signing)
- ✅ Key compromise (limited by rotation + revocation)
- ✅ Replay attacks (timestamps + manifest binding)

**NOT Protected Against**:
- ❌ Root key compromise (game over; requires new root + engine update)
- ❌ Compromised build system (signs malicious assets with valid key)
- ❌ Vulnerabilities in signature verification code (implementation bugs)
- ❌ Social engineering (tricking maintainer to sign malicious assets)
- ❌ Post-quantum attacks (Ed25519 vulnerable to quantum computers; mitigated by TUF's algorithm agility)

### 9.3 Compliance Alignment

**SLSA Framework** (Supply Chain Levels for Software Artifacts):
- Level 1: Provenance exists (manifests + signatures)
- Level 2: Signed provenance (transparency log)
- Level 3: Hardened builds (CI/CD with isolated signing keys)
- Level 4: Reviewable provenance (public transparency log)

**NIST SP 800-57** (Key Management):
- Key lifecycle: Generation → storage → rotation → revocation → destruction
- Cryptoperiod: Defined validity periods for all keys
- Key strength: Ed25519 ~128-bit security (meets 2030+ requirements)

**PCI DSS** (if handling payment data in assets):
- Requirement 6.3.2: Secure key management
- Requirement 10: Audit trails (transparency log)

---

## 10. Operational Considerations

### 10.1 Key Backup and Disaster Recovery

**Root Key Backup**:
- **Method**: Shamir secret sharing (2-of-3 threshold)
- **Storage**: 
  - Share 1: Password manager (1Password, Bitwarden)
  - Share 2: Encrypted USB drive in safe deposit box
  - Share 3: Trusted co-founder/CTO
- **Recovery**: Any 2 shares can reconstruct root key
- **Test**: Quarterly recovery drill

**Targets Key Backup**:
- **Method**: Encrypted age files (password-protected)
- **Storage**: 
  - Primary: OS keyring (daily use)
  - Backup: Encrypted file in password manager
- **Recovery**: Decrypt from backup, reimport to keyring

**Disaster Scenarios**:
| Scenario | Impact | Recovery |
|----------|--------|----------|
| Lost developer laptop | Low | Use backup, rotate key |
| CI key leaked | Medium | Emergency rotation, revoke old key |
| Root key lost | High | Use Shamir shares to reconstruct |
| Root key compromised | Critical | Generate new root, notify all users, engine update |

### 10.2 Key Rotation Schedule

**Automated** (GitHub Actions cron):
- CI targets key: Every 90 days
- Timestamp key (if using TUF): Daily

**Manual** (calendar reminders):
- Developer targets keys: Annually (birthday month?)
- Production targets key: Annually (Q1)
- Root key: Every 10 years OR on-demand

**Rotation Checklist**:
1. [ ] Generate new key
2. [ ] Sign with appropriate parent key (root for targets, etc.)
3. [ ] Update trusted keys list
4. [ ] Deploy updated keyring
5. [ ] Test old signatures still verify (grace period)
6. [ ] Monitor for errors
7. [ ] After grace period: Revoke old key
8. [ ] Update documentation

### 10.3 Monitoring and Alerting

**Key Usage Metrics**:
- Signatures created per day (per key)
- Verification failures (per asset type)
- Key age (alert when approaching expiry)

**Alerts**:
- **Critical**: Root key used (should be rare)
- **High**: Verification failure rate > 1%
- **Medium**: Key expiring in < 30 days
- **Low**: Unusual signing activity (e.g., 10x normal volume)

**Implementation** (Prometheus + Grafana):
```rust
// Increment counter on each signature
lazy_static! {
    static ref SIGNATURES_CREATED: IntCounterVec = register_int_counter_vec!(
        "astraweave_signatures_created_total",
        "Number of asset signatures created",
        &["key_role"]
    ).unwrap();
}

fn sign_asset_with_metrics(asset: &str, role: &str) -> Result<Vec<u8>> {
    let sig = sign_asset_inner(asset, role)?;
    SIGNATURES_CREATED.with_label_values(&[role]).inc();
    Ok(sig)
}
```

### 10.4 Incident Response Plan

**Key Compromise Detected**:
1. **Immediate** (< 1 hour):
   - Revoke compromised key
   - Generate and deploy new key
   - Publish revocation certificate
2. **Short-term** (< 24 hours):
   - Notify users via security advisory
   - Re-sign all assets with new key
   - Audit: What was signed with compromised key?
3. **Medium-term** (< 1 week):
   - Forensics: How was key compromised?
   - Post-mortem document
   - Update security procedures
4. **Long-term**:
   - Review and improve key storage (HSM upgrade?)
   - Security training for team

**Template: Security Advisory**
```markdown
# Security Advisory: Asset Signing Key Compromised

**Date**: 2025-11-15
**Severity**: High
**CVE**: CVE-2025-XXXXX

## Summary
The CI asset signing key was compromised due to [root cause]. Assets signed 
between [date range] should be re-verified.

## Impact
Assets signed with the compromised key (fingerprint: `abc123...`) may not be 
trustworthy. No evidence of malicious use has been found.

## Remediation
1. Update AstraWeave engine to version X.Y.Z (includes new trusted keys)
2. Re-download assets dated after [cutoff date]
3. Run `aw_asset_cli verify-all` to check your assets

## Timeline
- 2025-11-14 10:00 UTC: Compromise detected
- 2025-11-14 10:30 UTC: Key revoked
- 2025-11-14 12:00 UTC: New key deployed
- 2025-11-15 08:00 UTC: All assets re-signed

## Contact
security@astraweave.dev
```

---

## 11. Implementation Plan

### 11.1 Scope and Priorities

**P0 (Must-Have for Production)**:
- [x] Analysis and design (this document)
- [ ] Persistent key storage (OS keyring)
- [ ] Runtime signature verification
- [ ] CI/CD signing automation
- [ ] Key rotation for targets keys
- [ ] Revocation mechanism

**P1 (Should-Have for Scale)**:
- [ ] Root key hierarchy
- [ ] Key certificates (signed by root)
- [ ] YubiKey/HSM support (production releases)
- [ ] Transparency log (basic)
- [ ] Comprehensive documentation

**P2 (Nice-to-Have for Future)**:
- [ ] Public transparency dashboard
- [ ] Shamir secret sharing for root key
- [ ] Multi-signature support (m-of-n signers)
- [ ] Keyserver infrastructure
- [ ] Post-quantum migration path

### 11.2 Timeline Estimate

**Assuming 1 full-time developer**:

| Phase | Duration | Deliverables |
|-------|----------|--------------|
| Phase 1: Persistent Keys | 2 weeks | `keygen`, keyring integration |
| Phase 2: Verification | 2 weeks | Runtime verification, `.sig` files |
| Phase 3: Key Hierarchy | 2 weeks | Root key, certificates |
| Phase 4: CI/CD | 2 weeks | GitHub Actions, age encryption |
| Phase 5: Production Hardening | 4 weeks | YubiKey, rotation automation, docs |
| Phase 6: Transparency Log | 4 weeks | Rekor integration, dashboard |
| **Total** | **16 weeks** | Full production system |

**Minimum Viable Signing (MVS)**: Phases 1-2 (4 weeks)

### 11.3 Concrete Code Changes

**New Files**:
```
tools/aw_asset_cli/src/
├── keygen.rs          # Key generation CLI
├── keyring.rs         # OS keyring integration (keyring-rs)
├── signing.rs         # High-level signing API
├── verification.rs    # Verification workflows
└── yubikey.rs         # PKCS11 / YubiKey support

astraweave-asset/src/
├── signing.rs         # Signature verification during asset loading
└── keys.rs            # Embedded trusted keys

tools/
├── ASSET_SIGNING_DESIGN.md  # This document
├── key_rotation.sh          # Automation scripts
└── signing_ceremony.md      # Runbook for offline signing
```

**Modified Files**:
```
tools/aw_asset_cli/src/main.rs
  - Modify sign_manifest() to use persistent keys
  - Add verify-all subcommand

tools/aw_asset_cli/Cargo.toml
  + keyring = "2.0"
  + age = "0.9"
  + cryptoki = "0.6"  # PKCS11

tools/asset_signing/src/lib.rs
  - No changes (API is already correct)

astraweave-asset/src/loader.rs
  + Integrate signature verification
```

**New Dependencies**:
```toml
[dependencies]
keyring = "2.0"               # OS credential storage
age = "0.9"                   # Modern encryption (for CI keys)
cryptoki = "0.6"              # PKCS11 (for YubiKey/HSM)
ed25519-dalek = "2.0"         # Already present
sha2 = "0.10"                 # Already present
serde = { version = "1", features = ["derive"] }
chrono = "0.4"                # Timestamps, expiry
```

### 11.4 Testing Strategy

**Unit Tests**:
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_sign_and_verify_roundtrip() { /* ... */ }
    
    #[test]
    fn test_key_rotation_grace_period() { /* ... */ }
    
    #[test]
    fn test_revoked_key_rejected() { /* ... */ }
    
    #[test]
    fn test_expired_key_rejected() { /* ... */ }
    
    #[test]
    fn test_tampered_asset_rejected() { /* ... */ }
}
```

**Integration Tests**:
```bash
# tests/integration/signing_workflow.sh

# Generate root key
aw_asset_cli keygen --role root --output /tmp/root.key

# Generate and sign targets key
aw_asset_cli keygen --role dev --output /tmp/dev.key
aw_asset_cli sign-key --root /tmp/root.key --targets /tmp/dev.key

# Cook assets with signing
aw_asset_cli cook --sign-with /tmp/dev.key test_pipeline.toml

# Verify
aw_asset_cli verify-all --manifest cooked_assets/manifest.json --pubkey /tmp/root.pubkey

# Should succeed
assert_exit_code 0

# Tamper with asset
echo "tampered" >> cooked_assets/test.png

# Verify should fail
aw_asset_cli verify-all --manifest cooked_assets/manifest.json --pubkey /tmp/root.pubkey
assert_exit_code 1
```

**Performance Tests**:
```rust
#[bench]
fn bench_verify_1000_assets(b: &mut Bencher) {
    let assets = generate_test_assets(1000);
    b.iter(|| {
        for asset in &assets {
            verify_asset(asset, PUBKEY, &asset.signature).unwrap();
        }
    });
}
// Target: < 20ms for 1000 assets (50k/sec verification rate)
```

### 11.5 Documentation Deliverables

**User-Facing**:
1. **Quickstart Guide** (`docs/asset_signing_quickstart.md`)
   - How to verify downloaded assets
   - How to check signature validity
   - What to do if verification fails

2. **Developer Guide** (`docs/asset_signing_developer.md`)
   - Setting up signing keys
   - Daily workflow (auto-signing)
   - Troubleshooting verification errors

**Maintainer-Facing**:
3. **Key Management Runbook** (`tools/signing_ceremony.md`)
   - Root key generation (offline)
   - Targets key rotation
   - Emergency revocation procedure

4. **Architecture Decision Record** (`docs/adr/0042-asset-signing.md`)
   - Why Ed25519 over RSA/ECDSA?
   - Why TUF-inspired hierarchy?
   - Why OS keyring over GPG?

**Security**:
5. **Threat Model** (`SECURITY.md`)
   - What signatures protect against
   - What they don't protect against
   - How to report key compromise

---

## 12. Comparison Matrix

### 12.1 Key Storage Options

| Option | Security | UX | Cost | Offline | Recommended For |
|--------|----------|----|----|---------|-----------------|
| **OS Keyring** | Medium | Excellent | Free | Yes | Developers |
| **YubiKey** | High | Good | $50 | Yes | Production releases |
| **AWS KMS** | High | Good | ~$100/mo | No | Cloud-native teams |
| **age-encrypted** | Medium | Fair | Free | Yes | CI/CD |
| **GPG** | Medium | Poor | Free | Yes | Legacy compat |
| **Plaintext file** | Low | Excellent | Free | Yes | **NEVER** |

### 12.2 Signing Standards

| Standard | Complexity | Maturity | Rust Support | Use Case |
|----------|-----------|----------|--------------|----------|
| **Sigstore** | Medium | High (2021+) | Good (sigstore-rs) | Cloud-native, keyless |
| **TUF** | High | Very High (2010+) | Medium (tough-rs) | Update systems |
| **NPM Provenance** | Low | Medium (2024+) | N/A (npm-specific) | Reference only |
| **Docker Notary** | High | High (2015+) | Medium | Container images |
| **Custom Ed25519** | Low | N/A | Excellent | Simple, greenfield |

**Recommendation**: **Custom Ed25519 with TUF-inspired hierarchy**
- Rationale: AstraWeave is greenfield (no legacy constraints), needs simplicity (small team), but should follow proven patterns (TUF hierarchy)

---

## 13. FAQ and Troubleshooting

### Q: Why Ed25519 instead of RSA?

**A**: Ed25519 is modern (2011), faster (50k verifications/sec vs 10k for RSA-2048), smaller signatures (64 bytes vs 256 bytes), and harder to implement incorrectly. RSA requires careful padding (PKCS#1 v1.5 vs PSS) and key size selection (2048 bits minimum today, 4096 future-proof). Ed25519 has one standard way to use it.

### Q: What if a developer loses their signing key?

**A**: 
1. Developer generates new key: `aw_asset_cli keygen --role dev`
2. Maintainer signs new key with root key
3. Old key is revoked (added to CRL)
4. Assets signed with old key remain valid during grace period (30 days)
5. After grace period, old signatures rejected (users must re-download)

### Q: Can I use existing GPG keys?

**A**: No. GPG uses different signature formats (OpenPGP) and key types (RSA, DSA, ECDSA over NIST curves). You'd need to convert, which adds complexity. Better to generate fresh Ed25519 keys. If you must integrate GPG, use `gpg --export` → convert to Ed25519 (lossy, not recommended).

### Q: How do I verify assets without the engine?

**A**: Use the CLI tool:
```bash
aw_asset_cli verify --asset texture.png --signature texture.png.sig --pubkey <hex>
```

Or manually with `openssl` (not recommended):
```bash
# Ed25519 verification is not directly supported by openssl CLI
# Better to use the provided tool or minisign
```

Or with `minisign` (compatible format):
```bash
minisign -Vm texture.png -p root.pub
```

### Q: What's the performance impact?

**A**: Negligible. Ed25519 verification:
- **Speed**: ~20 μs per signature (50,000/sec on typical CPU)
- **Typical game**: 1,000 assets → 20ms one-time cost at startup
- **Mitigation**: Verify asynchronously during loading screen

### Q: Does this work offline?

**A**: Yes. All verification happens locally with embedded/bundled public keys. No network calls. Optional transparency log requires internet but can be disabled.

### Q: How do I rotate the root key?

**A**: See Section 4.1. Summary:
1. Generate new root key (offline, air-gapped)
2. Sign new root with old root (proves legitimacy)
3. Embed both keys in next engine release (grace period)
4. After 1 year, remove old root key

Requires engine update, so plan for infrequent rotation (10 years).

### Q: Can multiple developers sign the same asset?

**A**: Yes (multi-signature). Not implemented in initial version, but design supports it:
```rust
struct MultiSignedAsset {
    asset_hash: [u8; 32],
    signatures: Vec<Signature>,
    required_signers: usize, // m-of-n threshold
}
```

Use case: Release assets require 2-of-3 maintainer approval.

### Q: What if the asset is modified after signing?

**A**: Verification will fail. SHA-256 hash will not match, signature verification returns error. Asset is rejected.

---

## 14. References and Further Reading

### Industry Standards
1. **Sigstore Documentation**: https://docs.sigstore.dev
2. **The Update Framework (TUF)**: https://theupdateframework.io
3. **Docker Content Trust**: https://docs.docker.com/engine/security/trust/
4. **NPM Provenance**: https://docs.npmjs.com/generating-provenance-statements
5. **SLSA Framework**: https://slsa.dev

### Cryptography
6. **Ed25519 Paper** (Bernstein et al.): https://ed25519.cr.yp.to
7. **NIST SP 800-57** (Key Management): https://csrc.nist.gov/pubs/sp/800/57/pt1/r5/final
8. **Age Encryption**: https://age-encryption.org
9. **Shamir Secret Sharing**: https://en.wikipedia.org/wiki/Shamir%27s_Secret_Sharing

### Rust Libraries
10. **ed25519-dalek**: https://docs.rs/ed25519-dalek
11. **ring**: https://docs.rs/ring (alternative crypto library)
12. **keyring-rs**: https://docs.rs/keyring
13. **cryptoki**: https://docs.rs/cryptoki (PKCS11 bindings)

### Security Best Practices
14. **Key Management Best Practices** (AWS): https://docs.aws.amazon.com/cloudhsm/latest/userguide/bp-hsm-key-management.html
15. **Certificate Revocation** (OCSP vs CRL): https://smallstep.com/blog/ocsp-vs-crl-explained/
16. **Air-Gapped Signing**: https://www.improwised.com/blog/ci-cd-in-air-gapped-environments/

---

## 15. Appendix: Code Snippets

### A. Complete Signing Workflow (v2)

```rust
// tools/aw_asset_cli/src/signing.rs

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use keyring::Entry;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize)]
pub struct AssetSignature {
    pub asset_hash: String,      // hex-encoded SHA-256
    pub signature: String,        // base64-encoded Ed25519 signature
    pub signer_role: String,      // "dev", "ci", "production"
    pub signed_at: String,        // ISO 8601 timestamp
    pub tool_version: String,     // aw_asset_cli version
}

pub fn sign_asset_file(
    asset_path: &Path,
    key_role: &str,
) -> anyhow::Result<AssetSignature> {
    // Read asset and hash
    let asset_bytes = fs::read(asset_path)?;
    let mut hasher = Sha256::new();
    hasher.update(&asset_bytes);
    let hash = hasher.finalize();
    let hash_hex = hex::encode(&hash);
    
    // Load signing key from keyring
    let entry = Entry::new("AstraWeave", &format!("signing_key_{}", key_role))?;
    let key_b64 = entry.get_password()?;
    let key_bytes = base64::decode(&key_b64)?;
    let signing_key = SigningKey::from_bytes(&key_bytes.try_into()?);
    
    // Sign the hash
    let signature = signing_key.sign(&hash);
    let sig_b64 = base64::encode(signature.to_bytes());
    
    Ok(AssetSignature {
        asset_hash: hash_hex,
        signature: sig_b64,
        signer_role: key_role.to_string(),
        signed_at: chrono::Utc::now().to_rfc3339(),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

pub fn verify_asset_file(
    asset_path: &Path,
    sig: &AssetSignature,
    trusted_pubkey: &[u8; 32],
) -> anyhow::Result<bool> {
    // Hash asset
    let asset_bytes = fs::read(asset_path)?;
    let mut hasher = Sha256::new();
    hasher.update(&asset_bytes);
    let hash = hasher.finalize();
    let hash_hex = hex::encode(&hash);
    
    // Check hash matches
    if hash_hex != sig.asset_hash {
        return Ok(false);
    }
    
    // Verify signature
    let vk = VerifyingKey::from_bytes(trusted_pubkey)?;
    let signature = Signature::from_bytes(&base64::decode(&sig.signature)?.try_into()?);
    Ok(vk.verify_strict(&hash, &signature).is_ok())
}

pub fn save_signature(asset_path: &Path, sig: &AssetSignature) -> anyhow::Result<()> {
    let sig_path = asset_path.with_extension("sig");
    let sig_json = serde_json::to_string_pretty(sig)?;
    fs::write(sig_path, sig_json)?;
    Ok(())
}
```

### B. Runtime Verification (Engine)

```rust
// astraweave-asset/src/signing.rs

pub const ROOT_PUBKEY: [u8; 32] = [
    // TODO: Replace with actual root public key after generation
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

pub const DEV_PUBKEY: [u8; 32] = [ /* ... */ ];
pub const CI_PUBKEY: [u8; 32] = [ /* ... */ ];
pub const PROD_PUBKEY: [u8; 32] = [ /* ... */ ];

pub fn get_trusted_keys() -> Vec<[u8; 32]> {
    vec![
        #[cfg(debug_assertions)]
        DEV_PUBKEY,
        CI_PUBKEY,
        PROD_PUBKEY,
    ]
}

pub fn verify_asset_signature(
    asset_bytes: &[u8],
    signature_json: &str,
) -> Result<bool, AssetError> {
    let sig: AssetSignature = serde_json::from_str(signature_json)?;
    
    // Hash asset
    let mut hasher = Sha256::new();
    hasher.update(asset_bytes);
    let hash = hasher.finalize();
    
    // Try each trusted key
    for pubkey in get_trusted_keys() {
        if verify_with_key(&hash, &sig.signature, &pubkey).unwrap_or(false) {
            return Ok(true);
        }
    }
    
    Ok(false)
}

fn verify_with_key(hash: &[u8], sig_b64: &str, pubkey: &[u8; 32]) -> Result<bool, AssetError> {
    let vk = VerifyingKey::from_bytes(pubkey)?;
    let signature = Signature::from_bytes(&base64::decode(sig_b64)?.try_into()?);
    Ok(vk.verify_strict(hash, &signature).is_ok())
}
```

### C. Key Generation CLI

```rust
// tools/aw_asset_cli/src/keygen.rs

use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;

pub fn generate_key(role: &str, output: Option<&Path>) -> anyhow::Result<()> {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();
    
    if let Some(path) = output {
        // Save to file
        fs::write(path, signing_key.to_bytes())?;
        fs::write(
            path.with_extension("pub"),
            hex::encode(verifying_key.to_bytes()),
        )?;
        println!("✓ Key saved to {}", path.display());
    } else {
        // Save to keyring
        let entry = Entry::new("AstraWeave", &format!("signing_key_{}", role))?;
        entry.set_password(&base64::encode(signing_key.to_bytes()))?;
        println!("✓ Key saved to OS keyring (role: {})", role);
    }
    
    println!("Public key: {}", hex::encode(verifying_key.to_bytes()));
    Ok(())
}
```

---

## 16. Conclusion

This design document provides a plan for upgrading AstraWeave's asset signing system from ephemeral keys to a more robust solution. The proposed architecture:

✅ **Draws from industry best practices** (Sigstore, TUF, Docker, NPM)  
✅ **Balances security and usability** (OS keyring for devs, HSM for prod)  
✅ **Supports key rotation and revocation** (TUF-inspired hierarchy)  
✅ **Enables offline workflows** (air-gapped signing, no network dependency)  
✅ **Provides clear migration path** (6 phases, 16 weeks)  
✅ **Includes operational runbooks** (backup, rotation, incident response)  
✅ **Future-proof** (transparency log, post-quantum migration path)

**Next Steps**:
1. Review and approve this design with team/stakeholders
2. Begin Phase 1 implementation (persistent keys)
3. Iterate based on feedback and testing

**Success Criteria**:
- ✅ No more ephemeral keys (verifiable signatures)
- ✅ CI/CD fully automated (GitHub Actions)
- ✅ Production releases signed with HSM/YubiKey
- ✅ < 1% verification failures in production
- ✅ < 50ms verification overhead at asset load time

---

**Document Version**: 1.0  
**Last Updated**: 2025-11-13  
**Author**: Verdent AI (AstraWeave Team)  
**Status**: **READY FOR REVIEW**
