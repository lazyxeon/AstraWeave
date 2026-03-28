//! Blender Python export script generation.
//!
//! This module generates Python scripts that Blender executes to export
//! .blend files to glTF/GLB format with all configured options.

use crate::options::{
    AnimationOptions, BoundingBoxMode, ConversionOptions, DecompositionGrouping,
    GltfExportOptions, LinkedLibraryOptions, MaterialOptions, MeshOptions,
    SceneDecompositionOptions, TextureOptions,
};
use std::path::Path;

/// Generates the complete Python export script.
pub fn generate_export_script(
    blend_path: &Path,
    output_path: &Path,
    options: &ConversionOptions,
    blend_hash: &str,
) -> String {
    let mut script = String::with_capacity(8192);

    // Script header
    script.push_str(&generate_header());

    // Import statements
    script.push_str(&generate_imports());

    // Configuration variables
    script.push_str(&generate_config(
        blend_path,
        output_path,
        options,
        blend_hash,
    ));

    // Texture unpacking function
    script.push_str(&generate_texture_unpacking(&options.textures, blend_hash));

    // Linked library handling
    if options.linked_libraries.process_recursively {
        script.push_str(&generate_linked_library_handler(&options.linked_libraries));
    }

    // Main export function
    script.push_str(&generate_main_export(
        &options.gltf,
        &options.mesh,
        &options.animation,
        &options.materials,
    ));

    // Script entry point
    script.push_str(&generate_entry_point());

    script
}

