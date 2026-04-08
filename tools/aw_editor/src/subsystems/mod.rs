//! Editor subsystem methods extracted from main.rs
//!
//! This module contains methods on `EditorApp` that were extracted from the
//! monolithic `update()` loop to improve code organization and reduce main.rs size.
//! Each method handles a coherent subsystem of the editor.

mod audio_animation;
mod docking_sync;
mod hotkeys;
mod scene_stats;
