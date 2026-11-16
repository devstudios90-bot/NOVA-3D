//! Procedural geometric primitives matching PRD 8.4:
//! Cube, UV sphere, Icosphere, Cylinder, Cone, Torus, Plane, Circle,
//! plus metadata-backed Empty, Camera, and Light node types.

use std::collections::HashMap;
use std::f32::consts::{PI, TAU};

use crate::math::Vec3;
use crate::scene::MeshData;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveType {
    Cube = 0,
    UvSphere = 1,
    Icosphere = 2,
    Cylinder = 3,
    Cone = 4,
    Torus = 5,
    Plane = 6,
    Circle = 7,
    Empty = 8,
    Camera = 9,
    Light = 10,
}

impl PrimitiveType {
    pub fn from_u32(v: u32) -> Option<Self> {
        match v {
            0 => Some(Self::Cube),
            1 => Some(Self::UvSphere),
            2 => Some(Self::Icosphere),
            3 => Some(Self::Cylinder),
            4 => Some(Self::Cone),
            5 => Some(Self::Torus),
            6 => Some(Self::Plane),
            7 => Some(Self::Circle),
            8 => Some(Self::Empty),
            9 => Some(Self::Camera),
            10 => Some(Self::Light),
            _ => None,
        }
    }

    pub fn default_name(self) -> &'static str {
        match self {
            Self::Cube => "Cube",
            Self::UvSphere => "UV Sphere",
            Self::Icosphere => "Icosphere",
            Self::Cylinder => "Cylinder",
            Self::Cone => "Cone",
            Self::Torus => "Torus",
            Self::Plane => "Plane",
            Self::Circle => "Circle",
            Self::Empty => "Empty",
            Self::Camera => "Camera",
            Self::Light => "Light",
        }
    }
}

fn finish_mesh(
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
) -> MeshData {
    let (bounds_min, bounds_max) = MeshData::compute_bounds(&positions);
    MeshData {
        positions,
        normals,
        uvs,
        indices,
        bounds_min,
        bounds_max,
        version: 1,
    }
}

pub fn build_primitive_mesh(kind: PrimitiveType) -> Option<MeshData> {
    match kind {
        PrimitiveType::Cube => Some(MeshData::studio_cube(1.2)),
        PrimitiveType::UvSphere => Some(uv_sphere(0.75, 24, 16)),
        PrimitiveType::Icosphere => Some(icosphere(0.75, 2)),
        PrimitiveType::Cylinder => Some(cylinder(0.6, 1.4, 24)),
        PrimitiveType::Cone => Some(cone(0.7, 1.4, 24)),
        PrimitiveType::Torus => Some(torus(0.68, 0.24, 28, 16)),
        PrimitiveType::Plane => Some(plane(2.0, 2.0)),
        PrimitiveType::Circle => Some(circle(0.85, 28)),
        PrimitiveType::Empty | PrimitiveType::Camera | PrimitiveType::Light => None,
    }
}

pub fn uv_sphere(radius: f32, segments: u32, rings: u32) -> MeshData {
    let segs = segments.max(6);
    let rgs = rings.max(4);
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    for y in 0..=rgs {
        let v = y as f32 / rgs as f32;
        let phi = v * PI;
        let sin_p = phi.sin();
        let cos_p = phi.cos();
        for x in 0..=segs {
            let u = x as f32 / segs as f32;
            let theta = u * TAU;
            let nx = sin_p * theta.cos();
            let ny = cos_p;
            let nz = sin_p * theta.sin();
            positions.push([nx * radius, ny * radius, nz * radius]);
            normals.push([nx, ny, nz]);
            uvs.push([u, 1.0 - v]);
        }
    }

    let stride = segs + 1;
    for y in 0..rgs {
        for x in 0..segs {
            let i0 = y * stride + x;
            let i1 = i0 + stride;
            indices.extend_from_slice(&[i0, i0 + 1, i1, i0 + 1, i1 + 1, i1]);
        }
    }

    finish_mesh(positions, normals, uvs, indices)
}

