//! Documented versioned `.nova3d` JSON project serialization & strict parser (FM-4, FM-7, FM-9).
//! Zero external dependencies; validates types, bounds, version, and hierarchy integrity.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::Write;

use crate::animation::{
    AnimChannelKind, AnimTargetKind, AnimTrack, AnimationState, InterpolationMode, Keyframe,
    TimelineSettings,
};
use crate::camera::{ProjectionMode, ViewportCamera};
use crate::command::CommandHistory;
use crate::lookdev::{
    base64_decode, base64_encode, default_builtin_textures, AlphaMode, EnvironmentPreset,
    EnvironmentSpec, MaterialSpec, TextureAsset, TextureStatus,
};
use crate::math::Vec3;
use crate::poly::{EditSelectMode, InteractionMode, ModifierSpec, PolyMesh};
use crate::scene::{
    CameraNodeParams, GizmoState, LightKind, LightParams, MaterialParams, MeshData, NodeId,
    NodeKind, NovaEngineState, SceneNode, Transform,
};

pub const NOVA_FORMAT_ID: &str = "nova3d";
pub const NOVA_FORMAT_VERSION: u32 = 1;

pub(crate) fn escape_json_str(s: &str, out: &mut String) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

pub(crate) fn push_f32(val: f32, out: &mut String) {
    let clean = if val.is_finite() { val } else { 0.0 };
    let _ = write!(out, "{clean}");
}

fn push_f32_slice(vals: &[f32], out: &mut String) {
    out.push('[');
    for (i, &v) in vals.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        push_f32(v, out);
    }
    out.push(']');
}

