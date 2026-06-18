# AstraWeave Asset Library

> **Version**: 2.0 | **Total**: 1,445 files, 8.83 GB | **License**: Mixed (see per-asset)

Asset library for the AstraWeave AI-Native Game Engine. Every biome defined in the engine has complete material coverage, PBR texture sets, and HDRI environment lighting.

---

## Directory Structure

```
assets/
├── audio/                    # Sound effects & music
│   ├── loops/                # Looping ambient sounds (wav/ogg/mp3)
│   ├── tracks/               # Music tracks (wav/ogg/mp3)
│   └── water_ambient_*/      # Water ambience variants (mono + stereo)
│
├── castles_forts_asset_pack/ # Modular castle/fort 3D models (GLB)
│
├── cells/                    # World streaming cell definitions (RON)
│
├── cinematics/               # Cinematic sequence data
│
├── exemplars/                # Entity prefab definitions (RON)
│
├── hdri/                     # Environment lighting
│   ├── hdri_catalog.toml     # Biome → HDRI mapping + time-of-day
│   └── polyhaven/            # PolyHaven CC0 HDRIs (2K HDR)
│       ├── kloppenheim/      # Clear sky daytime
│       ├── spruit_sunrise/   # Morning golden hour
│       └── venice_sunset/    # Evening sunset
│
├── materials/                # Terrain material system
│   ├── *.png                 # Source PBR textures (22 sets × 3 maps)
│   ├── baked/                # GPU-ready KTX2 compressed textures
│   ├── polyhaven/            # PolyHaven CC0 PBR material sets
│   │   ├── cobblestone/
│   │   ├── metal_plate/
│   │   ├── plastered_wall/
│   │   └── wood_floor/
│   ├── beach/                # Biome material configs
│   ├── desert/
│   ├── forest/
│   ├── grassland/
│   ├── mountain/
│   ├── river/
│   ├── swamp/
│   ├── terrain/              # Default/fallback biome
│   └── tundra/
│
├── models/                   # 3D models
│   ├── Amber-Npc/            # Character model (CC4, FBX + textures)
│   └── greybox/              # Prototype placeholder meshes
│
├── npc/                      # NPC configuration data
│
├── shaders/                  # WGSL shader source files
│
├── textures/                 # General-purpose textures
│   ├── demo/                 # Demo/example textures
│   ├── desert/               # Desert biome extras
│   ├── fabrics/              # Fabric textures
│   ├── forest/               # Forest biome extras
│   ├── grass_hd/             # High-res grass
│   ├── grassland/            # Grassland biome extras
│   ├── models/houses/        # Building textures
│   ├── pbr/PBR_2K/           # 2K PBR texture sets (4 materials)
│   ├── pbr/PBR_4K/           # 4K PBR texture sets (4 materials)
│   └── pine_forest/          # Pine forest environment
│
├── tests/                    # Test fixture assets
│   ├── biome/                # Biome test data
│   └── textures/             # Test textures
│
├── asset_manifest.toml       # Multi-source asset manifest
├── polyhaven_manifest.toml   # PolyHaven download manifest
└── README.md                 # This file
```

## File Statistics by Format

| Format | Count | Size (GB) | Purpose |
|--------|------:|----------:|---------|
| `.png` | 425 | 3.14 | PBR textures (albedo, normal, MRA) |
| `.wav` | 104 | 2.73 | Lossless audio (SFX, ambience) |
| `.glb` | 474 | 1.20 | 3D models (glTF binary) |
| `.ktx2` | 72 | 1.02 | GPU-compressed textures (baked) |
| `.exr` | 30 | 0.32 | OpenEXR textures (HDR data) |
| `.fbx` | 77 | 0.25 | Legacy 3D models (Amber NPC) |
| `.mp3` | 20 | 0.14 | Compressed audio |
| `.ogg` | 20 | 0.10 | Compressed audio (Vorbis) |
| `.hdr` | 6 | 0.03 | HDRI environment maps |
| `.toml` | 42 | <0.01 | Configuration files |
| `.ron` | 15 | <0.01 | Rusty Object Notation data |
| Other | 160 | 0.07 | JSON, WGSL, OBJ, USDC, etc. |

---

## Terrain Material System

### Overview

The terrain renderer uses **biome-specific material layers**. Each biome defines 5 PBR texture layers that the GPU blends via splatmaps. All 8 engine biomes plus a terrain fallback have complete material coverage.

### Biome Coverage (9/9 Complete)

