//! Sim sources stay on integers. Floats, maps, and clocks are rejected.

use std::fs;
use std::path::Path;

#[test]
fn sim_sources_have_no_float_map_or_clock() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect(&root, &mut files);
    files.sort();
    for path in files {
        let text = fs::read_to_string(&path).expect("read");
        for banned in ["f32", "f64", "HashMap", "Instant", "thread_rng"] {
            assert!(
                !text.contains(banned),
                "{} contains {banned}",
                path.display()
            );
        }
    }
}

fn collect(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(dir).expect("read dir") {
        let entry = entry.expect("entry");
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}
