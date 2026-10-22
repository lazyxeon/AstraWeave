//! Every WGSL uniform or storage struct that has a same-named `#[repr(C)]`
//! Rust struct must agree on size and field offsets.
//!
//! WGSL `vec3` is 16-byte aligned and 12 bytes wide. `#[repr(C)]` `[f32; 3]`
//! is 4-byte aligned. A missing pad makes the CPU buffer shorter than the
//! shader, and wgpu rejects the dispatch.

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
struct Field {
    name: String,
    ty: String,
}

#[derive(Clone, Debug)]
struct StructDef {
    name: String,
    fields: Vec<Field>,
    origin: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Space {
    Uniform,
    Storage,
}

fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let bytes = src.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn parse_wgsl_structs(src: &str, origin: &str) -> Vec<StructDef> {
    let text = strip_comments(src);
    let mut structs = Vec::new();
    let mut rest = text.as_str();
    while let Some(idx) = rest.find("struct ") {
        rest = &rest[idx + "struct ".len()..];
        let Some(name_end) = rest.find(|c: char| c == '{' || c.is_whitespace()) else {
            break;
        };
        let name = rest[..name_end].trim();
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            continue;
        }
        let Some(brace) = rest.find('{') else { break };
        let body_start = brace + 1;
        let Some(body) = take_brace(&rest[body_start..]) else {
            break;
        };
        let fields = parse_wgsl_fields(body);
        if !fields.is_empty() {
            structs.push(StructDef {
                name: name.to_string(),
                fields,
                origin: origin.to_string(),
            });
        }
        rest = &rest[body_start + body.len() + 1..];
    }
    structs
}

