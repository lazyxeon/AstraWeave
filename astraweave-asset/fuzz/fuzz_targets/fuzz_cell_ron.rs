#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let path = std::env::temp_dir().join("aw-fuzz-cell.ron");
    if std::fs::write(&path, data).is_ok() {
        let _ = astraweave_asset::cell_loader::load_cell_from_ron_sync(&path);
    }
});