| Biome | Layers | Materials Used |
|-------|--------|----------------|
| **Beach** | 5 | sand, gravel, cobblestone, rock_slate, grass |
| **Desert** | 5 | sand, dirt, rock_slate, plaster, stone |
| **Forest** | 5 | forest_floor, tree_bark, tree_leaves, rock_lichen, rock_slate |
| **Grassland** | 5 | grass, dirt, stone, rock_lichen, forest_floor |
| **Mountain** | 5 | mountain_rock, rock_slate, gravel, snow, ice |
| **River** | 5 | mud, gravel, moss, rock_lichen, sand |
| **Swamp** | 5 | mud, moss, tree_bark, forest_floor, dirt |
| **Terrain** | 5 | grass, dirt, stone, rock_slate, sand (fallback) |
| **Tundra** | 5 | snow, ice, rock_slate, gravel, dirt |

### Material Config Schema

Each biome folder contains two TOML files:

**`materials.toml`** — Defines 5 texture layers:
```toml
[biome]
name = "forest"

[[layer]]
key = "forest_floor"
albedo = "../forest_floor.png"        # Relative to biome folder
normal = "../forest_floor_n.png"
mra    = "../forest_floor_mra.png"    # Packed: R=Metallic, G=Roughness, B=AO
tiling = [2.0, 2.0]                   # UV tiling multiplier
triplanar_scale = 20.0                # Triplanar projection scale (meters)
```

**`arrays.toml`** — GPU texture array slot indices:
```toml
[array_indices]
albedo = [0, 1, 2, 3, 4]    # D2 array texture slots
normal = [0, 1, 2, 3, 4]
mra    = [0, 1, 2, 3, 4]
```

### PBR Texture Sets (22 Materials)

Every material has 3 texture maps at 1024x1024:

| Material | Albedo | Normal | MRA | Properties |
|----------|--------|--------|-----|------------|
| cloth | `cloth.png` | `cloth_n.png` | `cloth_mra.png` | Low metallic, medium rough |
| cobblestone | `cobblestone.png` | `cobblestone_n.png` | `cobblestone_mra.png` | Non-metallic, high rough |
| default | `default.png` | `default_n.png` | `default_mra.png` | Neutral fallback |
| dirt | `dirt.png` | `dirt_n.png` | `dirt_mra.png` | Non-metallic, high rough |
| forest_floor | `forest_floor.png` | `forest_floor_n.png` | `forest_floor_mra.png` | Organic, high AO |
| grass | `grass.png` | `grass_n.png` | `grass_mra.png` | Non-metallic, medium rough |
| gravel | `gravel.png` | `gravel_n.png` | `gravel_mra.png` | Non-metallic, very rough |
| ice | `ice.png` | `ice_n.png` | `ice_mra.png` | Low metallic, very smooth |
| metal_rusted | `metal_rusted.png` | `metal_rusted_n.png` | `metal_rusted_mra.png` | High metallic, varied rough |
| moss | `moss.png` | `moss_n.png` | `moss_mra.png` | Non-metallic, medium rough |
| mountain_rock | `mountain_rock.png` | `mountain_rock_n.png` | `mountain_rock_mra.png` | Non-metallic, high rough |
| mud | `mud.png` | `mud_n.png` | `mud_mra.png` | Non-metallic, high rough |
| plaster | `plaster.png` | `plaster_n.png` | `plaster_mra.png` | Non-metallic, medium rough |
| rock_lichen | `rock_lichen.png` | `rock_lichen_n.png` | `rock_lichen_mra.png` | Non-metallic, varied rough |
| rock_slate | `rock_slate.png` | `rock_slate_n.png` | `rock_slate_mra.png` | Non-metallic, medium rough |
| roof_tile | `roof_tile.png` | `roof_tile_n.png` | `roof_tile_mra.png` | Non-metallic, medium rough |
| sand | `sand.png` | `sand_n.png` | `sand_mra.png` | Non-metallic, medium rough |
| snow | `snow.png` | `snow_n.png` | `snow_mra.png` | Non-metallic, low rough |
| stone | `stone.png` | `stone_n.png` | `stone_mra.png` | Non-metallic, high rough |
| tree_bark | `tree_bark.png` | `tree_bark_n.png` | `tree_bark_mra.png` | Non-metallic, high rough |
| tree_leaves | `tree_leaves.png` | `tree_leaves_n.png` | `tree_leaves_mra.png` | Non-metallic, medium rough |
| wood_planks | `wood_planks.png` | `wood_planks_n.png` | `wood_planks_mra.png` | Non-metallic, medium rough |

### MRA Packing Convention

The engine uses **MRA** packed textures (not ORM or ARM):
- **Red channel**: Metallic (0.0 = dielectric, 1.0 = metal)
- **Green channel**: Roughness (0.0 = mirror, 1.0 = matte)
- **Blue channel**: Ambient Occlusion (0.0 = fully occluded, 1.0 = fully exposed)

### Baking Pipeline