pub fn serialize_project(state: &NovaEngineState) -> String {
    let mut out = String::with_capacity(8192);
    out.push_str("{\n");
    out.push_str("  \"format\": \"nova3d\",\n");
    let _ = writeln!(out, "  \"version\": {},", NOVA_FORMAT_VERSION);
    let _ = writeln!(out, "  \"next_node_id\": {},", state.next_node_id);
    let _ = writeln!(out, "  \"show_grid\": {},", state.show_grid);
    let _ = writeln!(out, "  \"show_axes\": {},", state.show_axes);
    let _ = writeln!(out, "  \"shading_mode\": {},", state.shading_mode);

    out.push_str("  \"selected_ids\": [");
    for (i, id) in state.selected_ids.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let _ = write!(out, "{id}");
    }
    out.push_str("],\n");

    let cam = &state.camera;
    out.push_str("  \"camera\": {\n");
    out.push_str("    \"target\": ");
    push_f32_slice(&[cam.target.x, cam.target.y, cam.target.z], &mut out);
    out.push_str(",\n");
    let _ = writeln!(out, "    \"distance\": {},", cam.distance);
    let _ = writeln!(out, "    \"yaw\": {},", cam.yaw);
    let _ = writeln!(out, "    \"pitch\": {},", cam.pitch);
    let _ = writeln!(out, "    \"projection\": {},", cam.projection as u32);
    let _ = writeln!(out, "    \"fov_y_degrees\": {},", cam.fov_y_degrees);
    let _ = writeln!(out, "    \"near_plane\": {},", cam.near_plane);
    let _ = writeln!(out, "    \"far_plane\": {}", cam.far_plane);
    out.push_str("  },\n");

    out.push_str("  \"nodes\": [\n");
    for (idx, node) in state.nodes.iter().enumerate() {
        out.push_str("    {\n");
        let _ = writeln!(out, "      \"id\": {},", node.id);
        match node.parent_id {
            Some(pid) => {
                let _ = writeln!(out, "      \"parent_id\": {pid},");
            }
            None => out.push_str("      \"parent_id\": null,\n"),
        }
        out.push_str("      \"name\": ");
        escape_json_str(&node.name, &mut out);
        out.push_str(",\n");
        let _ = writeln!(out, "      \"kind\": {},", node.kind as u32);
        let _ = writeln!(out, "      \"visible\": {},", node.visible);

        out.push_str("      \"transform\": {\n");
        out.push_str("        \"position\": ");
        push_f32_slice(
            &[node.transform.position.x, node.transform.position.y, node.transform.position.z],
            &mut out,
        );
        out.push_str(",\n        \"rotation\": ");
        push_f32_slice(
            &[
                node.transform.rotation_euler.x,
                node.transform.rotation_euler.y,
                node.transform.rotation_euler.z,
            ],
            &mut out,
        );
        out.push_str(",\n        \"scale\": ");
        push_f32_slice(
            &[node.transform.scale.x, node.transform.scale.y, node.transform.scale.z],
            &mut out,
        );
        out.push_str("\n      },\n");

        out.push_str("      \"material\": {\n");
        match node.material_id {
            Some(mid) => {
                let _ = writeln!(out, "        \"material_id\": {mid},");
            }
            None => out.push_str("        \"material_id\": null,\n"),
        }
        out.push_str("        \"base_color\": ");
        push_f32_slice(&node.material.base_color, &mut out);
        out.push_str(",\n");
        let _ = writeln!(out, "        \"metallic\": {},", node.material.metallic);
        let _ = writeln!(out, "        \"roughness\": {}", node.material.roughness);
        out.push_str("      },\n");

        out.push_str("      \"light\": {\n");
        let _ = writeln!(out, "        \"kind\": {},", node.light.kind as u32);
        out.push_str("        \"color\": ");
        push_f32_slice(&node.light.color, &mut out);
        out.push_str(",\n");
        let _ = writeln!(out, "        \"intensity\": {},", node.light.intensity);
        let _ = writeln!(out, "        \"cast_shadows\": {},", node.light.cast_shadows);
        let _ = writeln!(out, "        \"spot_inner_deg\": {},", node.light.spot_inner_deg);
        let _ = writeln!(out, "        \"spot_outer_deg\": {},", node.light.spot_outer_deg);
        let _ = writeln!(out, "        \"range\": {}", node.light.range);
        out.push_str("      },\n");

        out.push_str("      \"camera_node\": {\n");
        let _ = writeln!(out, "        \"fov_y_degrees\": {},", node.camera_node.fov_y_degrees);
        let _ = writeln!(out, "        \"near_plane\": {},", node.camera_node.near_plane);
        let _ = writeln!(out, "        \"far_plane\": {}", node.camera_node.far_plane);
        out.push_str("      },\n");

        match &node.mesh {
            Some(mesh) => {
                out.push_str("      \"mesh\": {\n");
                out.push_str("        \"positions\": [");
                for (i, p) in mesh.positions.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    push_f32(p[0], &mut out);
                    out.push(',');
                    push_f32(p[1], &mut out);
                    out.push(',');
                    push_f32(p[2], &mut out);
                }
                out.push_str("],\n        \"normals\": [");
                for (i, n) in mesh.normals.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    push_f32(n[0], &mut out);
                    out.push(',');
                    push_f32(n[1], &mut out);
                    out.push(',');
                    push_f32(n[2], &mut out);
                }
                out.push_str("],\n        \"uvs\": [");
                for (i, uv) in mesh.uvs.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    push_f32(uv[0], &mut out);
                    out.push(',');
                    push_f32(uv[1], &mut out);
                }
                out.push_str("],\n        \"indices\": [");
                for (i, ind) in mesh.indices.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    let _ = write!(out, "{ind}");
                }
                out.push_str("]\n      },\n");
            }
            None => out.push_str("      \"mesh\": null,\n"),
        }

        match &node.base_poly {
            Some(poly) => {
                out.push_str("      \"poly\": {\n");
                let _ = writeln!(out, "        \"smooth_shading\": {},", poly.smooth_shading);
                out.push_str("        \"vertices\": [");
                for (i, v) in poly.vertices.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    push_f32(v.x, &mut out);
                    out.push(',');
                    push_f32(v.y, &mut out);
                    out.push(',');
                    push_f32(v.z, &mut out);
                }
                out.push_str("],\n        \"faces\": [");
                for (fi, f) in poly.faces.iter().enumerate() {
                    if fi > 0 {
                        out.push(',');
                    }
                    out.push('[');
                    for (vi_idx, &vi) in f.iter().enumerate() {
                        if vi_idx > 0 {
                            out.push(',');
                        }
                        let _ = write!(out, "{vi}");
                    }
                    out.push(']');
                }
                out.push_str("]\n      },\n");
            }
            None => out.push_str("      \"poly\": null,\n"),
        }

        out.push_str("      \"modifiers\": [");
        for (mi, mod_spec) in node.modifiers.iter().enumerate() {
            if mi > 0 {
                out.push(',');
            }
            match *mod_spec {
                ModifierSpec::Mirror { axis, merge, enabled } => {
                    let _ = write!(
                        out,
                        "{{\"kind\":\"mirror\",\"axis\":{axis},\"merge\":{merge},\"enabled\":{enabled}}}"
                    );
                }
                ModifierSpec::Subdivision { levels, enabled } => {
                    let _ = write!(
                        out,
                        "{{\"kind\":\"subdiv\",\"levels\":{levels},\"enabled\":{enabled}}}"
                    );
                }
                ModifierSpec::Solidify { thickness, enabled } => {
                    out.push_str("{\"kind\":\"solidify\",\"thickness\":");
                    push_f32(thickness, &mut out);
                    let _ = write!(out, ",\"enabled\":{enabled}}}");
                }
                ModifierSpec::Array { count, offset, enabled } => {
                    let _ = write!(out, "{{\"kind\":\"array\",\"count\":{count},\"offset\":[");
                    push_f32(offset[0], &mut out);
                    out.push(',');
                    push_f32(offset[1], &mut out);
                    out.push(',');
                    push_f32(offset[2], &mut out);
                    let _ = write!(out, "],\"enabled\":{enabled}}}");
                }
            }
        }
        out.push_str("]\n");

        if idx + 1 < state.nodes.len() {
            out.push_str("    },\n");
        } else {
            out.push_str("    }\n");
        }
    }
    out.push_str("  ],\n");

    match state.selected_material_id {
        Some(sid) => {
            let _ = writeln!(out, "  \"selected_material_id\": {sid},");
        }
        None => out.push_str("  \"selected_material_id\": null,\n"),
    }

    out.push_str("  \"environment\": {\n");
    let _ = writeln!(out, "    \"preset\": {},", state.environment.preset as u32);
    let _ = writeln!(out, "    \"rotation_deg\": {},", state.environment.rotation_deg);
    let _ = writeln!(out, "    \"intensity\": {},", state.environment.intensity);
    let _ = writeln!(out, "    \"show_skybox\": {},", state.environment.show_skybox);
    let _ = writeln!(out, "    \"bloom_enabled\": {},", state.environment.bloom_enabled);
    let _ = writeln!(out, "    \"bloom_strength\": {},", state.environment.bloom_strength);
    let _ = writeln!(out, "    \"exposure\": {}", state.environment.exposure);
    out.push_str("  },\n");

    let push_opt_u64 = |v: Option<u64>, s: &mut String| match v {
        Some(id) => {
            let _ = write!(s, "{id}");
        }
        None => s.push_str("null"),
    };

    out.push_str("  \"materials\": [\n");
    for (idx, m) in state.materials.iter().enumerate() {
        out.push_str("    {\n");
        let _ = writeln!(out, "      \"id\": {},", m.id);
        out.push_str("      \"name\": ");
        escape_json_str(&m.name, &mut out);
        out.push_str(",\n");
        out.push_str("      \"base_color\": ");
        push_f32_slice(&m.base_color, &mut out);
        out.push_str(",\n");
        let _ = writeln!(out, "      \"metallic\": {},", m.metallic);
        let _ = writeln!(out, "      \"roughness\": {},", m.roughness);
        out.push_str("      \"emissive\": ");
        push_f32_slice(&m.emissive, &mut out);
        out.push_str(",\n");
        let _ = writeln!(out, "      \"emissive_intensity\": {},", m.emissive_intensity);
        let _ = writeln!(out, "      \"ao_strength\": {},", m.ao_strength);
        let _ = writeln!(out, "      \"alpha_mode\": {},", m.alpha_mode as u32);
        out.push_str("      \"albedo_tex_id\": ");
        push_opt_u64(m.albedo_tex_id, &mut out);
        out.push_str(",\n");
        out.push_str("      \"normal_tex_id\": ");
        push_opt_u64(m.normal_tex_id, &mut out);
        out.push_str(",\n");
        out.push_str("      \"roughness_tex_id\": ");
        push_opt_u64(m.roughness_tex_id, &mut out);
        out.push_str(",\n");
        out.push_str("      \"ao_tex_id\": ");
        push_opt_u64(m.ao_tex_id, &mut out);
        out.push_str(",\n");
        out.push_str("      \"emissive_tex_id\": ");
        push_opt_u64(m.emissive_tex_id, &mut out);
        out.push_str(",\n");
        out.push_str("      \"uv_scale\": ");
        push_f32_slice(&m.uv_scale, &mut out);
        out.push_str(",\n");
        out.push_str("      \"uv_offset\": ");
        push_f32_slice(&m.uv_offset, &mut out);
        out.push('\n');
        if idx + 1 < state.materials.len() {
            out.push_str("    },\n");
        } else {
            out.push_str("    }\n");
        }
    }
    out.push_str("  ],\n");

    out.push_str("  \"textures\": [\n");
    for (idx, t) in state.textures.iter().enumerate() {
        out.push_str("    {\n");
        let _ = writeln!(out, "      \"id\": {},", t.id);
        out.push_str("      \"name\": ");
        escape_json_str(&t.name, &mut out);
        out.push_str(",\n");
        out.push_str("      \"source_uri\": ");
        escape_json_str(&t.source_uri, &mut out);
        out.push_str(",\n");
        let _ = writeln!(out, "      \"width\": {},", t.width);
        let _ = writeln!(out, "      \"height\": {},", t.height);
        let _ = writeln!(out, "      \"status\": {},", t.status as u32);
        if t.status == TextureStatus::Loaded && !t.rgba.is_empty() {
            let b64 = base64_encode(&t.rgba);
            out.push_str("      \"rgba_b64\": ");
            escape_json_str(&b64, &mut out);
            out.push('\n');
        } else {
            out.push_str("      \"rgba_b64\": null\n");
        }
        if idx + 1 < state.textures.len() {
            out.push_str("    },\n");
        } else {
            out.push_str("    }\n");
        }
    }
    out.push_str("  ],\n");

    out.push_str("  \"active_camera_node_id\": ");
    push_opt_u64(state.active_camera_node_id, &mut out);
    out.push_str(",\n");

    let tl = &state.animation.timeline;
    out.push_str("  \"timeline\": {\n");
    let _ = writeln!(out, "    \"start_frame\": {},", tl.start_frame);
    let _ = writeln!(out, "    \"end_frame\": {},", tl.end_frame);
    out.push_str("    \"fps\": ");
    push_f32(tl.fps, &mut out);
    out.push_str(",\n    \"current_frame\": ");
    push_f32(tl.current_frame, &mut out);
    out.push_str(",\n");
    let _ = writeln!(out, "    \"looping\": {},", tl.looping);
    let _ = writeln!(out, "    \"auto_key\": {}", tl.auto_key);
    out.push_str("  },\n");

    out.push_str("  \"tracks\": [\n");
    for (ti, tr) in state.animation.tracks.iter().enumerate() {
        out.push_str("    {\n");
        let _ = writeln!(out, "      \"id\": {},", tr.id);
        let _ = writeln!(out, "      \"target_kind\": {},", tr.target_kind as u32);
        let _ = writeln!(out, "      \"target_id\": {},", tr.target_id);
        let _ = writeln!(out, "      \"channel\": {},", tr.channel as u32);
        out.push_str("      \"keyframes\": [\n");
        for (ki, kf) in tr.keyframes.iter().enumerate() {
            out.push_str("        {");
            let _ = write!(
                out,
                "\"id\":{},\"frame\":{},\"value\":",
                kf.id, kf.frame
            );
            push_f32(kf.value, &mut out);
            let _ = write!(out, ",\"interp\":{},\"in\":[", kf.interpolation as u32);
            push_f32(kf.in_handle[0], &mut out);
            out.push(',');
            push_f32(kf.in_handle[1], &mut out);
            out.push_str("],\"out\":[");
            push_f32(kf.out_handle[0], &mut out);
            out.push(',');
            push_f32(kf.out_handle[1], &mut out);
            out.push(']');
            if ki + 1 < tr.keyframes.len() {
                out.push_str("},\n");
            } else {
                out.push_str("}\n");
            }
        }
        out.push_str("      ]\n");
        if ti + 1 < state.animation.tracks.len() {
            out.push_str("    },\n");
        } else {
            out.push_str("    }\n");
        }
    }
    out.push_str("  ]\n");
    out.push_str("}\n");
    out
}

