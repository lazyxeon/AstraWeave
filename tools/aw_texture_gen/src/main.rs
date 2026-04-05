use anyhow::Result;
use astraweave_security::path::safe_under;
use clap::Parser;
use image::{ImageBuffer, Rgba};
use std::{fs, path::PathBuf};

#[derive(Parser, Debug)]
#[command(
    name = "aw_texture_gen",
    about = "Generate high-fidelity terrain textures into assets/"
)]
struct Args {
    /// Output directory (defaults to repo-level assets/)
    #[arg(short, long)]
    out: Option<PathBuf>,

    /// Force overwrite existing files
    #[arg(long, default_value_t = true)]
    force: bool,

    /// Base seed
    #[arg(long, default_value_t = 1337u32)]
    seed: u32,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let requested_out = args.out.unwrap_or_else(|| PathBuf::from("assets"));

    // Security: Validate output path is within allowed base directory
    let base = PathBuf::from(".");
    let out = safe_under(&base, &requested_out)
        .map_err(|e| anyhow::anyhow!("Invalid output path: {}", e))?;

    fs::create_dir_all(&out)?;
    println!(
        "Writing textures to {} (force={})",
        out.display(),
        args.force
    );

    synth_if_missing(
        &out,
        "grass.png",
        args.seed ^ 0x101,
        args.force,
        synth_grass,
    )?;
    synth_mra_if_missing(&out, "grass_mra.png", 0.2, 0.7, 0.0, args.force)?;
    synth_emissive_if_missing(&out, "grass_e.png", args.force)?;

    synth_if_missing(&out, "dirt.png", args.seed ^ 0x202, args.force, synth_dirt)?;
    synth_mra_if_missing(&out, "dirt_mra.png", 0.9, 0.85, 0.0, args.force)?;
    synth_emissive_if_missing(&out, "dirt_e.png", args.force)?;

    synth_if_missing(&out, "sand.png", args.seed ^ 0x303, args.force, synth_sand)?;
    synth_mra_if_missing(&out, "sand_mra.png", 0.8, 0.6, 0.0, args.force)?;
    synth_emissive_if_missing(&out, "sand_e.png", args.force)?;

    synth_if_missing(
        &out,
        "stone.png",
        args.seed ^ 0x404,
        args.force,
        synth_stone,
    )?;
    synth_mra_if_missing(&out, "stone_mra.png", 0.6, 0.9, 0.05, args.force)?;
    synth_emissive_if_missing(&out, "stone_e.png", args.force)?;

    synth_if_missing(
        &out,
        "forest_floor.png",
        args.seed ^ 0x505,
        args.force,
        synth_forest_floor,
    )?;
    synth_mra_if_missing(&out, "forest_floor_mra.png", 0.9, 0.8, 0.0, args.force)?;
    synth_emissive_if_missing(&out, "forest_floor_e.png", args.force)?;

    println!("Done.");
    Ok(())
}

fn synth_if_missing<F: Fn(u32, u32, u32) -> ImageBuffer<Rgba<u8>, Vec<u8>>>(
    out_dir: &std::path::Path,
    name: &str,
    seed: u32,
    force: bool,
    f: F,
) -> Result<()> {
    let path = out_dir.join(name);
    if force || !path.exists() {
        let img = f(2048, 2048, seed); // 2K maps to keep repo size reasonable
        img.save(&path)?;
        // normal
        let npath = out_dir.join(name.replace(".png", "_n.png"));
        let normal = height_to_normal(
            &img,
            match name {
                n if n.ends_with("grass.png") => 1.8,
                n if n.ends_with("dirt.png") => 2.2,
                n if n.ends_with("sand.png") => 1.3,
                n if n.ends_with("stone.png") => 2.8,
                n if n.ends_with("forest_floor.png") => 2.2,
                _ => 2.0,
            },
        );
        normal.save(npath)?;
    }
    Ok(())
}

fn synth_mra_if_missing(
    out_dir: &std::path::Path,
    name: &str,
    ao: f32,
    rough: f32,
    metal: f32,
    force: bool,
) -> Result<()> {
    let path = out_dir.join(name);
    if force || !path.exists() {
        let (w, h) = (8u32, 8u32);
        let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(w, h);
        let to_u8 = |v: f32| ((v.clamp(0.0, 1.0)) * 255.0) as u8;
        let px = Rgba([to_u8(ao), to_u8(rough), to_u8(metal), 255]);
        for y in 0..h {
            for x in 0..w {
                img.put_pixel(x, y, px);
            }
        }
        img.save(path)?;
    }
    Ok(())
}

