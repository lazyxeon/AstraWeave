//! Rigid Kenney props and the node-animated figurine used as an enemy.
//!
//! The figurine is not a skinned mesh. Each limb is its own primitive, and
//! clips rotate those nodes. `NodeActor::pose` bakes one `CpuMesh`.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use astraweave_render::mesh::{CpuMesh, MeshVertex};
use glam::{Mat4, Quat, Vec2, Vec3, Vec4};
use gltf::animation::util::ReadOutputs;

pub struct TexturedMesh {
    pub mesh: CpuMesh,
    pub albedo: Option<Rgba>,
}

pub struct Rgba {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

struct Part {
    node: usize,
    vertices: Vec<MeshVertex>,
    indices: Vec<u32>,
}

struct NodeRest {
    translation: Vec3,
    rotation: Quat,
    scale: Vec3,
}

enum ChannelValues {
    Translation(Vec<Vec3>),
    Rotation(Vec<Quat>),
}

struct Channel {
    node: usize,
    times: Vec<f32>,
    values: ChannelValues,
}

struct Clip {
    duration: f32,
    channels: Vec<Channel>,
}

pub struct NodeActor {
    parts: Vec<Part>,
    rest: Vec<NodeRest>,
    parents: Vec<Option<usize>>,
    clips: BTreeMap<String, Clip>,
    pub albedo: Option<Rgba>,
}

impl NodeActor {
    pub fn load(path: &Path) -> Result<Self> {
        let (doc, buffers, images) = crate::assets::import_model(path)?;
        let albedo = rgba_from_images(&images);
        let mut parents = vec![None; doc.nodes().len()];
        for node in doc.nodes() {
            for child in node.children() {
                parents[child.index()] = Some(node.index());
            }
        }
        let mut rest = Vec::with_capacity(doc.nodes().len());
        for node in doc.nodes() {
            let (translation, rotation, scale) = node.transform().decomposed();
            rest.push(NodeRest {
                translation: Vec3::from(translation),
                rotation: Quat::from_array(rotation),
                scale: Vec3::from(scale),
            });
        }
        let mut parts = Vec::new();
        for node in doc.nodes() {
            let Some(mesh) = node.mesh() else {
                continue;
            };
            for primitive in mesh.primitives() {
                if let Some((vertices, indices)) = read_primitive(&primitive, &buffers) {
                    parts.push(Part {
                        node: node.index(),
                        vertices,
                        indices,
                    });
                }
            }
        }
        let mut clips = BTreeMap::new();
        for animation in doc.animations() {
            let name = animation.name().unwrap_or("clip").to_string();
            if let Some(clip) = read_clip(&animation, &buffers) {
                clips.insert(name, clip);
            }
        }
        Ok(Self {
            parts,
            rest,
            parents,
            clips,
            albedo,
        })
    }

    pub fn figurine_path() -> std::path::PathBuf {
        crate::assets::asset_dir().join("models/figurine.glb")
    }

    pub fn has_clip(&self, name: &str) -> bool {
        self.clips.contains_key(name)
    }

    pub fn duration(&self, clip_name: &str) -> Option<f32> {
        self.clips.get(clip_name).map(|clip| clip.duration)
    }

    pub fn pose(&self, clip_name: &str, time: f32) -> Result<CpuMesh> {
        self.pose_at(clip_name, time, true)
    }

    pub fn pose_at(&self, clip_name: &str, time: f32, looping: bool) -> Result<CpuMesh> {
        let clip = self
            .clips
            .get(clip_name)
            .with_context(|| format!("missing clip {clip_name}"))?;
        let duration = clip.duration.max(0.001);
        let t = if looping {
            time.rem_euclid(duration)
        } else {
            time.clamp(0.0, duration - 0.001)
        };
        let locals = self.locals_at(clip, t);
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for part in &self.parts {
            let world = world_matrix(part.node, &locals, &self.parents);
            let base = vertices.len() as u32;
            for vertex in &part.vertices {
                let position = world.transform_point3(Vec3::from(vertex.position));
                let normal = world
                    .transform_vector3(Vec3::from(vertex.normal))
                    .normalize_or_zero();
                vertices.push(MeshVertex::new(
                    position,
                    normal,
                    Vec4::from(vertex.tangent),
                    Vec2::from(vertex.uv),
                ));
            }
            indices.extend(part.indices.iter().map(|index| index + base));
        }
        Ok(CpuMesh {
            vertices,
            indices,
            albedo_image: None,
            texture_source_hint: None,
        })
    }

