// Procedural texture generation for AstraWeave Unified Showcase
use image::{Rgba, RgbaImage};

/// Generate a simple wood bark texture (brown with vertical grain)
pub fn generate_tree_bark_texture(width: u32, height: u32) -> RgbaImage {
    let mut img = RgbaImage::new(width, height);

    for y in 0..height {
        for x in 0..width {
            // Base brown color
            let base_r = 90 + (x % 30) as u8;
            let base_g = 60 + (x % 20) as u8;
            let base_b = 40;

            // Add vertical grain with noise
            let grain = ((x as f32 * 0.1).sin() * 20.0) as i32;
            let noise = ((x as f32 * 0.5 + y as f32 * 0.3).sin() * 10.0) as i32;

            let r = (base_r as i32 + grain + noise).clamp(50, 120) as u8;
            let g = (base_g as i32 + grain + noise).clamp(30, 90) as u8;
            let b = (base_b as i32 + noise).clamp(20, 60) as u8;

            img.put_pixel(x, y, Rgba([r, g, b, 255]));
        }
    }

    img
}

/// Generate oak leaves texture (green with variation)
pub fn generate_leaves_texture(width: u32, height: u32) -> RgbaImage {
    let mut img = RgbaImage::new(width, height);

    for y in 0..height {
        for x in 0..width {
            // Base green color
            let base_r = 60;
            let base_g = 120;
            let base_b = 40;

            // Add organic noise
            let noise_r = ((x as f32 * 0.3).sin() * (y as f32 * 0.3).cos() * 30.0) as i32;
            let noise_g = ((x as f32 * 0.2).cos() * (y as f32 * 0.2).sin() * 40.0) as i32;
            let noise_b = ((x as f32 * 0.4).sin() * (y as f32 * 0.1).cos() * 20.0) as i32;

            let r = (base_r as i32 + noise_r).clamp(40, 100) as u8;
            let g = (base_g as i32 + noise_g).clamp(80, 180) as u8;
            let b = (base_b as i32 + noise_b).clamp(20, 80) as u8;

            img.put_pixel(x, y, Rgba([r, g, b, 255]));
        }
    }

    img
}

/// Generate grass texture (green with blade variation)
pub fn generate_grass_texture(width: u32, height: u32) -> RgbaImage {
    let mut img = RgbaImage::new(width, height);

    for y in 0..height {
        for x in 0..width {
            // Base grass green
            let base_r = 50;
            let base_g = 140;
            let base_b = 50;

            // Add blade-like patterns
            let blade = ((x as f32 * 0.8).sin() * (y as f32 * 1.2).cos() * 40.0) as i32;
            let variation = ((x as f32 * 0.1 + y as f32 * 0.1).sin() * 30.0) as i32;

            let r = (base_r as i32 + variation).clamp(30, 80) as u8;
            let g = (base_g as i32 + blade + variation).clamp(100, 200) as u8;
            let b = (base_b as i32 + variation).clamp(30, 80) as u8;

            img.put_pixel(x, y, Rgba([r, g, b, 255]));
        }
    }

    img
}

/// Generate dirt texture (brown with rocky patches)
pub fn generate_dirt_texture(width: u32, height: u32) -> RgbaImage {
    let mut img = RgbaImage::new(width, height);

    for y in 0..height {
        for x in 0..width {
            // Base brown dirt
            let base_r = 120;
            let base_g = 90;
            let base_b = 60;

            // Add rocky patches and variation
            let rocks = ((x as f32 * 0.15).sin() * (y as f32 * 0.15).cos() * 50.0) as i32;
            let grain = ((x as f32 * 0.5 + y as f32 * 0.5).sin() * 20.0) as i32;

            let r = (base_r as i32 + rocks + grain).clamp(80, 160) as u8;
            let g = (base_g as i32 + rocks + grain).clamp(60, 130) as u8;
            let b = (base_b as i32 + grain).clamp(40, 90) as u8;

            img.put_pixel(x, y, Rgba([r, g, b, 255]));
        }
    }

    img
}

/// Generate simple normal map (slight bumps)
pub fn generate_normal_map(width: u32, height: u32) -> RgbaImage {
    let mut img = RgbaImage::new(width, height);

    for y in 0..height {
        for x in 0..width {
            // Generate height variations
            let height_val =
                ((x as f32 * 0.2).sin() * (y as f32 * 0.2).cos() * 0.3 + 0.5).clamp(0.0, 1.0);

            // Calculate simple normals (pointing mostly up)
            let nx = ((height_val - 0.5) * 0.3 + 0.5) * 255.0;
            let ny = ((height_val - 0.5) * 0.3 + 0.5) * 255.0;
            let nz = 200; // Mostly pointing up (blue in normal map)

            img.put_pixel(x, y, Rgba([nx as u8, ny as u8, nz, 255]));
        }
    }

    img
}

/// Generate thatch roof texture (straw-like)
pub fn generate_thatch_texture(width: u32, height: u32) -> RgbaImage {
    let mut img = RgbaImage::new(width, height);

    for y in 0..height {
        for x in 0..width {
            // Base straw color
            let base_r = 200;
            let base_g = 170;
            let base_b = 100;

            // Add straw fiber patterns
            let fibers = ((x as f32 * 1.5).sin() * 30.0 + (y as f32 * 0.3).cos() * 20.0) as i32;
            let variation = ((x as f32 * 0.1 + y as f32 * 0.1).sin() * 25.0) as i32;

            let r = (base_r as i32 + fibers + variation).clamp(150, 230) as u8;
            let g = (base_g as i32 + fibers + variation).clamp(120, 200) as u8;
            let b = (base_b as i32 + variation).clamp(60, 140) as u8;

            img.put_pixel(x, y, Rgba([r, g, b, 255]));
        }
    }

    img
}

/// Generate adobe/plaster wall texture (beige with roughness)
pub fn generate_adobe_texture(width: u32, height: u32) -> RgbaImage {
    let mut img = RgbaImage::new(width, height);

    for y in 0..height {
        for x in 0..width {
            // Base beige/tan color
            let base_r = 220;
            let base_g = 200;
            let base_b = 170;

            // Add plaster roughness
            let rough = ((x as f32 * 0.3).sin() * (y as f32 * 0.3).cos() * 30.0) as i32;
            let variation = ((x as f32 * 0.05 + y as f32 * 0.05).sin() * 20.0) as i32;

            let r = (base_r as i32 + rough + variation).clamp(180, 240) as u8;
            let g = (base_g as i32 + rough + variation).clamp(160, 220) as u8;
            let b = (base_b as i32 + variation).clamp(130, 200) as u8;

            img.put_pixel(x, y, Rgba([r, g, b, 255]));
        }
    }

    img
}
