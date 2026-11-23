//! Autosave files and the session lock file.
//!
//! The editor UI calls these and turns the result into toasts and console
//! lines. The file work is here so a test can run it without an editor frame.

use std::path::{Path, PathBuf};

use astraweave_core::World;

use crate::entity_manager::EntityManager;
use crate::local_clock;
use crate::scene_serialization;

#[derive(Debug)]
pub enum AutosaveError {
    Directory(String),
    Save(String),
}

pub struct AutosaveRequest<'a> {
    pub world: Option<&'a World>,
    pub entities: &'a EntityManager,
    pub content_root: &'a Path,
    pub scene_path: Option<&'a Path>,
    pub separate_dir: bool,
    pub keep_count: usize,
}

#[derive(Debug)]
pub struct SavedAutosave {
    pub filename: String,
    pub notes: Vec<String>,
}

pub fn write_lock_file(path: &Path, scene: &str) {
    use std::io::Write;

    if let Ok(mut file) = std::fs::File::create(path) {
        let session_info = format!(
            "pid={}\nstarted={}\nscene={}\n",
            std::process::id(),
            local_clock::readable(local_clock::now()),
            scene
        );
        let _ = file.write_all(session_info.as_bytes());
    }
}

pub fn write_autosave(
    request: AutosaveRequest<'_>,
) -> Result<Option<SavedAutosave>, AutosaveError> {
    let Some(world) = request.world else {
        return Ok(None);
    };

    let auto_save_path = if request.separate_dir {
        let autosave_dir = request.content_root.join(".autosave");
        if let Err(error) = std::fs::create_dir_all(&autosave_dir) {
            return Err(AutosaveError::Directory(format!(
                "Failed to create autosave dir: {error}"
            )));
        }
        let timestamp = local_clock::compact(local_clock::now());
        let base_name = scene_stem(request.scene_path);
        let filename = format!("{base_name}_{timestamp}.autosave.scene.ron");
        autosave_dir.join(filename)
    } else {
        request
            .scene_path
            .map(Path::to_path_buf)
            .unwrap_or_else(|| request.content_root.join("scenes/autosave.scene.ron"))
    };

    match scene_serialization::save_scene_with_overlay(world, request.entities, &auto_save_path) {
        Ok(()) => {
            let filename = auto_save_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let notes = if request.separate_dir {
                cleanup_old_autosaves(
                    &request.content_root.join(".autosave"),
                    scene_stem(request.scene_path),
                    request.keep_count,
                )
            } else {
                Vec::new()
            };
            Ok(Some(SavedAutosave { filename, notes }))
        }
        Err(error) => Err(AutosaveError::Save(format!("Auto-save failed: {error}"))),
    }
}

pub fn cleanup_old_autosaves(
    autosave_dir: &Path,
    base_name: &str,
    keep_count: usize,
) -> Vec<String> {
    if !autosave_dir.exists() {
        return Vec::new();
    }

    let mut autosaves: Vec<PathBuf> = match std::fs::read_dir(autosave_dir) {
        Ok(entries) => entries
            .filter_map(|entry| {
                entry
                    .map_err(|err| tracing::debug!("Autosave dir entry error: {err}"))
                    .ok()
            })
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| {
                        name.starts_with(base_name) && name.ends_with(".autosave.scene.ron")
                    })
                    .unwrap_or(false)
            })
            .collect(),
        Err(_) => return Vec::new(),
    };

    autosaves.sort_by(|left, right| {
        let left_time = std::fs::metadata(left)
            .and_then(|meta| meta.modified())
            .ok();
        let right_time = std::fs::metadata(right)
            .and_then(|meta| meta.modified())
            .ok();
        right_time.cmp(&left_time)
    });

    let mut notes = Vec::new();
    for old_file in autosaves.iter().skip(keep_count) {
        if let Err(error) = std::fs::remove_file(old_file) {
            notes.push(format!(
                "Failed to remove old autosave {:?}: {error}",
                old_file.file_name().unwrap_or_default()
            ));
        } else {
            notes.push(format!(
                "Removed old autosave: {:?}",
                old_file.file_name().unwrap_or_default()
            ));
        }
    }
    notes
}

