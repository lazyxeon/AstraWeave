#!/usr/bin/env python3
"""
Import real PBR terrain textures from PolyHaven pine_forest and grass_hd packs,
replacing procedural placeholder textures.

Usage:
    python scripts/import_terrain_textures.py [--dry-run]

Sources:
    assets/textures/pine_forest/  (PolyHaven CC0 forest textures)
    assets/textures/grass_hd/     (PolyHaven CC0 4K grass)
    Procedural (sand, snow)       (no real sources available in workspace)

Output (to assets/materials/):
    {name}.png       RGBA 2048x2048 sRGB albedo
    {name}_n.png     RGBA 2048x2048 linear OpenGL normal
    {name}_mra.png   RGBA 2048x2048 linear (R=metallic, G=roughness, B=AO)
"""

import os
import sys
import shutil
from pathlib import Path

import numpy as np
from PIL import Image
from scipy.ndimage import gaussian_filter

# ── Paths ─────────────────────────────────────────────────────────────────────

REPO       = Path(__file__).resolve().parent.parent
ASSETS     = REPO / "assets"
MATERIALS  = ASSETS / "materials"
PINE       = ASSETS / "textures" / "pine_forest"
GRASS_HD   = ASSETS / "textures" / "grass_hd"
BACKUP_DIR = MATERIALS / "placeholder_backup"

TARGET = 2048   # must match BIOME_TEX_SIZE in terrain_renderer.rs

# ── Optional EXR support via OpenCV ──────────────────────────────────────────

os.environ["OPENCV_IO_ENABLE_OPENEXR"] = "1"   # must be set before cv2 import
try:
    import cv2
    _CV2 = True
except ImportError:
    _CV2 = False

# ── Texture source table ────────────────────────────────────────────────────
# Keys match the engine filenames expected by BIOME_ALBEDO/NORMAL/MRA_FILES
# in tools/aw_editor/src/viewport/terrain_renderer.rs

SOURCES = {
    # idx 0 — grass.png
    "grass": dict(
        albedo = GRASS_HD / "grass_medium_01_diff_4k.jpg",
        normal = PINE / "grass_medium_01_nor_gl.png",
        rough  = PINE / "grass_medium_01_rough.png",
        disp   = None,
    ),
    # idx 1 — sand.png   → procedural (no real sand in workspace)
    # idx 2 — forest_floor.png
    "forest_floor": dict(
        albedo = PINE / "forest_ground_04_diff.png",
        normal = PINE / "forest_ground_04_nor_gl.png",
        rough  = PINE / "forest_ground_04_rough.png",
        disp   = PINE / "forest_ground_04_disp.png",
    ),
    # idx 3 — mountain_rock.png
    "mountain_rock": dict(
        albedo = PINE / "rock_face_03_diff_1k.jpg",
        normal = PINE / "rock_face_03_nor_gl_1k.exr",
        rough  = PINE / "rock_face_03_rough_1k.exr",
        disp   = PINE / "rock_face_03_disp_1k.png",
    ),
    # idx 4 — snow.png   → procedural (no real snow in workspace)
    # idx 5 — mud.png
    "mud": dict(
        albedo = PINE / "forrest_ground_01_diff_1k.jpg",
        normal = PINE / "forrest_ground_01_nor_gl_1k.exr",
        rough  = PINE / "forrest_ground_01_rough_1k.jpg",
        disp   = PINE / "forrest_ground_01_disp_1k.png",
    ),
    # idx 6 — sand.png   (reuses idx 1, no action needed)
    # idx 7 — stone.png
    "stone": dict(
        albedo = PINE / "rocky_trail_diff.png",
        normal = PINE / "rocky_trail_nor_gl.png",
        rough  = PINE / "rocky_trail_rough.png",
        disp   = PINE / "rocky_trail_disp.png",
    ),
    # idx 8 — rock_slate.png
    "rock_slate": dict(
        albedo = PINE / "rock_moss_set_01_diff.png",
        normal = PINE / "rock_moss_set_01_nor_gl.png",
        rough  = PINE / "rock_moss_set_01_rough.png",
        disp   = None,
    ),
    # idx 9 — dirt.png
    "dirt": dict(
        albedo = PINE / "ganges_river_pebbles_diff_1k.jpg",
        normal = PINE / "ganges_river_pebbles_nor_gl_1k.exr",
        rough  = PINE / "ganges_river_pebbles_rough_1k.exr",
        disp   = PINE / "ganges_river_pebbles_disp_1k.png",
    ),
}

