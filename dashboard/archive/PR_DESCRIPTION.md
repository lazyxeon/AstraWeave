# AstraWeave Tooling Enhancements

This PR implements three key pillars to enhance the AstraWeave engine's tooling ecosystem:

1. **Level & Encounter Editor** - A non-programmer friendly GUI tool for creating game content
2. **Asset Pipeline CLI** - A command-line tool for processing and optimizing game assets
3. **Debug & Profiling Toolkit** - A comprehensive toolkit for debugging and performance analysis

## Overview

These tools build upon AstraWeave's existing foundations:
- Leveraging the Rhai scripting system for authoring
- Integrating with the egui overlay for debugging
- Supporting the data-driven content approach

## Key Features

### Level & Encounter Editor (`tools/aw_editor`)

- Visual editing of biomes, obstacles, and NPCs
- Fate thread system for trigger-based events
- Boss encounter configuration with Rhai scripting
- Hot-reload support via the `reload.signal` mechanism
- Saves levels in human-readable TOML format

### Asset Pipeline CLI (`tools/aw_asset_cli`)

- Texture processing (PNG/JPEG → KTX2/Basis)
- Model processing (glTF/GLB → meshbin)
- Audio processing (WAV/MP3/OGG/FLAC → Ogg Vorbis)
- Asset manifest generation with SHA-256 hashes
- Configurable pipeline rules via TOML

### Debug & Profiling Toolkit (`tools/aw_debug`)

- Performance HUD with FPS counter and frame time graph
- System timing visualization
- Entity count tracking
- Event logging system
- Chrome tracing integration
- Script hot-reload support

## Integration Example

The PR includes a `debug_toolkit_demo` example that demonstrates how to integrate these tools with the existing AstraWeave engine components.

## Implementation Details

### Alignment with Existing Architecture

- The Level Editor builds on the existing `astraweave-author` module and Rhai scripting system
- The Asset Pipeline produces assets compatible with the engine's loading system
- The Debug Toolkit integrates with the existing egui overlay system

### File Format Compatibility

- Level files use TOML format for human readability
- Asset manifest uses JSON for compatibility with various loaders
- Debug traces use Chrome's trace format for industry-standard tooling

### Hot-Reload Support

All tools support hot-reloading for rapid iteration:
- The Level Editor emits a `reload.signal` file when saving
- The Debug Toolkit includes watchers for script changes and reload signals
- The Asset Pipeline generates deterministic outputs for reliable loading

## Testing

The PR has been tested with:
- Various level configurations in the editor
- Different asset types through the pipeline
- Performance profiling using the debug toolkit

## Future Enhancements

Potential future improvements:
- Add a 2.5D preview to the Level Editor
- Integrate navmesh baking into the editor
- Expand the Asset Pipeline to support more formats
- Add more detailed performance analysis to the Debug Toolkit

## Documentation

Each tool includes comprehensive documentation:
- README files explaining usage and integration
- Code comments for implementation details
- Example configurations and scripts

## Screenshots

[Screenshots would be included in an actual PR]