fn take_brace(src: &str) -> Option<&str> {
    let mut depth = 1i32;
    let mut angle = 0i32;
    for (i, ch) in src.char_indices() {
        match ch {
            '<' => angle += 1,
            '>' => angle = angle.saturating_sub(1),
            '{' if angle == 0 => depth += 1,
            '}' if angle == 0 => {
                depth -= 1;
                if depth == 0 {
                    return Some(&src[..i]);
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_wgsl_fields(body: &str) -> Vec<Field> {
    split_top_commas(body)
        .into_iter()
        .filter_map(|raw| {
            let raw = raw.trim().trim_end_matches(';').trim();
            if raw.is_empty() {
                return None;
            }
            let raw = strip_wgsl_attrs(raw);
            let (name, ty) = raw.split_once(':')?;
            let name = name.split_whitespace().last()?.trim();
            let ty = ty.trim().trim_end_matches(',').trim();
            if name.is_empty() || ty.is_empty() {
                return None;
            }
            Some(Field {
                name: name.to_string(),
                ty: ty.split_whitespace().collect::<Vec<_>>().join(""),
            })
        })
        .collect()
}

fn strip_wgsl_attrs(raw: &str) -> String {
    let mut s = raw.trim();
    while s.starts_with('@') {
        let bytes = s.as_bytes();
        let mut i = 1;
        while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
            i += 1;
        }
        if i < bytes.len() && bytes[i] == b'(' {
            let mut depth = 0;
            for (j, ch) in s[i..].char_indices() {
                match ch {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            s = s[i + j + 1..].trim();
                            break;
                        }
                    }
                    _ => {}
                }
            }
        } else {
            s = s[i..].trim();
        }
    }
    s.to_string()
}

fn split_top_commas(body: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut angle = 0i32;
    let mut brace = 0i32;
    for (i, ch) in body.char_indices() {
        match ch {
            '<' => angle += 1,
            '>' => angle = angle.saturating_sub(1),
            '{' => brace += 1,
            '}' => brace = brace.saturating_sub(1),
            ',' if angle == 0 && brace == 0 => {
                parts.push(&body[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&body[start..]);
    parts
}

fn bound_structs(src: &str) -> Vec<(String, Space)> {
    let text = strip_comments(src);
    let mut found = Vec::new();
    for (token, space) in [
        ("var<uniform", Space::Uniform),
        ("var<storage", Space::Storage),
    ] {
        let mut rest = text.as_str();
        while let Some(idx) = rest.find(token) {
            rest = &rest[idx..];
            let Some(colon) = rest.find(':') else { break };
            let after = rest[colon + 1..].trim_start();
            let ty = after
                .split(|c: char| c == ';' || c == '\n' || c == '@')
                .next()
                .unwrap_or("")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join("");
            if let Some(name) = binding_struct_name(&ty) {
                found.push((name, space));
            }
            rest = &rest[token.len()..];
        }
    }
    found
}

fn binding_struct_name(ty: &str) -> Option<String> {
    let ty = ty.trim();
    if ty.starts_with("array<") {
        let inner = ty.trim_start_matches("array<").trim_end_matches('>');
        let elem = inner.split(',').next()?.trim();
        if elem.starts_with("atomic<") || is_scalar_or_vec(elem) {
            return None;
        }
        return Some(elem.to_string());
    }
    if is_scalar_or_vec(ty) || ty.starts_with("atomic<") || ty.starts_with("texture") {
        return None;
    }
    Some(ty.to_string())
}

fn is_scalar_or_vec(ty: &str) -> bool {
    matches!(
        ty,
        "f32" | "u32" | "i32" | "bool" | "vec2" | "vec3" | "vec4"
    ) || ty.starts_with("vec2<")
        || ty.starts_with("vec3<")
        || ty.starts_with("vec4<")
        || ty.starts_with("mat")
}

#[derive(Clone, Debug)]
struct Layout {
    size: usize,
    align: usize,
    /// name, offset, size
    fields: Vec<(String, usize, usize)>,
}

fn round_up(align: usize, value: usize) -> usize {
    if align == 0 {
        return value;
    }
    value.div_ceil(align) * align
}

fn wgsl_layout(def: &StructDef, space: Space, structs: &[StructDef]) -> Result<Layout, String> {
    layout_fields(&def.fields, space, structs, true)
}

fn layout_fields(
    fields: &[Field],
    space: Space,
    structs: &[StructDef],
    is_struct: bool,
) -> Result<Layout, String> {
    let mut offset = 0;
    let mut max_align = 1;
    let mut placed = Vec::new();
    for field in fields {
        if field.ty.starts_with("array<") && !field.ty.contains(',') {
            break;
        }
        let (align, size) = wgsl_type(&field.ty, space, structs)?;
        offset = round_up(align, offset);
        placed.push((field.name.clone(), offset, size));
        offset += size;
        max_align = max_align.max(align);
    }
    if is_struct && space == Space::Uniform {
        max_align = round_up(16, max_align);
    }
    max_align = max_align.max(1);
    let size = round_up(max_align, offset);
    Ok(Layout {
        size,
        align: max_align,
        fields: placed,
    })
}

fn wgsl_type(ty: &str, space: Space, structs: &[StructDef]) -> Result<(usize, usize), String> {
    let ty = ty.trim();
    let scalar = |align, size| Ok((align, size));
    match ty {
        "f32" | "i32" | "u32" => scalar(4, 4),
        "bool" => scalar(4, 4),
        "vec2<f32>" | "vec2<i32>" | "vec2<u32>" => scalar(8, 8),
        "vec3<f32>" | "vec3<i32>" | "vec3<u32>" => scalar(16, 12),
        "vec4<f32>" | "vec4<i32>" | "vec4<u32>" => scalar(16, 16),
        "mat4x4<f32>" => scalar(16, 64),
        "mat3x3<f32>" => scalar(16, 48),
        "mat2x2<f32>" => scalar(8, 16),
        "mat3x4<f32>" | "mat4x3<f32>" => scalar(16, 64),
        _ if ty.starts_with("array<") => {
            let inner = &ty["array<".len()..ty.len() - 1];
            let mut parts = split_top_commas(inner);
            parts.retain(|p| !p.trim().is_empty());
            if parts.len() != 2 {
                return Err(format!("runtime or unparsed array {ty}"));
            }
            let (elem_align, elem_size) = wgsl_type(parts[0].trim(), space, structs)?;
            let count: usize = parts[1]
                .trim()
                .trim_end_matches('u')
                .parse()
                .map_err(|_| format!("bad array count in {ty}"))?;
            let mut stride = round_up(elem_align, elem_size);
            let mut align = elem_align;
            if space == Space::Uniform {
                stride = round_up(16, stride);
                align = round_up(16, align);
            }
            Ok((align, count * stride))
        }
        _ => {
            let nested = structs
                .iter()
                .find(|s| s.name == ty)
                .ok_or_else(|| format!("unknown type {ty}"))?;
            let layout = wgsl_layout(nested, space, structs)?;
            Ok((layout.align, layout.size))
        }
    }
}

fn parse_rust_structs(src: &str, origin: &str) -> Vec<StructDef> {
    let text = strip_comments(src);
    let mut structs = Vec::new();
    let mut rest = text.as_str();
    while let Some(idx) = rest.find("struct ") {
        let before = &rest[..idx];
        let repr_c = before.ends_with("#[repr(C)]\n")
            || before.ends_with("#[repr(C)]\r\n")
            || before.contains("#[repr(C)]")
                && before
                    .rsplit("#[repr(C)]")
                    .next()
                    .is_some_and(|tail| !tail.contains("struct "));
        rest = &rest[idx + "struct ".len()..];
        if !repr_c {
            continue;
        }
        let name: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if name.is_empty() {
            continue;
        }
        let Some(brace) = rest.find('{') else { break };
        let Some(body) = take_brace(&rest[brace + 1..]) else {
            break;
        };
        let fields = parse_rust_fields(body);
        if !fields.is_empty() {
            structs.push(StructDef {
                name,
                fields,
                origin: origin.to_string(),
            });
        }
        rest = &rest[brace + 1 + body.len() + 1..];
    }
    structs
}

fn parse_rust_fields(body: &str) -> Vec<Field> {
    split_top_commas(body)
        .into_iter()
        .filter_map(|raw| {
            let mut raw = raw.trim();
            if raw.is_empty() || raw.starts_with("//") {
                return None;
            }
            while raw.starts_with('#') || raw.starts_with("pub(") || raw.starts_with("pub ") {
                if let Some(stripped) = raw.strip_prefix("pub ") {
                    raw = stripped.trim();
                    continue;
                }
                if raw.starts_with("pub(") {
                    if let Some(end) = raw.find(')') {
                        raw = raw[end + 1..].trim().trim_start_matches("pub ").trim();
                        continue;
                    }
                }
                if raw.starts_with('#') {
                    let start = raw.find('[')?;
                    let attr = take_bracket(&raw[start + 1..])?;
                    raw = raw[start + 1 + attr.len() + 1..].trim();
                    continue;
                }
                break;
            }
            let (name, ty) = raw.split_once(':')?;
            let name = name.trim();
            if name.contains(' ') || name.is_empty() {
                return None;
            }
            let ty = ty.trim().trim_end_matches(',').trim();
            Some(Field {
                name: name.to_string(),
                ty: ty.split_whitespace().collect::<Vec<_>>().join(""),
            })
        })
        .collect()
}

fn take_bracket(src: &str) -> Option<&str> {
    let mut depth = 1;
    for (i, ch) in src.char_indices() {
        match ch {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&src[..i]);
                }
            }
            _ => {}
        }
    }
    None
}

fn rust_layout(def: &StructDef, structs: &[StructDef]) -> Result<Layout, String> {
    let mut offset = 0;
    let mut max_align = 1;
    let mut placed = Vec::new();
    for field in &def.fields {
        let (align, size) = rust_type(&field.ty, structs)?;
        offset = round_up(align, offset);
        placed.push((field.name.clone(), offset, size));
        offset += size;
        max_align = max_align.max(align);
    }
    Ok(Layout {
        size: round_up(max_align, offset),
        align: max_align,
        fields: placed,
    })
}

fn rust_type(ty: &str, structs: &[StructDef]) -> Result<(usize, usize), String> {
    let ty = ty.trim();
    if let Some(inner) = ty.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
        let (elem, count) = inner
            .rsplit_once(';')
            .ok_or_else(|| format!("bad array {ty}"))?;
        let count: usize = count
            .trim()
            .parse()
            .map_err(|_| format!("bad count {ty}"))?;
        let (align, size) = rust_type(elem.trim(), structs)?;
        return Ok((align, size * count));
    }
    match ty {
        "f32" | "u32" | "i32" => Ok((4, 4)),
        "f64" | "u64" | "i64" => Ok((8, 8)),
        "bool" => Ok((1, 1)),
        "Vec2" | "glam::Vec2" => Ok((4, 8)),
        "Vec3" | "glam::Vec3" => Ok((4, 12)),
        "Vec4" | "glam::Vec4" => Ok((16, 16)),
        "Mat4" | "glam::Mat4" => Ok((16, 64)),
        "UVec3" | "glam::UVec3" => Ok((4, 12)),
        "UVec4" | "glam::UVec4" => Ok((16, 16)),
        _ => {
            let nested = structs
                .iter()
                .find(|s| s.name == ty || ty.ends_with(&format!("::{name}", name = s.name)))
                .ok_or_else(|| format!("unknown rust type {ty}"))?;
            let layout = rust_layout(nested, structs)?;
            Ok((layout.align, layout.size))
        }
    }
}

fn rust_name_matches(rust_name: &str, wgsl_name: &str) -> bool {
    rust_name == wgsl_name
        || (wgsl_name == "IblParams" && rust_name == "IblParamsGpu")
        || (wgsl_name == "DirectionalLight" && rust_name == "DirectionalLightGpu")
}

fn layout_diffs(wgsl: &Layout, rust: &Layout) -> Vec<String> {
    let mut diffs = Vec::new();
    if rust.size != wgsl.size {
        diffs.push(format!("size rust {} != wgsl {}", rust.size, wgsl.size));
    }
    for (field, wgsl_off, wgsl_size) in &wgsl.fields {
        if let Some((_, rust_off, _)) = rust.fields.iter().find(|(n, _, _)| n == field) {
            if rust_off != wgsl_off {
                diffs.push(format!("{field} rust @{rust_off} != wgsl @{wgsl_off}"));
            }
            continue;
        }
        let covered = rust
            .fields
            .iter()
            .any(|(_, off, size)| *wgsl_off >= *off && wgsl_off + wgsl_size <= off + size);
        if !covered {
            diffs.push(format!("missing rust field {field} at @{wgsl_off}"));
        }
    }
    diffs
}

fn walk(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().and_then(|n| n.to_str()) == Some("_archive") {
                continue;
            }
            walk(&path, ext, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some(ext) {
            out.push(path);
        }
    }
}

#[test]
fn shared_uniform_and_storage_structs_match_wgsl() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut shader_files = Vec::new();
    walk(&root.join("shaders"), "wgsl", &mut shader_files);
    let mut rust_files = Vec::new();
    walk(&root.join("src"), "rs", &mut rust_files);

    let mut rust_structs = Vec::new();
    for path in &rust_files {
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with("_tests.rs"))
        {
            continue;
        }
        let text = fs::read_to_string(path).unwrap();
        let origin = path.strip_prefix(&root).unwrap().display().to_string();
        rust_structs.extend(parse_rust_structs(&text, &origin));
    }

    let mut all_wgsl = Vec::new();
    for path in &shader_files {
        let text = fs::read_to_string(path).unwrap();
        let origin = path.strip_prefix(&root).unwrap().display().to_string();
        all_wgsl.extend(parse_wgsl_structs(&text, &origin));
    }

    let mut failures = Vec::new();
    let mut checked = 0usize;
    for path in &shader_files {
        let text = fs::read_to_string(path).unwrap();
        let origin = path.strip_prefix(&root).unwrap().display().to_string();
        let local = parse_wgsl_structs(&text, &origin);
        let bound = bound_structs(&text);
        for (name, space) in bound {
            let rust_defs: Vec<_> = rust_structs
                .iter()
                .filter(|s| rust_name_matches(&s.name, &name))
                .collect();
            if rust_defs.is_empty() {
                continue;
            }
            let mut defs: Vec<&StructDef> = local.iter().filter(|s| s.name == name).collect();
            if defs.is_empty() {
                defs = all_wgsl.iter().filter(|s| s.name == name).collect();
            }
            if defs.is_empty() {
                failures.push(format!("{origin}: bound struct {name} was not parsed"));
                continue;
            }
            checked += 1;
            let mut matched = false;
            let mut attempts = Vec::new();
            for wgsl in defs {
                let catalog: Vec<StructDef> = all_wgsl
                    .iter()
                    .filter(|s| s.origin == wgsl.origin)
                    .cloned()
                    .collect();
                let wgsl_layout = match wgsl_layout(wgsl, space, &catalog) {
                    Ok(layout) => layout,
                    Err(err) => {
                        attempts.push(format!("{}: WGSL layout: {err}", wgsl.origin));
                        continue;
                    }
                };
                for rust in &rust_defs {
                    let rust_layout = match rust_layout(rust, &rust_structs) {
                        Ok(layout) => layout,
                        Err(err) => {
                            attempts.push(format!("{}: rust layout: {err}", rust.origin));
                            continue;
                        }
                    };
                    let diffs = layout_diffs(&wgsl_layout, &rust_layout);
                    if diffs.is_empty() {
                        matched = true;
                        break;
                    }
                    attempts.push(format!(
                        "{} vs {}: {}",
                        wgsl.origin,
                        rust.origin,
                        diffs.join("; ")
                    ));
                }
                if matched {
                    break;
                }
            }
            if !matched {
                let space = match space {
                    Space::Uniform => "uniform",
                    Space::Storage => "storage",
                };
                failures.push(format!(
                    "{origin}::{name} ({space}): {}",
                    attempts.join(" | ")
                ));
            }
        }
    }

    assert!(
        checked > 0,
        "the layout audit did not pair any shader struct with Rust"
    );
    assert!(
        failures.is_empty(),
        "GPU struct layout mismatches:\n{}",
        failures.join("\n")
    );
}
