//! Topological polygon mesh representation (`PolyMesh`), Edit Mode operations (MD-1..MD-6),
//! and non-destructive Modifier Stack evaluation (MD-7).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::f32::consts::{PI, TAU};

use crate::math::Vec3;
use crate::primitives::PrimitiveType;
use crate::scene::MeshData;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionMode {
    Object = 0,
    Edit = 1,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditSelectMode {
    Vertex = 0,
    Edge = 1,
    Face = 2,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModifierSpec {
    Mirror { axis: u8, merge: bool, enabled: bool },
    Subdivision { levels: u8, enabled: bool },
    Solidify { thickness: f32, enabled: bool },
    Array { count: u32, offset: [f32; 3], enabled: bool },
}

impl ModifierSpec {
    pub fn kind_code(&self) -> u32 {
        match self {
            ModifierSpec::Mirror { .. } => 0,
            ModifierSpec::Subdivision { .. } => 1,
            ModifierSpec::Solidify { .. } => 2,
            ModifierSpec::Array { .. } => 3,
        }
    }

    pub fn is_enabled(&self) -> bool {
        match *self {
            ModifierSpec::Mirror { enabled, .. }
            | ModifierSpec::Subdivision { enabled, .. }
            | ModifierSpec::Solidify { enabled, .. }
            | ModifierSpec::Array { enabled, .. } => enabled,
        }
    }
}

fn triplanar_uv(p: Vec3, n: Vec3) -> [f32; 2] {
    let ax = n.x.abs();
    let ay = n.y.abs();
    let az = n.z.abs();
    if ax >= ay && ax >= az {
        [p.z + 0.5, p.y + 0.5]
    } else if ay >= ax && ay >= az {
        [p.x + 0.5, p.z + 0.5]
    } else {
        [p.x + 0.5, p.y + 0.5]
    }
}

pub fn canonical_edge(a: usize, b: usize) -> (usize, usize) {
    if a < b { (a, b) } else { (b, a) }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolyMesh {
    pub vertices: Vec<Vec3>,
    pub faces: Vec<Vec<usize>>,
    pub smooth_shading: bool,
}

impl PolyMesh {
    pub fn cube(size: f32) -> Self {
        let h = size * 0.5;
        let vertices = vec![
            Vec3::new(-h, -h,  h), // 0
            Vec3::new( h, -h,  h), // 1
            Vec3::new( h,  h,  h), // 2
            Vec3::new(-h,  h,  h), // 3
            Vec3::new(-h, -h, -h), // 4
            Vec3::new( h, -h, -h), // 5
            Vec3::new( h,  h, -h), // 6
            Vec3::new(-h,  h, -h), // 7
        ];
        let faces = vec![
            vec![0, 1, 2, 3], // +Z front
            vec![5, 4, 7, 6], // -Z back
            vec![1, 5, 6, 2], // +X right
            vec![4, 0, 3, 7], // -X left
            vec![3, 2, 6, 7], // +Y top
            vec![4, 5, 1, 0], // -Y bottom
        ];
        Self {
            vertices,
            faces,
            smooth_shading: false,
        }
    }

    pub fn plane(width: f32, depth: f32) -> Self {
        let hx = width * 0.5;
        let hz = depth * 0.5;
        Self {
            vertices: vec![
                Vec3::new(-hx, 0.0,  hz),
                Vec3::new( hx, 0.0,  hz),
                Vec3::new( hx, 0.0, -hz),
                Vec3::new(-hx, 0.0, -hz),
            ],
            faces: vec![vec![0, 1, 2, 3]],
            smooth_shading: false,
        }
    }

    pub fn circle(radius: f32, segments: u32) -> Self {
        let segs = segments.max(6) as usize;
        let mut vertices = Vec::with_capacity(segs);
        let mut face = Vec::with_capacity(segs);
        for i in 0..segs {
            let theta = (i as f32 / segs as f32) * TAU;
            vertices.push(Vec3::new(radius * theta.cos(), 0.0, -radius * theta.sin()));
            face.push(i);
        }
        Self {
            vertices,
            faces: vec![face],
            smooth_shading: false,
        }
    }

    pub fn cylinder(radius: f32, height: f32, segments: u32) -> Self {
        let segs = segments.max(6) as usize;
        let hy = height * 0.5;
        let mut vertices = Vec::with_capacity(segs * 2);
        for i in 0..segs {
            let theta = (i as f32 / segs as f32) * TAU;
            let x = radius * theta.cos();
            let z = -radius * theta.sin();
            vertices.push(Vec3::new(x, -hy, z)); // 2*i bottom
            vertices.push(Vec3::new(x,  hy, z)); // 2*i+1 top
        }
        let mut faces = Vec::with_capacity(segs + 2);
        let mut top = Vec::with_capacity(segs);
        let mut bot = Vec::with_capacity(segs);
        for i in 0..segs {
            let next = (i + 1) % segs;
            faces.push(vec![2 * i, 2 * next, 2 * next + 1, 2 * i + 1]);
            top.push(2 * i + 1);
            bot.push(2 * (segs - 1 - i));
        }
        faces.push(top);
        faces.push(bot);
        Self {
            vertices,
            faces,
            smooth_shading: true,
        }
    }

    pub fn cone(radius: f32, height: f32, segments: u32) -> Self {
        let segs = segments.max(6) as usize;
        let hy = height * 0.5;
        let mut vertices = Vec::with_capacity(segs + 1);
        for i in 0..segs {
            let theta = (i as f32 / segs as f32) * TAU;
            vertices.push(Vec3::new(radius * theta.cos(), -hy, -radius * theta.sin()));
        }
        let apex = vertices.len();
        vertices.push(Vec3::new(0.0, hy, 0.0));
        let mut faces = Vec::with_capacity(segs + 1);
        let mut bot = Vec::with_capacity(segs);
        for i in 0..segs {
            let next = (i + 1) % segs;
            faces.push(vec![i, next, apex]);
            bot.push(segs - 1 - i);
        }
        faces.push(bot);
        Self {
            vertices,
            faces,
            smooth_shading: false,
        }
    }

    pub fn uv_sphere(radius: f32, segments: u32, rings: u32) -> Self {
        let segs = segments.max(6) as usize;
        let rgs = rings.max(4) as usize;
        let mut vertices = Vec::new();
        vertices.push(Vec3::new(0.0, radius, 0.0)); // top pole = 0
        for r in 1..rgs {
            let phi = (r as f32 / rgs as f32) * PI;
            let y = radius * phi.cos();
            let rr = radius * phi.sin();
            for s in 0..segs {
                let theta = (s as f32 / segs as f32) * TAU;
                vertices.push(Vec3::new(rr * theta.cos(), y, -rr * theta.sin()));
            }
        }
        let bot_pole = vertices.len();
        vertices.push(Vec3::new(0.0, -radius, 0.0));

        let mut faces = Vec::new();
        // Top cap triangles
        for s in 0..segs {
            let next = (s + 1) % segs;
            faces.push(vec![0, 1 + s, 1 + next]);
        }
        // Middle quad rings
        for r in 0..(rgs - 2) {
            let row0 = 1 + r * segs;
            let row1 = row0 + segs;
            for s in 0..segs {
                let next = (s + 1) % segs;
                faces.push(vec![row0 + s, row1 + s, row1 + next, row0 + next]);
            }
        }
        // Bottom cap triangles
        let last_row = 1 + (rgs - 2) * segs;
        for s in 0..segs {
            let next = (s + 1) % segs;
            faces.push(vec![last_row + s, bot_pole, last_row + next]);
        }
        Self {
            vertices,
            faces,
            smooth_shading: true,
        }
    }

    pub fn torus(major_r: f32, minor_r: f32, major_segs: u32, minor_segs: u32) -> Self {
        let maj = major_segs.max(8) as usize;
        let min = minor_segs.max(6) as usize;
        let mut vertices = Vec::with_capacity(maj * min);
        for i in 0..maj {
            let u = (i as f32 / maj as f32) * TAU;
            let cu = u.cos();
            let su = u.sin();
            for j in 0..min {
                let v = (j as f32 / min as f32) * TAU;
                let cv = v.cos();
                let sv = v.sin();
                let r = major_r + minor_r * cv;
                vertices.push(Vec3::new(r * cu, minor_r * sv, -r * su));
            }
        }
        let mut faces = Vec::with_capacity(maj * min);
        for i in 0..maj {
            let ni = (i + 1) % maj;
            for j in 0..min {
                let nj = (j + 1) % min;
                faces.push(vec![
                    i * min + j,
                    ni * min + j,
                    ni * min + nj,
                    i * min + nj,
                ]);
            }
        }
        Self {
            vertices,
            faces,
            smooth_shading: true,
        }
    }

    pub fn from_primitive(kind: PrimitiveType) -> Option<Self> {
        match kind {
            PrimitiveType::Cube => Some(Self::cube(1.2)),
            PrimitiveType::UvSphere => Some(Self::uv_sphere(0.75, 20, 14)),
            PrimitiveType::Icosphere => {
                let m = crate::primitives::icosphere(0.75, 2);
                Some(Self::from_mesh_data_fallback(&m, true))
            }
            PrimitiveType::Cylinder => Some(Self::cylinder(0.6, 1.4, 20)),
            PrimitiveType::Cone => Some(Self::cone(0.7, 1.4, 20)),
            PrimitiveType::Torus => Some(Self::torus(0.68, 0.24, 24, 14)),
            PrimitiveType::Plane => Some(Self::plane(2.0, 2.0)),
            PrimitiveType::Circle => Some(Self::circle(0.85, 24)),
            PrimitiveType::Empty | PrimitiveType::Camera | PrimitiveType::Light => None,
        }
    }

    pub fn from_mesh_data_fallback(mesh: &MeshData, smooth_hint: bool) -> Self {
        let mut key_to_idx: HashMap<(i64, i64, i64), usize> = HashMap::new();
        let mut vertices = Vec::new();
        let mut remap = Vec::with_capacity(mesh.positions.len());

        for p in &mesh.positions {
            let key = (
                (p[0] * 10_000.0).round() as i64,
                (p[1] * 10_000.0).round() as i64,
                (p[2] * 10_000.0).round() as i64,
            );
            let idx = *key_to_idx.entry(key).or_insert_with(|| {
                let n = vertices.len();
                vertices.push(Vec3::new(p[0], p[1], p[2]));
                n
            });
            remap.push(idx);
        }

        let mut faces = Vec::with_capacity(mesh.indices.len() / 3);
        for tri in mesh.indices.chunks_exact(3) {
            let a = remap[tri[0] as usize];
            let b = remap[tri[1] as usize];
            let c = remap[tri[2] as usize];
            if a != b && b != c && c != a {
                faces.push(vec![a, b, c]);
            }
        }
        Self {
            vertices,
            faces,
            smooth_shading: smooth_hint,
        }
    }

    pub fn face_normal(&self, face_idx: usize) -> Vec3 {
        let f = match self.faces.get(face_idx) {
            Some(f) if f.len() >= 3 => f,
            _ => return Vec3::UNIT_Y,
        };
        let mut n = Vec3::ZERO;
        for i in 0..f.len() {
            let a = self.vertices[f[i]];
            let b = self.vertices[f[(i + 1) % f.len()]];
            n = n + Vec3::new(
                (a.y - b.y) * (a.z + b.z),
                (a.z - b.z) * (a.x + b.x),
                (a.x - b.x) * (a.y + b.y),
            );
        }
        n.normalized_or(Vec3::UNIT_Y)
    }

    pub fn face_area_normal(&self, face_idx: usize) -> Vec3 {
        let f = match self.faces.get(face_idx) {
            Some(f) if f.len() >= 3 => f,
            _ => return Vec3::ZERO,
        };
        let p0 = self.vertices[f[0]];
        let mut sum = Vec3::ZERO;
        for i in 1..(f.len() - 1) {
            let e1 = self.vertices[f[i]] - p0;
            let e2 = self.vertices[f[i + 1]] - p0;
            sum = sum + e1.cross(e2);
        }
        sum * 0.5
    }

    pub fn face_centroid(&self, face_idx: usize) -> Vec3 {
        let f = match self.faces.get(face_idx) {
            Some(f) if !f.is_empty() => f,
            _ => return Vec3::ZERO,
        };
        let mut c = Vec3::ZERO;
        for &vi in f {
            c = c + self.vertices[vi];
        }
        c * (1.0 / f.len() as f32)
    }

    pub fn unique_edges(&self) -> Vec<(usize, usize)> {
        let mut set = BTreeSet::new();
        for f in &self.faces {
            if f.len() < 2 {
                continue;
            }
            for i in 0..f.len() {
                let a = f[i];
                let b = f[(i + 1) % f.len()];
                if a != b {
                    set.insert(canonical_edge(a, b));
                }
            }
        }
        set.into_iter().collect()
    }

    /// Returns `(mesh_data, tri_to_face)` where `tri_to_face[t]` gives the polygon face index for triangle `t`.
    pub fn to_mesh_data_mapped(&self, version: u64) -> (MeshData, Vec<usize>) {
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        let mut indices = Vec::new();
        let mut tri_to_face = Vec::new();

        if self.faces.is_empty() || self.vertices.is_empty() {
            return (
                MeshData {
                    positions,
                    normals,
                    uvs,
                    indices,
                    bounds_min: Vec3::ZERO,
                    bounds_max: Vec3::ZERO,
                    version,
                },
                tri_to_face,
            );
        }

        if self.smooth_shading {
            let mut vnormals = vec![Vec3::ZERO; self.vertices.len()];
            for fi in 0..self.faces.len() {
                let an = self.face_area_normal(fi);
                for &vi in &self.faces[fi] {
                    vnormals[vi] = vnormals[vi] + an;
                }
            }
            for n in &mut vnormals {
                *n = n.normalized_or(Vec3::UNIT_Y);
            }
            for (vi, p) in self.vertices.iter().enumerate() {
                positions.push(p.to_array());
                normals.push(vnormals[vi].to_array());
                uvs.push(triplanar_uv(*p, vnormals[vi]));
            }
            for (fi, f) in self.faces.iter().enumerate() {
                if f.len() < 3 {
                    continue;
                }
                for tri in triangulate_polygon(&self.vertices, f) {
                    indices.push(tri[0] as u32);
                    indices.push(tri[1] as u32);
                    indices.push(tri[2] as u32);
                    tri_to_face.push(fi);
                }
            }
        } else {
            for (fi, f) in self.faces.iter().enumerate() {
                if f.len() < 3 {
                    continue;
                }
                let fn_ = self.face_normal(fi);
                let n_arr = fn_.to_array();
                let base = positions.len() as u32;
                for &vi in f {
                    let p = self.vertices[vi];
                    positions.push(p.to_array());
                    normals.push(n_arr);
                    uvs.push(triplanar_uv(p, fn_));
                }
                for tri in triangulate_polygon(&self.vertices, f) {
                    let i0 = f.iter().position(|&x| x == tri[0]).unwrap_or(0) as u32;
                    let i1 = f.iter().position(|&x| x == tri[1]).unwrap_or(0) as u32;
                    let i2 = f.iter().position(|&x| x == tri[2]).unwrap_or(0) as u32;
                    indices.push(base + i0);
                    indices.push(base + i1);
                    indices.push(base + i2);
                    tri_to_face.push(fi);
                }
            }
        }

        let (bounds_min, bounds_max) = MeshData::compute_bounds(&positions);
        (
            MeshData {
                positions,
                normals,
                uvs,
                indices,
                bounds_min,
                bounds_max,
                version,
            },
            tri_to_face,
        )
    }

    pub fn to_mesh_data(&self, version: u64) -> MeshData {
        self.to_mesh_data_mapped(version).0
    }

    pub fn clean_faces(&mut self) {
        for f in &mut self.faces {
            let mut dedup: Vec<usize> = Vec::with_capacity(f.len());
            for &v in f.iter() {
                if dedup.last().copied() != Some(v) {
                    dedup.push(v);
                }
            }
            if dedup.len() > 1 && dedup.first() == dedup.last() {
                dedup.pop();
            }
            *f = dedup;
        }
        self.faces.retain(|f| f.len() >= 3);
    }

    pub fn prune_unused_vertices(
        &mut self,
        sel_verts: &mut BTreeSet<usize>,
        sel_edges: &mut BTreeSet<(usize, usize)>,
    ) {
        let mut used = vec![false; self.vertices.len()];
        for f in &self.faces {
            for &vi in f {
                if vi < used.len() {
                    used[vi] = true;
                }
            }
        }
        let mut new_verts = Vec::with_capacity(self.vertices.len());
        let mut old_to_new = vec![usize::MAX; self.vertices.len()];
        for (i, &u) in used.iter().enumerate() {
            if u {
                old_to_new[i] = new_verts.len();
                new_verts.push(self.vertices[i]);
            }
        }
        self.vertices = new_verts;
        for f in &mut self.faces {
            for vi in f.iter_mut() {
                *vi = old_to_new[*vi];
            }
        }
        *sel_verts = sel_verts
            .iter()
            .filter_map(|&v| old_to_new.get(v).copied().filter(|&nv| nv != usize::MAX))
            .collect();
        *sel_edges = sel_edges
            .iter()
            .filter_map(|&(a, b)| {
                let na = old_to_new.get(a).copied().filter(|&nv| nv != usize::MAX)?;
                let nb = old_to_new.get(b).copied().filter(|&nv| nv != usize::MAX)?;
                if na != nb {
                    Some(canonical_edge(na, nb))
                } else {
                    None
                }
            })
            .collect();
    }

    pub fn selected_vertex_closure(
        &self,
        mode: EditSelectMode,
        sel_v: &BTreeSet<usize>,
        sel_e: &BTreeSet<(usize, usize)>,
        sel_f: &BTreeSet<usize>,
    ) -> BTreeSet<usize> {
        let mut out = BTreeSet::new();
        match mode {
            EditSelectMode::Vertex => {
                for &v in sel_v {
                    if v < self.vertices.len() {
                        out.insert(v);
                    }
                }
            }
            EditSelectMode::Edge => {
                for &(a, b) in sel_e {
                    if a < self.vertices.len() {
                        out.insert(a);
                    }
                    if b < self.vertices.len() {
                        out.insert(b);
                    }
                }
            }
            EditSelectMode::Face => {
                for &fi in sel_f {
                    if let Some(f) = self.faces.get(fi) {
                        for &v in f {
                            out.insert(v);
                        }
                    }
                }
            }
        }
        out
    }

    pub fn sync_selection_from_faces(
        &self,
        sel_f: &BTreeSet<usize>,
        sel_v: &mut BTreeSet<usize>,
        sel_e: &mut BTreeSet<(usize, usize)>,
    ) {
        sel_v.clear();
        sel_e.clear();
        for &fi in sel_f {
            if let Some(f) = self.faces.get(fi) {
                for i in 0..f.len() {
                    let a = f[i];
                    let b = f[(i + 1) % f.len()];
                    sel_v.insert(a);
                    sel_e.insert(canonical_edge(a, b));
                }
            }
        }
    }

    pub fn extrude(
        &mut self,
        mode: EditSelectMode,
        distance: f32,
        sel_v: &mut BTreeSet<usize>,
        sel_e: &mut BTreeSet<(usize, usize)>,
        sel_f: &mut BTreeSet<usize>,
    ) -> Result<(), &'static str> {
        let effective_mode = if !sel_f.is_empty() && mode == EditSelectMode::Face {
            EditSelectMode::Face
        } else if !sel_e.is_empty() && mode == EditSelectMode::Edge {
            EditSelectMode::Edge
        } else if !sel_v.is_empty() && mode == EditSelectMode::Vertex {
            EditSelectMode::Vertex
        } else if !sel_f.is_empty() {
            EditSelectMode::Face
        } else if !sel_e.is_empty() {
            EditSelectMode::Edge
        } else if !sel_v.is_empty() {
            EditSelectMode::Vertex
        } else {
            return Err("Select at least one vertex, edge, or face to extrude");
        };

        match effective_mode {
            EditSelectMode::Face => {
                let valid_faces: BTreeSet<usize> = sel_f
                    .iter()
                    .copied()
                    .filter(|&fi| fi < self.faces.len())
                    .collect();
                if valid_faces.is_empty() {
                    return Err("Select faces to extrude");
                }

                let mut avg_n = Vec3::ZERO;
                let mut directed_edges: HashSet<(usize, usize)> = HashSet::new();
                let mut directed_order: Vec<(usize, usize)> = Vec::new();
                let mut touched_verts = BTreeSet::new();

                for &fi in &valid_faces {
                    avg_n = avg_n + self.face_normal(fi);
                    let f = &self.faces[fi];
                    for i in 0..f.len() {
                        let a = f[i];
                        let b = f[(i + 1) % f.len()];
                        touched_verts.insert(a);
                        directed_edges.insert((a, b));
                        directed_order.push((a, b));
                    }
                }
                let dir = avg_n.normalized_or(Vec3::UNIT_Y);

                let mut vert_map: HashMap<usize, usize> = HashMap::new();
                for v in touched_verts {
                    let nv = self.vertices.len();
                    self.vertices.push(self.vertices[v] + dir * distance);
                    vert_map.insert(v, nv);
                }

                // Remap selected cap faces to extruded vertices
                for &fi in &valid_faces {
                    for v in &mut self.faces[fi] {
                        *v = vert_map[v];
                    }
                }

                // Stitch quad walls along boundary half-edges
                for (a, b) in directed_order {
                    if !directed_edges.contains(&(b, a)) {
                        let na = vert_map[&a];
                        let nb = vert_map[&b];
                        self.faces.push(vec![a, b, nb, na]);
                    }
                }

                self.clean_faces();
                self.prune_unused_vertices(sel_v, sel_e);
                *sel_f = valid_faces;
                self.sync_selection_from_faces(sel_f, sel_v, sel_e);
                Ok(())
            }
            EditSelectMode::Edge => {
                let edges: Vec<(usize, usize)> = sel_e
                    .iter()
                    .copied()
                    .filter(|&(a, b)| a < self.vertices.len() && b < self.vertices.len() && a != b)
                    .collect();
                if edges.is_empty() {
                    return Err("Select edges to extrude");
                }
                let mut sum_n = Vec3::ZERO;
                let mut touched = BTreeSet::new();
                for &(a, b) in &edges {
                    touched.insert(a);
                    touched.insert(b);
                }
                for fi in 0..self.faces.len() {
                    if self.faces[fi].iter().any(|v| touched.contains(v)) {
                        sum_n = sum_n + self.face_normal(fi);
                    }
                }
                let dir = sum_n.normalized_or(Vec3::UNIT_Y);
                let mut map = HashMap::new();
                for v in touched {
                    let nv = self.vertices.len();
                    self.vertices.push(self.vertices[v] + dir * distance);
                    map.insert(v, nv);
                }
                let mut new_edges = BTreeSet::new();
                for (a, b) in edges {
                    let na = map[&a];
                    let nb = map[&b];
                    self.faces.push(vec![a, b, nb, na]);
                    new_edges.insert(canonical_edge(na, nb));
                }
                *sel_e = new_edges;
                sel_v.clear();
                for &(a, b) in sel_e.iter() {
                    sel_v.insert(a);
                    sel_v.insert(b);
                }
                sel_f.clear();
                Ok(())
            }
            EditSelectMode::Vertex => {
                let verts: Vec<usize> = sel_v
                    .iter()
                    .copied()
                    .filter(|&v| v < self.vertices.len())
                    .collect();
                if verts.is_empty() {
                    return Err("Select vertices to extrude");
                }
                let mut new_verts = BTreeSet::new();
                let mut sum_n = Vec3::ZERO;
                for fi in 0..self.faces.len() {
                    if self.faces[fi].iter().any(|v| sel_v.contains(v)) {
                        sum_n = sum_n + self.face_normal(fi);
                    }
                }
                let dir = sum_n.normalized_or(Vec3::UNIT_Y);
                for v in verts {
                    let nv = self.vertices.len();
                    self.vertices.push(self.vertices[v] + dir * distance);
                    new_verts.insert(nv);
                }
                *sel_v = new_verts;
                sel_e.clear();
                sel_f.clear();
                Ok(())
            }
        }
    }

    pub fn inset(
        &mut self,
        factor: f32,
        sel_v: &mut BTreeSet<usize>,
        sel_e: &mut BTreeSet<(usize, usize)>,
        sel_f: &mut BTreeSet<usize>,
    ) -> Result<(), &'static str> {
        let valid_faces: Vec<usize> = sel_f
            .iter()
            .copied()
            .filter(|&fi| fi < self.faces.len() && self.faces[fi].len() >= 3)
            .collect();
        if valid_faces.is_empty() {
            return Err("Select one or more faces to inset");
        }
        let t = factor.clamp(0.05, 0.90);
        for fi in &valid_faces {
            let centroid = self.face_centroid(*fi);
            let outer = self.faces[*fi].clone();
            let n = outer.len();
            let mut inner = Vec::with_capacity(n);
            for &vi in &outer {
                let p = self.vertices[vi];
                let ip = p.lerp(centroid, t);
                let idx = self.vertices.len();
                self.vertices.push(ip);
                inner.push(idx);
            }
            for i in 0..n {
                let next = (i + 1) % n;
                self.faces.push(vec![outer[i], outer[next], inner[next], inner[i]]);
            }
            self.faces[*fi] = inner;
        }
        self.clean_faces();
        self.sync_selection_from_faces(sel_f, sel_v, sel_e);
        Ok(())
    }

    pub fn bevel(
        &mut self,
        mode: EditSelectMode,
        width: f32,
        sel_v: &mut BTreeSet<usize>,
        sel_e: &mut BTreeSet<(usize, usize)>,
        sel_f: &mut BTreeSet<usize>,
    ) -> Result<(), &'static str> {
        let w = width.clamp(0.02, 0.45);
        let mut target_edges: BTreeSet<(usize, usize)> = BTreeSet::new();

        match mode {
            EditSelectMode::Edge => {
                target_edges = sel_e.clone();
            }
            EditSelectMode::Face => {
                let mut half_counts: HashMap<(usize, usize), usize> = HashMap::new();
                for &fi in sel_f.iter() {
                    if let Some(f) = self.faces.get(fi) {
                        for i in 0..f.len() {
                            let e = canonical_edge(f[i], f[(i + 1) % f.len()]);
                            *half_counts.entry(e).or_insert(0) += 1;
                        }
                    }
                }
                for (e, cnt) in &half_counts {
                    if *cnt == 1 {
                        target_edges.insert(*e);
                    }
                }
                if target_edges.is_empty() {
                    for e in half_counts.keys() {
                        target_edges.insert(*e);
                    }
                }
            }
            EditSelectMode::Vertex => {
                for f in &self.faces {
                    for i in 0..f.len() {
                        let a = f[i];
                        let b = f[(i + 1) % f.len()];
                        if sel_v.contains(&a) || sel_v.contains(&b) {
                            target_edges.insert(canonical_edge(a, b));
                        }
                    }
                }
            }
        }

        if target_edges.is_empty() {
            return Err("Select edges, faces, or vertices to bevel");
        }

        // Precompute smoothed vertex normals for ordering vertex bevel caps
        let mut vnorms = vec![Vec3::ZERO; self.vertices.len()];
        for fi in 0..self.faces.len() {
            let an = self.face_area_normal(fi);
            for &vi in &self.faces[fi] {
                vnorms[vi] = vnorms[vi] + an;
            }
        }
        for n in &mut vnorms {
            *n = n.normalized_or(Vec3::UNIT_Y);
        }

        // Vertices touched by any beveled edge get per-(face, corner) split positions
        let mut beveled_verts = BTreeSet::new();
        for &(a, b) in &target_edges {
            beveled_verts.insert(a);
            beveled_verts.insert(b);
        }

        // Map (face_idx, orig_vert) -> new corner vertex index
        let mut corner_map: HashMap<(usize, usize), usize> = HashMap::new();
        // Track edge->[(face_idx, corner_a, corner_b)] for building bevel edge quads
        let mut edge_sides: HashMap<(usize, usize), Vec<(usize, usize, usize)>> = HashMap::new();
        // Track all new corner verts created around each beveled original vertex
        let mut vert_corners: HashMap<usize, BTreeSet<usize>> = HashMap::new();

        let orig_face_count = self.faces.len();
        for fi in 0..orig_face_count {
            let f = self.faces[fi].clone();
            let n = f.len();
            if n < 3 {
                continue;
            }
            let mut new_face = Vec::with_capacity(n);
            for i in 0..n {
                let v = f[i];
                let prev = f[(i + n - 1) % n];
                let next = f[(i + 1) % n];
                let new_v = if !beveled_verts.contains(&v) {
                    v
                } else {
                    let e_prev = target_edges.contains(&canonical_edge(v, prev));
                    let e_next = target_edges.contains(&canonical_edge(v, next));
                    let vp = self.vertices[v];
                    let dp = self.vertices[prev] - vp;
                    let dn = self.vertices[next] - vp;
                    let step_p = if e_next {
                        let max_p = dp.length() * 0.42;
                        dp.normalized_or(Vec3::ZERO) * w.min(max_p)
                    } else {
                        Vec3::ZERO
                    };
                    let step_n = if e_prev {
                        let max_n = dn.length() * 0.42;
                        dn.normalized_or(Vec3::ZERO) * w.min(max_n)
                    } else {
                        Vec3::ZERO
                    };
                    let offset_pos = vp + step_p + step_n;
                    if (offset_pos - vp).length_squared() < 1e-10 {
                        v
                    } else {
                        let nv = self.vertices.len();
                        self.vertices.push(offset_pos);
                        nv
                    }
                };
                corner_map.insert((fi, v), new_v);
                if beveled_verts.contains(&v) {
                    vert_corners.entry(v).or_default().insert(new_v);
                }
                new_face.push(new_v);
            }
            self.faces[fi] = new_face;

            for i in 0..n {
                let a = f[i];
                let b = f[(i + 1) % n];
                let ek = canonical_edge(a, b);
                if target_edges.contains(&ek) {
                    let ca = corner_map[&(fi, a)];
                    let cb = corner_map[&(fi, b)];
                    edge_sides.entry(ek).or_default().push((fi, ca, cb));
                }
            }
        }

        let mut new_bevel_faces = BTreeSet::new();

        // Build bevel quad strips for every beveled manifold edge
        for ((orig_a, orig_b), sides) in edge_sides {
            if sides.len() != 2 {
                continue;
            }
            // sides[0] came from a directed edge (orig_a->orig_b) or (orig_b->orig_a).
            let (f0, c0_a, c0_b) = sides[0];
            let (f1, c1_a, c1_b) = sides[1];
            // Check directed order in original face f0: corner_map keyed by orig vertex
            let c0_orig_a = corner_map[&(f0, orig_a)];
            let c0_orig_b = corner_map[&(f0, orig_b)];
            let c1_orig_a = corner_map[&(f1, orig_a)];
            let c1_orig_b = corner_map[&(f1, orig_b)];

            let quad = if c0_a == c0_orig_a && c0_b == c0_orig_b {
                // f0 ran orig_a -> orig_b, so bevel quad runs opposite: orig_b(f0) -> orig_a(f0) -> orig_a(f1) -> orig_b(f1)
                vec![c0_orig_b, c0_orig_a, c1_orig_a, c1_orig_b]
            } else {
                // f0 ran orig_b -> orig_a
                vec![c0_orig_a, c0_orig_b, c1_orig_b, c1_orig_a]
            };
            let _ = (c1_a, c1_b);
            let fi = self.faces.len();
            self.faces.push(quad);
            new_bevel_faces.insert(fi);
        }

        // Fill vertex corner chamfer faces where >= 3 distinct corner vertices meet
        for (orig_v, corner_set) in vert_corners {
            if corner_set.len() < 3 {
                continue;
            }
            let nrm = vnorms[orig_v];
            let center = self.vertices[orig_v];
            let tangent = if nrm.y.abs() < 0.9 {
                nrm.cross(Vec3::UNIT_Y).normalized_or(Vec3::UNIT_X)
            } else {
                nrm.cross(Vec3::UNIT_X).normalized_or(Vec3::UNIT_Z)
            };
            let bitangent = nrm.cross(tangent).normalized_or(Vec3::UNIT_Z);

            let mut ordered: Vec<(f32, usize)> = corner_set
                .into_iter()
                .map(|cv| {
                    let d = self.vertices[cv] - center;
                    let angle = d.dot(bitangent).atan2(d.dot(tangent));
                    (angle, cv)
                })
                .collect();
            ordered.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            let poly: Vec<usize> = ordered.into_iter().map(|(_, cv)| cv).collect();
            let fi = self.faces.len();
            self.faces.push(poly);
            new_bevel_faces.insert(fi);
        }

        self.clean_faces();
        self.prune_unused_vertices(sel_v, sel_e);
        *sel_f = new_bevel_faces
            .into_iter()
            .filter(|&fi| fi < self.faces.len())
            .collect();
        self.sync_selection_from_faces(sel_f, sel_v, sel_e);
        Ok(())
    }

    pub fn loop_cut(
        &mut self,
        sel_v: &mut BTreeSet<usize>,
        sel_e: &mut BTreeSet<(usize, usize)>,
        sel_f: &mut BTreeSet<usize>,
    ) -> Result<(), &'static str> {
        let seed_edge = sel_e.iter().copied().next().or_else(|| {
            sel_f.iter().copied().next().and_then(|fi| {
                self.faces.get(fi).and_then(|f| {
                    if f.len() == 4 {
                        Some(canonical_edge(f[0], f[1]))
                    } else {
                        None
                    }
                })
            })
        }).or_else(|| {
            self.faces.iter().find_map(|f| {
                if f.len() == 4 {
                    Some(canonical_edge(f[0], f[1]))
                } else {
                    None
                }
            })
        });

        let Some(seed) = seed_edge else {
            return Err("Loop cut requires quad faces or a selected edge");
        };

        // Map canonical edge -> quad faces containing it
        let mut edge_to_quads: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
        for (fi, f) in self.faces.iter().enumerate() {
            if f.len() == 4 {
                for i in 0..4 {
                    edge_to_quads
                        .entry(canonical_edge(f[i], f[(i + 1) % 4]))
                        .or_default()
                        .push(fi);
                }
            }
        }

        // Walk ring across opposing edges of quad faces
        let mut ring_edges: BTreeSet<(usize, usize)> = BTreeSet::new();
        let mut crossed_quads: BTreeMap<usize, (usize, usize)> = BTreeMap::new(); // fi -> corner pair offset
        let mut queue = VecDeque::new();
        ring_edges.insert(seed);
        queue.push_back(seed);

        while let Some(edge) = queue.pop_front() {
            if let Some(quads) = edge_to_quads.get(&edge) {
                for &fi in quads {
                    if crossed_quads.contains_key(&fi) {
                        continue;
                    }
                    let f = &self.faces[fi];
                    for i in 0..4 {
                        if canonical_edge(f[i], f[(i + 1) % 4]) == edge {
                            let opp = canonical_edge(f[(i + 2) % 4], f[(i + 3) % 4]);
                            crossed_quads.insert(fi, (i, (i + 2) % 4));
                            if ring_edges.insert(opp) {
                                queue.push_back(opp);
                            }
                            break;
                        }
                    }
                }
            }
        }

        if crossed_quads.is_empty() {
            return Err("Selected edge does not belong to any quad loop");
        }

        // Create midpoint vertex for each edge in ring_edges
        let mut edge_mid: HashMap<(usize, usize), usize> = HashMap::new();
        for e in &ring_edges {
            let p = (self.vertices[e.0] + self.vertices[e.1]) * 0.5;
            let mv = self.vertices.len();
            self.vertices.push(p);
            edge_mid.insert(*e, mv);
        }

        // Split non-ring faces sharing any cut edge so mesh remains watertight
        let orig_count = self.faces.len();
        let mut new_cut_edges = BTreeSet::new();
        let mut new_faces = BTreeSet::new();

        for fi in 0..orig_count {
            if let Some(&(c0, c2)) = crossed_quads.get(&fi) {
                let f = self.faces[fi].clone();
                let v0 = f[c0];
                let v1 = f[(c0 + 1) % 4];
                let v2 = f[c2];
                let v3 = f[(c2 + 1) % 4];
                let m01 = edge_mid[&canonical_edge(v0, v1)];
                let m23 = edge_mid[&canonical_edge(v2, v3)];
                self.faces[fi] = vec![v0, m01, m23, v3];
                let new_fi = self.faces.len();
                self.faces.push(vec![m01, v1, v2, m23]);
                new_cut_edges.insert(canonical_edge(m01, m23));
                new_faces.insert(fi);
                new_faces.insert(new_fi);
            } else {
                let f = self.faces[fi].clone();
                let n = f.len();
                let mut rebuilt = Vec::with_capacity(n + 2);
                let mut changed = false;
                for i in 0..n {
                    let a = f[i];
                    let b = f[(i + 1) % n];
                    rebuilt.push(a);
                    if let Some(&mv) = edge_mid.get(&canonical_edge(a, b)) {
                        rebuilt.push(mv);
                        changed = true;
                    }
                }
                if changed {
                    self.faces[fi] = rebuilt;
                }
            }
        }

        self.clean_faces();
        *sel_e = new_cut_edges;
        sel_v.clear();
        for &(a, b) in sel_e.iter() {
            sel_v.insert(a);
            sel_v.insert(b);
        }
        *sel_f = new_faces;
        Ok(())
    }

    pub fn subdivide(
        &mut self,
        sel_v: &mut BTreeSet<usize>,
        sel_e: &mut BTreeSet<(usize, usize)>,
        sel_f: &mut BTreeSet<usize>,
    ) -> Result<(), &'static str> {
        let mut target_faces: BTreeSet<usize> = sel_f
            .iter()
            .copied()
            .filter(|&fi| fi < self.faces.len())
            .collect();
        if target_faces.is_empty() && !sel_e.is_empty() {
            for (fi, f) in self.faces.iter().enumerate() {
                for i in 0..f.len() {
                    if sel_e.contains(&canonical_edge(f[i], f[(i + 1) % f.len()])) {
                        target_faces.insert(fi);
                    }
                }
            }
        }
        if target_faces.is_empty() && !sel_v.is_empty() {
            for (fi, f) in self.faces.iter().enumerate() {
                if f.iter().any(|v| sel_v.contains(v)) {
                    target_faces.insert(fi);
                }
            }
        }
        if target_faces.is_empty() {
            target_faces = (0..self.faces.len()).collect();
        }
        if target_faces.is_empty() {
            return Err("No faces available to subdivide");
        }

        let mut edge_mid: HashMap<(usize, usize), usize> = HashMap::new();
        for &fi in &target_faces {
            let f = &self.faces[fi];
            for i in 0..f.len() {
                let ek = canonical_edge(f[i], f[(i + 1) % f.len()]);
                edge_mid.entry(ek).or_insert_with(|| {
                    let mv = self.vertices.len();
                    let p = (self.vertices[ek.0] + self.vertices[ek.1]) * 0.5;
                    self.vertices.push(p);
                    mv
                });
            }
        }

        let orig_count = self.faces.len();
        let mut new_selected_faces = BTreeSet::new();

        for fi in 0..orig_count {
            if target_faces.contains(&fi) {
                let f = self.faces[fi].clone();
                let n = f.len();
                let center_pos = {
                    let mut c = Vec3::ZERO;
                    for &v in &f {
                        c = c + self.vertices[v];
                    }
                    c * (1.0 / n as f32)
                };
                let center_idx = self.vertices.len();
                self.vertices.push(center_pos);

                let mids: Vec<usize> = (0..n)
                    .map(|i| edge_mid[&canonical_edge(f[i], f[(i + 1) % n])])
                    .collect();

                for i in 0..n {
                    let prev_m = mids[(i + n - 1) % n];
                    let quad = vec![prev_m, f[i], mids[i], center_idx];
                    if i == 0 {
                        self.faces[fi] = quad;
                        new_selected_faces.insert(fi);
                    } else {
                        let nf = self.faces.len();
                        self.faces.push(quad);
                        new_selected_faces.insert(nf);
                    }
                }
            } else {
                // Keep unselected neighbors stitched cleanly along split boundary edges
                let f = self.faces[fi].clone();
                let n = f.len();
                let mut rebuilt = Vec::with_capacity(n * 2);
                let mut changed = false;
                for i in 0..n {
                    let a = f[i];
                    let b = f[(i + 1) % n];
                    rebuilt.push(a);
                    if let Some(&mv) = edge_mid.get(&canonical_edge(a, b)) {
                        rebuilt.push(mv);
                        changed = true;
                    }
                }
                if changed {
                    self.faces[fi] = rebuilt;
                }
            }
        }

        self.clean_faces();
        *sel_f = new_selected_faces;
        self.sync_selection_from_faces(sel_f, sel_v, sel_e);
        Ok(())
    }

    pub fn merge_selected(
        &mut self,
        mode: EditSelectMode,
        sel_v: &mut BTreeSet<usize>,
        sel_e: &mut BTreeSet<(usize, usize)>,
        sel_f: &mut BTreeSet<usize>,
    ) -> Result<(), &'static str> {
        let verts = self.selected_vertex_closure(mode, sel_v, sel_e, sel_f);
        if verts.len() < 2 {
            return Err("Select at least 2 vertices to merge");
        }
        let mut center = Vec3::ZERO;
        for &v in &verts {
            center = center + self.vertices[v];
        }
        center = center * (1.0 / verts.len() as f32);
        let keep = *verts.iter().next().unwrap();
        self.vertices[keep] = center;

        for f in &mut self.faces {
            for vi in f.iter_mut() {
                if verts.contains(vi) {
                    *vi = keep;
                }
            }
        }
        self.clean_faces();

        sel_v.clear();
        sel_v.insert(keep);
        sel_e.clear();
        sel_f.clear();
        self.prune_unused_vertices(sel_v, sel_e);
        Ok(())
    }

    pub fn delete_selected(
        &mut self,
        mode: EditSelectMode,
        sel_v: &mut BTreeSet<usize>,
        sel_e: &mut BTreeSet<(usize, usize)>,
        sel_f: &mut BTreeSet<usize>,
    ) -> Result<(), &'static str> {
        let before = self.faces.len();
        match mode {
            EditSelectMode::Face => {
                if sel_f.is_empty() {
                    return Err("Select faces to delete");
                }
                let mut idx = 0usize;
                self.faces.retain(|_| {
                    let keep = !sel_f.contains(&idx);
                    idx += 1;
                    keep
                });
            }
            EditSelectMode::Edge => {
                if sel_e.is_empty() {
                    return Err("Select edges to delete");
                }
                self.faces.retain(|f| {
                    for i in 0..f.len() {
                        if sel_e.contains(&canonical_edge(f[i], f[(i + 1) % f.len()])) {
                            return false;
                        }
                    }
                    true
                });
            }
            EditSelectMode::Vertex => {
                if sel_v.is_empty() {
                    return Err("Select vertices to delete");
                }
                self.faces.retain(|f| !f.iter().any(|v| sel_v.contains(v)));
            }
        }
        if self.faces.len() == before {
            return Err("Nothing deleted");
        }
        sel_v.clear();
        sel_e.clear();
        sel_f.clear();
        self.prune_unused_vertices(sel_v, sel_e);
        Ok(())
    }

    pub fn dissolve_edges(
        &mut self,
        sel_v: &mut BTreeSet<usize>,
        sel_e: &mut BTreeSet<(usize, usize)>,
        sel_f: &mut BTreeSet<usize>,
    ) -> Result<(), &'static str> {
        if sel_e.is_empty() {
            return Err("Select edges to dissolve");
        }
        let mut dissolved_any = false;
        let targets: Vec<(usize, usize)> = sel_e.iter().copied().collect();

        for (ea, eb) in targets {
            let mut hits: Vec<(usize, usize, bool)> = Vec::new();
            for (fi, f) in self.faces.iter().enumerate() {
                if f.len() < 3 {
                    continue;
                }
                for i in 0..f.len() {
                    let a = f[i];
                    let b = f[(i + 1) % f.len()];
                    if a == ea && b == eb {
                        hits.push((fi, i, true));
                    } else if a == eb && b == ea {
                        hits.push((fi, i, false));
                    }
                }
            }
            if hits.len() != 2 || hits[0].0 == hits[1].0 {
                continue;
            }
            let (f0_idx, pos0, _) = hits[0];
            let (f1_idx, pos1, _) = hits[1];
            let f0 = &self.faces[f0_idx];
            let f1 = &self.faces[f1_idx];
            // f0 has edge pos0 -> (pos0+1), f1 has opposite edge pos1 -> (pos1+1).
            // Merged polygon starts at f0[(pos0+1)], walks around f0 up to f0[pos0],
            // skips f1[(pos1+1)] (= f0[pos0]), walks around f1 to f1[pos1] (= f0[pos0+1]).
            let mut merged = Vec::with_capacity(f0.len() + f1.len() - 2);
            for step in 1..f0.len() {
                merged.push(f0[(pos0 + step) % f0.len()]);
            }
            for step in 1..(f1.len() - 1) {
                merged.push(f1[(pos1 + 1 + step) % f1.len()]);
            }
            if merged.len() >= 3 {
                let keep_idx = f0_idx.min(f1_idx);
                let drop_idx = f0_idx.max(f1_idx);
                self.faces[keep_idx] = merged;
                self.faces.remove(drop_idx);
                dissolved_any = true;
            }
        }

        if !dissolved_any {
            return Err("Selected edges must be shared between two faces to dissolve");
        }
        self.clean_faces();
        sel_e.clear();
        sel_f.clear();
        sel_v.clear();
        self.prune_unused_vertices(sel_v, sel_e);
        Ok(())
    }

    pub fn recalculate_normals(&mut self) {
        let n_faces = self.faces.len();
        if n_faces == 0 {
            return;
        }
        // Build edge -> [(face_idx, u, v)] directed incidence
        let mut edge_faces: HashMap<(usize, usize), Vec<(usize, usize, usize)>> = HashMap::new();
        for (fi, f) in self.faces.iter().enumerate() {
            for i in 0..f.len() {
                let a = f[i];
                let b = f[(i + 1) % f.len()];
                edge_faces.entry(canonical_edge(a, b)).or_default().push((fi, a, b));
            }
        }

        let mut visited = vec![false; n_faces];
        let mut components: Vec<Vec<usize>> = Vec::new();

        for start in 0..n_faces {
            if visited[start] {
                continue;
            }
            visited[start] = true;
            let mut comp = vec![start];
            let mut q = VecDeque::new();
            q.push_back(start);

            while let Some(cur) = q.pop_front() {
                let cur_face = self.faces[cur].clone();
                for i in 0..cur_face.len() {
                    let a = cur_face[i];
                    let b = cur_face[(i + 1) % cur_face.len()];
                    if let Some(inc) = edge_faces.get(&canonical_edge(a, b)) {
                        for &(nb, na, nb_v) in inc {
                            if nb != cur && !visited[nb] {
                                // Neighbor should traverse shared edge in opposite direction (b -> a)
                                if na == a && nb_v == b {
                                    self.faces[nb].reverse();
                                }
                                visited[nb] = true;
                                comp.push(nb);
                                q.push_back(nb);
                            }
                        }
                    }
                }
            }
            components.push(comp);
        }

        // Orient each connected component outward via signed volume (or radial fallback for open shells)
        for comp in components {
            let mut signed_vol = 0.0f32;
            let mut center = Vec3::ZERO;
            let mut vcount = 0usize;
            for &fi in &comp {
                let f = &self.faces[fi];
                if f.len() < 3 {
                    continue;
                }
                let p0 = self.vertices[f[0]];
                for i in 1..(f.len() - 1) {
                    let p1 = self.vertices[f[i]];
                    let p2 = self.vertices[f[i + 1]];
                    signed_vol += p0.dot(p1.cross(p2));
                }
                center = center + self.face_centroid(fi);
                vcount += 1;
            }
            let invert = if signed_vol.abs() > 1e-4 {
                signed_vol < 0.0
            } else if vcount > 0 {
                let c = center * (1.0 / vcount as f32);
                let mut dot_sum = 0.0f32;
                for &fi in &comp {
                    dot_sum += self.face_normal(fi).dot(self.face_centroid(fi) - c);
                }
                dot_sum < 0.0
            } else {
                false
            };
            if invert {
                for fi in comp {
                    self.faces[fi].reverse();
                }
            }
        }
    }

    pub fn flip_normals(&mut self, sel_f: &BTreeSet<usize>) {
        if sel_f.is_empty() {
            for f in &mut self.faces {
                f.reverse();
            }
        } else {
            for &fi in sel_f {
                if let Some(f) = self.faces.get_mut(fi) {
                    f.reverse();
                }
            }
        }
    }
}

