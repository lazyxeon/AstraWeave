//! KayKit Adventurers knight: one skinned mesh, clips retargeted by joint name.
//!
//! Sword and shield are the Adventurers one-handed sword and round shield.
//! This module reports the hand frames they attach to. Clip maps are
//! `BTreeMap`s, and clip files are merged in sorted path order.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use anyhow::{Context, Result};
use astraweave_render::animation::{
    compute_joint_matrices, skin_vertex_cpu, AnimationChannel, AnimationClip, ChannelData,
    Interpolation, Joint, Skeleton, Transform,
};
use astraweave_render::mesh::{CpuMesh, MeshVertex};
use glam::{Mat4, Quat, Vec2, Vec3, Vec4};
use gltf::animation::util::ReadOutputs;
use gltf::mesh::util::{ReadJoints, ReadWeights};

pub struct BindVertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
    pub joints: [u16; 4],
    pub weights: [f32; 4],
}

pub struct Primitive {
    pub vertices: Vec<BindVertex>,
    pub indices: Vec<u32>,
    pub color: [f32; 4],
}

pub struct Figure {
    pub skeleton: Skeleton,
    pub clips: BTreeMap<String, AnimationClip>,
    pub primitives: Vec<Primitive>,
    pub albedo: Option<crate::props::Rgba>,
    pub hand_l: usize,
    pub hand_r: usize,
}