PROCEDURAL = ["sand", "snow"]

# ── I/O helpers ──────────────────────────────────────────────────────────────

def load(path: Path) -> np.ndarray:
    """Load PNG/JPG/EXR → uint8 RGBA array."""
    if not path.exists():
        raise FileNotFoundError(path)
    if path.suffix.lower() == ".exr":
        return _load_exr(path)
    return np.array(Image.open(path).convert("RGBA"))


def _load_exr(path: Path) -> np.ndarray:
    """Load EXR via OpenCV → uint8 RGBA."""
    if not _CV2:
        raise RuntimeError(
            f"EXR needs opencv-python:  pip install opencv-python"
        )
    raw = cv2.imread(str(path), cv2.IMREAD_UNCHANGED)
    if raw is None:
        raise RuntimeError(f"cv2 could not read {path.name}")
    if raw.dtype in (np.float32, np.float64):
        if raw.min() < -0.1:          # [-1,1] range → [0,1]
            raw = raw * 0.5 + 0.5
        raw = np.clip(raw, 0.0, 1.0)
        raw = (raw * 255.0 + 0.5).astype(np.uint8)
    if raw.ndim == 2:                 # grayscale
        h, w = raw.shape
        out = np.full((h, w, 4), 255, np.uint8)
        out[:, :, 0] = out[:, :, 1] = out[:, :, 2] = raw
        return out
    if raw.shape[2] == 3:             # BGR → RGBA
        rgb = cv2.cvtColor(raw, cv2.COLOR_BGR2RGB)
        h, w = rgb.shape[:2]
        out = np.full((h, w, 4), 255, np.uint8)
        out[:, :, :3] = rgb
        return out
    return cv2.cvtColor(raw, cv2.COLOR_BGRA2RGBA)


def resize(arr: np.ndarray, sz: int = TARGET) -> np.ndarray:
    im = Image.fromarray(arr, "RGBA")
    if im.size != (sz, sz):
        im = im.resize((sz, sz), Image.LANCZOS)
    return np.array(im)


def save(arr: np.ndarray, path: Path):
    Image.fromarray(arr, "RGBA").save(path, "PNG")
    kb = path.stat().st_size // 1024
    print(f"    wrote {path.name}  ({kb:,} KB)")


# ── Normal-from-height & MRA helpers ────────────────────────────────────────

def normal_from_height(h_u8: np.ndarray, strength: float = 2.0) -> np.ndarray:
    """Grayscale height → OpenGL tangent-space normal (RGBA uint8)."""
    h = h_u8.astype(np.float32) / 255.0
    dy, dx = np.gradient(h)
    nx = -dx * strength
    ny = -dy * strength
    nz = np.ones_like(h)
    ln = np.sqrt(nx * nx + ny * ny + nz * nz)
    nx /= ln; ny /= ln; nz /= ln
    sz = h.shape[0]
    out = np.full((sz, sz, 4), 255, np.uint8)
    out[:, :, 0] = ((nx * 0.5 + 0.5) * 255).astype(np.uint8)
    out[:, :, 1] = ((ny * 0.5 + 0.5) * 255).astype(np.uint8)
    out[:, :, 2] = ((nz * 0.5 + 0.5) * 255).astype(np.uint8)
    return out


def build_mra(rough: np.ndarray, disp_path) -> np.ndarray:
    """Roughness RGBA + optional displacement → MRA RGBA."""
    h, w = rough.shape[:2]
    out = np.full((h, w, 4), 255, np.uint8)
    out[:, :, 0] = 0                                                   # metallic
    out[:, :, 1] = rough[:, :, 0] if rough.ndim == 3 else rough        # roughness
    if disp_path and disp_path.exists():
        d = resize(load(disp_path), h)
        hf = d[:, :, 0].astype(np.float32) / 255.0
        ao = gaussian_filter(1.0 - hf, sigma=4.0)
        ao = 0.55 + 0.45 * ao
        out[:, :, 2] = (np.clip(ao, 0, 1) * 255).astype(np.uint8)
    else:
        out[:, :, 2] = 217                                             # flat ~0.85
    return out