fn generate_header() -> String {
    r#"#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
AstraWeave Blender Export Script
Generated automatically - do not edit manually.

This script exports a .blend file to glTF/GLB format with
engine-optimized settings.
"""

"#
    .to_string()
}

fn generate_imports() -> String {
    r#"import bpy
import os
import sys
import json
import hashlib
import traceback
import time
from pathlib import Path

"#
    .to_string()
}

fn generate_config(
    blend_path: &Path,
    output_path: &Path,
    options: &ConversionOptions,
    blend_hash: &str,
) -> String {
    let blend_path_escaped = blend_path.display().to_string().replace('\\', "\\\\");
    let output_path_escaped = output_path.display().to_string().replace('\\', "\\\\");
    let output_format = options.format.blender_format();

    format!(
        r#"# Configuration
BLEND_FILE = r"{blend_path_escaped}"
OUTPUT_FILE = r"{output_path_escaped}"
OUTPUT_FORMAT = "{output_format}"
BLEND_HASH = "{blend_hash}"
DRACO_COMPRESSION = {draco}
DRACO_LEVEL = {draco_level}
UNPACK_TEXTURES = {unpack}
MAX_TEXTURE_RESOLUTION = {max_res}

"#,
        draco = python_bool(options.gltf.draco_compression),
        draco_level = options.gltf.draco_compression_level,
        unpack = python_bool(options.textures.unpack_embedded),
        max_res = options
            .textures
            .max_resolution
            .map_or("None".to_string(), |r| r.to_string()),
    )
}

fn generate_texture_unpacking(options: &TextureOptions, blend_hash: &str) -> String {
    let texture_format = match options.format {
        crate::options::TextureFormat::Png => "PNG",
        crate::options::TextureFormat::Jpeg => "JPEG",
        crate::options::TextureFormat::WebP => "WEBP",
        crate::options::TextureFormat::Original => "NONE",
    };

    format!(
        r#"
def unpack_and_process_textures(output_dir):
    """Unpack embedded textures with deterministic naming."""
    texture_map = {{}}
    
    for image in bpy.data.images:
        if image.packed_file is None:
            continue
            
        # Generate deterministic filename: {{blend_hash}}_{{texture_name}}.ext
        original_name = Path(image.name).stem
        safe_name = "".join(c if c.isalnum() or c in "._-" else "_" for c in original_name)
        texture_filename = f"{blend_hash}_{{safe_name}}.{texture_format_lower}"
        texture_path = output_dir / texture_filename
        
        try:
            # Unpack to file
            image.unpack(method='WRITE_ORIGINAL')
            
            # Process resolution limit if needed
            if MAX_TEXTURE_RESOLUTION and (image.size[0] > MAX_TEXTURE_RESOLUTION or image.size[1] > MAX_TEXTURE_RESOLUTION):
                scale = MAX_TEXTURE_RESOLUTION / max(image.size[0], image.size[1])
                new_width = int(image.size[0] * scale)
                new_height = int(image.size[1] * scale)
                image.scale(new_width, new_height)
            
            # Save with desired format
            if "{texture_format}" != "NONE":
                scene = bpy.context.scene
                scene.render.image_settings.file_format = "{texture_format}"
                if "{texture_format}" == "JPEG":
                    scene.render.image_settings.quality = {jpeg_quality}
                image.save_render(str(texture_path))
                
            texture_map[image.name] = str(texture_path)
            print(f"Unpacked texture: {{image.name}} -> {{texture_path}}")
            
        except Exception as e:
            print(f"Warning: Failed to unpack texture {{image.name}}: {{e}}")
            
    return texture_map

"#,
        blend_hash = blend_hash,
        texture_format = texture_format,
        texture_format_lower = texture_format.to_lowercase(),
        jpeg_quality = options.jpeg_quality,
    )
}

fn generate_linked_library_handler(options: &LinkedLibraryOptions) -> String {
    let search_paths: Vec<String> = options
        .search_paths
        .iter()
        .map(|p| format!("r\"{}\"", p.display().to_string().replace('\\', "\\\\")))
        .collect();
    let search_paths_str = if search_paths.is_empty() {
        "[]".to_string()
    } else {
        format!("[{}]", search_paths.join(", "))
    };

    format!(
        r#"
# Linked library tracking for circular reference detection
_processed_libraries = set()

def process_linked_libraries(max_depth={max_depth}):
    """Recursively process linked .blend libraries."""
    search_paths = {search_paths}
    
    def process_library(lib_path, depth=0):
        if depth > max_depth:
            print(f"Warning: Max library depth exceeded for {{lib_path}}")
            return False
            
        # Normalize path for comparison
        normalized = os.path.normpath(os.path.abspath(lib_path))
        
        # Circular reference detection
        if normalized in _processed_libraries:
            {circular_handling}
            return True
            
        _processed_libraries.add(normalized)
        
        if not os.path.exists(lib_path):
            # Try search paths
            found = False
            for search_dir in search_paths:
                candidate = os.path.join(search_dir, os.path.basename(lib_path))
                if os.path.exists(candidate):
                    lib_path = candidate
                    found = True
                    break
            
            if not found:
                {missing_handling}
                return False
        
        print(f"Processing linked library (depth {{depth}}): {{lib_path}}")
        
        # Link the library data
        try:
            with bpy.data.libraries.load(lib_path, link=False) as (data_from, data_to):
                # Import all objects and meshes
                data_to.objects = data_from.objects
                data_to.meshes = data_from.meshes
                data_to.materials = data_from.materials
                data_to.textures = data_from.textures
                data_to.images = data_from.images
                
        except Exception as e:
            print(f"Error loading library {{lib_path}}: {{e}}")
            return False
            
        return True
    
    # Process all libraries in the current file
    for lib in bpy.data.libraries:
        if lib.filepath:
            abs_path = bpy.path.abspath(lib.filepath)
            process_library(abs_path, depth=1)
    
    print(f"Processed {{len(_processed_libraries)}} linked libraries")

"#,
        max_depth = options.max_recursion_depth,
        search_paths = search_paths_str,
        circular_handling = if options.detect_circular_references {
            r#"print(f"Skipping circular reference: {normalized}")
            return True"#
        } else {
            "pass"
        },
        missing_handling = match options.missing_library_action {
            crate::options::MissingLibraryAction::Skip => {
                r#"print(f"Skipping missing library: {lib_path}")
                return True"#
            }
            crate::options::MissingLibraryAction::Warn => {
                r#"print(f"Warning: Missing linked library: {lib_path}")
                return True"#
            }
            crate::options::MissingLibraryAction::Fail => {
                r#"raise FileNotFoundError(f"Missing linked library: {lib_path}")"#
            }
        },
    )
}

fn generate_main_export(
    gltf: &GltfExportOptions,
    mesh: &MeshOptions,
    animation: &AnimationOptions,
    materials: &MaterialOptions,
) -> String {
    format!(
        r#"
def export_gltf():
    """Perform the glTF export with configured options."""
    
    # Prepare output directory
    output_path = Path(OUTPUT_FILE)
    output_dir = output_path.parent
    output_dir.mkdir(parents=True, exist_ok=True)
    
    # Unpack textures if enabled
    if UNPACK_TEXTURES:
        unpack_and_process_textures(output_dir)
    
    # Export parameters
    export_params = {{
        'filepath': OUTPUT_FILE,
        'export_format': OUTPUT_FORMAT,
        
        # Mesh options
        'export_apply': {apply_modifiers},
        'export_tangents': {export_tangents},
        'export_normals': {export_normals},
        'export_colors': {export_vertex_colors},
        'use_mesh_edges': {export_loose_edges},
        'use_mesh_vertices': {export_loose_points},
        
        # Material options
        'export_materials': '{export_materials}',
        
        # Animation options
        'export_animations': {export_animations},
        'export_morph': {export_shape_keys},
        'export_morph_normal': {export_shape_keys},
        'export_morph_tangent': False,
        'export_nla_strips': {export_nla_strips},
        'optimize_animation_size': {optimize_animation},
        
        # Armature options  
        'export_skins': {export_skins},
        
        # Other options
        'export_cameras': {export_cameras},
        'export_lights': {export_lights},
        'export_extras': {export_extras},
        'export_yup': {y_up},
        
        # Selection
        'use_selection': {selected_only},
        'use_visible': {visible_only},
        'use_active_collection': {active_collection_only},
    }}
    
    # Add Draco compression if enabled
    if DRACO_COMPRESSION:
        export_params['export_draco_mesh_compression_enable'] = True
        export_params['export_draco_mesh_compression_level'] = DRACO_LEVEL
        export_params['export_draco_position_quantization'] = 14
        export_params['export_draco_normal_quantization'] = 10
        export_params['export_draco_texcoord_quantization'] = 12
        export_params['export_draco_color_quantization'] = 10
        export_params['export_draco_generic_quantization'] = 12
    
    # Add copyright if specified
    copyright_text = {copyright}
    if copyright_text:
        export_params['export_copyright'] = copyright_text
    
    print(f"Exporting to: {{OUTPUT_FILE}}")
    print(f"Format: {{OUTPUT_FORMAT}}")
    print(f"Draco: {{DRACO_COMPRESSION}}")
    
    # Perform export
    bpy.ops.export_scene.gltf(**export_params)
    
    # Verify output
    if not output_path.exists():
        raise RuntimeError(f"Export failed: output file not created at {{OUTPUT_FILE}}")
    
    file_size = output_path.stat().st_size
    print(f"Export complete: {{file_size}} bytes")
    
    return {{
        'output_file': OUTPUT_FILE,
        'file_size': file_size,
        'format': OUTPUT_FORMAT,
    }}

"#,
        apply_modifiers = python_bool(mesh.apply_modifiers),
        export_tangents = python_bool(mesh.export_tangents),
        export_normals = python_bool(mesh.export_normals),
        export_vertex_colors = python_bool(mesh.export_vertex_colors),
        export_loose_edges = python_bool(mesh.export_loose_edges),
        export_loose_points = python_bool(mesh.export_loose_points),
        export_materials = if materials.export_materials {
            "EXPORT"
        } else {
            "NONE"
        },
        export_animations = python_bool(animation.export_animations),
        export_shape_keys = python_bool(animation.export_shape_keys),
        export_nla_strips = python_bool(animation.export_nla_strips),
        optimize_animation = python_bool(animation.optimize_animation_size),
        export_skins = python_bool(gltf.export_skins),
        export_cameras = python_bool(gltf.export_cameras),
        export_lights = python_bool(gltf.export_lights),
        export_extras = python_bool(gltf.export_extras),
        y_up = python_bool(gltf.y_up),
        selected_only = python_bool(gltf.selected_only),
        visible_only = python_bool(gltf.visible_only),
        active_collection_only = python_bool(gltf.active_collection_only),
        copyright = gltf
            .copyright
            .as_ref()
            .map_or("None".to_string(), |c| format!(
                "\"{}\"",
                c.replace('"', "\\\"")
            )),
    )
}

fn generate_entry_point() -> String {
    r#"
def main():
    """Main entry point."""
    result = {'success': False, 'error': None, 'output': None}
    
    try:
        print(f"Loading: {BLEND_FILE}")
        
        # Open the blend file
        bpy.ops.wm.open_mainfile(filepath=BLEND_FILE)
        
        # Process linked libraries if function exists
        if 'process_linked_libraries' in dir():
            process_linked_libraries()
        
        # Perform export
        export_result = export_gltf()
        
        result['success'] = True
        result['output'] = export_result
        
    except Exception as e:
        result['error'] = str(e)
        result['traceback'] = traceback.format_exc()
        print(f"Export failed: {e}", file=sys.stderr)
        traceback.print_exc()
        sys.exit(1)
    
    # Write result JSON for parsing by Rust
    result_file = OUTPUT_FILE + '.result.json'
    with open(result_file, 'w') as f:
        json.dump(result, f, indent=2)
    
    print(f"Result written to: {result_file}")
    print("Export completed successfully!")
    
if __name__ == '__main__':
    main()
"#
    .to_string()
}

/// Converts a bool to Python boolean string.
fn python_bool(b: bool) -> &'static str {
    if b {
        "True"
    } else {
        "False"
    }
}

/// Generates a Python script that decomposes a .blend scene into individual
/// per-object GLB files plus a manifest.json.
///
/// The script:
/// 1. Opens the .blend file
/// 2. Iterates over all mesh objects (optionally grouped by collection/material)
/// 3. Selects each object individually and exports via `use_selection=True`
/// 4. Extracts textures referenced by each object's materials
/// 5. Discovers HDRI images from World shader nodes
/// 6. Writes a manifest.json describing every extracted asset
pub fn generate_decomposition_script(
    blend_path: &Path,
    output_dir: &Path,
    options: &ConversionOptions,
    blend_hash: &str,
) -> String {
    let decomp = &options.decomposition;
    let mut script = String::with_capacity(16384);

    script.push_str(&generate_header());
    script.push_str(&generate_imports());
    script.push_str(&generate_decomp_config(
        blend_path, output_dir, options, blend_hash,
    ));
    script.push_str(&generate_category_classifier(decomp));
    script.push_str(&generate_bounding_box_fn(decomp));
    script.push_str(&generate_texture_collector());
    script.push_str(&generate_hdri_extractor(decomp));
    script.push_str(&generate_per_object_exporter(options));
    script.push_str(&generate_decomp_entry_point(decomp));

    script
}

fn generate_decomp_config(
    blend_path: &Path,
    output_dir: &Path,
    options: &ConversionOptions,
    blend_hash: &str,
) -> String {
    let blend_escaped = blend_path.display().to_string().replace('\\', "\\\\");
    let output_escaped = output_dir.display().to_string().replace('\\', "\\\\");
    let output_format = options.format.blender_format();

    let exclude_list: Vec<String> = options
        .decomposition
        .exclude_patterns
        .iter()
        .map(|p| format!("\"{}\"", p.replace('"', "\\\"")))
        .collect();

    format!(
        r#"# Decomposition Configuration
BLEND_FILE = r"{blend_escaped}"
OUTPUT_DIR = r"{output_escaped}"
OUTPUT_FORMAT = "{output_format}"
BLEND_HASH = "{blend_hash}"
DRACO_COMPRESSION = {draco}
DRACO_LEVEL = {draco_level}
MIN_VERTEX_COUNT = {min_verts}
EXCLUDE_PATTERNS = [{excludes}]
GROUP_BY = "{group_by}"
EXTRACT_TEXTURES = {extract_tex}
EXTRACT_HDRIS = {extract_hdri}
INCLUDE_EMPTIES = {include_empties}
GENERATE_MANIFEST = {gen_manifest}

"#,
        draco = python_bool(options.gltf.draco_compression),
        draco_level = options.gltf.draco_compression_level,
        min_verts = options.decomposition.min_vertex_count,
        excludes = exclude_list.join(", "),
        group_by = match options.decomposition.group_by {
            DecompositionGrouping::ByObject => "OBJECT",
            DecompositionGrouping::ByCollection => "COLLECTION",
            DecompositionGrouping::ByMaterial => "MATERIAL",
        },
        extract_tex = python_bool(options.decomposition.extract_textures),
        extract_hdri = python_bool(options.decomposition.extract_hdris),
        include_empties = python_bool(options.decomposition.include_empties),
        gen_manifest = python_bool(options.decomposition.generate_manifest),
    )
}

fn generate_category_classifier(_decomp: &SceneDecompositionOptions) -> String {
    r#"
def classify_asset(obj):
    """Classify an object's asset category based on name heuristics."""
    name_lower = obj.name.lower()

    rock_keywords = ['boulder', 'cliff', 'stone', 'rock', 'pebble', 'mineral', 'ore', 'crystal']
    veg_keywords = ['tree', 'bush', 'flower', 'plant', 'grass', 'leaf',
                    'fern', 'succulent', 'shrub', 'branch', 'dead_branch',
                    'vine', 'moss', 'cactus', 'palm', 'reed', 'weed',
                    'mushroom', 'fungus', 'log', 'stump', 'bark', 'hedge']
    terrain_keywords = ['terrain', 'ground', 'landscape', 'plane']
    billboard_keywords = ['billboard', 'card', 'sprite', 'imposter']
    structure_keywords = ['house', 'building', 'wall', 'fence', 'bridge',
                          'tower', 'gate', 'door', 'window', 'roof',
                          'pillar', 'column', 'arch', 'ruin', 'cabin',
                          'shed', 'hut', 'barn', 'fort']
    furniture_keywords = ['chair', 'table', 'bench', 'bed', 'shelf',
                          'desk', 'stool', 'crate', 'barrel', 'chest',
                          'pot', 'vase', 'rug', 'carpet', 'curtain',
                          'lantern', 'lamp', 'candle']
    light_keywords = ['light', 'torch', 'fire', 'campfire', 'bonfire',
                      'glow', 'ember', 'flame']

    for kw in rock_keywords:
        if kw in name_lower:
            return 'rock'
    for kw in veg_keywords:
        if kw in name_lower:
            return 'vegetation'
    for kw in terrain_keywords:
        if kw in name_lower:
            return 'terrain'
    for kw in billboard_keywords:
        if kw in name_lower:
            return 'billboard'
    for kw in structure_keywords:
        if kw in name_lower:
            return 'structure'
    for kw in furniture_keywords:
        if kw in name_lower:
            return 'furniture'
    for kw in light_keywords:
        if kw in name_lower:
            return 'light'

    # Collection-name fallback: check if any parent collection hints at category
    for col in obj.users_collection:
        col_lower = col.name.lower()
        if any(kw in col_lower for kw in veg_keywords):
            return 'vegetation'
        if any(kw in col_lower for kw in rock_keywords):
            return 'rock'
        if any(kw in col_lower for kw in structure_keywords):
            return 'structure'

    return 'prop'

"#
    .to_string()
}