/// Evaluates non-destructive modifier stack cleanly without mutating the base `PolyMesh`.
pub fn evaluate_modifier_stack(base: &PolyMesh, stack: &[ModifierSpec]) -> PolyMesh {
    let mut current = base.clone();
    for m in stack {
        if !m.is_enabled() {
            continue;
        }
        current = match *m {
            ModifierSpec::Mirror { axis, merge, .. } => apply_mirror(&current, axis, merge),
            ModifierSpec::Subdivision { levels, .. } => {
                let mut tmp = current;
                for _ in 0..levels.clamp(1, 2) {
                    tmp = apply_catmull_clark(&tmp);
                }
                tmp
            }
            ModifierSpec::Solidify { thickness, .. } => apply_solidify(&current, thickness),
            ModifierSpec::Array { count, offset, .. } => apply_array(&current, count, offset),
        };
    }
    current
}

fn apply_mirror(mesh: &PolyMesh, axis: u8, merge: bool) -> PolyMesh {
    let mut verts = mesh.vertices.clone();
    let mut faces = mesh.faces.clone();
    let mirror_fn = |p: Vec3| match axis {
        1 => Vec3::new(p.x, -p.y, p.z),
        2 => Vec3::new(p.x, p.y, -p.z),
        _ => Vec3::new(-p.x, p.y, p.z),
    };
    let on_plane = |p: Vec3| -> bool {
        let coord = match axis {
            1 => p.y,
            2 => p.z,
            _ => p.x,
        };
        coord.abs() < 1e-4
    };

    let mut mirror_map = Vec::with_capacity(mesh.vertices.len());
    for &v in &mesh.vertices {
        if merge && on_plane(v) {
            mirror_map.push(mirror_map.len()); // self-mapped below: find index
        } else {
            let nv = verts.len();
            verts.push(mirror_fn(v));
            mirror_map.push(nv);
        }
    }
    // Fix self-mapped indices cleanly
    for (i, &v) in mesh.vertices.iter().enumerate() {
        if merge && on_plane(v) {
            mirror_map[i] = i;
        }
    }

    for f in &mesh.faces {
        let mut mf: Vec<usize> = f.iter().rev().map(|&vi| mirror_map[vi]).collect();
        mf.dedup();
        if mf.len() >= 3 {
            faces.push(mf);
        }
    }
    let mut out = PolyMesh {
        vertices: verts,
        faces,
        smooth_shading: mesh.smooth_shading,
    };
    out.clean_faces();
    out
}

