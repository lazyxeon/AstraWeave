use std::fs;

#[path = "support/gpu_capability.rs"]
mod gpu_capability;

fn headless_device(label: &'static str) -> Option<(wgpu::Device, wgpu::Queue)> {
    pollster::block_on(gpu_capability::request_headless(
        label,
        wgpu::Features::empty(),
        wgpu::Limits::downlevel_webgl2_defaults(),
        wgpu::PowerPreference::LowPower,
        false,
    ))
    .map(|gpu| (gpu.device, gpu.queue))
}

fn write_file(path: &std::path::Path, contents: &str) {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).unwrap();
    }
    fs::write(path, contents).unwrap();
}

fn tmpdir(prefix: &str) -> tempfile::TempDir {
    tempfile::Builder::new().prefix(prefix).tempdir().unwrap()
}

#[test]
#[cfg(feature = "textures")]
fn parses_materials_and_arrays_and_orders_layers() {
    let dir = tmpdir("materials_order");
    let base = dir.path().to_path_buf();
    // Create minimal images (1x1) for IO sanity
    let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 255]));
    img.save(base.join("tex_a.png")).unwrap();
    img.save(base.join("tex_b.png")).unwrap();
    img.save(base.join("nrm.png")).unwrap();
    img.save(base.join("mra.png")).unwrap();
    // materials.toml with two layers where the arrays.toml defines a reverse order
    write_file(
        &base.join("materials.toml"),
        r#"
        [biome]
        name = "test"

        [[layer]]
        key = "grass"
        albedo = "tex_a.png"
        normal = "nrm.png"
        mra = "mra.png"

        [[layer]]
        key = "rock"
        albedo = "tex_b.png"
        normal = "nrm.png"
        mra = "mra.png"
    "#,
    );
    write_file(
        &base.join("arrays.toml"),
        r#"
        [layers]
        rock = 0
        grass = 1
    "#,
    );

    let Some((device, queue)) = headless_device("materials-test device") else {
        return;
    };

    let mut mm = astraweave_render::MaterialManager::new();
    let (gpu, stats) = pollster::block_on(mm.load_pack_from_toml(
        &device,
        &queue,
        &base,
        &base.join("materials.toml"),
        &base.join("arrays.toml"),
    ))
    .expect("load pack");

    // Layer ordering should follow arrays.toml: rock=0, grass=1
    let layout = gpu.layout;
    assert_eq!(layout.count, 2);
    assert_eq!(layout.layer_indices.get("rock").copied(), Some(0));
    assert_eq!(layout.layer_indices.get("grass").copied(), Some(1));

    // Telemetry sanity
    assert_eq!(stats.biome, "test");
    assert_eq!(stats.layers_total, 2);
    // We provided all three for both layers
    assert_eq!(stats.albedo_loaded, 2);
    assert_eq!(stats.normal_loaded, 2);
    assert_eq!(stats.mra_loaded, 2);
}

#[test]
#[cfg(feature = "textures")]
fn packs_mra_from_separate_planes_when_missing_mra() {
    let dir = tmpdir("materials_mra_pack");
    let base = dir.path().to_path_buf();
    // 2x1 gray images for channels
    let mut m = image::GrayImage::new(2, 1);
    m.put_pixel(0, 0, image::Luma([10]));
    m.put_pixel(1, 0, image::Luma([20]));
    let mut r = image::GrayImage::new(2, 1);
    r.put_pixel(0, 0, image::Luma([30]));
    r.put_pixel(1, 0, image::Luma([40]));
    let mut a = image::GrayImage::new(2, 1);
    a.put_pixel(0, 0, image::Luma([50]));
    a.put_pixel(1, 0, image::Luma([60]));
    m.save(base.join("m.png")).unwrap();
    r.save(base.join("r.png")).unwrap();
    a.save(base.join("a.png")).unwrap();
    // Minimal albedo/normal
    let alb = image::RgbaImage::from_pixel(2, 1, image::Rgba([255, 255, 255, 255]));
    let nrm = image::RgbaImage::from_pixel(2, 1, image::Rgba([128, 128, 255, 255]));
    alb.save(base.join("alb.png")).unwrap();
    nrm.save(base.join("nrm.png")).unwrap();
    // materials.toml: no MRA path, but metallic/roughness/ao present
    write_file(
        &base.join("materials.toml"),
        r#"
        [biome]
        name = "pack"

        [[layer]]
        key = "only"
        albedo = "alb.png"
        normal = "nrm.png"
        metallic = "m.png"
        roughness = "r.png"
        ao = "a.png"
    "#,
    );
    write_file(
        &base.join("arrays.toml"),
        r#"
        [layers]
        only = 0
    "#,
    );

    let Some((device, queue)) = headless_device("materials-test device") else {
        return;
    };

    let mut mm = astraweave_render::MaterialManager::new();
    let (_gpu, stats) = pollster::block_on(mm.load_pack_from_toml(
        &device,
        &queue,
        &base,
        &base.join("materials.toml"),
        &base.join("arrays.toml"),
    ))
    .expect("load pack");
    // We expect the loader to have packed MRA from the separate planes
    assert_eq!(stats.mra_loaded, 1);
    assert_eq!(stats.mra_packed, 1);
    assert_eq!(stats.mra_substituted, 0);
}