fn generate_bounding_box_fn(decomp: &SceneDecompositionOptions) -> String {
    match decomp.bounding_box_mode {
        BoundingBoxMode::Aabb => r#"
def compute_bounds(obj):
    """Compute AABB from mesh vertices in world space."""
    if obj.type != 'MESH' or obj.data is None:
        return None
    depsgraph = bpy.context.evaluated_depsgraph_get()
    eval_obj = obj.evaluated_get(depsgraph)
    mesh = eval_obj.to_mesh()
    if mesh is None or len(mesh.vertices) == 0:
        return None
    world_matrix = obj.matrix_world
    verts = [world_matrix @ v.co for v in mesh.vertices]
    xs = [v.x for v in verts]
    ys = [v.y for v in verts]
    zs = [v.z for v in verts]
    eval_obj.to_mesh_clear()
    return {
        'min': [min(xs), min(ys), min(zs)],
        'max': [max(xs), max(ys), max(zs)],
    }

"#
        .to_string(),
        BoundingBoxMode::BlenderBounds => r#"
def compute_bounds(obj):
    """Use Blender's bounding_box property."""
    if obj.type != 'MESH' or obj.data is None:
        return None
    bb = [obj.matrix_world @ mathutils.Vector(corner) for corner in obj.bound_box]
    xs = [v.x for v in bb]
    ys = [v.y for v in bb]
    zs = [v.z for v in bb]
    return {
        'min': [min(xs), min(ys), min(zs)],
        'max': [max(xs), max(ys), max(zs)],
    }

"#
        .to_string(),
        BoundingBoxMode::None => r#"
def compute_bounds(obj):
    """Bounding box computation disabled."""
    return None

"#
        .to_string(),
    }
}

