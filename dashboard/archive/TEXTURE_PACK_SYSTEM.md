# Texture Pack System for Unified Showcase

## Overview
The unified_showcase demo has been enhanced with a texture pack system that transforms the gray environment with colored cubes into realistic landscapes with structures and environmental elements.

## Features Implemented

### 1. Texture Loading System
- LoadedTexture struct with wgpu texture, view, and sampler
- Dynamic texture loading from PNG files
- Fallback to default white texture if assets are missing
- Texture bind group creation and management

### 2. Environment Configuration
- TexturePack configuration system using TOML files
- Environment definitions in `assets_src/environments/`:
  - `grassland.toml` - Peaceful grassland with trees and cottages
  - `desert.toml` - Sandy desert with cacti and adobe houses
- Configurable ground textures, colors, structures, sky, and ambient settings

### 3. Dynamic Environment Generation
- Procedural placement of environment-specific objects
- **Grassland Environment:**
  - Green trees (tall thin objects)
  - Brown cottages (wider house-like structures)
  - Grass ground texture
- **Desert Environment:**
  - Green cacti (thin vertical objects)  
  - Sandy adobe houses
  - Dirt/sand ground texture

### 4. Runtime Texture Pack Switching
- **Keyboard Controls:**
  - Press `1` to switch to Grassland environment
  - Press `2` to switch to Desert environment
- Real-time texture reloading and object regeneration
- Dynamic physics object management

### 5. Enhanced Rendering
- Modified WGSL shader to sample ground textures instead of procedural patterns
- Texture bind group support in render pipeline
- Dynamic instance buffer management (up to 100 objects)
- Object coloring based on type (trees=green, houses=brown, cacti=bright green, etc.)

## Assets Created
- **Textures:** `grass.png`, `dirt.png`, `stone.png` (procedurally generated)
- **Processed Assets:** Converted to `.ktx2` format via asset pipeline
- **Environment Configs:** Grassland and desert environment definitions

## Technical Implementation

### Shader Changes
- Added texture sampling with `@group(1)` for ground textures
- Maintained slight checkerboard variation for visual interest
- UV mapping based on world position coordinates

### Physics Integration
- Environment objects use unique `user_data` IDs for identification
- Dynamic object removal and regeneration when switching environments
- Fixed objects for structures, dynamic objects for interactive elements

### Asset Pipeline Enhancement
- Fixed glob pattern support for brace expansion (`*.{png,jpg,jpeg}`)
- Improved error handling and debug output
- Support for multiple texture formats

## Usage
1. **Build:** `cargo build -p unified_showcase`
2. **Run:** `cargo run -p unified_showcase`
3. **Controls:**
   - WASD + mouse: Camera movement
   - Right mouse: Look around
   - P: Pause physics
   - T: Teleport sphere
   - E: Apply impulse to objects
   - **1**: Switch to grassland environment
   - **2**: Switch to desert environment

## Future Enhancements
The system is designed to support:
- Additional texture packs and environments
- NPC/character placement systems
- More complex procedural generation
- Sky gradient and lighting changes per environment
- Structure variation based on texture pack configuration
- Advanced terrain generation beyond flat ground