pub fn icosphere(radius: f32, subdivisions: u32) -> MeshData {
    let t = (1.0 + 5.0_f32.sqrt()) * 0.5;
    let mut verts: Vec<Vec3> = vec![
        Vec3::new(-1.0, t, 0.0),
        Vec3::new(1.0, t, 0.0),
        Vec3::new(-1.0, -t, 0.0),
        Vec3::new(1.0, -t, 0.0),
        Vec3::new(0.0, -1.0, t),
        Vec3::new(0.0, 1.0, t),
        Vec3::new(0.0, -1.0, -t),
        Vec3::new(0.0, 1.0, -t),
        Vec3::new(t, 0.0, -1.0),
        Vec3::new(t, 0.0, 1.0),
        Vec3::new(-t, 0.0, -1.0),
        Vec3::new(-t, 0.0, 1.0),
    ]
    .into_iter()
    .map(|v| v.normalized_or(Vec3::UNIT_Y))
    .collect();

    let mut tris: Vec<[u32; 3]> = vec![
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ];

    for _ in 0..subdivisions.min(3) {
        let mut cache: HashMap<(u32, u32), u32> = HashMap::new();
        let mut next_tris = Vec::with_capacity(tris.len() * 4);
        let mut midpoint = |a: u32, b: u32| -> u32 {
            let key = if a < b { (a, b) } else { (b, a) };
            if let Some(&idx) = cache.get(&key) {
                return idx;
            }
            let mid = (verts[a as usize] + verts[b as usize]).normalized_or(Vec3::UNIT_Y);
            let idx = verts.len() as u32;
            verts.push(mid);
            cache.insert(key, idx);
            idx
        };

        for [a, b, c] in tris {
            let ab = midpoint(a, b);
            let bc = midpoint(b, c);
            let ca = midpoint(c, a);
            next_tris.push([a, ab, ca]);
            next_tris.push([b, bc, ab]);
            next_tris.push([c, ca, bc]);
            next_tris.push([ab, bc, ca]);
        }
        tris = next_tris;
    }

    let mut positions = Vec::with_capacity(verts.len());
    let mut normals = Vec::with_capacity(verts.len());
    let mut uvs = Vec::with_capacity(verts.len());
    for n in verts {
        positions.push([n.x * radius, n.y * radius, n.z * radius]);
        normals.push([n.x, n.y, n.z]);
        let u = 0.5 + n.z.atan2(n.x) / TAU;
        let v = 0.5 - n.y.clamp(-1.0, 1.0).asin() / PI;
        uvs.push([u, v]);
    }
    let mut indices = Vec::with_capacity(tris.len() * 3);
    for [a, b, c] in tris {
        indices.extend_from_slice(&[a, b, c]);
    }

    finish_mesh(positions, normals, uvs, indices)
}

pub fn cylinder(radius: f32, height: f32, segments: u32) -> MeshData {
    let segs = segments.max(6);
    let half_h = height * 0.5;
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    // Side wall
    for i in 0..=segs {
        let u = i as f32 / segs as f32;
        let theta = u * TAU;
        let nx = theta.cos();
        let nz = theta.sin();
        positions.push([nx * radius, half_h, nz * radius]);
        normals.push([nx, 0.0, nz]);
        uvs.push([u, 1.0]);

        positions.push([nx * radius, -half_h, nz * radius]);
        normals.push([nx, 0.0, nz]);
        uvs.push([u, 0.0]);
    }
    for i in 0..segs {
        let t0 = i * 2;
        let b0 = t0 + 1;
        let t1 = t0 + 2;
        let b1 = t0 + 3;
        indices.extend_from_slice(&[t0, t1, b0, t1, b1, b0]);
    }

    // Top cap
    let top_center = positions.len() as u32;
    positions.push([0.0, half_h, 0.0]);
    normals.push([0.0, 1.0, 0.0]);
    uvs.push([0.5, 0.5]);
    let top_ring_start = positions.len() as u32;
    for i in 0..=segs {
        let theta = (i as f32 / segs as f32) * TAU;
        let c = theta.cos();
        let s = theta.sin();
        positions.push([c * radius, half_h, s * radius]);
        normals.push([0.0, 1.0, 0.0]);
        uvs.push([0.5 + c * 0.5, 0.5 + s * 0.5]);
    }
    for i in 0..segs {
        indices.extend_from_slice(&[top_center, top_ring_start + i + 1, top_ring_start + i]);
    }

    // Bottom cap
    let bot_center = positions.len() as u32;
    positions.push([0.0, -half_h, 0.0]);
    normals.push([0.0, -1.0, 0.0]);
    uvs.push([0.5, 0.5]);
    let bot_ring_start = positions.len() as u32;
    for i in 0..=segs {
        let theta = (i as f32 / segs as f32) * TAU;
        let c = theta.cos();
        let s = theta.sin();
        positions.push([c * radius, -half_h, s * radius]);
        normals.push([0.0, -1.0, 0.0]);
        uvs.push([0.5 + c * 0.5, 0.5 - s * 0.5]);
    }
    for i in 0..segs {
        indices.extend_from_slice(&[bot_center, bot_ring_start + i, bot_ring_start + i + 1]);
    }

    finish_mesh(positions, normals, uvs, indices)
}