fn generate_texture_collector() -> String {
    r#"
def collect_textures_for_object(obj, output_dir):
    """Collect and save textures referenced by an object's materials."""
    textures = []
    if obj.type != 'MESH' or obj.data is None:
        return textures

    for mat_slot in obj.material_slots:
        mat = mat_slot.material
        if mat is None or not mat.use_nodes:
            continue

        for node in mat.node_tree.nodes:
            if node.type != 'TEX_IMAGE' or node.image is None:
                continue

            image = node.image
            safe_name = "".join(c if c.isalnum() or c in "._-" else "_" for c in image.name)
            ext = Path(image.filepath_raw).suffix if image.filepath_raw else '.png'
            if not ext:
                ext = '.png'

            tex_filename = f"{safe_name}{ext}"
            tex_path = output_dir / "textures" / tex_filename

            # Determine which PBR channel this texture maps to
            channel = 'unknown'
            for link in node.outputs[0].links:
                to_node = link.to_node
                to_socket = link.to_socket.name.lower()
                if 'color' in to_socket or 'base' in to_socket:
                    channel = 'diffuse'
                elif 'normal' in to_socket:
                    channel = 'normal'
                elif 'rough' in to_socket:
                    channel = 'roughness'
                elif 'metal' in to_socket:
                    channel = 'metallic'
                elif 'alpha' in to_socket:
                    channel = 'alpha'
                elif 'displace' in to_socket or 'height' in to_socket:
                    channel = 'displacement'

            if not tex_path.exists():
                try:
                    tex_path.parent.mkdir(parents=True, exist_ok=True)
                    image.save_render(str(tex_path))
                except Exception as e:
                    print(f"Warning: Failed to save texture {image.name}: {e}")
                    continue

            textures.append({
                'filename': tex_filename,
                'channel': channel,
                'original_name': image.name,
                'width': image.size[0],
                'height': image.size[1],
            })

    return textures

"#
    .to_string()
}

