//! 3D Import & Export engine for OBJ (.obj/.mtl) and glTF 2.0 / GLB (.gltf / .glb)
//! (PRD FM-5, FM-6, FM-7, Section 9.4). Zero external dependencies.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;

use crate::command::AddNodesCommand;
use crate::lookdev::{
    base64_decode, base64_encode, AlphaMode, MaterialId, MaterialSpec, TextureAsset, TextureId,
    TextureStatus,
};
use crate::math::{Mat4, Quat, Vec3};
use crate::poly::PolyMesh;
use crate::scene::{
    CameraNodeParams, LightParams, MaterialParams, MeshData, NodeId, NodeKind, NovaEngineState,
    SceneNode, Transform,
};
use crate::serialize::{
    as_f32, as_obj, as_str, as_u64, escape_json_str, push_f32, JsonParser, JsonVal,
};

pub const MAX_IMPORT_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_VERTICES_PER_MESH: usize = 1_000_000;
pub const MAX_INDICES_PER_MESH: usize = 3_000_000;
pub const MAX_NODES_PER_IMPORT: usize = 4096;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FormatReport {
    pub warnings: Vec<String>,
    pub imported_nodes: usize,
    pub imported_materials: usize,
    pub imported_textures: usize,
}

impl FormatReport {
    pub fn push_warning(&mut self, msg: impl Into<String>) {
        let s = msg.into();
        if !self.warnings.contains(&s) {
            self.warnings.push(s);
        }
    }
}

fn push_f32_json_array(vals: &[f32], out: &mut String) {
    out.push('[');
    for (i, &v) in vals.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        push_f32(v, out);
    }
    out.push(']');
}

fn sanitize_name(raw: &str, fallback: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.chars().take(64).collect()
    }
}

fn ensure_valid_mesh_bounds(positions: &[[f32; 3]], indices: &[u32]) -> Result<(), String> {
    if positions.is_empty() {
        return Err("Mesh contains zero vertices".to_string());
    }
    if positions.len() > MAX_VERTICES_PER_MESH {
        return Err(format!(
            "Mesh vertex count {} exceeds safety limit {}",
            positions.len(),
            MAX_VERTICES_PER_MESH
        ));
    }
    if indices.len() > MAX_INDICES_PER_MESH {
        return Err(format!(
            "Mesh index count {} exceeds safety limit {}",
            indices.len(),
            MAX_INDICES_PER_MESH
        ));
    }
    if indices.is_empty() || indices.len() % 3 != 0 {
        return Err("Mesh has empty or non-triangular index buffer".to_string());
    }
    for p in positions {
        if !p[0].is_finite() || !p[1].is_finite() || !p[2].is_finite() {
            return Err("Mesh contains non-finite vertex coordinates".to_string());
        }
    }
    let vcount = positions.len() as u32;
    for &idx in indices {
        if idx >= vcount {
            return Err(format!(
                "Mesh index {idx} out of bounds for {vcount} vertices"
            ));
        }
    }
    Ok(())
}

fn compute_missing_normals(positions: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut accum = vec![Vec3::ZERO; positions.len()];
    for tri in indices.chunks_exact(3) {
        let a = Vec3::new(
            positions[tri[0] as usize][0],
            positions[tri[0] as usize][1],
            positions[tri[0] as usize][2],
        );
        let b = Vec3::new(
            positions[tri[1] as usize][0],
            positions[tri[1] as usize][1],
            positions[tri[1] as usize][2],
        );
        let c = Vec3::new(
            positions[tri[2] as usize][0],
            positions[tri[2] as usize][1],
            positions[tri[2] as usize][2],
        );
        let n = (b - a).cross(c - a);
        accum[tri[0] as usize] = accum[tri[0] as usize] + n;
        accum[tri[1] as usize] = accum[tri[1] as usize] + n;
        accum[tri[2] as usize] = accum[tri[2] as usize] + n;
    }
    accum
        .into_iter()
        .map(|n| n.normalized_or(Vec3::UNIT_Y).to_array())
        .collect()
}

fn commit_imported_nodes(
    state: &mut NovaEngineState,
    nodes: Vec<SceneNode>,
) {
    if nodes.is_empty() {
        return;
    }
    let next_selected: Vec<NodeId> = nodes
        .iter()
        .filter(|n| n.parent_id.is_none() || nodes.iter().all(|p| Some(p.id) != n.parent_id))
        .map(|n| n.id)
        .collect();
    let prev_selected = state.selected_ids.clone();
    let cmd = AddNodesCommand {
        added: nodes,
        prev_selected,
        next_selected,
    };
    state
        .history
        .execute(Box::new(cmd), &mut state.nodes, &mut state.selected_ids);
    state.scene_revision += 1;
}

// ============================================================================
// Wavefront OBJ (.obj) Import & Export
// ============================================================================

#[derive(Default)]
struct ObjGroupBuilder {
    name: String,
    mat_name: Option<String>,
    vert_map: HashMap<(usize, usize, usize), u32>,
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
    has_normals: bool,
    has_uvs: bool,
}

fn resolve_obj_index(raw: isize, len: usize) -> Result<usize, String> {
    if raw > 0 {
        let idx = (raw - 1) as usize;
        if idx < len {
            return Ok(idx);
        }
    } else if raw < 0 {
        let abs = (-raw) as usize;
        if abs <= len {
            return Ok(len - abs);
        }
    }
    Err(format!("OBJ index {raw} out of bounds for length {len}"))
}

pub fn import_obj(
    state: &mut NovaEngineState,
    bytes: &[u8],
    asset_name: &str,
) -> Result<FormatReport, String> {
    if bytes.is_empty() {
        return Err("OBJ file is empty".to_string());
    }
    if bytes.len() > MAX_IMPORT_BYTES {
        return Err("OBJ file exceeds 64 MiB safety limit".to_string());
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| "OBJ file is not valid UTF-8 text".to_string())?;

    let mut report = FormatReport::default();
    let base_name = sanitize_name(asset_name, "Imported Mesh");

    let mut raw_pos: Vec<[f32; 3]> = Vec::new();
    let mut raw_uv: Vec<[f32; 2]> = Vec::new();
    let mut raw_norm: Vec<[f32; 3]> = Vec::new();

    let mut groups: Vec<ObjGroupBuilder> = Vec::new();
    let mut current = ObjGroupBuilder {
        name: base_name.clone(),
        ..Default::default()
    };

    for (line_idx, raw_line) in text.lines().enumerate() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(cmd) = parts.next() else {
            continue;
        };
        match cmd {
            "v" => {
                if raw_pos.len() >= MAX_VERTICES_PER_MESH {
                    return Err("OBJ vertex count exceeds safety limit".to_string());
                }
                let coords: Vec<f32> = parts
                    .take(3)
                    .map(|s| s.parse::<f32>().map_err(|_| format!("Invalid vertex on line {}", line_idx + 1)))
                    .collect::<Result<_, _>>()?;
                if coords.len() < 3 || !coords.iter().all(|f| f.is_finite()) {
                    return Err(format!("Malformed vertex on line {}", line_idx + 1));
                }
                raw_pos.push([coords[0], coords[1], coords[2]]);
            }
            "vt" => {
                let coords: Vec<f32> = parts
                    .take(2)
                    .map(|s| s.parse::<f32>().unwrap_or(0.0))
                    .collect();
                let u = if coords.first().copied().unwrap_or(0.0).is_finite() {
                    coords.first().copied().unwrap_or(0.0)
                } else {
                    0.0
                };
                let v = if coords.get(1).copied().unwrap_or(0.0).is_finite() {
                    coords.get(1).copied().unwrap_or(0.0)
                } else {
                    0.0
                };
                // Flip V from OBJ bottom-left to Filament/glTF top-left UV convention
                raw_uv.push([u, 1.0 - v]);
            }
            "vn" => {
                let coords: Vec<f32> = parts
                    .take(3)
                    .map(|s| s.parse::<f32>().unwrap_or(0.0))
                    .collect();
                if coords.len() == 3 {
                    let n = Vec3::new(coords[0], coords[1], coords[2]).normalized_or(Vec3::UNIT_Y);
                    raw_norm.push(n.to_array());
                }
            }
            "o" | "g" => {
                let gname = parts.collect::<Vec<_>>().join(" ");
                let clean = sanitize_name(&gname, &base_name);
                if !current.indices.is_empty() {
                    groups.push(std::mem::take(&mut current));
                }
                current.name = clean;
            }
            "usemtl" => {
                let mname = parts.collect::<Vec<_>>().join(" ");
                if !mname.is_empty() {
                    current.mat_name = Some(mname);
                }
            }
            "mtllib" => {
                let lib = parts.collect::<Vec<_>>().join(" ");
                if !lib.is_empty() {
                    report.push_warning(format!(
                        "External OBJ material library '{lib}' cannot be read from standalone stream; using default studio material."
                    ));
                }
            }
            "f" => {
                let tokens: Vec<&str> = parts.collect();
                if tokens.len() < 3 {
                    report.push_warning(format!(
                        "Degenerate face with < 3 vertices at line {} skipped.",
                        line_idx + 1
                    ));
                    continue;
                }
                let mut face_verts = Vec::with_capacity(tokens.len());
                for tok in tokens {
                    let mut segs = tok.split('/');
                    let v_str = segs.next().unwrap_or("");
                    let v_raw: isize = v_str
                        .parse()
                        .map_err(|_| format!("Invalid face vertex index '{v_str}' on line {}", line_idx + 1))?;
                    let vi = resolve_obj_index(v_raw, raw_pos.len())?;

                    let vt_str = segs.next().unwrap_or("");
                    let ti = if !vt_str.is_empty() {
                        current.has_uvs = true;
                        let r: isize = vt_str.parse().map_err(|_| {
                            format!("Invalid face UV index '{vt_str}' on line {}", line_idx + 1)
                        })?;
                        resolve_obj_index(r, raw_uv.len())? + 1
                    } else {
                        0
                    };

                    let vn_str = segs.next().unwrap_or("");
                    let ni = if !vn_str.is_empty() {
                        current.has_normals = true;
                        let r: isize = vn_str.parse().map_err(|_| {
                            format!("Invalid face normal index '{vn_str}' on line {}", line_idx + 1)
                        })?;
                        resolve_obj_index(r, raw_norm.len())? + 1
                    } else {
                        0
                    };

                    let key = (vi, ti, ni);
                    let out_idx = if let Some(&existing) = current.vert_map.get(&key) {
                        existing
                    } else {
                        if current.positions.len() >= MAX_VERTICES_PER_MESH {
                            return Err("OBJ group vertex count exceeds safety limit".to_string());
                        }
                        let new_idx = current.positions.len() as u32;
                        current.vert_map.insert(key, new_idx);
                        current.positions.push(raw_pos[vi]);
                        current.uvs.push(if ti > 0 { raw_uv[ti - 1] } else { [0.0, 0.0] });
                        current.normals.push(if ni > 0 {
                            raw_norm[ni - 1]
                        } else {
                            [0.0, 1.0, 0.0]
                        });
                        new_idx
                    };
                    face_verts.push(out_idx);
                }

                // Fan triangulate convex polygons/quads
                for k in 1..(face_verts.len() - 1) {
                    if current.indices.len() + 3 > MAX_INDICES_PER_MESH {
                        return Err("OBJ index count exceeds safety limit".to_string());
                    }
                    current.indices.push(face_verts[0]);
                    current.indices.push(face_verts[k]);
                    current.indices.push(face_verts[k + 1]);
                }
            }
            "curv" | "surf" | "cstype" | "vp" | "parm" | "trim" | "hole" => {
                report.push_warning(
                    "NURBS/free-form curves ('cstype'/'curv'/'surf') in OBJ are unsupported and were skipped.",
                );
            }
            "l" => {
                report.push_warning("OBJ polyline ('l') primitives were skipped; only polygonal faces are imported.");
            }
            _ => {}
        }
    }

    if !current.indices.is_empty() {
        groups.push(current);
    }

    if groups.is_empty() {
        return Err("OBJ file contains no valid polygonal faces".to_string());
    }
    if groups.len() > MAX_NODES_PER_IMPORT {
        return Err("OBJ object group count exceeds safety limit".to_string());
    }

    let mut new_nodes = Vec::with_capacity(groups.len());
    let default_mat_id = state.materials.first().map(|m| m.id);
    let default_mat_params = state
        .materials
        .first()
        .map(|m| MaterialParams {
            base_color: m.base_color,
            metallic: m.metallic,
            roughness: m.roughness,
        })
        .unwrap_or_default();

    // If multi-group OBJ, create an Empty root so user can move the whole asset cleanly
    let parent_id = if groups.len() > 1 {
        let root_id = state.allocate_id();
        let root_name = state.unique_name(&base_name);
        new_nodes.push(SceneNode {
            id: root_id,
            parent_id: None,
            name: root_name,
            kind: NodeKind::Empty,
            transform: Transform::default(),
            mesh: None,
            base_poly: None,
            modifiers: Vec::new(),
            material_id: None,
            material: default_mat_params.clone(),
            light: LightParams::default(),
            camera_node: CameraNodeParams::default(),
            visible: true,
        });
        Some(root_id)
    } else {
        None
    };

    for mut g in groups {
        if !g.has_normals {
            g.normals = compute_missing_normals(&g.positions, &g.indices);
        }
        ensure_valid_mesh_bounds(&g.positions, &g.indices)?;
        let (bounds_min, bounds_max) = MeshData::compute_bounds(&g.positions);
        let mesh = MeshData {
            positions: g.positions,
            normals: g.normals,
            uvs: g.uvs,
            indices: g.indices,
            bounds_min,
            bounds_max,
            version: 1,
        };
        let base_poly = Some(PolyMesh::from_mesh_data_fallback(&mesh, false));
        let id = state.allocate_id();
        let node_name = state.unique_name(&g.name);
        new_nodes.push(SceneNode {
            id,
            parent_id,
            name: node_name,
            kind: NodeKind::Mesh,
            transform: Transform::default(),
            mesh: Some(mesh),
            base_poly,
            modifiers: Vec::new(),
            material_id: default_mat_id,
            material: default_mat_params.clone(),
            light: LightParams::default(),
            camera_node: CameraNodeParams::default(),
            visible: true,
        });
    }

    report.imported_nodes = new_nodes.len();
    commit_imported_nodes(state, new_nodes);
    Ok(report)
}