#[derive(Debug, Clone)]
pub(crate) enum JsonVal {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<JsonVal>),
    Obj(HashMap<String, JsonVal>),
}

pub(crate) struct JsonParser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> JsonParser<'a> {
    pub(crate) fn new(input: &'a str) -> Self {
        Self {
            bytes: input.as_bytes(),
            pos: 0,
        }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip_ws();
        self.bytes.get(self.pos).copied()
    }

    fn expect_byte(&mut self, expected: u8) -> Result<(), String> {
        self.skip_ws();
        if self.bytes.get(self.pos).copied() == Some(expected) {
            self.pos += 1;
            Ok(())
        } else {
            Err(format!("Expected '{}' at offset {}", expected as char, self.pos))
        }
    }

    pub(crate) fn parse_value(&mut self, depth: usize) -> Result<JsonVal, String> {
        if depth > 64 {
            return Err("JSON nesting depth exceeded".to_string());
        }
        match self.peek() {
            Some(b'n') => self.parse_lit(b"null", JsonVal::Null),
            Some(b't') => self.parse_lit(b"true", JsonVal::Bool(true)),
            Some(b'f') => self.parse_lit(b"false", JsonVal::Bool(false)),
            Some(b'"') => Ok(JsonVal::Str(self.parse_string()?)),
            Some(b'[') => self.parse_array(depth + 1),
            Some(b'{') => self.parse_object(depth + 1),
            Some(b'-' | b'0'..=b'9') => Ok(JsonVal::Num(self.parse_number()?)),
            Some(b) => Err(format!("Unexpected byte '{}' at offset {}", b as char, self.pos)),
            None => Err("Unexpected end of JSON input".to_string()),
        }
    }

    fn parse_lit(&mut self, lit: &[u8], val: JsonVal) -> Result<JsonVal, String> {
        self.skip_ws();
        if self.bytes[self.pos..].starts_with(lit) {
            self.pos += lit.len();
            Ok(val)
        } else {
            Err(format!("Invalid literal at offset {}", self.pos))
        }
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.expect_byte(b'"')?;
        let mut s = String::new();
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            self.pos += 1;
            if b == b'"' {
                return Ok(s);
            } else if b == b'\\' {
                let esc = *self
                    .bytes
                    .get(self.pos)
                    .ok_or_else(|| "Unterminated string escape".to_string())?;
                self.pos += 1;
                match esc {
                    b'"' => s.push('"'),
                    b'\\' => s.push('\\'),
                    b'/' => s.push('/'),
                    b'n' => s.push('\n'),
                    b'r' => s.push('\r'),
                    b't' => s.push('\t'),
                    b'u' => {
                        if self.pos + 4 > self.bytes.len() {
                            return Err("Truncated \\u escape".to_string());
                        }
                        let hex = std::str::from_utf8(&self.bytes[self.pos..self.pos + 4])
                            .map_err(|_| "Invalid UTF-8 in \\u escape".to_string())?;
                        let code = u32::from_str_radix(hex, 16)
                            .map_err(|_| "Invalid hex in \\u escape".to_string())?;
                        if let Some(ch) = char::from_u32(code) {
                            s.push(ch);
                        }
                        self.pos += 4;
                    }
                    other => s.push(other as char),
                }
            } else if b < 0x80 {
                s.push(b as char);
            } else {
                // Multi-byte UTF-8 sequence starting at b
                let start = self.pos - 1;
                let width = if b & 0xE0 == 0xC0 {
                    2
                } else if b & 0xF0 == 0xE0 {
                    3
                } else if b & 0xF8 == 0xF0 {
                    4
                } else {
                    return Err("Invalid UTF-8 lead byte".to_string());
                };
                if start + width > self.bytes.len() {
                    return Err("Truncated UTF-8 sequence".to_string());
                }
                let slice = std::str::from_utf8(&self.bytes[start..start + width])
                    .map_err(|_| "Invalid UTF-8 sequence".to_string())?;
                s.push_str(slice);
                self.pos = start + width;
            }
        }
        Err("Unterminated JSON string".to_string())
    }

    fn parse_number(&mut self) -> Result<f64, String> {
        self.skip_ws();
        let start = self.pos;
        if self.bytes.get(self.pos) == Some(&b'-') {
            self.pos += 1;
        }
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            if matches!(b, b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-') {
                self.pos += 1;
            } else {
                break;
            }
        }
        let slice = std::str::from_utf8(&self.bytes[start..self.pos])
            .map_err(|_| "Invalid UTF-8 in number".to_string())?;
        let val: f64 = slice
            .parse()
            .map_err(|_| format!("Invalid number '{slice}'"))?;
        if !val.is_finite() {
            return Err("Non-finite number in project".to_string());
        }
        Ok(val)
    }

    fn parse_array(&mut self, depth: usize) -> Result<JsonVal, String> {
        self.expect_byte(b'[')?;
        let mut items = Vec::new();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(JsonVal::Arr(items));
        }
        loop {
            items.push(self.parse_value(depth)?);
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    return Ok(JsonVal::Arr(items));
                }
                _ => return Err(format!("Expected ',' or ']' at offset {}", self.pos)),
            }
        }
    }

    fn parse_object(&mut self, depth: usize) -> Result<JsonVal, String> {
        self.expect_byte(b'{')?;
        let mut map = HashMap::new();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(JsonVal::Obj(map));
        }
        loop {
            let key = self.parse_string()?;
            self.expect_byte(b':')?;
            let val = self.parse_value(depth)?;
            map.insert(key, val);
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(JsonVal::Obj(map));
                }
                _ => return Err(format!("Expected ',' or '}}' at offset {}", self.pos)),
            }
        }
    }
}