fn generate_hdri_extractor(decomp: &SceneDecompositionOptions) -> String {
    if !decomp.extract_hdris {
        return r#"
def extract_hdris(output_dir):
    """HDRI extraction disabled."""
    return []

"#
        .to_string();
    }

    r#"
def extract_hdris(output_dir):
    """Extract HDRI/environment maps from World shader nodes."""
    hdris = []
    world = bpy.context.scene.world
    if world is None or not world.use_nodes:
        return hdris

    for node in world.node_tree.nodes:
        if node.type != 'TEX_ENVIRONMENT' or node.image is None:
            continue

        image = node.image
        safe_name = "".join(c if c.isalnum() or c in "._-" else "_" for c in image.name)
        ext = Path(image.filepath_raw).suffix if image.filepath_raw else '.hdr'
        if not ext:
            ext = '.hdr'

        hdri_filename = f"{safe_name}{ext}"
        hdri_path = output_dir / "hdri" / hdri_filename

        try:
            hdri_path.parent.mkdir(parents=True, exist_ok=True)
            if image.packed_file:
                image.unpack(method='WRITE_ORIGINAL')
            if image.filepath_raw:
                import shutil
                abs_path = bpy.path.abspath(image.filepath_raw)
                if os.path.exists(abs_path):
                    shutil.copy2(abs_path, str(hdri_path))
                else:
                    image.save_render(str(hdri_path))
            else:
                image.save_render(str(hdri_path))
        except Exception as e:
            print(f"Warning: Failed to extract HDRI {image.name}: {e}")
            continue

        hdris.append({
            'filename': hdri_filename,
            'original_name': image.name,
            'width': image.size[0],
            'height': image.size[1],
        })

    return hdris

"#
    .to_string()
}