Source PNGs are baked to KTX2 (GPU-compressed) using `toktx`:
```bash
toktx --t2 --encode uastc --genmipmap output.ktx2 input.png
```

Baked textures live in `materials/baked/` and are bound to GPU D2 array textures at `group=1`:
- Binding 0: Albedo array
- Binding 1: Albedo sampler
- Binding 2: Normal array
- Binding 3: Linear sampler
- Binding 4: MRA array

---

## HDRI Environment Lighting

### Catalog System

The `hdri/hdri_catalog.toml` maps HDRIs to biomes and time-of-day. The renderer selects the best environment map based on the current biome and world clock.

### Available HDRIs (7)

| Name | Time | Biomes | Source |
|------|------|--------|--------|
| kloppenheim_daytime | Day | Grassland, Forest, Mountain | PolyHaven |
| kloppenheim_clear | Day | Desert, Beach | PolyHaven |
| spruit_sunrise | Morning | Grassland, Forest, Beach, River | PolyHaven |
| venice_sunset | Evening | Desert, Beach, Grassland | PolyHaven |
| qwantani_moonrise | Night | All biomes | PolyHaven |
| rogland_night | Night | Grassland, Forest, Mountain, Beach | PolyHaven |
| sky_equirect | Day | Grassland | Built-in |

### Time-of-Day Coverage

| Time | HDRI Count | Status |
|------|-----------|--------|
| Day | 3 | Complete |
| Morning | 1 | Minimum (expand recommended) |
| Evening | 1 | Minimum (expand recommended) |
| Night | 2 | Complete |

### Fallback Matrix

Every biome+time combination has a defined fallback in the catalog. The renderer will never encounter a missing environment map.

---

## Audio Library

### Format Policy

Three formats maintained per audio asset for platform flexibility:
- **WAV**: Lossless source (game runtime, editors)
- **OGG**: Compressed lossy (game runtime, streaming)
- **MP3**: Compressed lossy (web exports, legacy)

### Audio Categories

| Category | Count | Description |
|----------|-------|-------------|
| Loops | ~34 per format | Ambient loops (wind, rain, fire, etc.) |
| Tracks | ~20 per format | Music tracks |
| Water Ambient | 3 variants | Mono + stereo water ambience |

---

## 3D Models

| Collection | Format | Count | Description |
|-----------|--------|-------|-------------|
| Castles & Forts | GLB | ~470 | Modular castle/fort building pieces |
| Amber NPC | FBX | 77 | CC4 character model with full texture set |
| Greybox | GLB | ~4 | Prototype placeholder meshes |

---

## Validation

Run the asset validation script to check library integrity:

```powershell
# Full validation
.\scripts\validate_assets.ps1

# Verbose output
.\scripts\validate_assets.ps1 -Verbose

# Specific section only
.\scripts\validate_assets.ps1 -Section biomes

# Auto-fix issues
.\scripts\validate_assets.ps1 -Fix
```

### Validation Checks

| Section | What It Checks |
|---------|----------------|
| **Biomes** | All engine biomes have materials.toml + arrays.toml |
| **Textures** | All referenced textures exist, triples complete (albedo + normal + MRA) |
| **HDRI** | Catalog references valid files, all time-of-day slots covered |
| **Naming** | snake_case directories, no spaces in filenames |
| **Orphans** | No empty directories, no loose files in root |

---

## Adding New Materials

1. Create 3 textures at 1024x1024: `material.png`, `material_n.png`, `material_mra.png`
2. Place in `materials/`
3. Add to the appropriate biome `materials.toml` layer
4. Update `arrays.toml` indices
5. Bake to KTX2: `toktx --t2 --encode uastc --genmipmap materials/baked/material.ktx2 materials/material.png`
6. Run `.\scripts\validate_assets.ps1` to verify

## Adding New Biomes

1. Create `materials/<biome_name>/` folder
2. Add `materials.toml` with 5 layers (copy from existing biome as template)
3. Add `arrays.toml` with index assignments
4. Update `hdri/hdri_catalog.toml` fallback matrix
5. Add `BiomeType` variant to `astraweave-terrain/src/biome.rs`
6. Run validation

---

## Known Issues

| Issue | Severity | Notes |
|-------|----------|-------|
| 12 KTX2 MRA stubs in `baked/` | Medium | 84-byte placeholders, need rebake from source PNGs |
| Amber-Npc uses PascalCase | Low | 3rd-party model, renaming would break references |
| Morning/Evening HDRI gaps | Low | Only 1 HDRI each; more variety recommended |

---

## License Notes

- **PolyHaven assets** (materials, HDRIs): CC0 (public domain)
- **Amber NPC**: Check original CC4 license
- **Castles & Forts pack**: Check original asset pack license
- **Engine-generated textures** (MRA maps, procedural): MIT (engine license)
