//! Editor-side `.rhai` hot reload.
//!
//! The file watcher only classifies a path. This module reads the bytes through
//! the sandbox path check and holds them until [`RhaiHotReload::apply_at_tick_boundary`].
//! Applying is the tick boundary: the loaded table does not change when a file
//! event is staged, and it does not change in the middle of a simulation tick.

use anyhow::{Context, Result};
use astraweave_security::sandbox::{
    read_rhai_under_root, EngineCeilings, ScriptReloadQueue, StagedScript,
};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone, Debug, Default)]
pub struct RhaiHotReload {
    queue: ScriptReloadQueue,
    loaded: BTreeMap<String, String>,
}

impl RhaiHotReload {
    /// Read `event_path` if it canonicalizes inside `root`, and stage those bytes.
    ///
    /// Does not publish them to [`Self::loaded_source`].
    pub fn stage_file(&mut self, root: &Path, event_path: &Path) -> Result<()> {
        let ceilings = EngineCeilings::default();
        let (canon, source) = read_rhai_under_root(root, event_path, ceilings.max_script_bytes)?;
        let key = canon
            .to_str()
            .context("script path is not utf-8")?
            .replace('\\', "/");
        self.queue.stage(key, source);
        Ok(())
    }

    pub fn pending_len(&self) -> usize {
        self.queue.len()
    }

    /// Publish staged scripts in path order. Call this immediately before the sim tick.
    pub fn apply_at_tick_boundary(&mut self) -> Vec<StagedScript> {
        let pending = self.queue.take_boundary();
        let mut applied = Vec::with_capacity(pending.len());
        for (path, staged) in pending {
            self.loaded.insert(path, staged.source.clone());
            applied.push(staged);
        }
        applied
    }

    pub fn loaded_source(&self, path: &str) -> Option<&str> {
        self.loaded.get(path).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    #[test]
    fn changed_file_is_visible_only_after_the_tick_boundary() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("assets");
        fs::create_dir(&root).unwrap();
        let script = root.join("tick.rhai");
        let mut file = fs::File::create(&script).unwrap();
        writeln!(file, "let x = 1;").unwrap();
        drop(file);

        let mut hot = RhaiHotReload::default();
        hot.stage_file(&root, &script).unwrap();
        assert_eq!(hot.pending_len(), 1);
        assert!(hot.loaded_source(&canonical_key(&script)).is_none());

        fs::write(&script, "let x = 2;\n").unwrap();
        hot.stage_file(&root, &script).unwrap();
        assert!(hot.loaded_source(&canonical_key(&script)).is_none());

        let applied = hot.apply_at_tick_boundary();
        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0].source, "let x = 2;\n");
        assert_eq!(
            hot.loaded_source(&applied[0].path),
            Some(applied[0].source.as_str())
        );
        assert_eq!(hot.pending_len(), 0);

        fs::write(&script, "let x = 3;\n").unwrap();
        hot.stage_file(&root, &script).unwrap();
        assert_eq!(
            hot.loaded_source(&applied[0].path),
            Some("let x = 2;\n"),
            "a staged rewrite must not replace the bytes already applied"
        );
        let next = hot.apply_at_tick_boundary();
        assert_eq!(next[0].source, "let x = 3;\n");
    }

    #[test]
    fn editor_path_rejects_files_outside_the_mod_root() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("mod");
        fs::create_dir(&root).unwrap();
        let outside = temp.path().join("secret.rhai");
        fs::write(&outside, "let x = 1;\n").unwrap();
        let mut hot = RhaiHotReload::default();
        assert!(hot.stage_file(&root, &outside).is_err());
        assert!(hot
            .stage_file(&root, std::path::Path::new("../secret.rhai"))
            .is_err());
        assert_eq!(hot.pending_len(), 0);

        #[cfg(unix)]
        {
            let link = root.join("escape.rhai");
            std::os::unix::fs::symlink(&outside, &link).unwrap();
            assert!(hot.stage_file(&root, &link).is_err());
        }
    }

    fn canonical_key(path: &std::path::Path) -> String {
        path.canonicalize()
            .unwrap()
            .to_str()
            .unwrap()
            .replace('\\', "/")
    }
}