pub(crate) fn as_obj<'a>(v: &'a JsonVal, ctx: &str) -> Result<&'a HashMap<String, JsonVal>, String> {
    match v {
        JsonVal::Obj(m) => Ok(m),
        _ => Err(format!("Expected object for {ctx}")),
    }
}

fn req<'a>(m: &'a HashMap<String, JsonVal>, key: &str) -> Result<&'a JsonVal, String> {
    m.get(key).ok_or_else(|| format!("Missing field '{key}'"))
}

pub(crate) fn as_str<'a>(v: &'a JsonVal, ctx: &str) -> Result<&'a str, String> {
    match v {
        JsonVal::Str(s) => Ok(s.as_str()),
        _ => Err(format!("Expected string for {ctx}")),
    }
}

pub(crate) fn as_f32(v: &JsonVal, ctx: &str) -> Result<f32, String> {
    match v {
        JsonVal::Num(n) => {
            let f = *n as f32;
            if f.is_finite() {
                Ok(f)
            } else {
                Err(format!("Non-finite float for {ctx}"))
            }
        }
        _ => Err(format!("Expected number for {ctx}")),
    }
}

pub(crate) fn as_u64(v: &JsonVal, ctx: &str) -> Result<u64, String> {
    match v {
        JsonVal::Num(n) if *n >= 0.0 => Ok(*n as u64),
        _ => Err(format!("Expected non-negative integer for {ctx}")),
    }
}

pub(crate) fn as_bool(v: &JsonVal, ctx: &str) -> Result<bool, String> {
    match v {
        JsonVal::Bool(b) => Ok(*b),
        _ => Err(format!("Expected boolean for {ctx}")),
    }
}

fn as_vec3(v: &JsonVal, ctx: &str) -> Result<Vec3, String> {
    match v {
        JsonVal::Arr(arr) if arr.len() == 3 => Ok(Vec3::new(
            as_f32(&arr[0], ctx)?,
            as_f32(&arr[1], ctx)?,
            as_f32(&arr[2], ctx)?,
        )),
        _ => Err(format!("Expected 3-element vector for {ctx}")),
    }
}

fn as_f32_array<'a>(v: &'a JsonVal, ctx: &str) -> Result<Vec<f32>, String> {
    match v {
        JsonVal::Arr(arr) => {
            let mut out = Vec::with_capacity(arr.len());
            for item in arr {
                out.push(as_f32(item, ctx)?);
            }
            Ok(out)
        }
        _ => Err(format!("Expected numeric array for {ctx}")),
    }
}