fn generate_per_object_exporter(options: &ConversionOptions) -> String {
    let gltf = &options.gltf;
    let mesh = &options.mesh;
    let materials = &options.materials;

    format!(
        r#"
def export_single_object(obj, output_path):
    """Export a single object as a GLB/glTF with use_selection=True."""

    # Deselect all, then select only this object
    bpy.ops.object.select_all(action='DESELECT')
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj

    export_params = {{
        'filepath': str(output_path),
        'export_format': OUTPUT_FORMAT,

        # Selection — this is the key setting for per-object decomposition
        'use_selection': True,
        'use_visible': True,
        'use_active_collection': False,

        # Mesh options
        'export_apply': {apply_modifiers},
        'export_tangents': {export_tangents},
        'export_normals': {export_normals},
        'export_all_vertex_colors': {export_vertex_colors},
        'use_mesh_edges': False,
        'use_mesh_vertices': False,

        # Material
        'export_materials': '{export_materials}',

        # No animations for static assets
        'export_animations': False,
        'export_morph': False,
        'export_skins': False,

        # Extras
        'export_extras': {export_extras},
        'export_yup': {y_up},
        'export_cameras': False,
        'export_lights': False,
    }}

    if DRACO_COMPRESSION:
        export_params['export_draco_mesh_compression_enable'] = True
        export_params['export_draco_mesh_compression_level'] = DRACO_LEVEL
        export_params['export_draco_position_quantization'] = 14
        export_params['export_draco_normal_quantization'] = 10
        export_params['export_draco_texcoord_quantization'] = 12
        export_params['export_draco_color_quantization'] = 10
        export_params['export_draco_generic_quantization'] = 12

    bpy.ops.export_scene.gltf(**export_params)

    # Deselect
    obj.select_set(False)

"#,
        apply_modifiers = python_bool(mesh.apply_modifiers),
        export_tangents = python_bool(mesh.export_tangents),
        export_normals = python_bool(mesh.export_normals),
        export_vertex_colors = python_bool(mesh.export_vertex_colors),
        export_materials = if materials.export_materials {{
            "EXPORT"
        }} else {{
            "NONE"
        }},
        export_extras = python_bool(gltf.export_extras),
        y_up = python_bool(gltf.y_up),
    )
}