impl Figure {
    pub fn load(path: &Path) -> Result<Self> {
        let (doc, buffers, images) = crate::assets::import_model(path)?;
        let skin = doc.skins().next().context("gltf has no skin")?;
        let joint_nodes: Vec<usize> = skin.joints().map(|node| node.index()).collect();
        let node_to_joint: HashMap<usize, usize> = joint_nodes
            .iter()
            .enumerate()
            .map(|(joint, node)| (*node, joint))
            .collect();
        let reader = skin.reader(|buffer| Some(&buffers[buffer.index()]));
        let inverse_binds: Vec<Mat4> = reader
            .read_inverse_bind_matrices()
            .context("missing inverse bind matrices")?
            .map(|matrix| Mat4::from_cols_array_2d(&matrix))
            .collect();

        let mut parent_of = HashMap::new();
        for node in doc.nodes() {
            for child in node.children() {
                parent_of.insert(child.index(), node.index());
            }
        }

        let mut joints = Vec::with_capacity(joint_nodes.len());
        let mut root_indices = Vec::new();
        for (joint_index, node_index) in joint_nodes.iter().enumerate() {
            let node = doc.nodes().nth(*node_index).context("joint node")?;
            let (translation, rotation, scale) = node.transform().decomposed();
            let parent_index = parent_of
                .get(node_index)
                .and_then(|parent| node_to_joint.get(parent))
                .copied();
            if parent_index.is_none() {
                root_indices.push(joint_index);
            }
            joints.push(Joint {
                name: node.name().unwrap_or("joint").to_string(),
                parent_index,
                inverse_bind_matrix: inverse_binds
                    .get(joint_index)
                    .copied()
                    .unwrap_or(Mat4::IDENTITY),
                local_transform: Transform {
                    translation: Vec3::from(translation),
                    rotation: Quat::from_array(rotation),
                    scale: Vec3::from(scale),
                },
            });
        }
        let skeleton = Skeleton {
            joints,
            root_indices,
        };

        let mut clips = BTreeMap::new();
        for animation in doc.animations() {
            let name = animation.name().unwrap_or("clip").to_string();
            if let Some(clip) = convert_clip(&animation, &buffers, &node_to_joint) {
                clips.insert(name, clip);
            }
        }

        let mut primitives = Vec::new();
        for mesh in doc.meshes() {
            for primitive in mesh.primitives() {
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
                let Some(positions) = reader.read_positions() else {
                    continue;
                };
                let positions: Vec<[f32; 3]> = positions.collect();
                let normals: Vec<[f32; 3]> = reader
                    .read_normals()
                    .map(|iter| iter.collect())
                    .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; positions.len()]);
                let uvs: Vec<[f32; 2]> = reader
                    .read_tex_coords(0)
                    .map(|coords| coords.into_f32().collect())
                    .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()]);
                let joints = match reader.read_joints(0) {
                    Some(ReadJoints::U8(iter)) => iter
                        .map(|j| [j[0] as u16, j[1] as u16, j[2] as u16, j[3] as u16])
                        .collect::<Vec<_>>(),
                    Some(ReadJoints::U16(iter)) => iter.collect::<Vec<_>>(),
                    None => continue,
                };
                let weights = match reader.read_weights(0) {
                    Some(ReadWeights::F32(iter)) => iter.collect::<Vec<_>>(),
                    Some(ReadWeights::U8(iter)) => iter
                        .map(|w| {
                            let s =
                                (w[0] as f32 + w[1] as f32 + w[2] as f32 + w[3] as f32).max(1.0);
                            [
                                w[0] as f32 / s,
                                w[1] as f32 / s,
                                w[2] as f32 / s,
                                w[3] as f32 / s,
                            ]
                        })
                        .collect(),
                    Some(ReadWeights::U16(iter)) => iter
                        .map(|w| {
                            let s =
                                (w[0] as f32 + w[1] as f32 + w[2] as f32 + w[3] as f32).max(1.0);
                            [
                                w[0] as f32 / s,
                                w[1] as f32 / s,
                                w[2] as f32 / s,
                                w[3] as f32 / s,
                            ]
                        })
                        .collect(),
                    None => continue,
                };
                let indices = reader
                    .read_indices()
                    .map(|iter| iter.into_u32().collect())
                    .unwrap_or_else(|| (0..positions.len() as u32).collect());
                let color = primitive
                    .material()
                    .pbr_metallic_roughness()
                    .base_color_factor();
                let vertices = positions
                    .into_iter()
                    .enumerate()
                    .map(|(index, position)| BindVertex {
                        position: Vec3::from(position),
                        normal: Vec3::from(normals[index]),
                        uv: Vec2::from(uvs[index]),
                        joints: joints[index],
                        weights: weights[index],
                    })
                    .collect();
                primitives.push(Primitive {
                    vertices,
                    indices,
                    color,
                });
            }
        }

        let hand_l = first_joint(&skeleton, &["handslot.l", "hand.l", "DEF-hand.L"]);
        let hand_r = first_joint(&skeleton, &["handslot.r", "hand.r", "DEF-hand.R"]);
        Ok(Self {
            skeleton,
            clips,
            primitives,
            albedo: crate::props::rgba_from_images(&images),
            hand_l,
            hand_r,
        })
    }

    /// Skin from `mesh`, clips from `clip_files` retargeted by joint name.
    /// Clip files are read in sorted path order. The first clip of a name wins.
    pub fn load_with_clips(mesh: &Path, clip_files: &[&Path]) -> Result<Self> {
        let mut figure = Self::load(mesh)?;
        let mut files: Vec<&Path> = clip_files.to_vec();
        files.sort();
        for path in files {
            figure.merge_clips(path)?;
        }
        Ok(figure)
    }

    pub fn knight_path() -> std::path::PathBuf {
        crate::assets::asset_dir().join("models/kaykit/knight/Knight.glb")
    }

    pub fn medium_clip_paths() -> Vec<std::path::PathBuf> {
        let root = crate::assets::asset_dir().join("models/kaykit/anims");
        let mut paths = vec![
            root.join("Rig_Medium_CombatMelee.glb"),
            root.join("Rig_Medium_General.glb"),
            root.join("Rig_Medium_MovementAdvanced.glb"),
            root.join("Rig_Medium_MovementBasic.glb"),
        ];
        paths.sort();
        paths
    }

    pub fn load_knight() -> Result<Self> {
        let paths = Self::medium_clip_paths();
        let refs: Vec<&Path> = paths.iter().map(Path::new).collect();
        Self::load_with_clips(&Self::knight_path(), &refs)
    }

    fn merge_clips(&mut self, path: &Path) -> Result<()> {
        let (doc, buffers, _) = crate::assets::import_model(path)?;
        let skin = doc.skins().next().context("clip file has no skin")?;
        let joint_nodes: Vec<usize> = skin.joints().map(|node| node.index()).collect();
        let mut node_to_joint = HashMap::new();
        let mut joint_names = Vec::with_capacity(joint_nodes.len());
        for (joint, node_index) in joint_nodes.iter().enumerate() {
            node_to_joint.insert(*node_index, joint);
            let node = doc.nodes().nth(*node_index).context("clip joint")?;
            joint_names.push(node.name().unwrap_or("").to_string());
        }
        for animation in doc.animations() {
            let Some(mut clip) = convert_clip(&animation, &buffers, &node_to_joint) else {
                continue;
            };
            let mut channels = Vec::new();
            for channel in clip.channels.drain(..) {
                let Some(name) = joint_names.get(channel.target_joint_index) else {
                    continue;
                };
                let Some(index) = joint_named(&self.skeleton, name) else {
                    continue;
                };
                channels.push(AnimationChannel {
                    target_joint_index: index,
                    ..channel
                });
            }
            if channels.is_empty() {
                continue;
            }
            channels.sort_by_key(|channel| channel.target_joint_index);
            clip.channels = channels;
            self.clips.entry(clip.name.clone()).or_insert(clip);
        }
        Ok(())
    }

    pub fn library_path() -> std::path::PathBuf {
        crate::assets::asset_dir().join("models/AnimationLibrary_Godot_Standard.gltf")
    }

    pub fn clip<'a>(&'a self, name: &str) -> Option<&'a AnimationClip> {
        self.clips.get(name)
    }

    pub fn skin(&self, clip_name: &str, time: f32) -> Result<Vec<PosedPrimitive>> {
        self.skin_at(clip_name, time, true)
    }

    pub fn skin_at(
        &self,
        clip_name: &str,
        time: f32,
        looping: bool,
    ) -> Result<Vec<PosedPrimitive>> {
        let locals = self.locals_at(clip_name, time, looping)?;
        let joints = compute_joint_matrices(&self.skeleton, &locals)?;
        let mut posed = Vec::new();
        for primitive in &self.primitives {
            let mut vertices = Vec::with_capacity(primitive.vertices.len());
            for vertex in &primitive.vertices {
                let (position, normal) = skin_vertex_cpu(
                    vertex.position,
                    vertex.normal,
                    vertex.joints,
                    vertex.weights,
                    &joints,
                );
                let tangent = normal.cross(Vec3::Y).normalize_or_zero();
                vertices.push(MeshVertex::new(
                    position,
                    normal,
                    Vec4::new(tangent.x, tangent.y, tangent.z, 1.0),
                    vertex.uv,
                ));
            }
            posed.push(PosedPrimitive {
                mesh: CpuMesh {
                    vertices,
                    indices: primitive.indices.clone(),
                    albedo_image: None,
                    texture_source_hint: None,
                },
                color: primitive.color,
            });
        }
        Ok(posed)
    }

    pub fn hand_positions(&self, clip_name: &str, time: f32) -> Result<(Vec3, Vec3)> {
        let (left, right) = self.hand_frames(clip_name, time)?;
        Ok((left.w_axis.truncate(), right.w_axis.truncate()))
    }

    /// World matrices of the two hands in model space. Columns are the joint axes.
    pub fn hand_frames(&self, clip_name: &str, time: f32) -> Result<(Mat4, Mat4)> {
        self.hand_frames_at(clip_name, time, true)
    }

    pub fn hand_frames_at(
        &self,
        clip_name: &str,
        time: f32,
        looping: bool,
    ) -> Result<(Mat4, Mat4)> {
        let locals = self.locals_at(clip_name, time, looping)?;
        let joints = compute_joint_matrices(&self.skeleton, &locals)?;
        Ok((
            joint_world(&self.skeleton, &joints, self.hand_l),
            joint_world(&self.skeleton, &joints, self.hand_r),
        ))
    }

    pub fn duration(&self, clip_name: &str) -> Option<f32> {
        self.clips.get(clip_name).map(|clip| clip.duration)
    }

    fn locals_at(
        &self,
        clip_name: &str,
        time: f32,
        looping: bool,
    ) -> Result<Vec<astraweave_render::animation::Transform>> {
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
        Ok(clip.sample(t, &self.skeleton))
    }
}

