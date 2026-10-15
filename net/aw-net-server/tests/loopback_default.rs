use aw_net_proto::SigningKey;
use aw_net_server::ServerConfig;

#[test]
fn default_binds_loopback_and_recognizes_the_published_dev_key() {
    let config = ServerConfig::default();
    assert!(config.ws_listen.ip().is_loopback());
    assert!(config.http_listen.ip().is_loopback());
    assert_eq!(config.ws_listen.port(), 8788);
    assert_eq!(config.http_listen.port(), 8789);
    assert!(config.signing_key.is_published_dev_key());
    let other = SigningKey::from_bytes([7; 32]);
    assert!(!other.is_published_dev_key());
}