fn apply_catmull_clark(mesh: &PolyMesh) -> PolyMesh {
    if mesh.faces.is_empty() || mesh.vertices.is_empty() {
        return mesh.clone();
    }
    let nv = mesh.vertices.len();
    let nf = mesh.faces.len();

    // 1. Face points
    let face_points: Vec<Vec3> = (0..nf).map(|fi| mesh.face_centroid(fi)).collect();

    // Edge incidence
    let mut edge_faces: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
    let mut vert_faces: Vec<Vec<usize>> = vec![Vec::new(); nv];
    let mut vert_edges: Vec<BTreeSet<(usize, usize)>> = vec![BTreeSet::new(); nv];

    for (fi, f) in mesh.faces.iter().enumerate() {
        for i in 0..f.len() {
            let a = f[i];
            let b = f[(i + 1) % f.len()];
            let ek = canonical_edge(a, b);
            edge_faces.entry(ek).or_default().push(fi);
            vert_edges[a].insert(ek);
            vert_edges[b].insert(ek);
            vert_faces[a].push(fi);
        }
    }

    // 2. Edge points + vertices table
    let mut new_verts = vec![Vec3::ZERO; nv];
    let mut face_pt_idx = Vec::with_capacity(nf);
    for fp in &face_points {
        face_pt_idx.push(new_verts.len());
        new_verts.push(*fp);
    }

    let mut edge_pt_idx: HashMap<(usize, usize), usize> = HashMap::new();
    for (&ek, flist) in &edge_faces {
        let ep = if flist.len() == 2 {
            (mesh.vertices[ek.0]
                + mesh.vertices[ek.1]
                + face_points[flist[0]]
                + face_points[flist[1]])
                * 0.25
        } else {
            (mesh.vertices[ek.0] + mesh.vertices[ek.1]) * 0.5
        };
        let idx = new_verts.len();
        new_verts.push(ep);
        edge_pt_idx.insert(ek, idx);
    }

    // 3. Updated original vertices
    for i in 0..nv {
        let p = mesh.vertices[i];
        let boundary_edges: Vec<(usize, usize)> = vert_edges[i]
            .iter()
            .copied()
            .filter(|ek| edge_faces.get(ek).map_or(false, |v| v.len() == 1))
            .collect();

        new_verts[i] = if !boundary_edges.is_empty() {
            let mut sum = p * 2.0;
            for ek in &boundary_edges {
                let mid = (mesh.vertices[ek.0] + mesh.vertices[ek.1]) * 0.5;
                sum = sum + mid;
            }
            sum * (1.0 / (2.0 + boundary_edges.len() as f32))
        } else {
            let n = vert_edges[i].len().max(1) as f32;
            let f_avg = if vert_faces[i].is_empty() {
                p
            } else {
                let mut s = Vec3::ZERO;
                for &fi in &vert_faces[i] {
                    s = s + face_points[fi];
                }
                s * (1.0 / vert_faces[i].len() as f32)
            };
            let r_avg = {
                let mut s = Vec3::ZERO;
                for ek in &vert_edges[i] {
                    s = s + (mesh.vertices[ek.0] + mesh.vertices[ek.1]) * 0.5;
                }
                s * (1.0 / n)
            };
            (f_avg + r_avg * 2.0 + p * (n - 3.0).max(0.0)) * (1.0 / (n.max(3.0)))
        };
    }

    // 4. Emit quads
    let mut new_faces = Vec::with_capacity(nf * 4);
    for (fi, f) in mesh.faces.iter().enumerate() {
        let n = f.len();
        let fp = face_pt_idx[fi];
        for i in 0..n {
            let v = f[i];
            let prev_v = f[(i + n - 1) % n];
            let next_v = f[(i + 1) % n];
            let e_prev = edge_pt_idx[&canonical_edge(prev_v, v)];
            let e_next = edge_pt_idx[&canonical_edge(v, next_v)];
            new_faces.push(vec![e_prev, v, e_next, fp]);
        }
    }

    PolyMesh {
        vertices: new_verts,
        faces: new_faces,
        smooth_shading: mesh.smooth_shading,
    }
}