pub struct PosedPrimitive {
    pub mesh: CpuMesh,
    pub color: [f32; 4],
}

/// Capsule center to the ground. Cylinder half-height 0.8 plus radius 0.35.
pub const CAPSULE_FOOT_DROP: f32 = 1.15;

pub fn actor_transform(position: Vec3, yaw: f32) -> Mat4 {
    // KayKit feet sit on local Y=0. The knight is about 2.5 units tall.
    let feet = position - Vec3::Y * CAPSULE_FOOT_DROP;
    Mat4::from_translation(feet)
        * Mat4::from_rotation_y(std::f32::consts::FRAC_PI_2 - yaw)
        * Mat4::from_scale(Vec3::splat(0.72))
}

fn first_joint(skeleton: &Skeleton, names: &[&str]) -> usize {
    names
        .iter()
        .find_map(|name| joint_named(skeleton, name))
        .unwrap_or(0)
}

fn joint_named(skeleton: &Skeleton, name: &str) -> Option<usize> {
    skeleton.joints.iter().position(|joint| joint.name == name)
}

fn joint_world(skeleton: &Skeleton, skin: &[Mat4], index: usize) -> Mat4 {
    skin[index] * skeleton.joints[index].inverse_bind_matrix.inverse()
}

fn convert_clip(
    animation: &gltf::Animation,
    buffers: &[gltf::buffer::Data],
    node_to_joint: &HashMap<usize, usize>,
) -> Option<AnimationClip> {
    let mut channels = Vec::new();
    let mut duration = 0.0_f32;
    for channel in animation.channels() {
        let Some(&joint) = node_to_joint.get(&channel.target().node().index()) else {
            continue;
        };
        let reader = channel.reader(|buffer| Some(&buffers[buffer.index()]));
        let times: Vec<f32> = reader
            .read_inputs()
            .map(|iter| iter.collect())
            .unwrap_or_default();
        if times.is_empty() {
            continue;
        }
        duration = duration.max(*times.last().unwrap_or(&0.0));
        let interpolation = match channel.sampler().interpolation() {
            gltf::animation::Interpolation::Step => Interpolation::Step,
            _ => Interpolation::Linear,
        };
        let data = match reader.read_outputs()? {
            ReadOutputs::Translations(values) => {
                ChannelData::Translation(values.map(Vec3::from).collect())
            }
            ReadOutputs::Rotations(values) => {
                ChannelData::Rotation(values.into_f32().map(Quat::from_array).collect())
            }
            ReadOutputs::Scales(values) => ChannelData::Scale(values.map(Vec3::from).collect()),
            ReadOutputs::MorphTargetWeights(_) => continue,
        };
        channels.push(AnimationChannel {
            target_joint_index: joint,
            times,
            data,
            interpolation,
        });
    }
    if channels.is_empty() {
        return None;
    }
    Some(AnimationClip {
        name: animation.name().unwrap_or("clip").to_string(),
        duration,
        channels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_has_locomotion_and_combat_clips() {
        let figure = Figure::load_knight().unwrap();
        for name in [
            "Idle_A",
            "Walking_A",
            "Running_A",
            "Running_B",
            "Jump_Idle",
            "Crouching",
            "Dodge_Forward",
            "Melee_1H_Attack_Slice_Horizontal",
            "Melee_Blocking",
            "Hit_A",
            "Death_A",
        ] {
            assert!(figure.clip(name).is_some(), "missing {name}");
        }
        let idle = figure.skin("Idle_A", 0.0).unwrap();
        let walk = figure.skin("Walking_A", 0.3).unwrap();
        let verts: usize = idle.iter().map(|part| part.mesh.vertices.len()).sum();
        assert!(verts > 200, "knight {verts}");
        assert!(!walk.is_empty());
        assert!(figure.albedo.is_some());
        let (left, right) = figure.hand_positions("Idle_A", 0.0).unwrap();
        assert!(left.distance(right) > 0.05, "{left:?} {right:?}");
        let paths = Figure::medium_clip_paths();
        let mut sorted = paths.clone();
        sorted.sort();
        assert_eq!(paths, sorted);
        let golem = Figure::load_with_clips(
            &crate::assets::asset_dir().join("models/kaykit/skeletons/Skeleton_Golem.glb"),
            &paths.iter().map(Path::new).collect::<Vec<_>>(),
        )
        .unwrap();
        assert!(golem.clip("Idle_A").is_some());
        assert!(golem.clip("Death_A").is_some());
        assert!(golem.albedo.is_some());
    }
}