fn synth_emissive_if_missing(out_dir: &std::path::Path, name: &str, force: bool) -> Result<()> {
    let path = out_dir.join(name);
    if force || !path.exists() {
        let (w, h) = (4u32, 4u32);
        let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(w, h);
        let px = Rgba([0, 0, 0, 255]);
        for y in 0..h {
            for x in 0..w {
                img.put_pixel(x, y, px);
            }
        }
        img.save(path)?;
    }
    Ok(())
}

// ----- noise utils -----
fn hash(mut x: u32) -> u32 {
    x ^= x >> 17;
    x = x.wrapping_mul(0xed5ad4bb);
    x ^= x >> 11;
    x = x.wrapping_mul(0xac4c1b51);
    x ^= x >> 15;
    x = x.wrapping_mul(0x31848bab);
    x ^= x >> 14;
    x
}
fn noise2d(x: i32, y: i32, seed: u32) -> f32 {
    let h = hash(x as u32 ^ (y as u32).rotate_left(16) ^ seed);
    (h as f32 / u32::MAX as f32) * 2.0 - 1.0
}
fn smooth_noise(x: f32, y: f32, seed: u32) -> f32 {
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let xf = x - x0 as f32;
    let yf = y - y0 as f32;
    let n00 = noise2d(x0, y0, seed);
    let n10 = noise2d(x0 + 1, y0, seed);
    let n01 = noise2d(x0, y0 + 1, seed);
    let n11 = noise2d(x0 + 1, y0 + 1, seed);
    let sx = xf * xf * (3.0 - 2.0 * xf);
    let sy = yf * yf * (3.0 - 2.0 * yf);
    let ix0 = n00 * (1.0 - sx) + n10 * sx;
    let ix1 = n01 * (1.0 - sx) + n11 * sx;
    ix0 * (1.0 - sy) + ix1 * sy
}
fn fbm(x: f32, y: f32, seed: u32, oct: i32, lac: f32, gain: f32) -> f32 {
    let (mut f, mut a, mut sum, mut norm) = (1.0, 0.5, 0.0, 0.0);
    for i in 0..oct {
        let n = smooth_noise(x * f, y * f, seed.wrapping_add(i as u32));
        sum += a * n;
        norm += a;
        f *= lac;
        a *= gain;
    }
    sum / norm.max(1e-6)
}

// ----- material synth -----
fn synth_grass(w: u32, h: u32, seed: u32) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut img = ImageBuffer::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let u = x as f32 / w as f32 * 48.0;
            let v = y as f32 / h as f32 * 48.0;
            let base = fbm(u, v, seed, 7, 2.0, 0.5);
            let clump = fbm(u * 0.3, v * 0.3, seed ^ 0x5aa5, 5, 2.0, 0.6);
            let fine = fbm(u * 10.0, v * 10.0, seed ^ 0x1337, 4, 2.0, 0.35);
            let dirt = fbm(u * 0.9, v * 0.9, seed ^ 0xcafe, 5, 2.0, 0.45);
            let height = (0.6 + 0.25 * base + 0.2 * clump + 0.08 * fine).clamp(0.0, 1.0);
            let dirt_factor = (dirt > 0.4) as i32 as f32 * 0.25;
            let gcol = 95.0 + 120.0 * height;
            let ycol = 70.0 + 50.0 * (1.0 - height);
            let (r, g, b) = if dirt_factor > 0.0 {
                let mix = dirt_factor;
                let gr = gcol * 0.3 + ycol * 0.2;
                let gg = gcol;
                let gb = gcol * 0.15;
                let dr = 75.0 + 35.0 * height;
                let dg = 55.0 + 25.0 * height;
                let db = 35.0 + 15.0 * height;
                (
                    (gr * (1.0 - mix) + dr * mix) as u8,
                    (gg * (1.0 - mix) + dg * mix) as u8,
                    (gb * (1.0 - mix) + db * mix) as u8,
                )
            } else {
                (
                    (gcol * 0.3 + ycol * 0.2) as u8,
                    gcol as u8,
                    (gcol * 0.15) as u8,
                )
            };
            img.put_pixel(x, y, Rgba([r, g, b, 255]));
        }
    }
    img
}

fn synth_dirt(w: u32, h: u32, seed: u32) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut img = ImageBuffer::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let u = x as f32 / w as f32 * 20.0;
            let v = y as f32 / h as f32 * 20.0;
            let grains = fbm(u * 1.3, v * 1.1, seed ^ 0xdead00, 6, 2.0, 0.5);
            let pebbles = fbm(u * 0.4, v * 0.4, seed ^ 0xbeef11, 3, 2.0, 0.55);
            let height = (0.5 + 0.4 * grains + 0.25 * pebbles).clamp(0.0, 1.0);
            let r = (58.0 + 110.0 * height) as u8;
            let g = (44.0 + 65.0 * height) as u8;
            let b = (34.0 + 45.0 * height) as u8;
            img.put_pixel(x, y, Rgba([r, g, b, 255]));
        }
    }
    img
}