pub fn export_obj(state: &NovaEngineState) -> Result<(Vec<u8>, FormatReport), String> {
    let mut report = FormatReport::default();
    let mut out = String::with_capacity(16384);
    out.push_str("# Exported by NOVA 3D Studio v1.0\n");

    let mut mesh_count = 0usize;
    let mut v_offset = 1usize;
    let mut vt_offset = 1usize;
    let mut vn_offset = 1usize;
    let mut has_lights = false;
    let mut has_cameras = false;
    let mut has_textures_or_pbr = false;

    for node in &state.nodes {
        match node.kind {
            NodeKind::Light => {
                has_lights = true;
                continue;
            }
            NodeKind::Camera => {
                has_cameras = true;
                continue;
            }
            NodeKind::Empty => continue,
            NodeKind::Mesh => {}
        }
        let Some(mesh) = &node.mesh else {
            continue;
        };
        if !node.visible || mesh.positions.is_empty() || mesh.indices.is_empty() {
            continue;
        }
        mesh_count += 1;

        if let Some(mat_id) = node.material_id {
            if let Some(mat) = state.materials.iter().find(|m| m.id == mat_id) {
                if mat.albedo_tex_id.is_some()
                    || mat.normal_tex_id.is_some()
                    || mat.roughness_tex_id.is_some()
                    || mat.metallic > 0.01
                {
                    has_textures_or_pbr = true;
                }
            }
        }

        let world = state.node_world_matrix(node.id);
        // Normal transform using upper-left 3x3 inverse transpose when invertible
        let normal_mat = world.inverse().unwrap_or(Mat4::IDENTITY);
        let obj_label = node.name.replace(['\r', '\n', ' '], "_");
        let _ = writeln!(out, "\no {obj_label}");

        for p in &mesh.positions {
            let wp = world.transform_point(Vec3::new(p[0], p[1], p[2]));
            let _ = writeln!(out, "v {:.6} {:.6} {:.6}", wp.x, wp.y, wp.z);
        }
        for uv in &mesh.uvs {
            // Restore OBJ bottom-left V origin
            let _ = writeln!(out, "vt {:.6} {:.6}", uv[0], 1.0 - uv[1]);
        }
        for n in &mesh.normals {
            // Apply transpose of inverse upper-left 3x3
            let m = normal_mat.data;
            let nx = m[0] * n[0] + m[1] * n[1] + m[2] * n[2];
            let ny = m[4] * n[0] + m[5] * n[1] + m[6] * n[2];
            let nz = m[8] * n[0] + m[9] * n[1] + m[10] * n[2];
            let nn = Vec3::new(nx, ny, nz).normalized_or(Vec3::UNIT_Y);
            let _ = writeln!(out, "vn {:.6} {:.6} {:.6}", nn.x, nn.y, nn.z);
        }

        let has_vt = mesh.uvs.len() == mesh.positions.len();
        let has_vn = mesh.normals.len() == mesh.positions.len();
        for tri in mesh.indices.chunks_exact(3) {
            let a = tri[0] as usize;
            let b = tri[1] as usize;
            let c = tri[2] as usize;
            match (has_vt, has_vn) {
                (true, true) => {
                    let _ = writeln!(
                        out,
                        "f {}/{}/{} {}/{}/{} {}/{}/{}",
                        v_offset + a,
                        vt_offset + a,
                        vn_offset + a,
                        v_offset + b,
                        vt_offset + b,
                        vn_offset + b,
                        v_offset + c,
                        vt_offset + c,
                        vn_offset + c
                    );
                }
                (false, true) => {
                    let _ = writeln!(
                        out,
                        "f {}//{} {}//{} {}//{}",
                        v_offset + a,
                        vn_offset + a,
                        v_offset + b,
                        vn_offset + b,
                        v_offset + c,
                        vn_offset + c
                    );
                }
                _ => {
                    let _ = writeln!(out, "f {} {} {}", v_offset + a, v_offset + b, v_offset + c);
                }
            }
        }

        v_offset += mesh.positions.len();
        vt_offset += mesh.uvs.len();
        vn_offset += mesh.normals.len();
    }

    if mesh_count == 0 {
        return Err("Scene has no visible meshes to export as OBJ".to_string());
    }

    if has_lights {
        report.push_warning("Scene lights omitted: OBJ format does not support light sources.");
    }
    if has_cameras {
        report.push_warning("Scene cameras omitted: OBJ format does not support camera nodes.");
    }
    if !state.animation.tracks.is_empty() {
        report.push_warning("Animation keyframes omitted: OBJ stores static baked geometry only.");
    }
    if has_textures_or_pbr {
        report.push_warning("PBR metallic/roughness maps and textures are not embedded in standalone .obj; export GLB for full PBR materials.");
    }

    Ok((out.into_bytes(), report))
}

// ============================================================================
// Pure-Rust CRC32, Adler32, Stored-PNG Encoder & Deflate/PNG Decoder
// (Supports full glTF/GLB RGBA texture round-trip with zero external crates)
// ============================================================================

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

fn adler32(data: &[u8]) -> u32 {
    let mut s1 = 1u32;
    let mut s2 = 0u32;
    for &b in data {
        s1 = (s1 + b as u32) % 65521;
        s2 = (s2 + s1) % 65521;
    }
    (s2 << 16) | s1
}

fn push_png_chunk(out: &mut Vec<u8>, tag: &[u8; 4], payload: &[u8]) {
    out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    let start = out.len();
    out.extend_from_slice(tag);
    out.extend_from_slice(payload);
    let c = crc32(&out[start..]);
    out.extend_from_slice(&c.to_be_bytes());
}

/// Encodes RGBA8 pixels into a valid standard PNG using uncompressed zlib/DEFLATE stored blocks.
pub fn encode_png_rgba8(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    let row_bytes = (width as usize)
        .checked_mul(4)
        .ok_or_else(|| "PNG row overflow".to_string())?;
    let expected = row_bytes
        .checked_mul(height as usize)
        .ok_or_else(|| "PNG size overflow".to_string())?;
    if width == 0 || height == 0 || rgba.len() != expected {
        return Err("Invalid RGBA buffer dimensions for PNG encoding".to_string());
    }

    let raw_len = (1 + row_bytes) * (height as usize);
    let mut scanlines = Vec::with_capacity(raw_len);
    for y in 0..(height as usize) {
        scanlines.push(0); // filter 0 = None
        let start = y * row_bytes;
        scanlines.extend_from_slice(&rgba[start..start + row_bytes]);
    }

    // Wrap scanlines in RFC 1950 zlib stream with RFC 1951 stored blocks (max 65535 bytes each)
    let mut zlib = Vec::with_capacity(scanlines.len() + 64);
    zlib.push(0x78); // CMF: DEFLATE 32K window
    zlib.push(0x01); // FLG: FCHECK passes ((0x78*256 + 0x01) % 31 == 0)

    let chunks: Vec<&[u8]> = scanlines.chunks(65535).collect();
    for (i, chunk) in chunks.iter().enumerate() {
        let is_last = if i + 1 == chunks.len() { 1u8 } else { 0u8 };
        zlib.push(is_last); // BFINAL + BTYPE=00 (stored)
        let len = chunk.len() as u16;
        zlib.extend_from_slice(&len.to_le_bytes());
        zlib.extend_from_slice(&(!len).to_le_bytes());
        zlib.extend_from_slice(chunk);
    }
    zlib.extend_from_slice(&adler32(&scanlines).to_be_bytes());

    let mut png = Vec::with_capacity(zlib.len() + 64);
    png.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.push(8); // bit depth
    ihdr.push(6); // RGBA
    ihdr.push(0); // compression
    ihdr.push(0); // filter
    ihdr.push(0); // interlace
    push_png_chunk(&mut png, b"IHDR", &ihdr);
    push_png_chunk(&mut png, b"IDAT", &zlib);
    push_png_chunk(&mut png, b"IEND", &[]);
    Ok(png)
}

struct BitReader<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_buf: u32,
    bit_cnt: u8,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_pos: 0,
            bit_buf: 0,
            bit_cnt: 0,
        }
    }

    fn ensure(&mut self, n: u8) -> Result<(), String> {
        while self.bit_cnt < n {
            let b = *self
                .data
                .get(self.byte_pos)
                .ok_or_else(|| "Unexpected end of DEFLATE stream".to_string())?;
            self.byte_pos += 1;
            self.bit_buf |= (b as u32) << self.bit_cnt;
            self.bit_cnt += 8;
        }
        Ok(())
    }

    fn read_bits(&mut self, n: u8) -> Result<u32, String> {
        if n == 0 {
            return Ok(0);
        }
        self.ensure(n)?;
        let val = self.bit_buf & ((1u32 << n) - 1);
        self.bit_buf >>= n;
        self.bit_cnt -= n;
        Ok(val)
    }

    fn align_byte(&mut self) {
        let drop = self.bit_cnt & 7;
        self.bit_buf >>= drop;
        self.bit_cnt -= drop;
    }

    fn read_aligned_u16(&mut self) -> Result<u16, String> {
        self.align_byte();
        let b0 = self.read_bits(8)? as u16;
        let b1 = self.read_bits(8)? as u16;
        Ok(b0 | (b1 << 8))
    }
}