pub fn cone(radius: f32, height: f32, segments: u32) -> MeshData {
    let segs = segments.max(6);
    let half_h = height * 0.5;
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    let slope_len = (radius * radius + height * height).sqrt();
    let ny = radius / slope_len;
    let nr = height / slope_len;

    for i in 0..segs {
        let t0 = (i as f32 / segs as f32) * TAU;
        let t1 = ((i + 1) as f32 / segs as f32) * TAU;
        let tm = 0.5 * (t0 + t1);
        let base_idx = positions.len() as u32;

        positions.push([0.0, half_h, 0.0]);
        normals.push([tm.cos() * nr, ny, tm.sin() * nr]);
        uvs.push([0.5, 1.0]);

        positions.push([t1.cos() * radius, -half_h, t1.sin() * radius]);
        normals.push([t1.cos() * nr, ny, t1.sin() * nr]);
        uvs.push([(i + 1) as f32 / segs as f32, 0.0]);

        positions.push([t0.cos() * radius, -half_h, t0.sin() * radius]);
        normals.push([t0.cos() * nr, ny, t0.sin() * nr]);
        uvs.push([i as f32 / segs as f32, 0.0]);

        indices.extend_from_slice(&[base_idx, base_idx + 1, base_idx + 2]);
    }

    let bot_center = positions.len() as u32;
    positions.push([0.0, -half_h, 0.0]);
    normals.push([0.0, -1.0, 0.0]);
    uvs.push([0.5, 0.5]);
    let bot_ring_start = positions.len() as u32;
    for i in 0..=segs {
        let theta = (i as f32 / segs as f32) * TAU;
        let c = theta.cos();
        let s = theta.sin();
        positions.push([c * radius, -half_h, s * radius]);
        normals.push([0.0, -1.0, 0.0]);
        uvs.push([0.5 + c * 0.5, 0.5 - s * 0.5]);
    }
    for i in 0..segs {
        indices.extend_from_slice(&[bot_center, bot_ring_start + i, bot_ring_start + i + 1]);
    }

    finish_mesh(positions, normals, uvs, indices)
}

pub fn torus(major_r: f32, minor_r: f32, major_segs: u32, minor_segs: u32) -> MeshData {
    let maj = major_segs.max(8);
    let min = minor_segs.max(6);
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    for i in 0..=maj {
        let u = i as f32 / maj as f32;
        let phi = u * TAU;
        let cp = phi.cos();
        let sp = phi.sin();
        for j in 0..=min {
            let v = j as f32 / min as f32;
            let theta = v * TAU;
            let ct = theta.cos();
            let st = theta.sin();

            let nx = ct * cp;
            let ny = st;
            let nz = ct * sp;
            let ring = major_r + minor_r * ct;
            positions.push([ring * cp, minor_r * st, ring * sp]);
            normals.push([nx, ny, nz]);
            uvs.push([u, v]);
        }
    }

    let stride = min + 1;
    for i in 0..maj {
        for j in 0..min {
            let a = i * stride + j;
            let b = a + stride;
            indices.extend_from_slice(&[a, b, a + 1, b, b + 1, a + 1]);
        }
    }

    finish_mesh(positions, normals, uvs, indices)
}

pub fn plane(width: f32, depth: f32) -> MeshData {
    let hw = width * 0.5;
    let hd = depth * 0.5;
    let positions = vec![
        [-hw, 0.0, hd],
        [hw, 0.0, hd],
        [hw, 0.0, -hd],
        [-hw, 0.0, -hd],
    ];
    let normals = vec![[0.0, 1.0, 0.0]; 4];
    let uvs = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let indices = vec![0, 1, 2, 0, 2, 3];
    finish_mesh(positions, normals, uvs, indices)
}

pub fn circle(radius: f32, segments: u32) -> MeshData {
    let segs = segments.max(6);
    let mut positions = Vec::with_capacity((segs + 2) as usize);
    let mut normals = Vec::with_capacity((segs + 2) as usize);
    let mut uvs = Vec::with_capacity((segs + 2) as usize);
    let mut indices = Vec::with_capacity((segs * 3) as usize);

    positions.push([0.0, 0.0, 0.0]);
    normals.push([0.0, 1.0, 0.0]);
    uvs.push([0.5, 0.5]);

    for i in 0..=segs {
        let theta = (i as f32 / segs as f32) * TAU;
        let c = theta.cos();
        let s = theta.sin();
        positions.push([c * radius, 0.0, s * radius]);
        normals.push([0.0, 1.0, 0.0]);
        uvs.push([0.5 + c * 0.5, 0.5 - s * 0.5]);
    }
    for i in 0..segs {
        indices.extend_from_slice(&[0, 1 + i + 1, 1 + i]);
    }

    finish_mesh(positions, normals, uvs, indices)
}