fn apply_solidify(mesh: &PolyMesh, thickness: f32) -> PolyMesh {
    let t = thickness.clamp(-0.8, 0.8);
    if t.abs() < 1e-4 || mesh.faces.is_empty() {
        return mesh.clone();
    }
    let nv = mesh.vertices.len();
    let mut vnorms = vec![Vec3::ZERO; nv];
    for fi in 0..mesh.faces.len() {
        let an = mesh.face_area_normal(fi);
        for &vi in &mesh.faces[fi] {
            vnorms[vi] = vnorms[vi] + an;
        }
    }
    let mut verts = mesh.vertices.clone();
    for i in 0..nv {
        let n = vnorms[i].normalized_or(Vec3::UNIT_Y);
        verts.push(mesh.vertices[i] - n * t);
    }

    let mut faces = mesh.faces.clone();
    let mut dir_counts: HashSet<(usize, usize)> = HashSet::new();
    let mut dir_order = Vec::new();
    for f in &mesh.faces {
        let inner: Vec<usize> = f.iter().rev().map(|&vi| vi + nv).collect();
        faces.push(inner);
        for i in 0..f.len() {
            let a = f[i];
            let b = f[(i + 1) % f.len()];
            dir_counts.insert((a, b));
            dir_order.push((a, b));
        }
    }
    for (a, b) in dir_order {
        if !dir_counts.contains(&(b, a)) {
            faces.push(vec![b, a, a + nv, b + nv]);
        }
    }
    PolyMesh {
        vertices: verts,
        faces,
        smooth_shading: mesh.smooth_shading,
    }
}