fn generate_decomp_entry_point(_decomp: &SceneDecompositionOptions) -> String {
    r#"
def should_exclude(name):
    """Check if an object name matches any exclude pattern."""
    name_lower = name.lower()
    for pattern in EXCLUDE_PATTERNS:
        if pattern.lower() in name_lower:
            return True
    return False


def get_vertex_count(obj):
    """Get the vertex count of a mesh object."""
    if obj.type != 'MESH' or obj.data is None:
        return 0
    depsgraph = bpy.context.evaluated_depsgraph_get()
    eval_obj = obj.evaluated_get(depsgraph)
    mesh = eval_obj.to_mesh()
    count = len(mesh.vertices) if mesh else 0
    eval_obj.to_mesh_clear()
    return count


def main():
    """Decompose .blend scene into individual asset files."""
    result = {'success': False, 'error': None, 'assets': [], 'hdris': [], 'textures_dir': None}

    try:
        print(f"Loading: {BLEND_FILE}")
        bpy.ops.wm.open_mainfile(filepath=BLEND_FILE)

        output_dir = Path(OUTPUT_DIR)
        output_dir.mkdir(parents=True, exist_ok=True)
        meshes_dir = output_dir / "meshes"
        meshes_dir.mkdir(parents=True, exist_ok=True)

        # Ensure we're in object mode
        if bpy.context.object and bpy.context.object.mode != 'OBJECT':
            bpy.ops.object.mode_set(mode='OBJECT')

        # Realize collection instances — many scenes (e.g. Polyhaven) use Empty
        # objects that instance collections containing the actual meshes.  Without
        # this step those meshes are invisible to the per-object export loop.
        instance_empties = [o for o in bpy.data.objects if o.type == 'EMPTY' and o.instance_type == 'COLLECTION']
        if instance_empties:
            print(f"Realizing {len(instance_empties)} collection instances...")
            bpy.ops.object.select_all(action='DESELECT')
            for obj in instance_empties:
                obj.select_set(True)
            bpy.context.view_layer.objects.active = instance_empties[0]
            try:
                bpy.ops.object.duplicates_make_real()
            except Exception:
                try:
                    bpy.ops.object.make_instances_real()
                except Exception as e:
                    print(f"Warning: Could not realize instances: {e}")
            bpy.ops.object.select_all(action='DESELECT')
            print(f"After realization: {len(bpy.data.objects)} objects in scene")

        # Ensure all mesh objects are linked to the active view layer's scene
        # collection.  Polyhaven scenes use custom view layers and some objects
        # are not in the active layer, causing select_set() to fail during
        # per-object export.
        scene_col = bpy.context.scene.collection
        vl_objects = set(bpy.context.view_layer.objects)
        linked_count = 0
        for obj in list(bpy.data.objects):
            if obj not in vl_objects:
                try:
                    scene_col.objects.link(obj)
                    linked_count += 1
                except RuntimeError:
                    pass  # already linked at a higher level
        if linked_count > 0:
            # Refresh the dependency graph so the view layer picks them up
            bpy.context.view_layer.update()
            print(f"Linked {linked_count} objects to active view layer")

        # Collect exportable objects
        objects_to_export = []
        empties = []

        for obj in bpy.data.objects:
            if should_exclude(obj.name):
                print(f"Excluding: {obj.name}")
                continue

            if obj.type == 'EMPTY' and INCLUDE_EMPTIES:
                empties.append({
                    'name': obj.name,
                    'position': list(obj.location),
                    'rotation': list(obj.rotation_euler),
                    'scale': list(obj.scale),
                })
                continue

            if obj.type != 'MESH':
                continue

            vert_count = get_vertex_count(obj)
            if vert_count < MIN_VERTEX_COUNT:
                print(f"Skipping {obj.name}: only {vert_count} vertices")
                continue

            objects_to_export.append(obj)

        print(f"Found {len(objects_to_export)} mesh objects to export")

        # Deduplicate by mesh datablock — objects sharing the same mesh data
        # (e.g. boulder instances) are exported once; subsequent instances
        # reference the same file with their own transform.
        exported_meshdata = {}  # mesh_datablock_name -> (mesh_filename, file_size, vert_count)

        # Export each object
        assets = []
        export_errors = []
        skipped_instances = 0
        export_start = time.time()
        for idx, obj in enumerate(objects_to_export):
            safe_name = "".join(c if c.isalnum() or c in "._-" else "_" for c in obj.name)
            ext = "glb" if OUTPUT_FORMAT == "GLB" else "gltf"
            mesh_filename = f"{safe_name}.{ext}"
            mesh_path = meshes_dir / mesh_filename

            # Check if another object already exported this exact mesh datablock
            mesh_data_name = obj.data.name if obj.data else None
            if mesh_data_name and mesh_data_name in exported_meshdata:
                # Reuse the already-exported file — just record this instance
                ref_filename, ref_size, ref_verts = exported_meshdata[mesh_data_name]
                skipped_instances += 1
                elapsed = time.time() - export_start
                print(f"[{idx+1}/{len(objects_to_export)}] Instance: {obj.name} -> reuses {ref_filename} ({elapsed:.0f}s elapsed)")

                bounds = compute_bounds(obj)
                dimensions = None
                if bounds:
                    dimensions = [
                        bounds['max'][0] - bounds['min'][0],
                        bounds['max'][1] - bounds['min'][1],
                        bounds['max'][2] - bounds['min'][2],
                    ]

                asset_entry = {
                    'name': obj.name,
                    'filename': ref_filename,
                    'category': classify_asset(obj),
                    'vertex_count': ref_verts,
                    'file_size': ref_size,
                    'bounds': bounds,
                    'dimensions': dimensions,
                    'position': list(obj.location),
                    'rotation': list(obj.rotation_euler),
                    'scale': list(obj.scale),
                    'textures': [],
                    'materials': [ms.material.name for ms in obj.material_slots if ms.material],
                    'collections': [c.name for c in obj.users_collection],
                }
                assets.append(asset_entry)
                continue

            obj_start = time.time()
            elapsed = obj_start - export_start
            print(f"[{idx+1}/{len(objects_to_export)}] Exporting: {obj.name} -> {mesh_filename} ({elapsed:.0f}s elapsed)")

            try:
                export_single_object(obj, mesh_path)
            except Exception as e:
                msg = f"{type(e).__name__}: {e}"
                print(f"Warning: Failed to export {obj.name}: {msg}")
                export_errors.append({'name': obj.name, 'error': msg})
                continue

            if not mesh_path.exists():
                print(f"Warning: Export produced no file for {obj.name}")
                export_errors.append({'name': obj.name, 'error': 'No output file produced'})
                continue

            obj_elapsed = time.time() - obj_start
            file_size = mesh_path.stat().st_size
            print(f"  -> {file_size/1024/1024:.1f} MB in {obj_elapsed:.1f}s")

            # Register this mesh datablock as exported
            vert_count = get_vertex_count(obj)
            if mesh_data_name:
                exported_meshdata[mesh_data_name] = (f"meshes/{mesh_filename}", file_size, vert_count)

            # Collect textures for this object
            object_textures = []
            if EXTRACT_TEXTURES:
                object_textures = collect_textures_for_object(obj, output_dir)

            # Compute bounds
            bounds = compute_bounds(obj)

            # Compute dimensions from bounds
            dimensions = None
            if bounds:
                dimensions = [
                    bounds['max'][0] - bounds['min'][0],
                    bounds['max'][1] - bounds['min'][1],
                    bounds['max'][2] - bounds['min'][2],
                ]

            asset_entry = {
                'name': obj.name,
                'filename': f"meshes/{mesh_filename}",
                'category': classify_asset(obj),
                'vertex_count': vert_count,
                'file_size': file_size,
                'bounds': bounds,
                'dimensions': dimensions,
                'position': list(obj.location),
                'rotation': list(obj.rotation_euler),
                'scale': list(obj.scale),
                'textures': object_textures,
                'materials': [ms.material.name for ms in obj.material_slots if ms.material],
                'collections': [c.name for c in obj.users_collection],
            }
            assets.append(asset_entry)

        if skipped_instances > 0:
            print(f"Deduplicated {skipped_instances} mesh instances (shared datablocks)")

        # Extract HDRIs
        hdris = []
        if EXTRACT_HDRIS:
            hdris = extract_hdris(output_dir)

        result['success'] = True
        result['assets'] = assets
        result['empties'] = empties
        result['hdris'] = hdris
        result['total_objects'] = len(objects_to_export)
        result['export_errors'] = export_errors
        result['textures_dir'] = str(output_dir / "textures") if EXTRACT_TEXTURES else None

        # Write manifest
        if GENERATE_MANIFEST:
            manifest = {
                'blend_hash': BLEND_HASH,
                'source_file': BLEND_FILE,
                'assets': assets,
                'empties': empties,
                'hdris': hdris,
                'total_objects': len(objects_to_export),
            }
            manifest_path = output_dir / "manifest.json"
            with open(str(manifest_path), 'w') as f:
                json.dump(manifest, f, indent=2)
            print(f"Manifest written: {manifest_path}")

        print(f"Decomposition complete: {len(assets)} assets exported")

    except Exception as e:
        result['error'] = str(e)
        result['traceback'] = traceback.format_exc()
        print(f"Decomposition failed: {e}", file=sys.stderr)
        traceback.print_exc()
        sys.exit(1)

    # Write result JSON for parsing by Rust
    result_file = str(Path(OUTPUT_DIR) / "decomposition_result.json")
    with open(result_file, 'w') as f:
        json.dump(result, f, indent=2)

    print(f"Result written to: {result_file}")
    sys.stdout.flush()
    sys.stderr.flush()

    # Force-exit to skip Blender's slow scene cleanup.  All output files
    # have been written and flushed; the normal Python/Blender teardown
    # can take minutes on large scenes (freeing 7 GB+ of mesh data).
    os._exit(0)

if __name__ == '__main__':
    main()
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::OutputFormat;
    use std::path::PathBuf;

    #[test]
    fn test_generate_script_basic() {
        let blend_path = PathBuf::from("/test/model.blend");
        let output_path = PathBuf::from("/output/model.glb");
        let options = ConversionOptions::default();

        let script = generate_export_script(&blend_path, &output_path, &options, "abc123");

        assert!(script.contains("BLEND_FILE"));
        assert!(script.contains("OUTPUT_FILE"));
        assert!(script.contains("def export_gltf"));
        assert!(script.contains("def main"));
        assert!(script.contains("abc123"));
    }

    #[test]
    fn test_generate_script_with_draco() {
        let blend_path = PathBuf::from("/test/model.blend");
        let output_path = PathBuf::from("/output/model.glb");
        let mut options = ConversionOptions::default();
        options.gltf.draco_compression = true;

        let script = generate_export_script(&blend_path, &output_path, &options, "hash");

        assert!(script.contains("DRACO_COMPRESSION = True"));
        assert!(script.contains("export_draco_mesh_compression_enable"));
    }

    #[test]
    fn test_generate_script_linked_libraries() {
        let blend_path = PathBuf::from("/test/model.blend");
        let output_path = PathBuf::from("/output/model.glb");
        let mut options = ConversionOptions::default();
        options.linked_libraries.process_recursively = true;

        let script = generate_export_script(&blend_path, &output_path, &options, "hash");

        assert!(script.contains("process_linked_libraries"));
        assert!(script.contains("_processed_libraries"));
    }

    #[test]
    fn test_python_bool() {
        assert_eq!(python_bool(true), "True");
        assert_eq!(python_bool(false), "False");
    }

    #[test]
    fn test_generate_decomposition_script() {
        let blend_path = PathBuf::from("/test/scene.blend");
        let output_dir = PathBuf::from("/output/scene_assets");
        let options = ConversionOptions::scene_decomposition();

        let script =
            generate_decomposition_script(&blend_path, &output_dir, &options, "decomp_hash");

        assert!(script.contains("OUTPUT_DIR"));
        assert!(script.contains("decomp_hash"));
        assert!(script.contains("def classify_asset"));
        assert!(script.contains("def compute_bounds"));
        assert!(script.contains("def export_single_object"));
        assert!(script.contains("def collect_textures_for_object"));
        assert!(script.contains("manifest.json"));
        assert!(script.contains("use_selection"));
        // Verify excluded patterns set
        assert!(script.contains("Camera"));
        assert!(script.contains("Light"));
    }

    #[test]
    fn test_output_format_blender_string() {
        assert_eq!(OutputFormat::GlbBinary.blender_format(), "GLB");
        assert_eq!(OutputFormat::GltfEmbedded.blender_format(), "GLTF_EMBEDDED");
        assert_eq!(OutputFormat::GltfSeparate.blender_format(), "GLTF_SEPARATE");
    }
}