pub fn deserialize_project(input: &str) -> Result<NovaEngineState, String> {
    if input.len() > 64 * 1024 * 1024 {
        return Err("Project file exceeds 64 MiB safety limit".to_string());
    }
    let mut parser = JsonParser::new(input);
    let root_val = parser.parse_value(0)?;
    let root = as_obj(&root_val, "root")?;

    let format = as_str(req(root, "format")?, "format")?;
    if format != NOVA_FORMAT_ID {
        return Err(format!("Unsupported project format '{format}'"));
    }
    let version = as_u64(req(root, "version")?, "version")? as u32;
    if version != NOVA_FORMAT_VERSION {
        return Err(format!(
            "Unsupported project version {version} (expected {NOVA_FORMAT_VERSION})"
        ));
    }

    let show_grid = root
        .get("show_grid")
        .map(|v| as_bool(v, "show_grid"))
        .transpose()?
        .unwrap_or(true);
    let show_axes = root
        .get("show_axes")
        .map(|v| as_bool(v, "show_axes"))
        .transpose()?
        .unwrap_or(true);
    let shading_mode = root
        .get("shading_mode")
        .map(|v| as_u64(v, "shading_mode"))
        .transpose()?
        .unwrap_or(0) as u32;

    let cam_obj = as_obj(req(root, "camera")?, "camera")?;
    let mut camera = ViewportCamera::default();
    camera.target = as_vec3(req(cam_obj, "target")?, "camera.target")?;
    camera.distance = as_f32(req(cam_obj, "distance")?, "camera.distance")?.clamp(0.25, 250.0);
    camera.yaw = as_f32(req(cam_obj, "yaw")?, "camera.yaw")?;
    camera.pitch = as_f32(req(cam_obj, "pitch")?, "camera.pitch")?;
    camera.projection = match as_u64(req(cam_obj, "projection")?, "camera.projection")? {
        1 => ProjectionMode::Orthographic,
        _ => ProjectionMode::Perspective,
    };
    camera.fov_y_degrees = as_f32(req(cam_obj, "fov_y_degrees")?, "camera.fov_y_degrees")?.clamp(10.0, 120.0);
    camera.near_plane = as_f32(req(cam_obj, "near_plane")?, "camera.near_plane")?.max(0.001);
    camera.far_plane = as_f32(req(cam_obj, "far_plane")?, "camera.far_plane")?.max(camera.near_plane + 1.0);

    let nodes_val = match req(root, "nodes")? {
        JsonVal::Arr(arr) => arr,
        _ => return Err("Expected 'nodes' array".to_string()),
    };

    let mut nodes = Vec::with_capacity(nodes_val.len());
    let mut seen_ids = HashSet::new();
    let mut max_id = 0u64;

    for (idx, item) in nodes_val.iter().enumerate() {
        let obj = as_obj(item, &format!("nodes[{idx}]"))?;
        let id = as_u64(req(obj, "id")?, "node.id")?;
        if id == 0 || !seen_ids.insert(id) {
            return Err(format!("Duplicate or invalid node id {id}"));
        }
        max_id = max_id.max(id);

        let parent_id = match req(obj, "parent_id")? {
            JsonVal::Null => None,
            v => Some(as_u64(v, "node.parent_id")?),
        };
        let name = as_str(req(obj, "name")?, "node.name")?.to_string();
        let kind = match as_u64(req(obj, "kind")?, "node.kind")? {
            0 => NodeKind::Empty,
            1 => NodeKind::Mesh,
            2 => NodeKind::Light,
            3 => NodeKind::Camera,
            other => return Err(format!("Unknown node kind {other}")),
        };
        let visible = as_bool(req(obj, "visible")?, "node.visible")?;

        let tr_obj = as_obj(req(obj, "transform")?, "node.transform")?;
        let transform = Transform {
            position: as_vec3(req(tr_obj, "position")?, "transform.position")?,
            rotation_euler: as_vec3(req(tr_obj, "rotation")?, "transform.rotation")?,
            scale: as_vec3(req(tr_obj, "scale")?, "transform.scale")?,
        };

        let mat_obj = as_obj(req(obj, "material")?, "node.material")?;
        let material_id = match mat_obj.get("material_id") {
            Some(JsonVal::Null) | None => None,
            Some(v) => Some(as_u64(v, "material.material_id")?),
        };
        let bc = as_f32_array(req(mat_obj, "base_color")?, "material.base_color")?;
        if bc.len() != 4 {
            return Err("material.base_color must have 4 elements".to_string());
        }
        let material = MaterialParams {
            base_color: [bc[0], bc[1], bc[2], bc[3]],
            metallic: as_f32(req(mat_obj, "metallic")?, "material.metallic")?.clamp(0.0, 1.0),
            roughness: as_f32(req(mat_obj, "roughness")?, "material.roughness")?.clamp(0.04, 1.0),
        };

        let light = if let Some(l_val) = obj.get("light") {
            let l_obj = as_obj(l_val, "node.light")?;
            let lc = as_f32_array(req(l_obj, "color")?, "light.color")?;
            if lc.len() != 3 {
                return Err("light.color must have 3 elements".to_string());
            }
            let spot_inner_deg = l_obj
                .get("spot_inner_deg")
                .map(|v| as_f32(v, "light.spot_inner_deg"))
                .transpose()?
                .unwrap_or(18.0);
            let spot_outer_deg = l_obj
                .get("spot_outer_deg")
                .map(|v| as_f32(v, "light.spot_outer_deg"))
                .transpose()?
                .unwrap_or(38.0);
            let range = l_obj
                .get("range")
                .map(|v| as_f32(v, "light.range"))
                .transpose()?
                .unwrap_or(14.0);
            LightParams {
                kind: match as_u64(req(l_obj, "kind")?, "light.kind")? {
                    1 => LightKind::Point,
                    2 => LightKind::Spot,
                    _ => LightKind::Directional,
                },
                color: [lc[0], lc[1], lc[2]],
                intensity: as_f32(req(l_obj, "intensity")?, "light.intensity")?.max(0.0),
                cast_shadows: as_bool(req(l_obj, "cast_shadows")?, "light.cast_shadows")?,
                spot_inner_deg: spot_inner_deg.clamp(1.0, 80.0),
                spot_outer_deg: spot_outer_deg.clamp(spot_inner_deg.clamp(1.0, 80.0) + 1.0, 89.0),
                range: range.clamp(0.5, 120.0),
            }
        } else {
            LightParams::default()
        };

        let camera_node = if let Some(cn_val) = obj.get("camera_node") {
            let cn_obj = as_obj(cn_val, "node.camera_node")?;
            CameraNodeParams {
                fov_y_degrees: as_f32(req(cn_obj, "fov_y_degrees")?, "camera_node.fov_y_degrees")?,
                near_plane: as_f32(req(cn_obj, "near_plane")?, "camera_node.near_plane")?,
                far_plane: as_f32(req(cn_obj, "far_plane")?, "camera_node.far_plane")?,
            }
        } else {
            CameraNodeParams::default()
        };

        let mesh = match req(obj, "mesh")? {
            JsonVal::Null => None,
            m_val => {
                let m_obj = as_obj(m_val, "node.mesh")?;
                let pos_flat = as_f32_array(req(m_obj, "positions")?, "mesh.positions")?;
                let norm_flat = as_f32_array(req(m_obj, "normals")?, "mesh.normals")?;
                let uv_flat = as_f32_array(req(m_obj, "uvs")?, "mesh.uvs")?;
                let idx_raw = match req(m_obj, "indices")? {
                    JsonVal::Arr(arr) => arr,
                    _ => return Err("Expected 'mesh.indices' array".to_string()),
                };
                if pos_flat.len() % 3 != 0 || norm_flat.len() != pos_flat.len() {
                    return Err("Invalid mesh position/normal buffer lengths".to_string());
                }
                let vcount = pos_flat.len() / 3;
                if uv_flat.len() != vcount * 2 {
                    return Err("Invalid mesh UV buffer length".to_string());
                }
                let positions: Vec<[f32; 3]> =
                    pos_flat.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect();
                let normals: Vec<[f32; 3]> =
                    norm_flat.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect();
                let uvs: Vec<[f32; 2]> = uv_flat.chunks_exact(2).map(|c| [c[0], c[1]]).collect();
                let mut indices = Vec::with_capacity(idx_raw.len());
                for item in idx_raw {
                    let ind = as_u64(item, "mesh.index")? as u32;
                    if (ind as usize) >= vcount {
                        return Err(format!("Mesh index {ind} out of bounds ({vcount} vertices)"));
                    }
                    indices.push(ind);
                }
                let (bounds_min, bounds_max) = MeshData::compute_bounds(&positions);
                Some(MeshData {
                    positions,
                    normals,
                    uvs,
                    indices,
                    bounds_min,
                    bounds_max,
                    version: 1,
                })
            }
        };

        let base_poly = match obj.get("poly") {
            Some(JsonVal::Obj(p_obj)) => {
                let smooth_shading = p_obj
                    .get("smooth_shading")
                    .map(|v| as_bool(v, "poly.smooth_shading"))
                    .transpose()?
                    .unwrap_or(false);
                let verts_flat = as_f32_array(req(p_obj, "vertices")?, "poly.vertices")?;
                if verts_flat.len() % 3 != 0 {
                    return Err("Invalid poly.vertices buffer length".to_string());
                }
                let vertices: Vec<Vec3> = verts_flat
                    .chunks_exact(3)
                    .map(|c| Vec3::new(c[0], c[1], c[2]))
                    .collect();
                let faces_raw = match req(p_obj, "faces")? {
                    JsonVal::Arr(arr) => arr,
                    _ => return Err("Expected 'poly.faces' array".to_string()),
                };
                let mut faces = Vec::with_capacity(faces_raw.len());
                for f_val in faces_raw {
                    let f_arr = match f_val {
                        JsonVal::Arr(arr) => arr,
                        _ => return Err("Expected face index array in 'poly.faces'".to_string()),
                    };
                    if f_arr.len() < 3 {
                        return Err("Face in 'poly.faces' has fewer than 3 vertices".to_string());
                    }
                    let mut f_indices = Vec::with_capacity(f_arr.len());
                    for item in f_arr {
                        let vi = as_u64(item, "poly.face.index")? as usize;
                        if vi >= vertices.len() {
                            return Err(format!("Poly face index {vi} out of bounds ({})", vertices.len()));
                        }
                        f_indices.push(vi);
                    }
                    faces.push(f_indices);
                }
                Some(PolyMesh {
                    vertices,
                    faces,
                    smooth_shading,
                })
            }
            Some(JsonVal::Null) | None => None,
            _ => return Err("Invalid 'poly' object".to_string()),
        };

        let mut modifiers = Vec::new();
        if let Some(JsonVal::Arr(m_arr)) = obj.get("modifiers") {
            for m_val in m_arr {
                let m_obj = as_obj(m_val, "node.modifier")?;
                let mk = as_str(req(m_obj, "kind")?, "modifier.kind")?;
                let enabled = m_obj
                    .get("enabled")
                    .map(|v| as_bool(v, "modifier.enabled"))
                    .transpose()?
                    .unwrap_or(true);
                match mk {
                    "mirror" => {
                        let axis = as_u64(req(m_obj, "axis")?, "mirror.axis")? as u8;
                        let merge = m_obj
                            .get("merge")
                            .map(|v| as_bool(v, "mirror.merge"))
                            .transpose()?
                            .unwrap_or(true);
                        modifiers.push(ModifierSpec::Mirror {
                            axis: axis.min(2),
                            merge,
                            enabled,
                        });
                    }
                    "subdiv" => {
                        let levels = as_u64(req(m_obj, "levels")?, "subdiv.levels")? as u8;
                        modifiers.push(ModifierSpec::Subdivision {
                            levels: levels.clamp(1, 3),
                            enabled,
                        });
                    }
                    "solidify" => {
                        let thickness = as_f32(req(m_obj, "thickness")?, "solidify.thickness")?;
                        modifiers.push(ModifierSpec::Solidify { thickness, enabled });
                    }
                    "array" => {
                        let count = as_u64(req(m_obj, "count")?, "array.count")? as u32;
                        let off_v = as_vec3(req(m_obj, "offset")?, "array.offset")?;
                        modifiers.push(ModifierSpec::Array {
                            count: count.clamp(1, 16),
                            offset: [off_v.x, off_v.y, off_v.z],
                            enabled,
                        });
                    }
                    _ => {}
                }
            }
        }

        let mut node = SceneNode {
            id,
            parent_id,
            name,
            kind,
            transform,
            mesh,
            base_poly,
            modifiers,
            material_id,
            material,
            light,
            camera_node,
            visible,
        };
        if node.kind == NodeKind::Mesh && node.base_poly.is_some() && !node.modifiers.is_empty() {
            node.rebuild_evaluated_mesh(false);
        }
        nodes.push(node);
    }

    // Validate hierarchy parent references and ensure no cycles
    let parent_map: HashMap<NodeId, Option<NodeId>> =
        nodes.iter().map(|n| (n.id, n.parent_id)).collect();
    for node in &nodes {
        let mut cur = node.parent_id;
        let mut steps = 0usize;
        while let Some(pid) = cur {
            if pid == node.id || steps > nodes.len() {
                return Err(format!("Cycle detected in hierarchy for node {}", node.id));
            }
            cur = *parent_map
                .get(&pid)
                .ok_or_else(|| format!("Node {} references missing parent {pid}", node.id))?;
            steps += 1;
        }
    }

    let next_node_id = root
        .get("next_node_id")
        .map(|v| as_u64(v, "next_node_id"))
        .transpose()?
        .unwrap_or(max_id + 1)
        .max(max_id + 1);

    let selected_ids: Vec<NodeId> = match root.get("selected_ids") {
        Some(JsonVal::Arr(arr)) => arr
            .iter()
            .filter_map(|v| as_u64(v, "selected_ids").ok())
            .filter(|id| seen_ids.contains(id))
            .collect(),
        _ => Vec::new(),
    };

    let environment = if let Some(env_val) = root.get("environment") {
        let e_obj = as_obj(env_val, "environment")?;
        EnvironmentSpec {
            preset: EnvironmentPreset::from_u32(
                e_obj
                    .get("preset")
                    .map(|v| as_u64(v, "env.preset"))
                    .transpose()?
                    .unwrap_or(0) as u32,
            ),
            rotation_deg: e_obj
                .get("rotation_deg")
                .map(|v| as_f32(v, "env.rotation_deg"))
                .transpose()?
                .unwrap_or(25.0),
            intensity: e_obj
                .get("intensity")
                .map(|v| as_f32(v, "env.intensity"))
                .transpose()?
                .unwrap_or(1.0),
            show_skybox: e_obj
                .get("show_skybox")
                .map(|v| as_bool(v, "env.show_skybox"))
                .transpose()?
                .unwrap_or(false),
            bloom_enabled: e_obj
                .get("bloom_enabled")
                .map(|v| as_bool(v, "env.bloom_enabled"))
                .transpose()?
                .unwrap_or(true),
            bloom_strength: e_obj
                .get("bloom_strength")
                .map(|v| as_f32(v, "env.bloom_strength"))
                .transpose()?
                .unwrap_or(0.14),
            exposure: e_obj
                .get("exposure")
                .map(|v| as_f32(v, "env.exposure"))
                .transpose()?
                .unwrap_or(1.0),
        }
        .sanitized()
    } else {
        EnvironmentSpec::default()
    };

    let mut textures = default_builtin_textures();
    if let Some(JsonVal::Arr(tex_arr)) = root.get("textures") {
        for (idx, t_val) in tex_arr.iter().enumerate() {
            let t_obj = as_obj(t_val, &format!("textures[{idx}]"))?;
            let id = as_u64(req(t_obj, "id")?, "texture.id")?;
            let name = as_str(req(t_obj, "name")?, "texture.name")?.to_string();
            let source_uri = as_str(req(t_obj, "source_uri")?, "texture.source_uri")?.to_string();
            let width = as_u64(req(t_obj, "width")?, "texture.width")? as u32;
            let height = as_u64(req(t_obj, "height")?, "texture.height")? as u32;
            let st_raw = as_u64(req(t_obj, "status")?, "texture.status")? as u32;
            if let Some(existing) = textures.iter_mut().find(|x| x.id == id) {
                if st_raw == TextureStatus::Missing as u32 {
                    existing.status = TextureStatus::Missing;
                }
                continue;
            }
            let (rgba, status) = match st_raw {
                2 => (Vec::new(), TextureStatus::Missing),
                1 => {
                    if let Some(JsonVal::Str(b64)) = t_obj.get("rgba_b64") {
                        match base64_decode(b64) {
                            Ok(bytes)
                                if bytes.len()
                                    == (width as usize)
                                        .saturating_mul(height as usize)
                                        .saturating_mul(4) =>
                            {
                                (bytes, TextureStatus::Loaded)
                            }
                            _ => (Vec::new(), TextureStatus::Missing),
                        }
                    } else {
                        (Vec::new(), TextureStatus::Missing)
                    }
                }
                _ => (Vec::new(), TextureStatus::Missing),
            };
            textures.push(TextureAsset {
                id,
                name,
                source_uri,
                width: width.max(1),
                height: height.max(1),
                rgba,
                status,
                version: 1,
            });
        }
    }
    let next_texture_id = textures.iter().map(|t| t.id).max().unwrap_or(0) + 1;

    let parse_opt_u64 = |val: Option<&JsonVal>, label: &str| -> Result<Option<u64>, String> {
        match val {
            Some(JsonVal::Null) | None => Ok(None),
            Some(v) => Ok(Some(as_u64(v, label)?)),
        }
    };

    let mut materials: Vec<MaterialSpec> = Vec::new();
    if let Some(JsonVal::Arr(mat_arr)) = root.get("materials") {
        for (idx, m_val) in mat_arr.iter().enumerate() {
            let m_obj = as_obj(m_val, &format!("materials[{idx}]"))?;
            let id = as_u64(req(m_obj, "id")?, "mat.id")?;
            let name = as_str(req(m_obj, "name")?, "mat.name")?.to_string();
            let bc = as_f32_array(req(m_obj, "base_color")?, "mat.base_color")?;
            let em = as_f32_array(req(m_obj, "emissive")?, "mat.emissive")?;
            let uvs = as_f32_array(req(m_obj, "uv_scale")?, "mat.uv_scale")?;
            let uvo = as_f32_array(req(m_obj, "uv_offset")?, "mat.uv_offset")?;
            if bc.len() != 4 || em.len() != 3 || uvs.len() != 2 || uvo.len() != 2 {
                return Err("Invalid material vector field length".to_string());
            }
            materials.push(
                MaterialSpec {
                    id,
                    name,
                    base_color: [bc[0], bc[1], bc[2], bc[3]],
                    metallic: as_f32(req(m_obj, "metallic")?, "mat.metallic")?,
                    roughness: as_f32(req(m_obj, "roughness")?, "mat.roughness")?,
                    emissive: [em[0], em[1], em[2]],
                    emissive_intensity: as_f32(
                        req(m_obj, "emissive_intensity")?,
                        "mat.emissive_intensity",
                    )?,
                    ao_strength: as_f32(req(m_obj, "ao_strength")?, "mat.ao_strength")?,
                    alpha_mode: match as_u64(req(m_obj, "alpha_mode")?, "mat.alpha_mode")? {
                        1 => AlphaMode::Blend,
                        _ => AlphaMode::Opaque,
                    },
                    albedo_tex_id: parse_opt_u64(m_obj.get("albedo_tex_id"), "mat.albedo_tex_id")?,
                    normal_tex_id: parse_opt_u64(m_obj.get("normal_tex_id"), "mat.normal_tex_id")?,
                    roughness_tex_id: parse_opt_u64(
                        m_obj.get("roughness_tex_id"),
                        "mat.roughness_tex_id",
                    )?,
                    ao_tex_id: parse_opt_u64(m_obj.get("ao_tex_id"), "mat.ao_tex_id")?,
                    emissive_tex_id: parse_opt_u64(
                        m_obj.get("emissive_tex_id"),
                        "mat.emissive_tex_id",
                    )?,
                    uv_scale: [uvs[0], uvs[1]],
                    uv_offset: [uvo[0], uvo[1]],
                }
                .sanitized(),
            );
        }
    }
    if materials.is_empty() {
        materials.push(MaterialSpec::studio_default(1, "Default Studio"));
    }
    let next_material_id = materials.iter().map(|m| m.id).max().unwrap_or(0) + 1;
    let selected_material_id = parse_opt_u64(root.get("selected_material_id"), "selected_material_id")?
        .filter(|sid| materials.iter().any(|m| m.id == *sid))
        .or_else(|| materials.first().map(|m| m.id));

    let active_camera_node_id = parse_opt_u64(root.get("active_camera_node_id"), "active_camera_node_id")?
        .filter(|cid| nodes.iter().any(|n| n.id == *cid && n.kind == NodeKind::Camera));

    let mut timeline = TimelineSettings::default();
    if let Some(tl_val) = root.get("timeline") {
        let tl_obj = as_obj(tl_val, "timeline")?;
        if let Some(v) = tl_obj.get("start_frame") {
            timeline.start_frame = (as_f32(v, "timeline.start_frame")?.round() as i32).clamp(0, 99_999);
        }
        if let Some(v) = tl_obj.get("end_frame") {
            timeline.end_frame = (as_f32(v, "timeline.end_frame")?.round() as i32)
                .max(timeline.start_frame + 1)
                .clamp(1, 100_000);
        }
        if let Some(v) = tl_obj.get("fps") {
            timeline.fps = as_f32(v, "timeline.fps")?.clamp(1.0, 120.0);
        }
        if let Some(v) = tl_obj.get("current_frame") {
            timeline.current_frame = as_f32(v, "timeline.current_frame")?
                .clamp(timeline.start_frame as f32, timeline.end_frame as f32);
        }
        if let Some(v) = tl_obj.get("looping") {
            timeline.looping = as_bool(v, "timeline.looping")?;
        }
        if let Some(v) = tl_obj.get("auto_key") {
            timeline.auto_key = as_bool(v, "timeline.auto_key")?;
        }
    }

    let mut tracks: Vec<AnimTrack> = Vec::new();
    let mut max_track_id = 0u64;
    let mut max_kf_id = 0u64;
    if let Some(JsonVal::Arr(tr_arr)) = root.get("tracks") {
        for (ti, tr_val) in tr_arr.iter().enumerate() {
            let tr_obj = as_obj(tr_val, &format!("tracks[{ti}]"))?;
            let id = as_u64(req(tr_obj, "id")?, "track.id")?.max(1);
            max_track_id = max_track_id.max(id);
            let tk_u32 = as_u64(req(tr_obj, "target_kind")?, "track.target_kind")? as u32;
            let Some(target_kind) = AnimTargetKind::from_u32(tk_u32) else {
                continue;
            };
            let target_id = as_u64(req(tr_obj, "target_id")?, "track.target_id")?;
            let ch_u32 = as_u64(req(tr_obj, "channel")?, "track.channel")? as u32;
            let Some(channel) = AnimChannelKind::from_u32(ch_u32) else {
                continue;
            };
            let kf_arr = match tr_obj.get("keyframes") {
                Some(JsonVal::Arr(arr)) => arr,
                _ => return Err(format!("tracks[{ti}].keyframes must be an array")),
            };
            let mut keyframes = Vec::with_capacity(kf_arr.len());
            for kf_val in kf_arr {
                let k_obj = as_obj(kf_val, "keyframe")?;
                let kf_id = as_u64(req(k_obj, "id")?, "kf.id")?.max(1);
                max_kf_id = max_kf_id.max(kf_id);
                let frame = (as_f32(req(k_obj, "frame")?, "kf.frame")?.round() as i32).clamp(0, 100_000);
                let value = channel.clamp_value(as_f32(req(k_obj, "value")?, "kf.value")?);
                let interp_u32 = as_u64(req(k_obj, "interp")?, "kf.interp")? as u32;
                let interpolation = InterpolationMode::from_u32(interp_u32).unwrap_or(InterpolationMode::Linear);
                let in_arr = as_f32_array(req(k_obj, "in")?, "kf.in")?;
                let out_arr = as_f32_array(req(k_obj, "out")?, "kf.out")?;
                let in_handle = if in_arr.len() == 2 { [in_arr[0], in_arr[1]] } else { [-4.0, 0.0] };
                let out_handle = if out_arr.len() == 2 { [out_arr[0], out_arr[1]] } else { [4.0, 0.0] };
                keyframes.push(Keyframe {
                    id: kf_id,
                    frame,
                    value,
                    interpolation,
                    in_handle,
                    out_handle,
                });
            }
            let mut tr = AnimTrack {
                id,
                target_kind,
                target_id,
                channel,
                keyframes,
            };
            tr.sort_keyframes();
            if !tr.keyframes.is_empty() {
                tracks.push(tr);
            }
        }
    }

    let animation = AnimationState {
        timeline,
        tracks,
        next_track_id: max_track_id + 1,
        next_keyframe_id: max_kf_id + 1,
    };

    let mut restored = NovaEngineState {
        nodes,
        selected_ids,
        camera,
        show_grid,
        show_axes,
        shading_mode: shading_mode.min(3),
        gizmo: GizmoState::default(),
        scene_revision: 1,
        history: CommandHistory::new(128),
        next_node_id,
        interaction_mode: InteractionMode::Object,
        edit_node_id: None,
        edit_select_mode: EditSelectMode::Face,
        edit_sel_verts: BTreeSet::new(),
        edit_sel_edges: BTreeSet::new(),
        edit_sel_faces: BTreeSet::new(),
        show_face_orientation: false,
        edit_overlay_lines: Vec::new(),
        edit_overlay_tris: Vec::new(),
        materials,
        selected_material_id,
        textures,
        environment,
        environment_version: 1,
        next_material_id,
        next_texture_id,
        material_preview_sphere: false,
        animation,
        active_camera_node_id,
    };
    restored.eval_animation_at_current_frame();
    Ok(restored)
}