#[test]
#[cfg(feature = "textures")]
fn path_resolution_uses_base_dir_and_normalizes() {
    let dir = tmpdir("materials_paths");
    let base = dir.path().to_path_buf();
    // create nested file we reference via a path that includes ..
    let nested = base.join("nested");
    fs::create_dir_all(&nested).unwrap();
    let alb = image::RgbaImage::from_pixel(1, 1, image::Rgba([1, 2, 3, 4]));
    // Some OS/filesystems allow ".." normalization when writing, so don't assume it fails.
    // Write the file at the base and reference it via ".." from a subdir below.
    alb.save(base.join("alb.png")).unwrap();

    write_file(
        &base.join("materials.toml"),
        r#"
        [biome]
        name = "paths"

        [[layer]]
        key = "k"
        albedo = "../alb.png"
    "#,
    );
    write_file(
        &base.join("arrays.toml"),
        r#"
        [layers]
        k = 0
    "#,
    );

    let Some((device, queue)) = headless_device("materials-test device") else {
        return;
    };

    let mut mm = astraweave_render::MaterialManager::new();
    // Should not error: joins resolve against base_dir and normalize
    let _ = pollster::block_on(mm.load_pack_from_toml(
        &device,
        &queue,
        &base.join("subdir"),
        &base.join("subdir/materials.toml"),
        &base.join("subdir/arrays.toml"),
    ))
    .err();
    // Actually provide the files at base/subdir path to make it load
    fs::create_dir_all(base.join("subdir")).unwrap();
    fs::copy(
        base.join("materials.toml"),
        base.join("subdir/materials.toml"),
    )
    .unwrap();
    fs::copy(base.join("arrays.toml"), base.join("subdir/arrays.toml")).unwrap();
    let (_gpu, stats) = pollster::block_on(mm.load_pack_from_toml(
        &device,
        &queue,
        &base.join("subdir"),
        &base.join("subdir/materials.toml"),
        &base.join("subdir/arrays.toml"),
    ))
    .expect("load OK");
    assert_eq!(stats.layers_total, 1);
    assert_eq!(stats.albedo_loaded, 1);
}

#[test]
fn concise_summary_formats_expected_fields() {
    let s = astraweave_render::MaterialLoadStats {
        biome: "x".into(),
        layers_total: 3,
        albedo_loaded: 1,
        albedo_substituted: 2,
        normal_loaded: 3,
        normal_substituted: 0,
        mra_loaded: 2,
        mra_packed: 1,
        mra_substituted: 1,
        gpu_memory_bytes: 10 * 1024 * 1024,
    };
    let line = s.concise_summary();
    assert!(line.contains("biome=x"));
    assert!(line.contains("layers=3"));
    assert!(line.contains("albedo L/S=1/2"));
    assert!(line.contains("normal L/S=3/0"));
    assert!(line.contains("mra L+P/S=2+1/1"));
    assert!(line.contains("gpu=10.00 MiB"));
}