fn apply_array(mesh: &PolyMesh, count: u32, offset: [f32; 3]) -> PolyMesh {
    let cnt = count.clamp(1, 12) as usize;
    if cnt <= 1 {
        return mesh.clone();
    }
    let step = Vec3::new(offset[0], offset[1], offset[2]);
    let mut verts = Vec::with_capacity(mesh.vertices.len() * cnt);
    let mut faces = Vec::with_capacity(mesh.faces.len() * cnt);
    for c in 0..cnt {
        let base = verts.len();
        let shift = step * (c as f32);
        for &p in &mesh.vertices {
            verts.push(p + shift);
        }
        for f in &mesh.faces {
            faces.push(f.iter().map(|&vi| base + vi).collect());
        }
    }
    PolyMesh {
        vertices: verts,
        faces,
        smooth_shading: mesh.smooth_shading,
    }
}

fn triangulate_polygon(vertices: &[Vec3], face: &[usize]) -> Vec<[usize; 3]> {
    let n = face.len();
    if n < 3 {
        return Vec::new();
    }
    if n == 3 {
        return vec![[face[0], face[1], face[2]]];
    }
    if n == 4 {
        return vec![
            [face[0], face[1], face[2]],
            [face[0], face[2], face[3]],
        ];
    }
    // Simple ear-clipping fallback projected onto face best-fit normal plane
    let mut fn_ = Vec3::ZERO;
    for i in 0..n {
        let a = vertices[face[i]];
        let b = vertices[face[(i + 1) % n]];
        fn_ = fn_ + Vec3::new(
            (a.y - b.y) * (a.z + b.z),
            (a.z - b.z) * (a.x + b.x),
            (a.x - b.x) * (a.y + b.y),
        );
    }
    let normal = fn_.normalized_or(Vec3::UNIT_Y);
    let u = if normal.y.abs() < 0.9 {
        normal.cross(Vec3::UNIT_Y).normalized_or(Vec3::UNIT_X)
    } else {
        normal.cross(Vec3::UNIT_X).normalized_or(Vec3::UNIT_Z)
    };
    let v = normal.cross(u).normalized_or(Vec3::UNIT_Z);

    let mut rem: Vec<usize> = face.to_vec();
    let mut tris = Vec::with_capacity(n - 2);
    let mut guard = 0usize;
    while rem.len() > 3 && guard < n * n {
        guard += 1;
        let m = rem.len();
        let mut clipped = false;
        for i in 0..m {
            let a = rem[(i + m - 1) % m];
            let b = rem[i];
            let c = rem[(i + 1) % m];
            let pa = (vertices[a].dot(u), vertices[a].dot(v));
            let pb = (vertices[b].dot(u), vertices[b].dot(v));
            let pc = (vertices[c].dot(u), vertices[c].dot(v));
            let cross2 = (pb.0 - pa.0) * (pc.1 - pa.1) - (pb.1 - pa.1) * (pc.0 - pa.0);
            if cross2 <= 1e-7 {
                continue;
            }
            let any_inside = rem.iter().enumerate().any(|(j, &vk)| {
                if vk == a || vk == b || vk == c {
                    return false;
                }
                let _ = j;
                let pk = (vertices[vk].dot(u), vertices[vk].dot(v));
                point_in_tri_2d(pk, pa, pb, pc)
            });
            if !any_inside {
                tris.push([a, b, c]);
                rem.remove(i);
                clipped = true;
                break;
            }
        }
        if !clipped {
            break;
        }
    }
    if rem.len() >= 3 {
        for i in 1..(rem.len() - 1) {
            tris.push([rem[0], rem[i], rem[i + 1]]);
        }
    }
    tris
}

fn point_in_tri_2d(p: (f32, f32), a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> bool {
    let s1 = (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0);
    let s2 = (c.0 - b.0) * (p.1 - b.1) - (c.1 - b.1) * (p.0 - b.0);
    let s3 = (a.0 - c.0) * (p.1 - c.1) - (a.1 - c.1) * (p.0 - c.0);
    (s1 >= -1e-6 && s2 >= -1e-6 && s3 >= -1e-6)
        || (s1 <= 1e-6 && s2 <= 1e-6 && s3 <= 1e-6)
}