# ── Procedural generators ───────────────────────────────────────────────────

def _fbm(sz: int, rng: np.random.RandomState, octaves: int = 6) -> np.ndarray:
    """Fractional Brownian Motion → [0, 1] float32 array."""
    from scipy.ndimage import zoom as _zoom
    acc = np.zeros((sz, sz), np.float32)
    amp = 1.0
    for i in range(octaves):
        res = max(sz >> (i + 2), 4)
        raw = rng.randn(res, res).astype(np.float32)
        layer = _zoom(gaussian_filter(raw, 1.2), sz / res, order=3)[:sz, :sz]
        acc += layer * amp
        amp *= 0.5
    mn, mx = acc.min(), acc.max()
    return (acc - mn) / (mx - mn + 1e-12)


def gen_sand(sz: int = TARGET):
    """Realistic sand with dune ripples and grain noise."""
    rng = np.random.RandomState(600)
    noise = _fbm(sz, rng)

    x = np.linspace(0, 1, sz, dtype=np.float32)
    xx, yy = np.meshgrid(x, x)
    co = xx * np.cos(0.35) + yy * np.sin(0.35)
    ripple = np.sin(co * 90) * 0.35 + np.sin(co * 230 + noise * 3) * 0.12
    ripple = (ripple - ripple.min()) / (ripple.max() - ripple.min() + 1e-8)
    height = np.clip(noise * 0.55 + ripple * 0.45, 0, 1)

    grain = rng.randint(-6, 7, (sz, sz), np.int16)
    delta = (height * 40 - 20).astype(np.int16)
    r  = np.clip(np.int16(198) + delta + grain,           0, 255).astype(np.uint8)
    gv = np.clip(np.int16(178) + (delta * 9 // 10) + grain, 0, 255).astype(np.uint8)
    b  = np.clip(np.int16(142) + (delta * 3 // 4) + grain,  0, 255).astype(np.uint8)
    a  = np.full((sz, sz), 255, np.uint8)
    albedo = np.stack([r, gv, b, a], axis=-1)

    nm = normal_from_height((height * 255).astype(np.uint8), 1.5)

    mra = np.full((sz, sz, 4), 255, np.uint8)
    mra[:, :, 0] = 0
    mra[:, :, 1] = np.clip(200 + (noise * 30 - 15).astype(np.int16),
                            180, 230).astype(np.uint8)
    ao = gaussian_filter(1.0 - height, 3.0)
    mra[:, :, 2] = (np.clip(0.75 + 0.25 * ao, 0, 1) * 255).astype(np.uint8)

    return albedo, nm, mra


def gen_snow(sz: int = TARGET):
    """Snow with subtle undulations and crystalline sparkle."""
    rng = np.random.RandomState(400)
    noise = _fbm(sz, rng, 5)

    sparkle = (rng.rand(sz, sz) > 0.97).astype(np.float32) * 0.15
    sparkle = gaussian_filter(sparkle, 0.8)
    height = noise

    base = (235 + (height * 18 - 9)).astype(np.int16)
    sp = (sparkle * 80).astype(np.int16)
    grain = rng.randint(-3, 4, (sz, sz), np.int16)
    r  = np.clip(base - 4 + grain + sp, 0, 255).astype(np.uint8)
    gv = np.clip(base - 1 + grain + sp, 0, 255).astype(np.uint8)
    b  = np.clip(base + 4 + grain + sp, 0, 255).astype(np.uint8)
    a  = np.full((sz, sz), 255, np.uint8)
    albedo = np.stack([r, gv, b, a], axis=-1)

    nm = normal_from_height((height * 255).astype(np.uint8), 0.8)

    mra = np.full((sz, sz, 4), 255, np.uint8)
    mra[:, :, 0] = 0
    mra[:, :, 1] = np.clip(160 + (noise * 40 - 20).astype(np.int16),
                            140, 200).astype(np.uint8)
    ao = gaussian_filter(1.0 - height, 2.0)
    mra[:, :, 2] = (np.clip(0.88 + 0.12 * ao, 0, 1) * 255).astype(np.uint8)

    return albedo, nm, mra


# ── Main ─────────────────────────────────────────────────────────────────────

def main():
    dry = "--dry-run" in sys.argv
    tag = " (DRY RUN)" if dry else ""
    print(f"{'=' * 60}\n  AstraWeave Terrain Texture Import{tag}\n{'=' * 60}")

    # Sanity checks
    for d, label in [(PINE, "pine_forest"), (GRASS_HD, "grass_hd")]:
        if not d.exists():
            sys.exit(f"ERROR: {label} not found at {d}")
    if not _CV2:
        print("WARNING: opencv-python not installed — EXR textures will\n"
              "         fallback to displacement-derived normals.\n"
              "         pip install opencv-python\n")

    # ── Backup ───────────────────────────────────────────────────────────
    print("[1/3] Backing up current placeholders …")
    if not dry:
        BACKUP_DIR.mkdir(exist_ok=True)
    names = list(SOURCES) + PROCEDURAL
    backed = 0
    for nm in names:
        for sfx in ("", "_n", "_mra"):
            src = MATERIALS / f"{nm}{sfx}.png"
            if src.exists():
                dst = BACKUP_DIR / src.name
                if not dst.exists() and not dry:
                    shutil.copy2(src, dst)
                    backed += 1
    print(f"  {backed} files backed up to placeholder_backup/\n")

    # ── Real textures ────────────────────────────────────────────────────
    print(f"[2/3] Importing {len(SOURCES)} real texture sets …")
    for name, s in SOURCES.items():
        print(f"\n  -- {name} --")
        if dry:
            for key in ("albedo", "normal", "rough", "disp"):
                p = s.get(key)
                status = "OK" if (p and p.exists()) else ("skip" if p is None else "MISSING")
                print(f"    {key:7s}: {status}  {p}")
            continue

        try:
            # Albedo
            alb = resize(load(s["albedo"]))
            save(alb, MATERIALS / f"{name}.png")

            # Normal: try source first, fall back to displacement-derived
            try:
                nrm = resize(load(s["normal"]))
            except Exception as e:
                print(f"    normal fallback: {e}")
                dp = s.get("disp")
                if dp and dp.exists():
                    print("    -> generating from displacement map")
                    d = resize(load(dp))
                    nrm = normal_from_height(d[:, :, 0], 2.0)
                else:
                    print("    -> using flat neutral normal")
                    nrm = np.full((TARGET, TARGET, 4), 255, np.uint8)
                    nrm[:, :, 0] = 128
                    nrm[:, :, 1] = 128
                    nrm[:, :, 2] = 255
            save(nrm, MATERIALS / f"{name}_n.png")

            # Roughness + MRA
            try:
                rgh = resize(load(s["rough"]))
            except Exception:
                print("    roughness fallback: flat 0.80")
                rgh = np.full((TARGET, TARGET, 4), 204, np.uint8)
            mra = build_mra(rgh, s.get("disp"))
            save(mra, MATERIALS / f"{name}_mra.png")

        except Exception as exc:
            print(f"    FAILED: {exc} — placeholder kept")

    # ── Procedural ───────────────────────────────────────────────────────
    print(f"\n[3/3] Generating {len(PROCEDURAL)} procedural textures …")
    gens = {"sand": gen_sand, "snow": gen_snow}
    for name in PROCEDURAL:
        print(f"\n  -- {name} (procedural) --")
        if dry:
            print(f"    [dry] would generate {name}")
            continue
        alb, nrm, mra = gens[name]()
        save(alb, MATERIALS / f"{name}.png")
        save(nrm, MATERIALS / f"{name}_n.png")
        save(mra, MATERIALS / f"{name}_mra.png")

    print(f"\n{'=' * 60}")
    if not dry:
        print("  Import complete!  Launch editor to verify:")
        print("    cargo run -p aw_editor --release")
    else:
        print("  Dry run finished — no files were modified.")
    print(f"{'=' * 60}")


if __name__ == "__main__":
    main()
