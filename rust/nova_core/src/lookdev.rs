//! Look Development data model: PBR material library, procedural/imported textures,
//! missing-texture fallback, HDR environment settings, and studio lighting rigs.

pub type MaterialId = u64;
pub type TextureId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TextureStatus {
    Builtin = 0,
    Loaded = 1,
    Missing = 2,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextureAsset {
    pub id: TextureId,
    pub name: String,
    pub source_uri: String,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub status: TextureStatus,
    pub version: u64,
}

impl TextureAsset {
    pub fn effective_rgba(&self) -> Vec<u8> {
        let expected = (self.width as usize)
            .saturating_mul(self.height as usize)
            .saturating_mul(4);
        if self.status != TextureStatus::Missing && expected > 0 && self.rgba.len() == expected {
            return self.rgba.clone();
        }
        missing_checker_rgba(64, 64)
    }

    pub fn effective_dimensions(&self) -> (u32, u32) {
        let expected = (self.width as usize)
            .saturating_mul(self.height as usize)
            .saturating_mul(4);
        if self.status != TextureStatus::Missing && expected > 0 && self.rgba.len() == expected {
            (self.width, self.height)
        } else {
            (64, 64)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AlphaMode {
    Opaque = 0,
    Blend = 1,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MaterialSpec {
    pub id: MaterialId,
    pub name: String,
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub emissive: [f32; 3],
    pub emissive_intensity: f32,
    pub ao_strength: f32,
    pub alpha_mode: AlphaMode,
    pub albedo_tex_id: Option<TextureId>,
    pub normal_tex_id: Option<TextureId>,
    pub roughness_tex_id: Option<TextureId>,
    pub ao_tex_id: Option<TextureId>,
    pub emissive_tex_id: Option<TextureId>,
    pub uv_scale: [f32; 2],
    pub uv_offset: [f32; 2],
}

impl MaterialSpec {
    pub fn studio_default(id: MaterialId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            base_color: [0.76, 0.78, 0.82, 1.0],
            metallic: 0.05,
            roughness: 0.42,
            emissive: [0.0, 0.0, 0.0],
            emissive_intensity: 0.0,
            ao_strength: 1.0,
            alpha_mode: AlphaMode::Opaque,
            albedo_tex_id: None,
            normal_tex_id: None,
            roughness_tex_id: None,
            ao_tex_id: None,
            emissive_tex_id: None,
            uv_scale: [1.0, 1.0],
            uv_offset: [0.0, 0.0],
        }
    }

    pub fn from_preset(id: MaterialId, preset_idx: u32) -> Self {
        let mut mat = Self::studio_default(id, "Material");
        match preset_idx {
            // 0: Studio Sky Blue
            0 => {
                mat.name = "Studio Blue".to_string();
                mat.base_color = [0.64, 0.79, 0.90, 1.0];
                mat.metallic = 0.08;
                mat.roughness = 0.35;
            }
            // 1: Polished Gold
            1 => {
                mat.name = "Polished Gold".to_string();
                mat.base_color = [0.98, 0.78, 0.32, 1.0];
                mat.metallic = 1.0;
                mat.roughness = 0.14;
            }
            // 2: Brushed Steel
            2 => {
                mat.name = "Brushed Steel".to_string();
                mat.base_color = [0.76, 0.79, 0.84, 1.0];
                mat.metallic = 0.95;
                mat.roughness = 0.30;
                mat.albedo_tex_id = Some(2);
                mat.roughness_tex_id = Some(5);
            }
            // 3: Matte Terracotta
            3 => {
                mat.name = "Matte Terracotta".to_string();
                mat.base_color = [0.80, 0.40, 0.28, 1.0];
                mat.metallic = 0.0;
                mat.roughness = 0.78;
                mat.ao_tex_id = Some(6);
            }
            // 4: Frosted Glass
            4 => {
                mat.name = "Frosted Glass".to_string();
                mat.base_color = [0.74, 0.88, 0.98, 0.44];
                mat.metallic = 0.04;
                mat.roughness = 0.12;
                mat.alpha_mode = AlphaMode::Blend;
            }
            // 5: Neon Tech Emissive
            5 => {
                mat.name = "Neon Tech".to_string();
                mat.base_color = [0.10, 0.14, 0.20, 1.0];
                mat.metallic = 0.20;
                mat.roughness = 0.30;
                mat.emissive = [0.15, 0.82, 1.0];
                mat.emissive_intensity = 2.4;
                mat.emissive_tex_id = Some(7);
            }
            // 6: Brick Masonry
            6 => {
                mat.name = "Brick Masonry".to_string();
                mat.base_color = [1.0, 1.0, 1.0, 1.0];
                mat.metallic = 0.0;
                mat.roughness = 0.82;
                mat.albedo_tex_id = Some(3);
                mat.normal_tex_id = Some(4);
                mat.roughness_tex_id = Some(5);
                mat.ao_tex_id = Some(6);
                mat.uv_scale = [2.0, 2.0];
            }
            // 7: Crimson Car Coat
            _ => {
                mat.name = "Crimson Coat".to_string();
                mat.base_color = [0.82, 0.09, 0.14, 1.0];
                mat.metallic = 0.45;
                mat.roughness = 0.12;
            }
        }
        mat
    }

    pub fn sanitized(mut self) -> Self {
        for c in &mut self.base_color {
            *c = c.clamp(0.0, 1.0);
        }
        for e in &mut self.emissive {
            *e = e.clamp(0.0, 1.0);
        }
        self.metallic = self.metallic.clamp(0.0, 1.0);
        self.roughness = self.roughness.clamp(0.04, 1.0);
        self.emissive_intensity = self.emissive_intensity.clamp(0.0, 10.0);
        self.ao_strength = self.ao_strength.clamp(0.0, 1.0);
        self.uv_scale = [
            self.uv_scale[0].clamp(0.05, 32.0),
            self.uv_scale[1].clamp(0.05, 32.0),
        ];
        self.uv_offset = [
            self.uv_offset[0].clamp(-10.0, 10.0),
            self.uv_offset[1].clamp(-10.0, 10.0),
        ];
        if self.base_color[3] < 0.995 {
            self.alpha_mode = AlphaMode::Blend;
        }
        self
    }

    pub fn referenced_textures(&self) -> [Option<TextureId>; 5] {
        [
            self.albedo_tex_id,
            self.normal_tex_id,
            self.roughness_tex_id,
            self.ao_tex_id,
            self.emissive_tex_id,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum EnvironmentPreset {
    StudioSoft = 0,
    StudioContrast = 1,
    OutdoorDaylight = 2,
    GoldenSunset = 3,
    NeonNight = 4,
    NeutralGrey = 5,
}

impl EnvironmentPreset {
    pub fn from_u32(v: u32) -> Self {
        match v {
            0 => Self::StudioSoft,
            1 => Self::StudioContrast,
            2 => Self::OutdoorDaylight,
            3 => Self::GoldenSunset,
            4 => Self::NeonNight,
            _ => Self::NeutralGrey,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnvironmentSpec {
    pub preset: EnvironmentPreset,
    pub rotation_deg: f32,
    pub intensity: f32,
    pub show_skybox: bool,
    pub bloom_enabled: bool,
    pub bloom_strength: f32,
    pub exposure: f32,
}

impl Default for EnvironmentSpec {
    fn default() -> Self {
        Self {
            preset: EnvironmentPreset::StudioSoft,
            rotation_deg: 25.0,
            intensity: 1.0,
            show_skybox: false,
            bloom_enabled: true,
            bloom_strength: 0.14,
            exposure: 1.0,
        }
    }
}

impl EnvironmentSpec {
    pub fn sanitized(mut self) -> Self {
        self.rotation_deg = self.rotation_deg.rem_euclid(360.0);
        self.intensity = self.intensity.clamp(0.05, 4.0);
        self.bloom_strength = self.bloom_strength.clamp(0.0, 1.0);
        self.exposure = self.exposure.clamp(0.25, 4.0);
        self
    }
}

pub fn missing_checker_rgba(w: u32, h: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let cell = ((x / 8) + (y / 8)) % 2 == 0;
            if cell {
                out.extend_from_slice(&[245, 0, 215, 255]);
            } else {
                out.extend_from_slice(&[24, 22, 30, 255]);
            }
        }
    }
    out
}

pub fn default_builtin_textures() -> Vec<TextureAsset> {
    vec![
        TextureAsset {
            id: 1,
            name: "Checker Grid".to_string(),
            source_uri: "nova://builtin/checker".to_string(),
            width: 128,
            height: 128,
            rgba: gen_checker_128(),
            status: TextureStatus::Builtin,
            version: 1,
        },
        TextureAsset {
            id: 2,
            name: "Brushed Metal".to_string(),
            source_uri: "nova://builtin/brushed_metal".to_string(),
            width: 128,
            height: 128,
            rgba: gen_brushed_metal_128(),
            status: TextureStatus::Builtin,
            version: 1,
        },
        TextureAsset {
            id: 3,
            name: "Brick Albedo".to_string(),
            source_uri: "nova://builtin/brick_albedo".to_string(),
            width: 128,
            height: 128,
            rgba: gen_brick_albedo_128(),
            status: TextureStatus::Builtin,
            version: 1,
        },
        TextureAsset {
            id: 4,
            name: "Brick Normal".to_string(),
            source_uri: "nova://builtin/brick_normal".to_string(),
            width: 128,
            height: 128,
            rgba: gen_brick_normal_128(),
            status: TextureStatus::Builtin,
            version: 1,
        },
        TextureAsset {
            id: 5,
            name: "Roughness Micro".to_string(),
            source_uri: "nova://builtin/roughness_noise".to_string(),
            width: 128,
            height: 128,
            rgba: gen_roughness_noise_128(),
            status: TextureStatus::Builtin,
            version: 1,
        },
        TextureAsset {
            id: 6,
            name: "Soft AO Grid".to_string(),
            source_uri: "nova://builtin/ao_vignette".to_string(),
            width: 128,
            height: 128,
            rgba: gen_ao_vignette_128(),
            status: TextureStatus::Builtin,
            version: 1,
        },
        TextureAsset {
            id: 7,
            name: "Emissive Tech Grid".to_string(),
            source_uri: "nova://builtin/emissive_grid".to_string(),
            width: 128,
            height: 128,
            rgba: gen_emissive_grid_128(),
            status: TextureStatus::Builtin,
            version: 1,
        },
    ]
}

fn hash2(x: u32, y: u32) -> f32 {
    let mut n = x.wrapping_mul(374761393).wrapping_add(y.wrapping_mul(668265263));
    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    ((n ^ (n >> 16)) & 0xFFFF) as f32 / 65535.0
}

fn gen_checker_128() -> Vec<u8> {
    let mut out = Vec::with_capacity(128 * 128 * 4);
    for y in 0..128u32 {
        for x in 0..128u32 {
            let border = (x % 32 == 0) || (y % 32 == 0);
            let even = ((x / 32) + (y / 32)) % 2 == 0;
            if border {
                out.extend_from_slice(&[55, 64, 78, 255]);
            } else if even {
                out.extend_from_slice(&[226, 232, 240, 255]);
            } else {
                out.extend_from_slice(&[116, 146, 184, 255]);
            }
        }
    }
    out
}

fn gen_brushed_metal_128() -> Vec<u8> {
    let mut out = Vec::with_capacity(128 * 128 * 4);
    for y in 0..128u32 {
        let row_streak = hash2(3, y) * 28.0;
        for x in 0..128u32 {
            let micro = hash2(x / 4, y) * 16.0;
            let v = (178.0 + row_streak + micro).clamp(140.0, 245.0) as u8;
            out.extend_from_slice(&[v, v, (v as f32 * 1.02).min(255.0) as u8, 255]);
        }
    }
    out
}

fn brick_mortar_mask(x: u32, y: u32) -> bool {
    let row = y / 32;
    let off_x = if row % 2 == 1 { 32 } else { 0 };
    let bx = (x + off_x) % 64;
    let by = y % 32;
    bx < 4 || by < 4
}

fn gen_brick_albedo_128() -> Vec<u8> {
    let mut out = Vec::with_capacity(128 * 128 * 4);
    for y in 0..128u32 {
        for x in 0..128u32 {
            if brick_mortar_mask(x, y) {
                let n = (hash2(x, y) * 18.0) as u8;
                out.extend_from_slice(&[165 + n, 160 + n, 152 + n, 255]);
            } else {
                let row = y / 32;
                let off_x = if row % 2 == 1 { 32 } else { 0 };
                let brick_id = ((x + off_x) / 64) + row * 4;
                let var = (hash2(brick_id, 9) - 0.5) * 32.0;
                let gr = hash2(x, y) * 18.0;
                let r = (178.0 + var + gr).clamp(90.0, 235.0) as u8;
                let g = (78.0 + var * 0.4 + gr * 0.6).clamp(40.0, 145.0) as u8;
                let b = (56.0 + var * 0.3 + gr * 0.5).clamp(30.0, 115.0) as u8;
                out.extend_from_slice(&[r, g, b, 255]);
            }
        }
    }
    out
}

fn gen_brick_normal_128() -> Vec<u8> {
    let mut out = Vec::with_capacity(128 * 128 * 4);
    let height_at = |x: u32, y: u32| -> f32 {
        let row = y / 32;
        let off_x = if row % 2 == 1 { 32 } else { 0 };
        let bx = (x + off_x) % 64;
        let by = y % 32;
        if bx < 4 || by < 4 {
            0.0
        } else if bx < 7 || bx > 60 || by < 7 || by > 28 {
            0.65
        } else {
            1.0
        }
    };
    for y in 0..128u32 {
        for x in 0..128u32 {
            let hl = height_at((x + 127) % 128, y);
            let hr = height_at((x + 1) % 128, y);
            let hd = height_at(x, (y + 127) % 128);
            let hu = height_at(x, (y + 1) % 128);
            let dx = (hl - hr) * 1.8;
            let dy = (hd - hu) * 1.8;
            let dz = 1.0f32;
            let len = (dx * dx + dy * dy + dz * dz).sqrt();
            let nx = ((dx / len) * 0.5 + 0.5).clamp(0.0, 1.0) * 255.0;
            let ny = ((dy / len) * 0.5 + 0.5).clamp(0.0, 1.0) * 255.0;
            let nz = ((dz / len) * 0.5 + 0.5).clamp(0.0, 1.0) * 255.0;
            out.extend_from_slice(&[nx as u8, ny as u8, nz as u8, 255]);
        }
    }
    out
}

fn gen_roughness_noise_128() -> Vec<u8> {
    let mut out = Vec::with_capacity(128 * 128 * 4);
    for y in 0..128u32 {
        for x in 0..128u32 {
            let coarse = hash2(x / 8, y / 8);
            let fine = hash2(x, y);
            let v = ((0.25 + coarse * 0.52 + fine * 0.23) * 255.0).clamp(32.0, 245.0) as u8;
            out.extend_from_slice(&[v, v, v, 255]);
        }
    }
    out
}

fn gen_ao_vignette_128() -> Vec<u8> {
    let mut out = Vec::with_capacity(128 * 128 * 4);
    for y in 0..128u32 {
        for x in 0..128u32 {
            let ux = (x % 64) as f32 / 64.0 - 0.5;
            let uy = (y % 64) as f32 / 64.0 - 0.5;
            let d = (ux * ux + uy * uy).sqrt() * 2.0;
            let ao = (1.0 - d * d * 0.55).clamp(0.35, 1.0);
            let v = (ao * 255.0) as u8;
            out.extend_from_slice(&[v, v, v, 255]);
        }
    }
    out
}

fn gen_emissive_grid_128() -> Vec<u8> {
    let mut out = Vec::with_capacity(128 * 128 * 4);
    for y in 0..128u32 {
        for x in 0..128u32 {
            let gx = x % 32;
            let gy = y % 32;
            let line = gx < 3 || gy < 3 || (gx > 13 && gx < 18 && gy > 13 && gy < 18);
            if line {
                out.extend_from_slice(&[90, 235, 255, 255]);
            } else {
                out.extend_from_slice(&[8, 16, 26, 255]);
            }
        }
    }
    out
}

pub fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(((input.len() + 2) / 3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((n >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[((n >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(n & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

pub fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    let bytes: Vec<u8> = input.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if bytes.len() % 4 != 0 {
        return Err("Invalid base64 length".to_string());
    }
    let val = |c: u8| -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some((c - b'A') as u32),
            b'a'..=b'z' => Some((c - b'a' + 26) as u32),
            b'0'..=b'9' => Some((c - b'0' + 52) as u32),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    };
    let mut out = Vec::with_capacity((bytes.len() / 4) * 3);
    for chunk in bytes.chunks_exact(4) {
        let c0 = val(chunk[0]).ok_or_else(|| "Invalid base64 char".to_string())?;
        let c1 = val(chunk[1]).ok_or_else(|| "Invalid base64 char".to_string())?;
        let pad2 = chunk[2] == b'=';
        let pad3 = chunk[3] == b'=';
        let c2 = if pad2 {
            0
        } else {
            val(chunk[2]).ok_or_else(|| "Invalid base64 char".to_string())?
        };
        let c3 = if pad3 {
            0
        } else {
            val(chunk[3]).ok_or_else(|| "Invalid base64 char".to_string())?
        };
        let n = (c0 << 18) | (c1 << 12) | (c2 << 6) | c3;
        out.push(((n >> 16) & 0xFF) as u8);
        if !pad2 {
            out.push(((n >> 8) & 0xFF) as u8);
        }
        if !pad3 {
            out.push((n & 0xFF) as u8);
        }
    }
    Ok(out)
}
