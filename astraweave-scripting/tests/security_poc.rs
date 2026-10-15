//! SEC-POC. See `docs/current/SECURITY_AUDIT.md` (SEC-MOD-4).

use astraweave_scripting::loader::ScriptLoader;

#[tokio::test]
async fn poc_script_loader_rejects_parent_segments() {
    let err = ScriptLoader::load("../secret.rhai")
        .await
        .expect_err("hostile script path must not load");
    let msg = format!("{err:#}");
    assert!(
        msg.to_lowercase().contains("traversal") || msg.to_lowercase().contains("rejected"),
        "script path with parent segments was not rejected: {msg}"
    );
}