struct HuffTable {
    counts: [u16; 16],
    symbols: Vec<u16>,
}

impl HuffTable {
    fn from_lengths(lengths: &[u8]) -> Result<Self, String> {
        let mut counts = [0u16; 16];
        for &l in lengths {
            if l as usize >= counts.len() {
                return Err("Invalid Huffman code length".to_string());
            }
            if l > 0 {
                counts[l as usize] += 1;
            }
        }
        let mut offsets = [0u16; 16];
        let mut sum = 0u16;
        for bits in 1..16 {
            offsets[bits] = sum;
            sum = sum.saturating_add(counts[bits]);
        }
        let mut symbols = vec![0u16; lengths.len()];
        for (sym, &len) in lengths.iter().enumerate() {
            if len != 0 {
                let idx = offsets[len as usize] as usize;
                offsets[len as usize] += 1;
                if idx < symbols.len() {
                    symbols[idx] = sym as u16;
                }
            }
        }
        Ok(Self { counts, symbols })
    }

    fn decode(&self, r: &mut BitReader<'_>) -> Result<u16, String> {
        let mut code = 0i32;
        let mut first = 0i32;
        let mut index = 0i32;
        for len in 1..16usize {
            code |= r.read_bits(1)? as i32;
            let count = self.counts[len] as i32;
            if code - count < first {
                return Ok(self.symbols[(index + (code - first)) as usize]);
            }
            index += count;
            first += count;
            first <<= 1;
            code <<= 1;
        }
        Err("Invalid Huffman symbol in DEFLATE stream".to_string())
    }
}

fn inflate_zlib(zlib: &[u8], max_out: usize) -> Result<Vec<u8>, String> {
    if zlib.len() < 6 {
        return Err("Truncated zlib stream".to_string());
    }
    if (zlib[0] & 0x0F) != 8 {
        return Err("Unsupported zlib compression method".to_string());
    }
    let mut start = 2usize;
    if (zlib[1] & 0x20) != 0 {
        start += 4; // FDICT
    }
    if start >= zlib.len() {
        return Err("Invalid zlib header".to_string());
    }

    let mut r = BitReader::new(&zlib[start..]);
    let mut out = Vec::new();

    const LEN_BASE: [u16; 29] = [
        3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115,
        131, 163, 195, 227, 258,
    ];
    const LEN_EXTRA: [u8; 29] = [
        0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
    ];
    const DIST_BASE: [u16; 30] = [
        1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
        2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
    ];
    const DIST_EXTRA: [u8; 30] = [
        0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12,
        13, 13,
    ];
    const CL_ORDER: [usize; 19] = [
        16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
    ];

    loop {
        let bfinal = r.read_bits(1)?;
        let btype = r.read_bits(2)?;
        if btype == 0 {
            let len = r.read_aligned_u16()? as usize;
            let nlen = r.read_aligned_u16()?;
            if (len as u16) != !nlen {
                return Err("Corrupt stored DEFLATE block length".to_string());
            }
            if out.len() + len > max_out {
                return Err("Decompressed PNG exceeds safety limit".to_string());
            }
            for _ in 0..len {
                out.push(r.read_bits(8)? as u8);
            }
        } else if btype == 1 || btype == 2 {
            let (lit_tbl, dist_tbl) = if btype == 1 {
                let mut ll = [0u8; 288];
                for i in 0..144 {
                    ll[i] = 8;
                }
                for i in 144..256 {
                    ll[i] = 9;
                }
                for i in 256..280 {
                    ll[i] = 7;
                }
                for i in 280..288 {
                    ll[i] = 8;
                }
                let dl = [5u8; 32];
                (HuffTable::from_lengths(&ll)?, HuffTable::from_lengths(&dl)?)
            } else {
                let hlit = r.read_bits(5)? as usize + 257;
                let hdist = r.read_bits(5)? as usize + 1;
                let hclen = r.read_bits(4)? as usize + 4;
                let mut cl_lens = [0u8; 19];
                for i in 0..hclen {
                    cl_lens[CL_ORDER[i]] = r.read_bits(3)? as u8;
                }
                let cl_tbl = HuffTable::from_lengths(&cl_lens)?;
                let mut combined = Vec::with_capacity(hlit + hdist);
                while combined.len() < hlit + hdist {
                    let sym = cl_tbl.decode(&mut r)?;
                    match sym {
                        0..=15 => combined.push(sym as u8),
                        16 => {
                            let prev = *combined
                                .last()
                                .ok_or_else(|| "Invalid repeat code in DEFLATE".to_string())?;
                            let rep = r.read_bits(2)? as usize + 3;
                            combined.resize(combined.len() + rep, prev);
                        }
                        17 => {
                            let rep = r.read_bits(3)? as usize + 3;
                            combined.resize(combined.len() + rep, 0);
                        }
                        18 => {
                            let rep = r.read_bits(7)? as usize + 11;
                            combined.resize(combined.len() + rep, 0);
                        }
                        _ => return Err("Invalid code-length symbol".to_string()),
                    }
                }
                if combined.len() < hlit + hdist {
                    return Err("Truncated dynamic Huffman tables".to_string());
                }
                (
                    HuffTable::from_lengths(&combined[..hlit])?,
                    HuffTable::from_lengths(&combined[hlit..hlit + hdist])?,
                )
            };

            loop {
                let sym = lit_tbl.decode(&mut r)?;
                if sym < 256 {
                    if out.len() + 1 > max_out {
                        return Err("Decompressed PNG exceeds safety limit".to_string());
                    }
                    out.push(sym as u8);
                } else if sym == 256 {
                    break;
                } else {
                    let lidx = (sym - 257) as usize;
                    if lidx >= LEN_BASE.len() {
                        return Err("Invalid DEFLATE length symbol".to_string());
                    }
                    let length = LEN_BASE[lidx] as usize + r.read_bits(LEN_EXTRA[lidx])? as usize;
                    let dsym = dist_tbl.decode(&mut r)? as usize;
                    if dsym >= DIST_BASE.len() {
                        return Err("Invalid DEFLATE distance symbol".to_string());
                    }
                    let dist = DIST_BASE[dsym] as usize + r.read_bits(DIST_EXTRA[dsym])? as usize;
                    if dist == 0 || dist > out.len() {
                        return Err("Invalid DEFLATE back-reference".to_string());
                    }
                    if out.len() + length > max_out {
                        return Err("Decompressed PNG exceeds safety limit".to_string());
                    }
                    for _ in 0..length {
                        let b = out[out.len() - dist];
                        out.push(b);
                    }
                }
            }
        } else {
            return Err("Reserved DEFLATE block type 3".to_string());
        }
        if bfinal == 1 {
            break;
        }
    }
    Ok(out)
}

fn paeth_predictor(a: u8, b: u8, c: u8) -> u8 {
    let ia = a as i32;
    let ib = b as i32;
    let ic = c as i32;
    let p = ia + ib - ic;
    let pa = (p - ia).abs();
    let pb = (p - ib).abs();
    let pc = (p - ic).abs();
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

pub fn decode_png_rgba8(png: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    if png.len() < 33 || &png[..8] != b"\x89PNG\r\n\x1a\n" {
        return Err("Not a valid PNG signature".to_string());
    }
    let mut pos = 8usize;
    let mut width = 0u32;
    let mut height = 0u32;
    let mut bit_depth = 0u8;
    let mut color_type = 0u8;
    let mut interlace = 0u8;
    let mut idat = Vec::new();

    while pos + 12 <= png.len() {
        let len = u32::from_be_bytes([png[pos], png[pos + 1], png[pos + 2], png[pos + 3]]) as usize;
        let tag = &png[pos + 4..pos + 8];
        let payload_start = pos + 8;
        let payload_end = payload_start
            .checked_add(len)
            .ok_or_else(|| "PNG chunk overflow".to_string())?;
        if payload_end + 4 > png.len() {
            return Err("Truncated PNG chunk".to_string());
        }
        let payload = &png[payload_start..payload_end];
        if tag == b"IHDR" {
            if payload.len() < 13 {
                return Err("Invalid PNG IHDR".to_string());
            }
            width = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
            height = u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]);
            bit_depth = payload[8];
            color_type = payload[9];
            interlace = payload[12];
        } else if tag == b"IDAT" {
            idat.extend_from_slice(payload);
        } else if tag == b"IEND" {
            break;
        }
        pos = payload_end + 4;
    }

    if width == 0 || height == 0 || width > 4096 || height > 4096 {
        return Err(format!("Unsupported PNG dimensions {width}x{height}"));
    }
    if bit_depth != 8 || interlace != 0 {
        return Err("Only non-interlaced 8-bit PNG textures are supported".to_string());
    }
    let channels = match color_type {
        0 => 1usize,
        2 => 3usize,
        4 => 2usize,
        6 => 4usize,
        _ => return Err(format!("Unsupported PNG color type {color_type}")),
    };
    let stride = width as usize * channels;
    let expected_raw = (1 + stride) * height as usize;
    let raw = inflate_zlib(&idat, expected_raw + 1024)?;
    if raw.len() < expected_raw {
        return Err("Decompressed PNG IDAT too short".to_string());
    }

    let mut prev_row = vec![0u8; stride];
    let mut cur_row = vec![0u8; stride];
    let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);

    for y in 0..(height as usize) {
        let base = y * (1 + stride);
        let filter = raw[base];
        let src = &raw[base + 1..base + 1 + stride];
        for i in 0..stride {
            let left = if i >= channels { cur_row[i - channels] } else { 0 };
            let up = prev_row[i];
            let up_left = if i >= channels {
                prev_row[i - channels]
            } else {
                0
            };
            let decoded = match filter {
                0 => src[i],
                1 => src[i].wrapping_add(left),
                2 => src[i].wrapping_add(up),
                3 => src[i].wrapping_add(((left as u16 + up as u16) >> 1) as u8),
                4 => src[i].wrapping_add(paeth_predictor(left, up, up_left)),
                _ => return Err(format!("Unknown PNG scanline filter {filter}")),
            };
            cur_row[i] = decoded;
        }
        match channels {
            4 => rgba.extend_from_slice(&cur_row),
            3 => {
                for px in cur_row.chunks_exact(3) {
                    rgba.extend_from_slice(&[px[0], px[1], px[2], 255]);
                }
            }
            2 => {
                for px in cur_row.chunks_exact(2) {
                    rgba.extend_from_slice(&[px[0], px[0], px[0], px[1]]);
                }
            }
            1 => {
                for &g in &cur_row {
                    rgba.extend_from_slice(&[g, g, g, 255]);
                }
            }
            _ => {}
        }
        prev_row.copy_from_slice(&cur_row);
    }

    Ok((width, height, rgba))
}

// ============================================================================
// glTF 2.0 / Binary GLB Import & Export
// ============================================================================

const GLB_MAGIC: u32 = 0x4654_6C67; // "glTF"
const GLB_CHUNK_JSON: u32 = 0x4E4F_534A; // "JSON"
const GLB_CHUNK_BIN: u32 = 0x004E_4942; // "BIN\0"