    fn locals_at(&self, clip: &Clip, time: f32) -> Vec<Mat4> {
        self.rest
            .iter()
            .enumerate()
            .map(|(index, rest)| {
                let mut translation = rest.translation;
                let mut rotation = rest.rotation;
                for channel in &clip.channels {
                    if channel.node != index {
                        continue;
                    }
                    match &channel.values {
                        ChannelValues::Translation(values) => {
                            translation = sample_vec3(&channel.times, values, time);
                        }
                        ChannelValues::Rotation(values) => {
                            rotation = sample_quat(&channel.times, values, time);
                        }
                    }
                }
                Mat4::from_scale_rotation_translation(rest.scale, rotation, translation)
            })
            .collect()
    }
}

pub struct ColoredMesh {
    pub mesh: CpuMesh,
    pub color: [f32; 4],
}

/// One mesh per primitive so a trunk and a canopy keep their own colors.
pub fn load_colored(path: &Path) -> Result<Vec<ColoredMesh>> {
    let (doc, buffers, _) = crate::assets::import_model(path)?;
    let mut parents = vec![None; doc.nodes().len()];
    let mut locals = Vec::with_capacity(doc.nodes().len());
    for node in doc.nodes() {
        for child in node.children() {
            parents[child.index()] = Some(node.index());
        }
    }
    for node in doc.nodes() {
        let (t, r, s) = node.transform().decomposed();
        locals.push(Mat4::from_scale_rotation_translation(
            Vec3::from(s),
            Quat::from_array(r),
            Vec3::from(t),
        ));
    }
    let mut parts = Vec::new();
    for node in doc.nodes() {
        let Some(mesh) = node.mesh() else {
            continue;
        };
        let world = world_matrix(node.index(), &locals, &parents);
        for primitive in mesh.primitives() {
            let Some((prim_vertices, prim_indices)) = read_primitive(&primitive, &buffers) else {
                continue;
            };
            let mut vertices = Vec::with_capacity(prim_vertices.len());
            for vertex in prim_vertices {
                let position = world.transform_point3(Vec3::from(vertex.position));
                let normal = world
                    .transform_vector3(Vec3::from(vertex.normal))
                    .normalize_or_zero();
                vertices.push(MeshVertex::new(
                    position,
                    normal,
                    Vec4::from(vertex.tangent),
                    Vec2::from(vertex.uv),
                ));
            }
            parts.push(ColoredMesh {
                mesh: CpuMesh {
                    vertices,
                    indices: prim_indices,
                    albedo_image: None,
                    texture_source_hint: None,
                },
                color: primitive
                    .material()
                    .pbr_metallic_roughness()
                    .base_color_factor(),
            });
        }
    }
    Ok(parts)
}

pub fn load_rigid(path: &Path) -> Result<TexturedMesh> {
    let (doc, buffers, images) = crate::assets::import_model(path)?;
    let mut parents = vec![None; doc.nodes().len()];
    let mut locals = Vec::with_capacity(doc.nodes().len());
    for node in doc.nodes() {
        for child in node.children() {
            parents[child.index()] = Some(node.index());
        }
    }
    for node in doc.nodes() {
        let (t, r, s) = node.transform().decomposed();
        locals.push(Mat4::from_scale_rotation_translation(
            Vec3::from(s),
            Quat::from_array(r),
            Vec3::from(t),
        ));
    }
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for node in doc.nodes() {
        let Some(mesh) = node.mesh() else {
            continue;
        };
        let world = world_matrix(node.index(), &locals, &parents);
        for primitive in mesh.primitives() {
            let Some((prim_vertices, prim_indices)) = read_primitive(&primitive, &buffers) else {
                continue;
            };
            let base = vertices.len() as u32;
            for vertex in prim_vertices {
                let position = world.transform_point3(Vec3::from(vertex.position));
                let normal = world
                    .transform_vector3(Vec3::from(vertex.normal))
                    .normalize_or_zero();
                vertices.push(MeshVertex::new(
                    position,
                    normal,
                    Vec4::from(vertex.tangent),
                    Vec2::from(vertex.uv),
                ));
            }
            indices.extend(prim_indices.into_iter().map(|index| index + base));
        }
    }
    Ok(TexturedMesh {
        mesh: CpuMesh {
            vertices,
            indices,
            albedo_image: None,
            texture_source_hint: None,
        },
        albedo: rgba_from_images(&images),
    })
}

pub fn sword_path() -> std::path::PathBuf {
    crate::assets::asset_dir().join("models/kaykit/gear/sword_1handed.gltf")
}

pub fn shield_path() -> std::path::PathBuf {
    crate::assets::asset_dir().join("models/kaykit/gear/shield_round.gltf")
}

/// KayKit one-handed sword is already Y-up. The grip sits near the origin.
pub fn sword_on_hand(hand: Mat4) -> Mat4 {
    hand * Mat4::from_translation(Vec3::new(0.0, 0.02, 0.06)) * Mat4::from_scale(Vec3::splat(0.55))
}

/// Round shield faces local Z. Turn it across the off hand.
pub fn shield_on_hand(hand: Mat4) -> Mat4 {
    hand * Mat4::from_rotation_y(std::f32::consts::FRAC_PI_2) * Mat4::from_scale(Vec3::splat(0.65))
}

/// Kenney figurine origin is the feet and the mesh is about 0.85 tall.
pub fn figurine_transform(position: Vec3, yaw: f32) -> Mat4 {
    let feet = position - Vec3::Y * 1.15;
    Mat4::from_translation(feet)
        * Mat4::from_rotation_y(std::f32::consts::FRAC_PI_2 - yaw)
        * Mat4::from_scale(Vec3::splat(2.05))
}

fn world_matrix(index: usize, locals: &[Mat4], parents: &[Option<usize>]) -> Mat4 {
    match parents[index] {
        Some(parent) => world_matrix(parent, locals, parents) * locals[index],
        None => locals[index],
    }
}

fn read_primitive(
    primitive: &gltf::Primitive,
    buffers: &[gltf::buffer::Data],
) -> Option<(Vec<MeshVertex>, Vec<u32>)> {
    let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
    let positions: Vec<[f32; 3]> = reader.read_positions()?.collect();
    let normals: Vec<[f32; 3]> = reader
        .read_normals()
        .map(|iter| iter.collect())
        .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; positions.len()]);
    let uvs: Vec<[f32; 2]> = reader
        .read_tex_coords(0)
        .map(|coords| coords.into_f32().collect())
        .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()]);
    let indices = reader
        .read_indices()
        .map(|iter| iter.into_u32().collect())
        .unwrap_or_else(|| (0..positions.len() as u32).collect());
    let vertices = positions
        .into_iter()
        .enumerate()
        .map(|(index, position)| {
            let normal = Vec3::from(normals[index]).normalize_or_zero();
            let tangent = normal.cross(Vec3::Y).normalize_or_zero();
            MeshVertex::new(
                Vec3::from(position),
                normal,
                Vec4::new(tangent.x, tangent.y, tangent.z, 1.0),
                Vec2::from(uvs[index]),
            )
        })
        .collect();
    Some((vertices, indices))
}

