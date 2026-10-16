//! TLS Configuration for Secure WebSocket Connections
//!
//! This module provides utilities for setting up TLS/SSL for WebSocket connections.
//! Enable with the `tls` feature flag.
//!
//! # Example (Server)
//! ```ignore
//! use astraweave_net::tls::TlsServerConfig;
//!
//! let tls_config = TlsServerConfig::from_pem_files("cert.pem", "key.pem")?;
//! let acceptor = tls_config.acceptor();
//! // Use acceptor with tokio-tungstenite's accept_async_with_tls_acceptor
//! ```
//!
//! # Example (Client)
//! ```ignore
//! use astraweave_net::tls::TlsClientConfig;
//!
//! let connector = TlsClientConfig::default_connector()?;
//! // Use connector with tokio-tungstenite's connect_async_tls_with_config
//! ```

use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio_rustls::rustls::pki_types::pem::PemObject;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use tokio_rustls::rustls::{ClientConfig, RootCertStore, ServerConfig};
use tokio_rustls::{TlsAcceptor, TlsConnector};
use webpki_roots::TLS_SERVER_ROOTS;

fn redact_path_for_error(path: &Path) -> String {
    path.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "<unknown>".to_string())
}

/// TLS configuration for WebSocket server
pub struct TlsServerConfig {
    server_config: Arc<ServerConfig>,
}

impl TlsServerConfig {
    /// Load TLS configuration from PEM-encoded certificate and key files
    ///
    /// # Arguments
    /// * `cert_path` - Path to PEM-encoded certificate chain file
    /// * `key_path` - Path to PEM-encoded private key file
    ///
    /// # Returns
    /// TLS configuration ready to create an acceptor
    pub fn from_pem_files(cert_path: impl AsRef<Path>, key_path: impl AsRef<Path>) -> Result<Self> {
        let cert_chain: Vec<CertificateDer<'static>> =
            CertificateDer::pem_file_iter(cert_path.as_ref())
                .with_context(|| {
                    format!(
                        "Failed to open cert file: {}",
                        redact_path_for_error(cert_path.as_ref())
                    )
                })?
                .collect::<Result<Vec<_>, _>>()
                .context("Failed to parse certificate chain")?;

        if cert_chain.is_empty() {
            anyhow::bail!(
                "No certificates found in {}",
                redact_path_for_error(cert_path.as_ref())
            );
        }

        let private_key = PrivateKeyDer::from_pem_file(key_path.as_ref()).with_context(|| {
            format!(
                "Failed to parse private key in {}",
                redact_path_for_error(key_path.as_ref())
            )
        })?;

        // Build config immediately since PrivateKeyDer doesn't implement Clone
        let server_config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(cert_chain, private_key)
            .context("Failed to build TLS server config")?;

        Ok(Self {
            server_config: Arc::new(server_config),
        })
    }

    /// Create a TLS acceptor for accepting secure connections
    pub fn acceptor(&self) -> TlsAcceptor {
        TlsAcceptor::from(self.server_config.clone())
    }
}

/// TLS configuration for WebSocket client
pub struct TlsClientConfig;

impl TlsClientConfig {
    /// Create a TLS connector with default root certificates (WebPKI roots)
    ///
    /// This uses the standard trusted CA certificates for validating server identities.
    pub fn default_connector() -> Result<TlsConnector> {
        let mut root_store = RootCertStore::empty();
        root_store.extend(TLS_SERVER_ROOTS.iter().cloned());

        let config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        Ok(TlsConnector::from(Arc::new(config)))
    }

    /// Create a TLS connector with custom root certificates
    ///
    /// # Arguments
    /// * `ca_cert_path` - Path to PEM-encoded CA certificate file
    pub fn with_custom_ca(ca_cert_path: impl AsRef<Path>) -> Result<TlsConnector> {
        let ca_certs = CertificateDer::pem_file_iter(ca_cert_path.as_ref()).with_context(|| {
            format!(
                "Failed to open CA cert: {}",
                redact_path_for_error(ca_cert_path.as_ref())
            )
        })?;

        let mut root_store = RootCertStore::empty();
        for cert in ca_certs {
            let cert = cert.context("Failed to parse CA certificate")?;
            root_store
                .add(cert)
                .context("Failed to add CA certificate")?;
        }

        let config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        Ok(TlsConnector::from(Arc::new(config)))
    }

    /// Create a TLS connector that accepts any certificate (INSECURE - for testing only)
    ///
    /// # Warning
    /// This disables certificate verification! Only use for local development.
    #[cfg(feature = "dangerous-testing")]
    pub fn insecure_connector() -> Result<TlsConnector> {
        use tokio_rustls::rustls::client::danger::{
            HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier,
        };

        #[derive(Debug)]
        struct NoVerifier;

        impl ServerCertVerifier for NoVerifier {
            fn verify_server_cert(
                &self,
                _: &CertificateDer<'_>,
                _: &[CertificateDer<'_>],
                _: &ServerName<'_>,
                _: &[u8],
                _: tokio_rustls::rustls::pki_types::UnixTime,
            ) -> Result<ServerCertVerified, tokio_rustls::rustls::Error> {
                Ok(ServerCertVerified::assertion())
            }

            fn verify_tls12_signature(
                &self,
                _: &[u8],
                _: &CertificateDer<'_>,
                _: &tokio_rustls::rustls::DigitallySignedStruct,
            ) -> Result<HandshakeSignatureValid, tokio_rustls::rustls::Error> {
                Ok(HandshakeSignatureValid::assertion())
            }

            fn verify_tls13_signature(
                &self,
                _: &[u8],
                _: &CertificateDer<'_>,
                _: &tokio_rustls::rustls::DigitallySignedStruct,
            ) -> Result<HandshakeSignatureValid, tokio_rustls::rustls::Error> {
                Ok(HandshakeSignatureValid::assertion())
            }

            fn supported_verify_schemes(&self) -> Vec<tokio_rustls::rustls::SignatureScheme> {
                vec![
                    tokio_rustls::rustls::SignatureScheme::RSA_PKCS1_SHA256,
                    tokio_rustls::rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
                    tokio_rustls::rustls::SignatureScheme::RSA_PKCS1_SHA384,
                    tokio_rustls::rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
                    tokio_rustls::rustls::SignatureScheme::RSA_PKCS1_SHA512,
                    tokio_rustls::rustls::SignatureScheme::ED25519,
                ]
            }
        }

        let config = ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoVerifier))
            .with_no_client_auth();

        Ok(TlsConnector::from(Arc::new(config)))
    }
}

/// Parse a hostname for TLS server name indication (SNI)
pub fn parse_server_name(hostname: &str) -> Result<ServerName<'static>> {
    ServerName::try_from(hostname.to_string())
        .map_err(|_| anyhow::anyhow!("Invalid hostname for TLS: {}", hostname))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_connector_creation() {
        let result = TlsClientConfig::default_connector();
        assert!(result.is_ok(), "Should create default TLS connector");
    }

    #[test]
    fn test_server_name_parsing() {
        let result = parse_server_name("example.com");
        assert!(result.is_ok());

        let result = parse_server_name("localhost");
        assert!(result.is_ok());

        let _result = parse_server_name("192.168.1.1");
        // IP addresses should fail for SNI (domain names only)
        // This depends on rustls version behavior
    }

    #[test]
    fn test_missing_cert_file() {
        let result = TlsServerConfig::from_pem_files("nonexistent.pem", "key.pem");
        assert!(result.is_err());
        match result {
            Err(e) => assert!(e.to_string().contains("cert file")),
            Ok(_) => panic!("Expected error"),
        }
    }
}