fn unpack_glb(bytes: &[u8]) -> Result<(String, Option<Vec<u8>>), String> {
    if bytes.len() < 12 {
        return Err("GLB header too short".to_string());
    }
    let magic = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    if magic != GLB_MAGIC {
        return Err("Invalid GLB magic header".to_string());
    }
    let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    if version != 2 {
        return Err(format!("Unsupported GLB version {version} (expected 2)"));
    }
    let total_len = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    if total_len > bytes.len() || total_len < 20 {
        return Err("GLB length mismatches payload size".to_string());
    }

    let mut pos = 12usize;
    let mut json_str: Option<String> = None;
    let mut bin_chunk: Option<Vec<u8>> = None;

    while pos + 8 <= total_len {
        let chunk_len =
            u32::from_le_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]])
                as usize;
        let chunk_type =
            u32::from_le_bytes([bytes[pos + 4], bytes[pos + 5], bytes[pos + 6], bytes[pos + 7]]);
        let data_start = pos + 8;
        let data_end = data_start
            .checked_add(chunk_len)
            .ok_or_else(|| "GLB chunk size overflow".to_string())?;
        if data_end > total_len {
            return Err("GLB chunk exceeds container length".to_string());
        }
        let chunk = &bytes[data_start..data_end];
        if chunk_type == GLB_CHUNK_JSON && json_str.is_none() {
            let s = std::str::from_utf8(chunk)
                .map_err(|_| "GLB JSON chunk is not valid UTF-8".to_string())?;
            json_str = Some(s.to_string());
        } else if chunk_type == GLB_CHUNK_BIN && bin_chunk.is_none() {
            bin_chunk = Some(chunk.to_vec());
        }
        pos = data_end;
    }

    let json = json_str.ok_or_else(|| "GLB missing JSON chunk".to_string())?;
    Ok((json, bin_chunk))
}

fn json_arr<'a>(v: &'a JsonVal, ctx: &str) -> Result<&'a [JsonVal], String> {
    match v {
        JsonVal::Arr(arr) => Ok(arr.as_slice()),
        _ => Err(format!("Expected array for {ctx}")),
    }
}

fn opt_usize(obj: &HashMap<String, JsonVal>, key: &str) -> Option<usize> {
    obj.get(key).and_then(|v| match v {
        JsonVal::Num(n) if *n >= 0.0 => Some(*n as usize),
        _ => None,
    })
}

fn opt_str(obj: &HashMap<String, JsonVal>, key: &str) -> Option<String> {
    obj.get(key).and_then(|v| match v {
        JsonVal::Str(s) => Some(s.clone()),
        _ => None,
    })
}

fn decode_data_uri(uri: &str) -> Result<Vec<u8>, String> {
    if let Some(comma) = uri.find(',') {
        let meta = &uri[..comma];
        let payload = &uri[comma + 1..];
        if meta.contains(";base64") {
            return base64_decode(payload);
        }
    }
    Err("Only base64 data URIs are supported without external files".to_string())
}

fn read_accessor_bytes<'a>(
    accessors: &[JsonVal],
    buffer_views: &[JsonVal],
    buffers: &'a [Vec<u8>],
    acc_idx: usize,
) -> Result<(usize, u32, usize, usize, &'a [u8]), String> {
    let acc = as_obj(
        accessors
            .get(acc_idx)
            .ok_or_else(|| format!("Accessor index {acc_idx} out of bounds"))?,
        "accessor",
    )?;
    let bv_idx = opt_usize(acc, "bufferView")
        .ok_or_else(|| format!("Accessor {acc_idx} missing bufferView (sparse accessors unsupported)"))?;
    let bv = as_obj(
        buffer_views
            .get(bv_idx)
            .ok_or_else(|| format!("BufferView {bv_idx} out of bounds"))?,
        "bufferView",
    )?;
    let buf_idx = opt_usize(bv, "buffer").unwrap_or(0);
    let buf = buffers
        .get(buf_idx)
        .ok_or_else(|| format!("Buffer {buf_idx} missing or unresolvable"))?;
    let bv_offset = opt_usize(bv, "byteOffset").unwrap_or(0);
    let bv_len = opt_usize(bv, "byteLength")
        .ok_or_else(|| format!("BufferView {bv_idx} missing byteLength"))?;
    let bv_end = bv_offset
        .checked_add(bv_len)
        .ok_or_else(|| "BufferView offset overflow".to_string())?;
    if bv_end > buf.len() {
        return Err("BufferView extends past end of glTF buffer".to_string());
    }

    let comp_type = opt_usize(acc, "componentType").unwrap_or(0) as u32;
    let count = opt_usize(acc, "count").unwrap_or(0);
    let acc_offset = opt_usize(acc, "byteOffset").unwrap_or(0);
    let stride = opt_usize(bv, "byteStride").unwrap_or(0);

    // SAFETY NOTE: we return the slice of the buffer via unsafe? No! Buffers slice has lifetime bound to local?
    // Actually `&buffers[buf_idx][bv_offset..bv_end]` has lifetime tied to `buffers` parameter `'a`!
    let slice = &buf[bv_offset..bv_end];
    if acc_offset > slice.len() {
        return Err("Accessor byteOffset exceeds bufferView length".to_string());
    }
    Ok((count, comp_type, acc_offset, stride, slice))
}

fn read_f32_le(slice: &[u8], off: usize) -> Result<f32, String> {
    if off + 4 > slice.len() {
        return Err("Unexpected EOF reading f32 attribute".to_string());
    }
    let val = f32::from_le_bytes([slice[off], slice[off + 1], slice[off + 2], slice[off + 3]]);
    if !val.is_finite() {
        return Err("Non-finite float in glTF accessor".to_string());
    }
    Ok(val)
}

fn read_vec3_accessor(
    accessors: &[JsonVal],
    buffer_views: &[JsonVal],
    buffers: &[Vec<u8>],
    acc_idx: usize,
) -> Result<Vec<[f32; 3]>, String> {
    let (count, comp_type, acc_off, bv_stride, slice) =
        read_accessor_bytes(accessors, buffer_views, buffers, acc_idx)?;
    if count > MAX_VERTICES_PER_MESH {
        return Err(format!("glTF mesh vertex count {count} exceeds safety limit"));
    }
    if comp_type != 5126 {
        return Err(format!("Unsupported VEC3 componentType {comp_type} (expected 5126 f32)"));
    }
    let stride = if bv_stride == 0 { 12 } else { bv_stride.max(12) };
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let base = acc_off + i * stride;
        out.push([
            read_f32_le(slice, base)?,
            read_f32_le(slice, base + 4)?,
            read_f32_le(slice, base + 8)?,
        ]);
    }
    Ok(out)
}

fn read_vec2_accessor(
    accessors: &[JsonVal],
    buffer_views: &[JsonVal],
    buffers: &[Vec<u8>],
    acc_idx: usize,
) -> Result<Vec<[f32; 2]>, String> {
    let (count, comp_type, acc_off, bv_stride, slice) =
        read_accessor_bytes(accessors, buffer_views, buffers, acc_idx)?;
    if count > MAX_VERTICES_PER_MESH {
        return Err("glTF UV count exceeds safety limit".to_string());
    }
    let mut out = Vec::with_capacity(count);
    match comp_type {
        5126 => {
            let stride = if bv_stride == 0 { 8 } else { bv_stride.max(8) };
            for i in 0..count {
                let base = acc_off + i * stride;
                out.push([read_f32_le(slice, base)?, read_f32_le(slice, base + 4)?]);
            }
        }
        5123 => {
            let stride = if bv_stride == 0 { 4 } else { bv_stride.max(4) };
            for i in 0..count {
                let base = acc_off + i * stride;
                if base + 4 > slice.len() {
                    return Err("Truncated u16 UV accessor".to_string());
                }
                let u = u16::from_le_bytes([slice[base], slice[base + 1]]) as f32 / 65535.0;
                let v = u16::from_le_bytes([slice[base + 2], slice[base + 3]]) as f32 / 65535.0;
                out.push([u, v]);
            }
        }
        5121 => {
            let stride = if bv_stride == 0 { 2 } else { bv_stride.max(2) };
            for i in 0..count {
                let base = acc_off + i * stride;
                if base + 2 > slice.len() {
                    return Err("Truncated u8 UV accessor".to_string());
                }
                out.push([slice[base] as f32 / 255.0, slice[base + 1] as f32 / 255.0]);
            }
        }
        _ => return Err(format!("Unsupported VEC2 componentType {comp_type}")),
    }
    Ok(out)
}

fn read_indices_accessor(
    accessors: &[JsonVal],
    buffer_views: &[JsonVal],
    buffers: &[Vec<u8>],
    acc_idx: usize,
) -> Result<Vec<u32>, String> {
    let (count, comp_type, acc_off, bv_stride, slice) =
        read_accessor_bytes(accessors, buffer_views, buffers, acc_idx)?;
    if count > MAX_INDICES_PER_MESH {
        return Err(format!("glTF index count {count} exceeds safety limit"));
    }
    let mut out = Vec::with_capacity(count);
    match comp_type {
        5121 => {
            let stride = if bv_stride == 0 { 1 } else { bv_stride };
            for i in 0..count {
                let b = acc_off + i * stride;
                out.push(*slice.get(b).ok_or_else(|| "Truncated u8 indices".to_string())? as u32);
            }
        }
        5123 => {
            let stride = if bv_stride == 0 { 2 } else { bv_stride.max(2) };
            for i in 0..count {
                let b = acc_off + i * stride;
                if b + 2 > slice.len() {
                    return Err("Truncated u16 indices".to_string());
                }
                out.push(u16::from_le_bytes([slice[b], slice[b + 1]]) as u32);
            }
        }
        5125 => {
            let stride = if bv_stride == 0 { 4 } else { bv_stride.max(4) };
            for i in 0..count {
                let b = acc_off + i * stride;
                if b + 4 > slice.len() {
                    return Err("Truncated u32 indices".to_string());
                }
                out.push(u32::from_le_bytes([
                    slice[b],
                    slice[b + 1],
                    slice[b + 2],
                    slice[b + 3],
                ]));
            }
        }
        _ => return Err(format!("Unsupported index componentType {comp_type}")),
    }
    Ok(out)
}

