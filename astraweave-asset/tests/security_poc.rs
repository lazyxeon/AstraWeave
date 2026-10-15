//! SEC-POC tests. They assert the secure property and fail on today's loaders.
//! See `docs/current/SECURITY_AUDIT.md` (SEC-ASSET-1, SEC-ASSET-2).

use astraweave_asset::cell_loader::{load_asset, AssetKind, AssetRef};
use astraweave_asset::gltf_loader::load_first_mesh_and_material;

#[test]
fn poc_gltf_buffer_uri_rejects_parent_segments() {
    let json = br#"{
        "asset": { "version": "2.0" },
        "buffers": [ { "uri": "../secret.bin", "byteLength": 4 } ]
    }"#;
    let err = load_first_mesh_and_material(json).expect_err("hostile glTF must not load");
    let msg = format!("{err:#}");
    assert!(
        msg.to_lowercase().contains("traversal") || msg.to_lowercase().contains("rejected"),
        "glTF buffer URI with parent segments was not rejected: {msg}"
    );
}

#[tokio::test]
async fn poc_cell_asset_path_rejects_parent_segments() {
    let root = tempfile::tempdir().expect("tempdir");
    let asset = AssetRef::new("../outside.bin", AssetKind::Mesh);
    let err = load_asset(&asset, root.path())
        .await
        .expect_err("hostile cell path must not load");
    let msg = format!("{err:#}");
    assert!(
        msg.to_lowercase().contains("traversal") || msg.to_lowercase().contains("rejected"),
        "cell asset path with parent segments was not rejected: {msg}"
    );
}