fn read_clip(animation: &gltf::Animation, buffers: &[gltf::buffer::Data]) -> Option<Clip> {
    let mut channels = Vec::new();
    let mut duration = 0.0_f32;
    for channel in animation.channels() {
        let reader = channel.reader(|buffer| Some(&buffers[buffer.index()]));
        let times: Vec<f32> = reader
            .read_inputs()
            .map(|iter| iter.collect())
            .unwrap_or_default();
        if times.is_empty() {
            continue;
        }
        duration = duration.max(*times.last().unwrap_or(&0.0));
        let values = match reader.read_outputs()? {
            ReadOutputs::Translations(values) => {
                ChannelValues::Translation(values.map(Vec3::from).collect())
            }
            ReadOutputs::Rotations(values) => {
                ChannelValues::Rotation(values.into_f32().map(Quat::from_array).collect())
            }
            ReadOutputs::Scales(_) | ReadOutputs::MorphTargetWeights(_) => continue,
        };
        channels.push(Channel {
            node: channel.target().node().index(),
            times,
            values,
        });
    }
    if channels.is_empty() {
        return None;
    }
    Some(Clip { duration, channels })
}

fn sample_vec3(times: &[f32], values: &[Vec3], time: f32) -> Vec3 {
    let (i, j, u) = segment(times, time);
    values[i].lerp(values[j], u)
}