pub fn import_gltf_or_glb(
    state: &mut NovaEngineState,
    bytes: &[u8],
    asset_name: &str,
) -> Result<FormatReport, String> {
    if bytes.is_empty() {
        return Err("glTF/GLB file is empty".to_string());
    }
    if bytes.len() > MAX_IMPORT_BYTES {
        return Err("glTF/GLB file exceeds 64 MiB safety limit".to_string());
    }

    let is_glb = bytes.len() >= 4
        && u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) == GLB_MAGIC;
    let (json_text, bin_chunk) = if is_glb {
        unpack_glb(bytes)?
    } else {
        let s = std::str::from_utf8(bytes)
            .map_err(|_| "glTF file is not valid UTF-8 JSON".to_string())?;
        (s.to_string(), None)
    };

    let mut parser = JsonParser::new(&json_text);
    let root_val = parser.parse_value(0)?;
    let root = as_obj(&root_val, "gltf.root")?;
    let mut report = FormatReport::default();

    // Warnings for unsupported glTF top-level concepts (FM-7)
    if let Some(exts) = root.get("extensionsRequired").and_then(|v| json_arr(v, "ext").ok()) {
        for ext in exts {
            if let Ok(s) = as_str(ext, "ext") {
                if s == "KHR_draco_mesh_compression" {
                    return Err("Draco-compressed glTF meshes (KHR_draco_mesh_compression) require external decompression and are unsupported.".to_string());
                }
                report.push_warning(format!("Required extension '{s}' is not supported and may render inaccurately."));
            }
        }
    }
    if let Some(exts) = root.get("extensionsUsed").and_then(|v| json_arr(v, "ext").ok()) {
        for ext in exts {
            if let Ok(s) = as_str(ext, "ext") {
                report.push_warning(format!("glTF extension '{s}' skipped; imported base PBR properties."));
            }
        }
    }
    if root.get("skins").and_then(|v| json_arr(v, "skins").ok()).is_some_and(|a| !a.is_empty()) {
        report.push_warning("Skeletal skins/bones in glTF are not supported; meshes imported in bind pose.");
    }
    if root.get("animations").and_then(|v| json_arr(v, "animations").ok()).is_some_and(|a| !a.is_empty()) {
        report.push_warning("glTF keyframe animations were ignored during import.");
    }

    // Load buffers
    let buf_defs = root
        .get("buffers")
        .map(|v| json_arr(v, "buffers"))
        .transpose()?
        .unwrap_or(&[]);
    let mut buffers: Vec<Vec<u8>> = Vec::with_capacity(buf_defs.len());
    let mut bin_used = false;
    for (i, bval) in buf_defs.iter().enumerate() {
        let bobj = as_obj(bval, "buffer")?;
        if let Some(uri) = opt_str(bobj, "uri") {
            if uri.starts_with("data:") {
                buffers.push(decode_data_uri(&uri)?);
            } else {
                return Err(format!(
                    "External glTF buffer '{uri}' cannot be resolved from a single-file import; please export as self-contained .glb."
                ));
            }
        } else if !bin_used {
            if let Some(bin) = &bin_chunk {
                buffers.push(bin.clone());
                bin_used = true;
            } else {
                return Err(format!("glTF buffer[{i}] has no URI and no GLB BIN chunk"));
            }
        } else {
            return Err(format!("glTF buffer[{i}] missing data URI"));
        }
    }

    let buffer_views = root
        .get("bufferViews")
        .map(|v| json_arr(v, "bufferViews"))
        .transpose()?
        .unwrap_or(&[]);
    let accessors = root
        .get("accessors")
        .map(|v| json_arr(v, "accessors"))
        .transpose()?
        .unwrap_or(&[]);

    // Load embedded PNG textures
    let image_defs = root
        .get("images")
        .map(|v| json_arr(v, "images"))
        .transpose()?
        .unwrap_or(&[]);
    let mut imported_tex_ids: Vec<Option<TextureId>> = Vec::with_capacity(image_defs.len());
    for (idx, ival) in image_defs.iter().enumerate() {
        let iobj = as_obj(ival, "image")?;
        let img_name = opt_str(iobj, "name").unwrap_or_else(|| format!("tex_{idx}"));
        let raw_bytes = if let Some(bv_idx) = opt_usize(iobj, "bufferView") {
            let bv = as_obj(
                buffer_views
                    .get(bv_idx)
                    .ok_or_else(|| format!("Image bufferView {bv_idx} out of bounds"))?,
                "image.bufferView",
            )?;
            let buf_idx = opt_usize(bv, "buffer").unwrap_or(0);
            let off = opt_usize(bv, "byteOffset").unwrap_or(0);
            let len = opt_usize(bv, "byteLength").unwrap_or(0);
            let buf = buffers
                .get(buf_idx)
                .ok_or_else(|| "Image buffer missing".to_string())?;
            if off + len > buf.len() {
                return Err("Image bufferView out of bounds".to_string());
            }
            Some(buf[off..off + len].to_vec())
        } else if let Some(uri) = opt_str(iobj, "uri") {
            if uri.starts_with("data:") {
                decode_data_uri(&uri).ok()
            } else {
                report.push_warning(format!(
                    "External image URI '{uri}' omitted during single-file import."
                ));
                None
            }
        } else {
            None
        };

        if let Some(img_bytes) = raw_bytes {
            match decode_png_rgba8(&img_bytes) {
                Ok((w, h, rgba)) => {
                    let tid = state.next_texture_id;
                    state.next_texture_id += 1;
                    state.textures.push(TextureAsset {
                        id: tid,
                        name: sanitize_name(&img_name, "Imported Texture"),
                        source_uri: format!("gltf://image/{idx}"),
                        width: w,
                        height: h,
                        rgba,
                        status: TextureStatus::Loaded,
                        version: 1,
                    });
                    report.imported_textures += 1;
                    imported_tex_ids.push(Some(tid));
                }
                Err(_) => {
                    report.push_warning(format!(
                        "Texture '{img_name}' uses an unsupported image encoding (e.g. JPEG/KTX2); material falls back to scalar colors."
                    ));
                    imported_tex_ids.push(None);
                }
            }
        } else {
            imported_tex_ids.push(None);
        }
    }

    let tex_defs = root
        .get("textures")
        .map(|v| json_arr(v, "textures"))
        .transpose()?
        .unwrap_or(&[]);
    let resolve_tex = |tex_idx: usize| -> Option<TextureId> {
        let tobj = tex_defs.get(tex_idx).and_then(|v| as_obj(v, "texture").ok())?;
        let src = opt_usize(tobj, "source")?;
        imported_tex_ids.get(src).copied().flatten()
    };

    // Load PBR Materials
    let mat_defs = root
        .get("materials")
        .map(|v| json_arr(v, "materials"))
        .transpose()?
        .unwrap_or(&[]);
    let mut imported_mats: Vec<(MaterialId, MaterialParams)> = Vec::with_capacity(mat_defs.len());
    for (idx, mval) in mat_defs.iter().enumerate() {
        let mobj = as_obj(mval, "material")?;
        let mat_name = state.unique_material_name(&sanitize_name(
            &opt_str(mobj, "name").unwrap_or_else(|| format!("glTF Mat {}", idx + 1)),
            "glTF Material",
        ));
        let mid = state.next_material_id;
        state.next_material_id += 1;
        let mut spec = MaterialSpec::studio_default(mid, mat_name);

        if let Some(pbr) = mobj.get("pbrMetallicRoughness").and_then(|v| as_obj(v, "pbr").ok()) {
            if let Some(bcf) = pbr.get("baseColorFactor").and_then(|v| json_arr(v, "bcf").ok()) {
                if bcf.len() == 4 {
                    spec.base_color = [
                        as_f32(&bcf[0], "bcf[0]")?,
                        as_f32(&bcf[1], "bcf[1]")?,
                        as_f32(&bcf[2], "bcf[2]")?,
                        as_f32(&bcf[3], "bcf[3]")?,
                    ];
                }
            }
            if let Some(v) = pbr.get("metallicFactor") {
                spec.metallic = as_f32(v, "metallicFactor")?.clamp(0.0, 1.0);
            } else {
                spec.metallic = 1.0; // glTF 2.0 spec default when omitted
            }
            if let Some(v) = pbr.get("roughnessFactor") {
                spec.roughness = as_f32(v, "roughnessFactor")?.clamp(0.04, 1.0);
            } else {
                spec.roughness = 1.0;
            }
            if let Some(bct) = pbr.get("baseColorTexture").and_then(|v| as_obj(v, "bct").ok()) {
                if let Some(ti) = opt_usize(bct, "index") {
                    spec.albedo_tex_id = resolve_tex(ti);
                }
            }
            if let Some(mrt) = pbr
                .get("metallicRoughnessTexture")
                .and_then(|v| as_obj(v, "mrt").ok())
            {
                if let Some(ti) = opt_usize(mrt, "index") {
                    spec.roughness_tex_id = resolve_tex(ti);
                }
            }
        }
        if let Some(nt) = mobj.get("normalTexture").and_then(|v| as_obj(v, "nt").ok()) {
            if let Some(ti) = opt_usize(nt, "index") {
                spec.normal_tex_id = resolve_tex(ti);
            }
        }
        if let Some(ot) = mobj.get("occlusionTexture").and_then(|v| as_obj(v, "ot").ok()) {
            if let Some(ti) = opt_usize(ot, "index") {
                spec.ao_tex_id = resolve_tex(ti);
            }
        }
        if let Some(em) = mobj.get("emissiveFactor").and_then(|v| json_arr(v, "em").ok()) {
            if em.len() == 3 {
                spec.emissive = [
                    as_f32(&em[0], "em[0]")?,
                    as_f32(&em[1], "em[1]")?,
                    as_f32(&em[2], "em[2]")?,
                ];
                if spec.emissive.iter().any(|&c| c > 0.0) {
                    spec.emissive_intensity = 1.0;
                }
            }
        }
        if let Some(mode) = opt_str(mobj, "alphaMode") {
            match mode.as_str() {
                "BLEND" => spec.alpha_mode = AlphaMode::Blend,
                "MASK" => {
                    spec.alpha_mode = AlphaMode::Blend;
                    report.push_warning("Material alphaMode 'MASK' mapped to alpha 'BLEND'.");
                }
                _ => {}
            }
        }

        let params = MaterialParams {
            base_color: spec.base_color,
            metallic: spec.metallic,
            roughness: spec.roughness,
        };
        state.materials.push(spec);
        imported_mats.push((mid, params));
        report.imported_materials += 1;
    }

    let fallback_mat_id = state.materials.first().map(|m| m.id);
    let fallback_params = state
        .materials
        .first()
        .map(|m| MaterialParams {
            base_color: m.base_color,
            metallic: m.metallic,
            roughness: m.roughness,
        })
        .unwrap_or_default();

    // Parse glTF primitives per mesh
    struct ParsedPrim {
        mesh: MeshData,
        material_id: Option<MaterialId>,
        material: MaterialParams,
    }
    let mesh_defs = root
        .get("meshes")
        .map(|v| json_arr(v, "meshes"))
        .transpose()?
        .unwrap_or(&[]);
    let mut parsed_meshes: Vec<Vec<ParsedPrim>> = Vec::with_capacity(mesh_defs.len());

    for mval in mesh_defs {
        let mobj = as_obj(mval, "mesh")?;
        let prims = json_arr(
            mobj.get("primitives")
                .ok_or_else(|| "glTF mesh missing primitives".to_string())?,
            "mesh.primitives",
        )?;
        let mut out_prims = Vec::new();
        for pval in prims {
            let pobj = as_obj(pval, "primitive")?;
            let mode = opt_usize(pobj, "mode").unwrap_or(4);
            if mode != 4 {
                report.push_warning(format!(
                    "Non-triangle glTF primitive mode {mode} was skipped."
                ));
                continue;
            }
            if pobj
                .get("targets")
                .and_then(|v| json_arr(v, "targets").ok())
                .is_some_and(|t| !t.is_empty())
            {
                report.push_warning("glTF morph targets (blend shapes) are unsupported and were ignored.");
            }
            let attrs = as_obj(
                pobj.get("attributes")
                    .ok_or_else(|| "glTF primitive missing attributes".to_string())?,
                "primitive.attributes",
            )?;
            let pos_acc = opt_usize(attrs, "POSITION")
                .ok_or_else(|| "glTF primitive missing POSITION attribute".to_string())?;
            let positions = read_vec3_accessor(accessors, buffer_views, &buffers, pos_acc)?;
            let indices = if let Some(idx_acc) = opt_usize(pobj, "indices") {
                read_indices_accessor(accessors, buffer_views, &buffers, idx_acc)?
            } else {
                let tri_count = (positions.len() / 3) * 3;
                (0..tri_count as u32).collect()
            };
            let normals = if let Some(norm_acc) = opt_usize(attrs, "NORMAL") {
                let n = read_vec3_accessor(accessors, buffer_views, &buffers, norm_acc)?;
                if n.len() == positions.len() {
                    n
                } else {
                    compute_missing_normals(&positions, &indices)
                }
            } else {
                compute_missing_normals(&positions, &indices)
            };
            let uvs = if let Some(uv_acc) = opt_usize(attrs, "TEXCOORD_0") {
                let u = read_vec2_accessor(accessors, buffer_views, &buffers, uv_acc)?;
                if u.len() == positions.len() {
                    u
                } else {
                    vec![[0.0, 0.0]; positions.len()]
                }
            } else {
                vec![[0.0, 0.0]; positions.len()]
            };

            ensure_valid_mesh_bounds(&positions, &indices)?;
            let (bounds_min, bounds_max) = MeshData::compute_bounds(&positions);
            let mesh = MeshData {
                positions,
                normals,
                uvs,
                indices,
                bounds_min,
                bounds_max,
                version: 1,
            };

            let (material_id, material) = if let Some(mi) = opt_usize(pobj, "material") {
                if let Some((mid, mp)) = imported_mats.get(mi) {
                    (Some(*mid), mp.clone())
                } else {
                    (fallback_mat_id, fallback_params.clone())
                }
            } else {
                (fallback_mat_id, fallback_params.clone())
            };

            out_prims.push(ParsedPrim {
                mesh,
                material_id,
                material,
            });
        }
        parsed_meshes.push(out_prims);
    }

    // Parse glTF cameras
    let cam_defs = root
        .get("cameras")
        .map(|v| json_arr(v, "cameras"))
        .transpose()?
        .unwrap_or(&[]);

    // Parse glTF nodes & build hierarchy
    let node_defs = root
        .get("nodes")
        .map(|v| json_arr(v, "nodes"))
        .transpose()?
        .unwrap_or(&[]);
    if node_defs.len() > MAX_NODES_PER_IMPORT {
        return Err("glTF node count exceeds safety limit".to_string());
    }

    let mut gltf_parent: Vec<Option<usize>> = vec![None; node_defs.len()];
    for (idx, nval) in node_defs.iter().enumerate() {
        let nobj = as_obj(nval, "node")?;
        if let Some(children) = nobj.get("children").and_then(|v| json_arr(v, "children").ok()) {
            for cval in children {
                let cidx = as_u64(cval, "child")? as usize;
                if cidx >= node_defs.len() || cidx == idx {
                    return Err("Invalid glTF node child index".to_string());
                }
                if gltf_parent[cidx].is_some() {
                    return Err("Cycle or duplicate parent in glTF hierarchy".to_string());
                }
                gltf_parent[cidx] = Some(idx);
            }
        }
    }

    let mut assigned_ids: Vec<NodeId> = Vec::with_capacity(node_defs.len());
    for _ in 0..node_defs.len() {
        assigned_ids.push(state.allocate_id());
    }

    let mut new_nodes: Vec<SceneNode> = Vec::new();
    let default_root_name = sanitize_name(asset_name, "glTF Asset");

    for (idx, nval) in node_defs.iter().enumerate() {
        let nobj = as_obj(nval, "node")?;
        let node_id = assigned_ids[idx];
        let parent_id = gltf_parent[idx].map(|pi| assigned_ids[pi]);
        let raw_name = opt_str(nobj, "name").unwrap_or_else(|| format!("{default_root_name} {}", idx + 1));
        let node_name = state.unique_name(&sanitize_name(&raw_name, &default_root_name));

        let transform = if let Some(mat_arr) = nobj.get("matrix").and_then(|v| json_arr(v, "matrix").ok()) {
            if mat_arr.len() == 16 {
                let mut m = [0.0f32; 16];
                for k in 0..16 {
                    m[k] = as_f32(&mat_arr[k], "matrix")?;
                }
                let (position, rotation_euler, scale) = Mat4 { data: m }.decompose_trs();
                Transform {
                    position,
                    rotation_euler,
                    scale,
                }
            } else {
                Transform::default()
            }
        } else {
            let t = if let Some(arr) = nobj.get("translation").and_then(|v| json_arr(v, "t").ok()) {
                if arr.len() == 3 {
                    Vec3::new(as_f32(&arr[0], "tx")?, as_f32(&arr[1], "ty")?, as_f32(&arr[2], "tz")?)
                } else {
                    Vec3::ZERO
                }
            } else {
                Vec3::ZERO
            };
            let s = if let Some(arr) = nobj.get("scale").and_then(|v| json_arr(v, "s").ok()) {
                if arr.len() == 3 {
                    Vec3::new(as_f32(&arr[0], "sx")?, as_f32(&arr[1], "sy")?, as_f32(&arr[2], "sz")?)
                } else {
                    Vec3::ONE
                }
            } else {
                Vec3::ONE
            };
            let r_euler = if let Some(arr) = nobj.get("rotation").and_then(|v| json_arr(v, "r").ok()) {
                if arr.len() == 4 {
                    let q = Quat {
                        x: as_f32(&arr[0], "rx")?,
                        y: as_f32(&arr[1], "ry")?,
                        z: as_f32(&arr[2], "rz")?,
                        w: as_f32(&arr[3], "rw")?,
                    }
                    .normalized();
                    Mat4::from_trs(Vec3::ZERO, q, Vec3::ONE).decompose_trs().1
                } else {
                    Vec3::ZERO
                }
            } else {
                Vec3::ZERO
            };
            Transform {
                position: t,
                rotation_euler: r_euler,
                scale: s,
            }
        };

        if let Some(mesh_idx) = opt_usize(nobj, "mesh") {
            let prims = parsed_meshes
                .get(mesh_idx)
                .ok_or_else(|| format!("glTF node mesh index {mesh_idx} out of bounds"))?;
            if prims.len() == 1 {
                let p = &prims[0];
                let base_poly = Some(PolyMesh::from_mesh_data_fallback(&p.mesh, false));
                new_nodes.push(SceneNode {
                    id: node_id,
                    parent_id,
                    name: node_name,
                    kind: NodeKind::Mesh,
                    transform,
                    mesh: Some(p.mesh.clone()),
                    base_poly,
                    modifiers: Vec::new(),
                    material_id: p.material_id,
                    material: p.material.clone(),
                    light: LightParams::default(),
                    camera_node: CameraNodeParams::default(),
                    visible: true,
                });
            } else if !prims.is_empty() {
                new_nodes.push(SceneNode {
                    id: node_id,
                    parent_id,
                    name: node_name.clone(),
                    kind: NodeKind::Empty,
                    transform,
                    mesh: None,
                    base_poly: None,
                    modifiers: Vec::new(),
                    material_id: None,
                    material: fallback_params.clone(),
                    light: LightParams::default(),
                    camera_node: CameraNodeParams::default(),
                    visible: true,
                });
                for (pi, p) in prims.iter().enumerate() {
                    let sub_id = state.allocate_id();
                    let base_poly = Some(PolyMesh::from_mesh_data_fallback(&p.mesh, false));
                    new_nodes.push(SceneNode {
                        id: sub_id,
                        parent_id: Some(node_id),
                        name: state.unique_name(&format!("{node_name} Part {}", pi + 1)),
                        kind: NodeKind::Mesh,
                        transform: Transform::default(),
                        mesh: Some(p.mesh.clone()),
                        base_poly,
                        modifiers: Vec::new(),
                        material_id: p.material_id,
                        material: p.material.clone(),
                        light: LightParams::default(),
                        camera_node: CameraNodeParams::default(),
                        visible: true,
                    });
                }
            }
        } else if let Some(cam_idx) = opt_usize(nobj, "camera") {
            let mut cam_params = CameraNodeParams::default();
            if let Some(cval) = cam_defs.get(cam_idx) {
                if let Ok(cobj) = as_obj(cval, "camera") {
                    if let Some(pobj) = cobj.get("perspective").and_then(|v| as_obj(v, "persp").ok()) {
                        if let Some(yfov) = pobj.get("yfov").and_then(|v| as_f32(v, "yfov").ok()) {
                            cam_params.fov_y_degrees = yfov.to_degrees().clamp(10.0, 120.0);
                        }
                        if let Some(znear) = pobj.get("znear").and_then(|v| as_f32(v, "znear").ok()) {
                            cam_params.near_plane = znear.max(0.005);
                        }
                        if let Some(zfar) = pobj.get("zfar").and_then(|v| as_f32(v, "zfar").ok()) {
                            cam_params.far_plane = zfar.max(cam_params.near_plane + 1.0);
                        }
                    } else if opt_str(cobj, "type").as_deref() == Some("orthographic") {
                        report.push_warning("Orthographic glTF camera imported as perspective camera node.");
                    }
                }
            }
            new_nodes.push(SceneNode {
                id: node_id,
                parent_id,
                name: node_name,
                kind: NodeKind::Camera,
                transform,
                mesh: None,
                base_poly: None,
                modifiers: Vec::new(),
                material_id: None,
                material: fallback_params.clone(),
                light: LightParams::default(),
                camera_node: cam_params,
                visible: true,
            });
        } else {
            new_nodes.push(SceneNode {
                id: node_id,
                parent_id,
                name: node_name,
                kind: NodeKind::Empty,
                transform,
                mesh: None,
                base_poly: None,
                modifiers: Vec::new(),
                material_id: None,
                material: fallback_params.clone(),
                light: LightParams::default(),
                camera_node: CameraNodeParams::default(),
                visible: true,
            });
        }
    }

    if new_nodes.is_empty() {
        return Err("glTF asset contained no importable nodes or meshes".to_string());
    }

    report.imported_nodes = new_nodes.len();
    commit_imported_nodes(state, new_nodes);
    Ok(report)
}