fn scene_stem(scene_path: Option<&Path>) -> &str {
    scene_path
        .and_then(|path| path.file_stem())
        .and_then(|stem| stem.to_str())
        .unwrap_or("untitled")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_file_records_the_pid_and_scene() {
        let path = std::env::temp_dir().join(format!(
            "aw-editor-lock-{}-{}.txt",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        write_lock_file(&path, "harbor.scene.ron");
        let text = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        assert!(text.contains(&format!("pid={}\n", std::process::id())));
        assert!(text.contains("scene=harbor.scene.ron\n"));
        assert!(text.contains("started="));
    }

    #[test]
    fn autosave_without_a_world_writes_nothing() {
        let entities = EntityManager::new();
        let saved = write_autosave(AutosaveRequest {
            world: None,
            entities: &entities,
            content_root: Path::new("missing"),
            scene_path: None,
            separate_dir: true,
            keep_count: 1,
        })
        .unwrap();
        assert!(saved.is_none());
    }

    #[test]
    fn autosave_directory_failure_is_reported() {
        let world = World::new();
        let entities = EntityManager::new();
        let blocker =
            std::env::temp_dir().join(format!("aw-editor-not-a-dir-{}", std::process::id()));
        std::fs::write(&blocker, b"file").unwrap();
        let error = write_autosave(AutosaveRequest {
            world: Some(&world),
            entities: &entities,
            content_root: &blocker,
            scene_path: Some(Path::new("demo.scene.ron")),
            separate_dir: true,
            keep_count: 1,
        })
        .unwrap_err();
        let _ = std::fs::remove_file(&blocker);
        match error {
            AutosaveError::Directory(message) => {
                assert!(message.contains("Failed to create autosave dir:"));
            }
            AutosaveError::Save(_) => panic!("directory failure must stay a directory error"),
        }
    }

    #[test]
    fn autosave_writes_a_timestamped_scene_under_the_content_root() {
        let folder = std::path::PathBuf::from(format!(".autosave-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        let landed = std::path::PathBuf::from("content").join(&folder);
        let _ = std::fs::remove_dir_all(&landed);
        let world = World::new();
        let entities = EntityManager::new();
        let scene = folder.join("demo.scene.ron");
        let saved = write_autosave(AutosaveRequest {
            world: Some(&world),
            entities: &entities,
            content_root: &folder,
            scene_path: Some(&scene),
            separate_dir: true,
            keep_count: 4,
        });
        let outcome = match saved {
            Ok(value) => value,
            Err(error) => {
                let _ = std::fs::remove_dir_all(&folder);
                let _ = std::fs::remove_dir_all(&landed);
                panic!("autosave failed: {error:?}");
            }
        };
        let saved = outcome.expect("a world produces a file");
        let written = landed.join(".autosave").join(&saved.filename);
        assert!(written.is_file(), "{}", written.display());
        assert!(saved.filename.contains("demo"));
        assert!(saved.filename.ends_with(".autosave.scene.ron"));
        let _ = std::fs::remove_dir_all(&folder);
        let _ = std::fs::remove_dir_all(&landed);
    }

    #[test]
    fn cleanup_keeps_only_the_requested_count() {
        let dir = std::env::temp_dir().join(format!("aw-editor-autosave-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for name in [
            "demo_a.autosave.scene.ron",
            "demo_b.autosave.scene.ron",
            "demo_c.autosave.scene.ron",
        ] {
            std::fs::write(dir.join(name), b"scene").unwrap();
        }
        std::fs::write(dir.join("other.autosave.scene.ron"), b"keep").unwrap();
        let notes = cleanup_old_autosaves(&dir, "demo", 1);
        let remaining: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        let _ = std::fs::remove_dir_all(&dir);
        let demo_left = remaining
            .iter()
            .filter(|name| name.starts_with("demo"))
            .count();
        assert_eq!(demo_left, 1);
        assert!(remaining
            .iter()
            .any(|name| name == "other.autosave.scene.ron"));
        assert_eq!(notes.len(), 2);
        assert!(notes
            .iter()
            .all(|note| note.contains("Removed old autosave")));
    }
}
