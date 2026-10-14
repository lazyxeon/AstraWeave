//! Property tests for mod manifest and script-path parsing.
//!
//! Arbitrary strings must not panic the parser. Paths that contain `..`, a
//! backslash, or an absolute prefix must be rejected.

use astraweave_security::sandbox::{
    parse_manifest_str, validate_manifest_script_path, EngineCeilings,
};
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn manifest_parser_does_not_panic(text in "\\PC{0,240}") {
        let _ = parse_manifest_str(&text, &EngineCeilings::default());
    }

    #[test]
    fn dotdot_script_paths_are_rejected(
        head in "[a-z0-9]{0,6}",
        tail in "[a-z0-9]{0,6}",
    ) {
        let path = format!("{}/../{}.rhai", head, tail);
        prop_assert!(validate_manifest_script_path(&path).is_err());
    }

    #[test]
    fn backslash_and_absolute_paths_are_rejected(name in "[a-z]{1,8}") {
        let windows = format!("{}\\tick.rhai", name);
        let unix_abs = format!("/{}.rhai", name);
        let drive = format!("C:/{}.rhai", name);
        prop_assert!(validate_manifest_script_path(&windows).is_err());
        prop_assert!(validate_manifest_script_path(&unix_abs).is_err());
        prop_assert!(validate_manifest_script_path(&drive).is_err());
    }
}

#[test]
fn legal_relative_script_path_is_accepted() {
    assert!(validate_manifest_script_path("scripts/tick.rhai").is_ok());
}