struct GltfBufferViewDesc {
    byte_offset: usize,
    byte_length: usize,
    target: Option<u32>,
}

struct GltfAccessorDesc {
    buffer_view: usize,
    component_type: u32,
    count: usize,
    acc_type: &'static str,
    min: Option<[f32; 3]>,
    max: Option<[f32; 3]>,
}

fn align4(buf: &mut Vec<u8>) {
    while buf.len() % 4 != 0 {
        buf.push(0);
    }
}

pub fn export_gltf(state: &NovaEngineState, as_glb: bool) -> Result<(Vec<u8>, FormatReport), String> {
    let mut report = FormatReport::default();

    let visible_nodes: Vec<&SceneNode> = state.nodes.iter().filter(|n| n.visible).collect();
    if visible_nodes.is_empty() {
        return Err("Scene has no visible nodes to export".to_string());
    }

    if state.nodes.iter().any(|n| n.kind == NodeKind::Light) {
        report.push_warning(
            "Light nodes are omitted in core glTF 2.0 export (KHR_lights_punctual not embedded).",
        );
    }
    if !state.animation.tracks.is_empty() {
        report.push_warning(
            "Animation tracks are not exported to glTF/GLB in v1.0; static pose at the current frame exported.",
        );
    }

    let mut bin: Vec<u8> = Vec::with_capacity(65536);
    let mut bvs: Vec<GltfBufferViewDesc> = Vec::new();
    let mut accs: Vec<GltfAccessorDesc> = Vec::new();

    // Map Nova MaterialId -> glTF material index
    let mut mat_map: HashMap<MaterialId, usize> = HashMap::new();
    let mut gltf_materials: Vec<MaterialSpec> = Vec::new();
    // Map TextureId -> (gltf image index, bufferView index)
    let mut tex_map: BTreeMap<TextureId, usize> = BTreeMap::new();
    let mut image_bvs: Vec<(String, usize)> = Vec::new();

    let mut ensure_texture = |tid: TextureId, texs: &[TextureAsset], bin: &mut Vec<u8>, bvs: &mut Vec<GltfBufferViewDesc>, rep: &mut FormatReport| -> Option<usize> {
        if let Some(&idx) = tex_map.get(&tid) {
            return Some(idx);
        }
        let tex = texs.iter().find(|t| t.id == tid)?;
        if tex.status == TextureStatus::Missing {
            rep.push_warning(format!("Missing texture '{}' was omitted from glTF export.", tex.name));
            return None;
        }
        let (w, h) = tex.effective_dimensions();
        let rgba = tex.effective_rgba();
        let Ok(png) = encode_png_rgba8(w, h, &rgba) else {
            return None;
        };
        align4(bin);
        let off = bin.len();
        bin.extend_from_slice(&png);
        let bv_idx = bvs.len();
        bvs.push(GltfBufferViewDesc {
            byte_offset: off,
            byte_length: png.len(),
            target: None,
        });
        let img_idx = image_bvs.len();
        image_bvs.push((tex.name.clone(), bv_idx));
        tex_map.insert(tid, img_idx);
        Some(img_idx)
    };

    for node in &visible_nodes {
        if node.kind == NodeKind::Mesh {
            if let Some(mid) = node.material_id {
                if !mat_map.contains_key(&mid) {
                    if let Some(spec) = state.materials.iter().find(|m| m.id == mid) {
                        let gi = gltf_materials.len();
                        mat_map.insert(mid, gi);
                        gltf_materials.push(spec.clone());
                        if let Some(t) = spec.albedo_tex_id {
                            ensure_texture(t, &state.textures, &mut bin, &mut bvs, &mut report);
                        }
                        if let Some(t) = spec.normal_tex_id {
                            ensure_texture(t, &state.textures, &mut bin, &mut bvs, &mut report);
                        }
                        if let Some(t) = spec.roughness_tex_id {
                            ensure_texture(t, &state.textures, &mut bin, &mut bvs, &mut report);
                        }
                        if let Some(t) = spec.ao_tex_id {
                            ensure_texture(t, &state.textures, &mut bin, &mut bvs, &mut report);
                        }
                    }
                }
            }
        }
    }

    // Export visible hierarchy: skip Light nodes while preserving parent chain when valid
    let mut node_index_by_id: HashMap<NodeId, usize> = HashMap::new();
    let export_nodes: Vec<&SceneNode> = visible_nodes
        .iter()
        .copied()
        .filter(|n| n.kind != NodeKind::Light)
        .collect();
    if export_nodes.is_empty() {
        return Err("No mesh, camera, or empty nodes visible to export".to_string());
    }
    for (i, n) in export_nodes.iter().enumerate() {
        node_index_by_id.insert(n.id, i);
    }

    let mut children_map: Vec<Vec<usize>> = vec![Vec::new(); export_nodes.len()];
    let mut root_indices: Vec<usize> = Vec::new();
    for (i, n) in export_nodes.iter().enumerate() {
        if let Some(pid) = n.parent_id {
            if let Some(&pi) = node_index_by_id.get(&pid) {
                children_map[pi].push(i);
                continue;
            }
        }
        root_indices.push(i);
    }

    struct GltfMeshOut {
        name: String,
        pos_acc: usize,
        norm_acc: usize,
        uv_acc: usize,
        idx_acc: usize,
        mat_idx: Option<usize>,
    }
    let mut gltf_meshes: Vec<GltfMeshOut> = Vec::new();
    let mut node_mesh_idx: Vec<Option<usize>> = vec![None; export_nodes.len()];
    let mut gltf_cameras: Vec<CameraNodeParams> = Vec::new();
    let mut node_cam_idx: Vec<Option<usize>> = vec![None; export_nodes.len()];

    for (ni, n) in export_nodes.iter().enumerate() {
        if !n.modifiers.is_empty() {
            report.push_warning(format!(
                "Node '{}' modifier stack exported as evaluated mesh geometry.",
                n.name
            ));
        }
        match n.kind {
            NodeKind::Mesh => {
                let Some(mesh) = &n.mesh else {
                    continue;
                };
                if mesh.positions.is_empty() || mesh.indices.is_empty() {
                    continue;
                }
                align4(&mut bin);
                let pos_off = bin.len();
                for p in &mesh.positions {
                    bin.extend_from_slice(&p[0].to_le_bytes());
                    bin.extend_from_slice(&p[1].to_le_bytes());
                    bin.extend_from_slice(&p[2].to_le_bytes());
                }
                let pos_bv = bvs.len();
                bvs.push(GltfBufferViewDesc {
                    byte_offset: pos_off,
                    byte_length: mesh.positions.len() * 12,
                    target: Some(34962),
                });
                let pos_acc = accs.len();
                accs.push(GltfAccessorDesc {
                    buffer_view: pos_bv,
                    component_type: 5126,
                    count: mesh.positions.len(),
                    acc_type: "VEC3",
                    min: Some(mesh.bounds_min.to_array()),
                    max: Some(mesh.bounds_max.to_array()),
                });

                align4(&mut bin);
                let norm_off = bin.len();
                let normals = if mesh.normals.len() == mesh.positions.len() {
                    &mesh.normals
                } else {
                    &vec![[0.0, 1.0, 0.0]; mesh.positions.len()]
                };
                for nm in normals {
                    bin.extend_from_slice(&nm[0].to_le_bytes());
                    bin.extend_from_slice(&nm[1].to_le_bytes());
                    bin.extend_from_slice(&nm[2].to_le_bytes());
                }
                let norm_bv = bvs.len();
                bvs.push(GltfBufferViewDesc {
                    byte_offset: norm_off,
                    byte_length: normals.len() * 12,
                    target: Some(34962),
                });
                let norm_acc = accs.len();
                accs.push(GltfAccessorDesc {
                    buffer_view: norm_bv,
                    component_type: 5126,
                    count: normals.len(),
                    acc_type: "VEC3",
                    min: None,
                    max: None,
                });

                align4(&mut bin);
                let uv_off = bin.len();
                let uvs = if mesh.uvs.len() == mesh.positions.len() {
                    &mesh.uvs
                } else {
                    &vec![[0.0, 0.0]; mesh.positions.len()]
                };
                for uv in uvs {
                    bin.extend_from_slice(&uv[0].to_le_bytes());
                    bin.extend_from_slice(&uv[1].to_le_bytes());
                }
                let uv_bv = bvs.len();
                bvs.push(GltfBufferViewDesc {
                    byte_offset: uv_off,
                    byte_length: uvs.len() * 8,
                    target: Some(34962),
                });
                let uv_acc = accs.len();
                accs.push(GltfAccessorDesc {
                    buffer_view: uv_bv,
                    component_type: 5126,
                    count: uvs.len(),
                    acc_type: "VEC2",
                    min: None,
                    max: None,
                });

                align4(&mut bin);
                let idx_off = bin.len();
                for &ix in &mesh.indices {
                    bin.extend_from_slice(&ix.to_le_bytes());
                }
                let idx_bv = bvs.len();
                bvs.push(GltfBufferViewDesc {
                    byte_offset: idx_off,
                    byte_length: mesh.indices.len() * 4,
                    target: Some(34963),
                });
                let idx_acc = accs.len();
                accs.push(GltfAccessorDesc {
                    buffer_view: idx_bv,
                    component_type: 5125,
                    count: mesh.indices.len(),
                    acc_type: "SCALAR",
                    min: None,
                    max: None,
                });

                let mat_idx = n
                    .material_id
                    .and_then(|mid| mat_map.get(&mid).copied())
                    .or_else(|| {
                        // Create an inline glTF material from node's MaterialParams if not linked
                        let gi = gltf_materials.len();
                        let mut m = MaterialSpec::studio_default(0, format!("{}_Mat", n.name));
                        m.base_color = n.material.base_color;
                        m.metallic = n.material.metallic;
                        m.roughness = n.material.roughness;
                        gltf_materials.push(m);
                        Some(gi)
                    });

                let mi = gltf_meshes.len();
                gltf_meshes.push(GltfMeshOut {
                    name: n.name.clone(),
                    pos_acc,
                    norm_acc,
                    uv_acc,
                    idx_acc,
                    mat_idx,
                });
                node_mesh_idx[ni] = Some(mi);
            }
            NodeKind::Camera => {
                let ci = gltf_cameras.len();
                gltf_cameras.push(n.camera_node.clone());
                node_cam_idx[ni] = Some(ci);
            }
            NodeKind::Empty | NodeKind::Light => {}
        }
    }

    if as_glb {
        align4(&mut bin);
    }

    let mut json = String::with_capacity(8192);
    json.push_str("{\n");
    json.push_str("  \"asset\": {\"version\": \"2.0\", \"generator\": \"NOVA 3D Studio 1.0\"},\n");
    json.push_str("  \"scene\": 0,\n");
    json.push_str("  \"scenes\": [{\"name\": \"Scene\", \"nodes\": [");
    for (i, &ri) in root_indices.iter().enumerate() {
        if i > 0 {
            json.push(',');
        }
        let _ = write!(json, "{ri}");
    }
    json.push_str("]}],\n");

    json.push_str("  \"nodes\": [\n");
    for (i, n) in export_nodes.iter().enumerate() {
        let q = Quat::from_euler_xyz(
            n.transform.rotation_euler.x,
            n.transform.rotation_euler.y,
            n.transform.rotation_euler.z,
        );
        json.push_str("    {\"name\": ");
        escape_json_str(&n.name, &mut json);
        json.push_str(", \"translation\": ");
        push_f32_json_array(&n.transform.position.to_array(), &mut json);
        json.push_str(", \"rotation\": ");
        push_f32_json_array(&[q.x, q.y, q.z, q.w], &mut json);
        json.push_str(", \"scale\": ");
        push_f32_json_array(&n.transform.scale.to_array(), &mut json);
        if !children_map[i].is_empty() {
            json.push_str(", \"children\": [");
            for (ci, &cidx) in children_map[i].iter().enumerate() {
                if ci > 0 {
                    json.push(',');
                }
                let _ = write!(json, "{cidx}");
            }
            json.push(']');
        }
        if let Some(mi) = node_mesh_idx[i] {
            let _ = write!(json, ", \"mesh\": {mi}");
        }
        if let Some(ci) = node_cam_idx[i] {
            let _ = write!(json, ", \"camera\": {ci}");
        }
        if i + 1 < export_nodes.len() {
            json.push_str("},\n");
        } else {
            json.push_str("}\n");
        }
    }
    json.push_str("  ],\n");

    json.push_str("  \"meshes\": [\n");
    for (i, m) in gltf_meshes.iter().enumerate() {
        json.push_str("    {\"name\": ");
        escape_json_str(&m.name, &mut json);
        let _ = write!(
            json,
            ", \"primitives\": [{{\"attributes\": {{\"POSITION\": {}, \"NORMAL\": {}, \"TEXCOORD_0\": {}}}, \"indices\": {}",
            m.pos_acc, m.norm_acc, m.uv_acc, m.idx_acc
        );
        if let Some(mat_i) = m.mat_idx {
            let _ = write!(json, ", \"material\": {mat_i}");
        }
        json.push_str("}]}");
        if i + 1 < gltf_meshes.len() {
            json.push_str(",\n");
        } else {
            json.push('\n');
        }
    }
    json.push_str("  ],\n");

    if !gltf_cameras.is_empty() {
        json.push_str("  \"cameras\": [\n");
        for (i, c) in gltf_cameras.iter().enumerate() {
            let yfov = c.fov_y_degrees.to_radians();
            json.push_str("    {\"type\": \"perspective\", \"perspective\": {\"yfov\": ");
            push_f32(yfov, &mut json);
            json.push_str(", \"znear\": ");
            push_f32(c.near_plane, &mut json);
            json.push_str(", \"zfar\": ");
            push_f32(c.far_plane, &mut json);
            json.push_str("}}");
            if i + 1 < gltf_cameras.len() {
                json.push_str(",\n");
            } else {
                json.push('\n');
            }
        }
        json.push_str("  ],\n");
    }

    if !gltf_materials.is_empty() {
        json.push_str("  \"materials\": [\n");
        for (i, m) in gltf_materials.iter().enumerate() {
            json.push_str("    {\"name\": ");
            escape_json_str(&m.name, &mut json);
            json.push_str(", \"pbrMetallicRoughness\": {\"baseColorFactor\": ");
            push_f32_json_array(&m.base_color, &mut json);
            json.push_str(", \"metallicFactor\": ");
            push_f32(m.metallic, &mut json);
            json.push_str(", \"roughnessFactor\": ");
            push_f32(m.roughness, &mut json);
            if let Some(tid) = m.albedo_tex_id.and_then(|t| tex_map.get(&t).copied()) {
                let _ = write!(json, ", \"baseColorTexture\": {{\"index\": {tid}}}");
            }
            if let Some(tid) = m.roughness_tex_id.and_then(|t| tex_map.get(&t).copied()) {
                let _ = write!(json, ", \"metallicRoughnessTexture\": {{\"index\": {tid}}}");
            }
            json.push('}');
            if let Some(tid) = m.normal_tex_id.and_then(|t| tex_map.get(&t).copied()) {
                let _ = write!(json, ", \"normalTexture\": {{\"index\": {tid}}}");
            }
            if let Some(tid) = m.ao_tex_id.and_then(|t| tex_map.get(&t).copied()) {
                let _ = write!(json, ", \"occlusionTexture\": {{\"index\": {tid}}}");
            }
            if m.emissive_intensity > 0.0 && m.emissive.iter().any(|&v| v > 0.0) {
                json.push_str(", \"emissiveFactor\": ");
                push_f32_json_array(&m.emissive, &mut json);
            }
            if m.alpha_mode == AlphaMode::Blend {
                json.push_str(", \"alphaMode\": \"BLEND\"");
            }
            if i + 1 < gltf_materials.len() {
                json.push_str("},\n");
            } else {
                json.push_str("}\n");
            }
        }
        json.push_str("  ],\n");
    }

    if !image_bvs.is_empty() {
        json.push_str("  \"textures\": [\n");
        for i in 0..image_bvs.len() {
            let _ = write!(json, "    {{\"source\": {i}}}");
            if i + 1 < image_bvs.len() {
                json.push_str(",\n");
            } else {
                json.push('\n');
            }
        }
        json.push_str("  ],\n");
        json.push_str("  \"images\": [\n");
        for (i, (name, bv_idx)) in image_bvs.iter().enumerate() {
            json.push_str("    {\"name\": ");
            escape_json_str(name, &mut json);
            let _ = write!(json, ", \"bufferView\": {bv_idx}, \"mimeType\": \"image/png\"}}");
            if i + 1 < image_bvs.len() {
                json.push_str(",\n");
            } else {
                json.push('\n');
            }
        }
        json.push_str("  ],\n");
    }

    json.push_str("  \"accessors\": [\n");
    for (i, a) in accs.iter().enumerate() {
        let _ = write!(
            json,
            "    {{\"bufferView\": {}, \"componentType\": {}, \"count\": {}, \"type\": \"{}\"",
            a.buffer_view, a.component_type, a.count, a.acc_type
        );
        if let Some(mn) = &a.min {
            json.push_str(", \"min\": ");
            push_f32_json_array(mn, &mut json);
        }
        if let Some(mx) = &a.max {
            json.push_str(", \"max\": ");
            push_f32_json_array(mx, &mut json);
        }
        if i + 1 < accs.len() {
            json.push_str("},\n");
        } else {
            json.push_str("}\n");
        }
    }
    json.push_str("  ],\n");

    json.push_str("  \"bufferViews\": [\n");
    for (i, bv) in bvs.iter().enumerate() {
        let _ = write!(
            json,
            "    {{\"buffer\": 0, \"byteOffset\": {}, \"byteLength\": {}",
            bv.byte_offset, bv.byte_length
        );
        if let Some(t) = bv.target {
            let _ = write!(json, ", \"target\": {t}");
        }
        if i + 1 < bvs.len() {
            json.push_str("},\n");
        } else {
            json.push_str("}\n");
        }
    }
    json.push_str("  ],\n");

    if as_glb {
        let _ = writeln!(json, "  \"buffers\": [{{\"byteLength\": {}}}]", bin.len());
        json.push_str("}\n");

        let mut json_bytes = json.into_bytes();
        while json_bytes.len() % 4 != 0 {
            json_bytes.push(b' ');
        }
        let total_len = 12 + 8 + json_bytes.len() + 8 + bin.len();
        let mut glb = Vec::with_capacity(total_len);
        glb.extend_from_slice(&GLB_MAGIC.to_le_bytes());
        glb.extend_from_slice(&2u32.to_le_bytes());
        glb.extend_from_slice(&(total_len as u32).to_le_bytes());

        glb.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
        glb.extend_from_slice(&GLB_CHUNK_JSON.to_le_bytes());
        glb.extend_from_slice(&json_bytes);

        glb.extend_from_slice(&(bin.len() as u32).to_le_bytes());
        glb.extend_from_slice(&GLB_CHUNK_BIN.to_le_bytes());
        glb.extend_from_slice(&bin);
        Ok((glb, report))
    } else {
        let encoded = base64_encode(&bin);
        let _ = writeln!(
            json,
            "  \"buffers\": [{{\"byteLength\": {}, \"uri\": \"data:application/octet-stream;base64,{}\"}}]",
            bin.len(),
            encoded
        );
        json.push_str("}\n");
        Ok((json.into_bytes(), report))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::InterpolationMode;
    use crate::primitives::PrimitiveType;

    #[test]
    fn obj_export_import_roundtrip_preserves_mesh_and_reports_warnings() {
        let mut src = NovaEngineState::default();
        src.add_primitive(PrimitiveType::Light);
        src.add_primitive(PrimitiveType::Camera);
        src.anim_insert_node_trs_keys(1, 0, 1, InterpolationMode::Linear);

        let (obj_bytes, exp_rep) = export_obj(&src).expect("export_obj should succeed");
        assert!(exp_rep.warnings.iter().any(|w| w.to_lowercase().contains("light")));
        assert!(exp_rep.warnings.iter().any(|w| w.to_lowercase().contains("camera")));
        assert!(exp_rep.warnings.iter().any(|w| w.to_lowercase().contains("animation")));

        let mut dst = NovaEngineState::new_empty();
        let imp_rep = import_obj(&mut dst, &obj_bytes, "RoundtripObj").expect("import_obj should succeed");
        assert_eq!(imp_rep.imported_nodes, 3); // Parent Empty + Studio Cube + Base Plinth
        let cube = dst
            .nodes
            .iter()
            .find(|n| n.kind == NodeKind::Mesh && n.name.contains("Studio_Cube"))
            .expect("Studio Cube mesh imported");
        let mesh = cube.mesh.as_ref().unwrap();
        assert_eq!(mesh.indices.len(), 36);
        assert!(mesh.positions.len() >= 24);
    }

    #[test]
    fn obj_rejects_external_mtl_silently_dropped_warning_and_oob_index() {
        let obj = b"mtllib missing.mtl\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";
        let mut state = NovaEngineState::new_empty();
        let rep = import_obj(&mut state, obj, "Triangle").expect("valid obj");
        assert!(rep.warnings.iter().any(|w| w.contains("missing.mtl")));

        let bad = b"v 0 0 0\nv 1 0 0\nf 1 2 99\n";
        assert!(import_obj(&mut state, bad, "Bad").is_err());
    }

    #[test]
    fn png_encode_decode_roundtrip() {
        let rgba = vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 128,
        ];
        let png = encode_png_rgba8(2, 2, &rgba).expect("encode png");
        let (w, h, decoded) = decode_png_rgba8(&png).expect("decode png");
        assert_eq!((w, h), (2, 2));
        assert_eq!(decoded, rgba);
    }

    #[test]
    fn glb_and_gltf_roundtrip_hierarchy_pbr_camera_and_textures() {
        let mut src = NovaEngineState::default();
        src.add_primitive(PrimitiveType::Light);
        src.add_primitive(PrimitiveType::Camera);
        let parent_id = src.nodes.iter().find(|n| n.kind == NodeKind::Mesh).unwrap().id;
        let child_id = src.add_primitive(PrimitiveType::UvSphere);
        src.reparent_node(child_id, Some(parent_id));

        // Create and assign a textured PBR material with alpha blend
        let mid = src.create_material_from_preset(1, false);
        let mut mat = src.find_material(mid).unwrap().clone();
        mat.base_color = [0.2, 0.75, 0.9, 0.85];
        mat.metallic = 0.82;
        mat.roughness = 0.24;
        mat.alpha_mode = AlphaMode::Blend;
        mat.albedo_tex_id = src.textures.first().map(|t| t.id);
        src.update_material(mat);
        src.selected_ids = vec![child_id];
        src.assign_material_to_selected(mid);

        for &as_glb in &[true, false] {
            let (bytes, exp_rep) = export_gltf(&src, as_glb).expect("export gltf/glb");
            assert!(exp_rep.warnings.iter().any(|w| w.contains("Light")));

            let mut dst = NovaEngineState::new_empty();
            let imp_rep =
                import_gltf_or_glb(&mut dst, &bytes, "RoundtripGltf").expect("import gltf/glb");
            assert!(imp_rep.imported_nodes >= 3, "mesh + child + camera expected");
            assert!(imp_rep.imported_materials >= 2);
            assert!(imp_rep.imported_textures >= 1);

            let cam = dst.nodes.iter().find(|n| n.kind == NodeKind::Camera);
            assert!(cam.is_some(), "camera node preserved across glTF/GLB");

            let imported_sphere = dst
                .nodes
                .iter()
                .find(|n| n.kind == NodeKind::Mesh && n.parent_id.is_some())
                .expect("parent-child hierarchy preserved");
            assert!((imported_sphere.material.metallic - 0.82).abs() < 0.01);
            assert!((imported_sphere.material.roughness - 0.24).abs() < 0.01);
        }
    }

    #[test]
    fn gltf_import_warns_on_skins_morphs_and_rejects_corrupt_glb() {
        let mut state = NovaEngineState::new_empty();
        // Invalid GLB magic
        assert!(import_gltf_or_glb(&mut state, b"NOT_GLB_DATA", "Bad").is_err());
        // Corrupt header length
        assert!(import_gltf_or_glb(&mut state, b"glTF\x02\x00\x00\x00\xff\xff\x00\x00", "Bad").is_err());

        // Minimal glTF JSON with skin & animation warning
        let json = br#"{
            "asset": {"version": "2.0"},
            "skins": [{"joints": [0]}],
            "animations": [{"channels": []}],
            "buffers": [{"byteLength": 36, "uri": "data:application/octet-stream;base64,AAAAAAAAAAAAAAAAAACAPwAAAAAAAAAAAAAAAAAAgD8AAAAA"}],
            "bufferViews": [{"buffer": 0, "byteOffset": 0, "byteLength": 36}],
            "accessors": [{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3"}],
            "meshes": [{"primitives": [{"attributes": {"POSITION": 0}}]}],
            "nodes": [{"name": "Tri", "mesh": 0}]
        }"#;
        let rep = import_gltf_or_glb(&mut state, json, "WarnTest").expect("valid minimal gltf");
        assert!(rep.warnings.iter().any(|w| w.contains("Skeletal")));
        assert!(rep.warnings.iter().any(|w| w.contains("animations")));
    }
}