fn sample_quat(times: &[f32], values: &[Quat], time: f32) -> Quat {
    let (i, j, u) = segment(times, time);
    values[i].slerp(values[j], u)
}

fn segment(times: &[f32], time: f32) -> (usize, usize, f32) {
    if times.len() < 2 || time <= times[0] {
        return (0, 0, 0.0);
    }
    let last = times.len() - 1;
    if time >= times[last] {
        return (last, last, 0.0);
    }
    let j = times.partition_point(|t| *t <= time).min(last);
    let i = j - 1;
    let span = (times[j] - times[i]).max(1e-5);
    (i, j, ((time - times[i]) / span).clamp(0.0, 1.0))
}

pub fn albedo_tile(path: &Path) -> Result<Rgba> {
    let mesh = load_rigid(path)?;
    let atlas = mesh.albedo.context("model has no albedo")?;
    if mesh.mesh.vertices.is_empty() {
        return Ok(atlas);
    }
    let mut min_u = f32::MAX;
    let mut min_v = f32::MAX;
    let mut max_u = f32::MIN;
    let mut max_v = f32::MIN;
    for vertex in &mesh.mesh.vertices {
        min_u = min_u.min(vertex.uv[0]);
        min_v = min_v.min(vertex.uv[1]);
        max_u = max_u.max(vertex.uv[0]);
        max_v = max_v.max(vertex.uv[1]);
    }
    let width = atlas.width.max(1) as f32;
    let height = atlas.height.max(1) as f32;
    let x0 = (min_u.clamp(0.0, 1.0) * (width - 1.0)).floor() as u32;
    let x1 = (max_u.clamp(0.0, 1.0) * (width - 1.0))
        .ceil()
        .max(x0 as f32 + 1.0) as u32;
    let x1 = x1.min(atlas.width).max(x0 + 1);
    // glTF v=0 is the bottom of the image.
    let y0 = ((1.0 - max_v.clamp(0.0, 1.0)) * (height - 1.0)).floor() as u32;
    let y1 = ((1.0 - min_v.clamp(0.0, 1.0)) * (height - 1.0))
        .ceil()
        .max(y0 as f32 + 1.0) as u32;
    let y1 = y1.min(atlas.height).max(y0 + 1);
    let tile_w = x1 - x0;
    let tile_h = y1 - y0;
    let mut pixels = Vec::with_capacity((tile_w * tile_h * 4) as usize);
    for y in y0..y1 {
        let row = y as usize * atlas.width as usize * 4;
        let start = row + x0 as usize * 4;
        let end = row + x1 as usize * 4;
        pixels.extend_from_slice(&atlas.pixels[start..end]);
    }
    Ok(Rgba {
        width: tile_w,
        height: tile_h,
        pixels,
    })
}