fn synth_sand(w: u32, h: u32, seed: u32) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut img = ImageBuffer::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let u = x as f32 / w as f32 * 28.0;
            let v = y as f32 / h as f32 * 28.0;
            let fine = fbm(u * 6.0, v * 6.0, seed ^ 0xc0ff33, 8, 2.0, 0.45);
            let dune = fbm(u * 0.4, v * 0.4, seed ^ 0xfade01, 6, 2.0, 0.55);
            let ripple = fbm(u * 14.0, v * 3.0, seed ^ 0x7ead33, 6, 2.0, 0.35);
            let macrof = fbm(u * 0.15, v * 0.15, seed ^ 0xbeac44, 4, 2.0, 0.65);
            let height =
                (0.55 + 0.18 * fine + 0.15 * dune + 0.12 * ripple + 0.08 * macrof).clamp(0.0, 1.0);
            let r = (215.0 + 35.0 * height) as u8;
            let g = (195.0 + 40.0 * height) as u8;
            let b = (145.0 + 25.0 * height) as u8;
            img.put_pixel(x, y, Rgba([r, g, b, 255]));
        }
    }
    img
}

fn synth_stone(w: u32, h: u32, seed: u32) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut img = ImageBuffer::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let u = x as f32 / w as f32 * 18.0;
            let v = y as f32 / h as f32 * 18.0;
            let veins = fbm(u * 2.2, v * 2.0, seed ^ 0x7777, 8, 2.1, 0.45);
            let base = fbm(u, v, seed ^ 0x1111, 7, 2.0, 0.5);
            let crack = fbm(u * 4.0, v * 4.0, seed ^ 0xc7ac4, 5, 2.0, 0.35);
            let weather = fbm(u * 0.6, v * 0.6, seed ^ 0xaea754, 5, 2.0, 0.55);
            let height =
                (0.6 + 0.25 * base + 0.15 * veins - 0.08 * crack + 0.08 * weather).clamp(0.0, 1.0);
            let base_gray = 140.0 + 70.0 * height;
            let crack_dark = if crack > 0.6 { -25.0 } else { 0.0 };
            let r = (base_gray + crack_dark) as u8;
            let g = (base_gray + crack_dark * 0.8) as u8;
            let b = (base_gray + crack_dark * 0.6) as u8;
            img.put_pixel(x, y, Rgba([r, g, b, 255]));
        }
    }
    img
}

fn synth_forest_floor(w: u32, h: u32, seed: u32) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut img = ImageBuffer::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let u = x as f32 / w as f32 * 30.0;
            let v = y as f32 / h as f32 * 30.0;
            let leaf = fbm(u * 3.0, v * 3.0, seed ^ 0x1eaf7, 7, 2.0, 0.45);
            let moss = fbm(u * 1.6, v * 1.6, seed ^ 0x90557, 6, 2.0, 0.5);
            let soil = fbm(u * 1.2, v * 1.2, seed ^ 0x5011, 5, 2.0, 0.55);
            let height = (0.55 + 0.2 * leaf + 0.15 * moss + 0.12 * soil).clamp(0.0, 1.0);
            let r = (95.0 + 35.0 * height) as u8;
            let g = (75.0 + 30.0 * height) as u8;
            let b = (45.0 + 20.0 * height) as u8;
            img.put_pixel(x, y, Rgba([r, g, b, 255]));
        }
    }
    img
}

fn height_to_normal(
    img: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    strength: f32,
) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let (w, h) = img.dimensions();
    let mut out = ImageBuffer::new(w, h);
    let h_sample = |x: i32, y: i32| -> f32 {
        let xi = ((x % w as i32) + w as i32) % w as i32;
        let yi = ((y % h as i32) + h as i32) % h as i32;
        let p = img.get_pixel(xi as u32, yi as u32);
        (0.2126 * p[0] as f32 + 0.7152 * p[1] as f32 + 0.0722 * p[2] as f32) / 255.0
    };
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let dx = (h_sample(x + 1, y) - h_sample(x - 1, y)) * strength;
            let dy = (h_sample(x, y + 1) - h_sample(x, y - 1)) * strength;
            let mut nx = -dx;
            let mut ny = -dy;
            let mut nz = 1.0;
            let len = (nx * nx + ny * ny + nz * nz).sqrt();
            nx /= len;
            ny /= len;
            nz /= len;
            let r = ((nx * 0.5 + 0.5) * 255.0) as u8;
            let g = ((ny * 0.5 + 0.5) * 255.0) as u8;
            let b = ((nz * 0.5 + 0.5) * 255.0) as u8;
            out.put_pixel(x as u32, y as u32, Rgba([r, g, b, 255]));
        }
    }
    out
}