pub fn rgba_from_images(images: &[gltf::image::Data]) -> Option<Rgba> {
    let image = images.first()?;
    let pixels = match image.format {
        gltf::image::Format::R8G8B8A8 => image.pixels.clone(),
        gltf::image::Format::R8G8B8 => image
            .pixels
            .chunks_exact(3)
            .flat_map(|rgb| [rgb[0], rgb[1], rgb[2], 255])
            .collect(),
        _ => return None,
    };
    Some(Rgba {
        width: image.width,
        height: image.height,
        pixels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kenney_weapons_and_figurine_load() {
        let sword = load_rigid(&sword_path()).unwrap();
        let shield = load_rigid(&shield_path()).unwrap();
        assert!(sword.mesh.vertices.len() > 40);
        assert!(shield.mesh.vertices.len() > 40);
        assert!(sword.albedo.is_some());
        assert!(shield.albedo.is_some());
        let foe = NodeActor::load(&NodeActor::figurine_path()).unwrap();
        for name in ["idle", "walk", "sprint", "die", "attack-melee-right"] {
            assert!(foe.has_clip(name), "missing {name}");
        }
        let idle = foe.pose("idle", 0.0).unwrap();
        let walk = foe.pose("walk", 0.25).unwrap();
        assert!(idle.vertices.len() > 100);
        let idle_y = idle.vertices[20].position[1];
        let walk_y = walk.vertices[20].position[1];
        assert!((idle_y - walk_y).abs() > 1e-4, "{idle_y} vs {walk_y}");
        let height = idle
            .vertices
            .iter()
            .map(|v| v.position[1])
            .fold(0.0_f32, f32::max);
        assert!(height > 0.6 && height < 1.2, "figurine height {height}");
    }

    #[test]
    fn sword_sits_on_the_knight_hand() {
        let figure = crate::figure::Figure::load_knight().unwrap();
        let (_, hand) = figure.hand_frames("Idle_A", 0.0).unwrap();
        let sword = load_rigid(&sword_path()).unwrap();
        assert!(sword.albedo.is_some());
        let ys: Vec<f32> = sword.mesh.vertices.iter().map(|v| v.position[1]).collect();
        let xs: Vec<f32> = sword.mesh.vertices.iter().map(|v| v.position[0]).collect();
        let height = ys.iter().copied().fold(f32::MIN, f32::max)
            - ys.iter().copied().fold(f32::MAX, f32::min);
        let width = xs.iter().copied().fold(f32::MIN, f32::max)
            - xs.iter().copied().fold(f32::MAX, f32::min);
        assert!(height > width, "sword should be Y-up, {height} vs {width}");
        let grip = Vec3::ZERO;
        let grip_w = sword_on_hand(hand).transform_point3(grip);
        assert!(
            grip_w.distance(hand.w_axis.truncate()) < 0.25,
            "grip {grip_w:?} hand {:?}",
            hand.w_axis.truncate()
        );
    }

    #[test]
    fn oak_keeps_separate_leaf_and_bark_colors() {
        let path = crate::assets::asset_dir().join("models/nature/tree_oak.glb");
        let parts = load_colored(&path).unwrap();
        assert!(parts.len() >= 2);
        assert!(parts.iter().all(|part| part.mesh.vertices.len() > 10));
        let a = parts[0].color;
        let b = parts[1].color;
        let delta = (a[0] - b[0]).abs() + (a[1] - b[1]).abs() + (a[2] - b[2]).abs();
        assert!(delta > 0.2, "{a:?} vs {b:?}");
    }

    #[test]
    fn kaykit_forest_tree_keeps_its_atlas() {
        let path = crate::assets::asset_dir().join("models/kaykit_forest/Tree_1_A_Color1.gltf");
        let tree = load_rigid(&path).unwrap();
        assert!(tree.mesh.vertices.len() > 50);
        let albedo = tree.albedo.expect("forest atlas");
        assert!(albedo.width > 0 && albedo.height > 0);
        assert_eq!(
            albedo.pixels.len(),
            albedo.width as usize * albedo.height as usize * 4
        );
    }
}
