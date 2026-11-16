//! Scene graph and geometry ownership in the Rust core.
//! The Rust core is the single source of truth for scene state; the renderer
//! consumes derived render data snapshots.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::f32::consts::TAU;

use crate::animation::{
    AnimChannelKind, AnimTargetKind, AnimationState, InterpolationMode, KeyframeId, TrackId,
};
use crate::camera::{CameraSnapshot, ProjectionMode, StandardView, ViewportCamera};
use crate::command::{
    AddNodesCommand, ApplyLightingRigCommand, BakeMeshTransformCommand, CommandHistory,
    DeleteNodesCommand, RenameNodeCommand, ReparentChange, ReparentNodesCommand,
    SetAnimationStateCommand, SetCameraNodeCommand, SetEnvironmentCommand, SetLightCommand,
    SetMaterialCommand, SetMaterialLibraryCommand, SetModifiersCommand, SetPolyMeshCommand,
    SetTransformCommand, SetVisibilityCommand,
};
use crate::lookdev::{
    default_builtin_textures, AlphaMode, EnvironmentPreset, EnvironmentSpec, MaterialId,
    MaterialSpec, TextureAsset, TextureId, TextureStatus,
};
use crate::math::{Mat4, Quat, Ray, Vec3};
use crate::poly::{evaluate_modifier_stack, EditSelectMode, InteractionMode, ModifierSpec, PolyMesh};
use crate::primitives::{build_primitive_mesh, PrimitiveType};

pub type NodeId = u64;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Empty = 0,
    Mesh = 1,
    Light = 2,
    Camera = 3,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightKind {
    Directional = 0,
    Point = 1,
    Spot = 2,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GizmoMode {
    None = 0,
    Move = 1,
    Rotate = 2,
    Scale = 3,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GizmoSpace {
    World = 0,
    Local = 1,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GizmoAxis {
    None = 0,
    X = 1,
    Y = 2,
    Z = 3,
    Center = 4,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub position: Vec3,
    pub rotation_euler: Vec3,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation_euler: Vec3::ZERO,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    pub fn to_matrix(self) -> Mat4 {
        let q = Quat::from_euler_xyz(
            self.rotation_euler.x,
            self.rotation_euler.y,
            self.rotation_euler.z,
        );
        Mat4::from_trs(self.position, q, self.scale)
    }
}

#[derive(Debug, Clone)]
pub struct MeshData {
    /// Separate buffers owned by Rust core.
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
    pub bounds_min: Vec3,
    pub bounds_max: Vec3,
    pub version: u64,
}

impl MeshData {
    pub fn compute_bounds(positions: &[[f32; 3]]) -> (Vec3, Vec3) {
        if positions.is_empty() {
            return (Vec3::ZERO, Vec3::ZERO);
        }
        let mut min_v = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
        let mut max_v = Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
        for p in positions {
            let v = Vec3::new(p[0], p[1], p[2]);
            min_v = min_v.min(v);
            max_v = max_v.max(v);
        }
        (min_v, max_v)
    }

    /// Generates a flat-shaded 24-vertex unit cube centered at origin with side length `size`.
    pub fn studio_cube(size: f32) -> Self {
        let h = size * 0.5;
        let faces: [([f32; 3], [[f32; 3]; 4]); 6] = [
            ([0.0, 0.0, 1.0], [[-h, -h, h], [h, -h, h], [h, h, h], [-h, h, h]]),
            ([0.0, 0.0, -1.0], [[h, -h, -h], [-h, -h, -h], [-h, h, -h], [h, h, -h]]),
            ([1.0, 0.0, 0.0], [[h, -h, h], [h, -h, -h], [h, h, -h], [h, h, h]]),
            ([-1.0, 0.0, 0.0], [[-h, -h, -h], [-h, -h, h], [-h, h, h], [-h, h, -h]]),
            ([0.0, 1.0, 0.0], [[-h, h, h], [h, h, h], [h, h, -h], [-h, h, -h]]),
            ([0.0, -1.0, 0.0], [[-h, -h, -h], [h, -h, -h], [h, -h, h], [-h, -h, h]]),
        ];

        let mut positions = Vec::with_capacity(24);
        let mut normals = Vec::with_capacity(24);
        let mut uvs = Vec::with_capacity(24);
        let mut indices = Vec::with_capacity(36);

        for (face_idx, (n, corners)) in faces.iter().enumerate() {
            let base = (face_idx * 4) as u32;
            positions.extend_from_slice(corners);
            normals.extend_from_slice(&[*n, *n, *n, *n]);
            uvs.extend_from_slice(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }

        let (bounds_min, bounds_max) = Self::compute_bounds(&positions);
        Self {
            positions,
            normals,
            uvs,
            indices,
            bounds_min,
            bounds_max,
            version: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MaterialParams {
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
}

impl Default for MaterialParams {
    fn default() -> Self {
        Self {
            base_color: [0.76, 0.78, 0.82, 1.0],
            metallic: 0.05,
            roughness: 0.42,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LightParams {
    pub kind: LightKind,
    pub color: [f32; 3],
    pub intensity: f32,
    pub cast_shadows: bool,
    pub spot_inner_deg: f32,
    pub spot_outer_deg: f32,
    pub range: f32,
}

impl Default for LightParams {
    fn default() -> Self {
        Self {
            kind: LightKind::Point,
            color: [1.0, 0.96, 0.90],
            intensity: 45000.0,
            cast_shadows: true,
            spot_inner_deg: 18.0,
            spot_outer_deg: 38.0,
            range: 14.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CameraNodeParams {
    pub fov_y_degrees: f32,
    pub near_plane: f32,
    pub far_plane: f32,
}

impl Default for CameraNodeParams {
    fn default() -> Self {
        Self {
            fov_y_degrees: 50.0,
            near_plane: 0.05,
            far_plane: 200.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SceneNode {
    pub id: NodeId,
    pub parent_id: Option<NodeId>,
    pub name: String,
    pub kind: NodeKind,
    pub transform: Transform,
    pub mesh: Option<MeshData>,
    pub base_poly: Option<PolyMesh>,
    pub modifiers: Vec<ModifierSpec>,
    pub material_id: Option<MaterialId>,
    pub material: MaterialParams,
    pub light: LightParams,
    pub camera_node: CameraNodeParams,
    pub visible: bool,
}

impl SceneNode {
    pub fn ensure_base_poly(&mut self) -> Option<&PolyMesh> {
        if self.kind != NodeKind::Mesh {
            return None;
        }
        if self.base_poly.is_none() {
            if let Some(m) = &self.mesh {
                self.base_poly = Some(PolyMesh::from_mesh_data_fallback(m, false));
            }
        }
        self.base_poly.as_ref()
    }

    pub fn rebuild_evaluated_mesh(&mut self, edit_mode_active: bool) -> bool {
        if self.base_poly.is_none() {
            self.ensure_base_poly();
        }
        let Some(base) = &self.base_poly else {
            return false;
        };
        let next_ver = self.mesh.as_ref().map_or(1, |m| m.version + 1);
        let eval = if edit_mode_active || self.modifiers.is_empty() {
            base.clone()
        } else {
            evaluate_modifier_stack(base, &self.modifiers)
        };
        self.mesh = Some(eval.to_mesh_data(next_ver));
        true
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EditOverlayLine {
    pub a: [f32; 3],
    pub b: [f32; 3],
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EditOverlayTri {
    pub v0: [f32; 3],
    pub v1: [f32; 3],
    pub v2: [f32; 3],
    pub color: [f32; 4],
}

#[derive(Debug, Clone)]
pub struct GizmoDragSession {
    pub axis: GizmoAxis,
    pub mode: GizmoMode,
    pub pivot_world: Vec3,
    pub axis_dir_world: Vec3,
    pub scale_size: f32,
    pub start_param: f32,
    pub start_plane_hit: Option<Vec3>,
    /// (node_id, initial_local_transform, initial_world_matrix, parent_world_inv)
    pub targets: Vec<(NodeId, Transform, Mat4, Mat4)>,
    /// Edit mode vertex drag: (node_id, init_poly, init_mesh, node_world, node_world_inv, pivot_local, [(vert_idx, init_local_pos)])
    pub edit_vertices: Option<(NodeId, PolyMesh, MeshData, Mat4, Mat4, Vec3, Vec<(usize, Vec3)>)>,
}

#[derive(Debug, Clone)]
pub struct GizmoState {
    pub mode: GizmoMode,
    pub space: GizmoSpace,
    pub active_axis: GizmoAxis,
    pub drag: Option<GizmoDragSession>,
}

impl Default for GizmoState {
    fn default() -> Self {
        Self {
            mode: GizmoMode::Move,
            space: GizmoSpace::World,
            active_axis: GizmoAxis::None,
            drag: None,
        }
    }
}

#[derive(Debug)]
pub struct NovaEngineState {
    pub nodes: Vec<SceneNode>,
    pub selected_ids: Vec<NodeId>,
    pub camera: ViewportCamera,
    pub show_grid: bool,
    pub show_axes: bool,
    pub shading_mode: u32,
    pub gizmo: GizmoState,
    pub scene_revision: u64,
    pub history: CommandHistory,
    pub next_node_id: NodeId,
    pub interaction_mode: InteractionMode,
    pub edit_node_id: Option<NodeId>,
    pub edit_select_mode: EditSelectMode,
    pub edit_sel_verts: BTreeSet<usize>,
    pub edit_sel_edges: BTreeSet<(usize, usize)>,
    pub edit_sel_faces: BTreeSet<usize>,
    pub show_face_orientation: bool,
    pub edit_overlay_lines: Vec<EditOverlayLine>,
    pub edit_overlay_tris: Vec<EditOverlayTri>,
    // M4 Look Development state
    pub materials: Vec<MaterialSpec>,
    pub selected_material_id: Option<MaterialId>,
    pub textures: Vec<TextureAsset>,
    pub environment: EnvironmentSpec,
    pub environment_version: u64,
    pub next_material_id: MaterialId,
    pub next_texture_id: TextureId,
    pub material_preview_sphere: bool,
    // M5 Animation state
    pub animation: AnimationState,
    pub active_camera_node_id: Option<NodeId>,
}

impl Default for NovaEngineState {
    fn default() -> Self {
        Self::new_with_starter_scene()
    }
}

impl NovaEngineState {
    pub fn new_empty() -> Self {
        let default_mat = MaterialSpec::studio_default(1, "Default Studio");
        let textures = default_builtin_textures();
        let next_texture_id = textures.iter().map(|t| t.id).max().unwrap_or(0) + 1;
        let mut engine = Self {
            nodes: Vec::new(),
            selected_ids: Vec::new(),
            camera: ViewportCamera::default(),
            show_grid: true,
            show_axes: true,
            shading_mode: 2,
            gizmo: GizmoState::default(),
            scene_revision: 1,
            history: CommandHistory::new(128),
            next_node_id: 1,
            interaction_mode: InteractionMode::Object,
            edit_node_id: None,
            edit_select_mode: EditSelectMode::Face,
            edit_sel_verts: BTreeSet::new(),
            edit_sel_edges: BTreeSet::new(),
            edit_sel_faces: BTreeSet::new(),
            show_face_orientation: false,
            edit_overlay_lines: Vec::new(),
            edit_overlay_tris: Vec::new(),
            materials: vec![default_mat],
            selected_material_id: Some(1),
            textures,
            environment: EnvironmentSpec::default(),
            environment_version: 1,
            next_material_id: 2,
            next_texture_id,
            material_preview_sphere: false,
            animation: AnimationState::default(),
            active_camera_node_id: None,
        };
        engine.camera.frame_bounds(Vec3::new(-1.5, -1.5, -1.5), Vec3::new(1.5, 1.5, 1.5));
        engine
    }

    pub fn new_with_starter_scene() -> Self {
        let mut engine = Self::new_empty();
        engine.history = CommandHistory::new(128);

        let mut studio_blue = MaterialSpec::from_preset(1, 0);
        studio_blue.base_color = [0.28, 0.56, 0.92, 1.0];
        studio_blue.metallic = 0.12;
        studio_blue.roughness = 0.35;

        let mut base_slate = MaterialSpec::studio_default(2, "Base Slate");
        base_slate.base_color = [0.22, 0.24, 0.28, 1.0];
        base_slate.metallic = 0.0;
        base_slate.roughness = 0.75;

        engine.materials = vec![studio_blue.clone(), base_slate.clone()];
        engine.selected_material_id = Some(studio_blue.id);
        engine.next_material_id = 3;

        let cube_id = engine.allocate_id();
        let mut cube_poly = PolyMesh::from_primitive(PrimitiveType::Cube).expect("cube poly");
        for v in &mut cube_poly.vertices {
            *v = *v * 1.5;
        }
        let cube_mesh = cube_poly.to_mesh_data(1);
        engine.nodes.push(SceneNode {
            id: cube_id,
            parent_id: None,
            name: "Studio Cube".to_string(),
            kind: NodeKind::Mesh,
            transform: Transform {
                position: Vec3::new(0.0, 0.75, 0.0),
                rotation_euler: Vec3::ZERO,
                scale: Vec3::ONE,
            },
            mesh: Some(cube_mesh),
            base_poly: Some(cube_poly),
            modifiers: Vec::new(),
            material_id: Some(studio_blue.id),
            material: MaterialParams {
                base_color: studio_blue.base_color,
                metallic: studio_blue.metallic,
                roughness: studio_blue.roughness,
            },
            light: LightParams::default(),
            camera_node: CameraNodeParams::default(),
            visible: true,
        });

        let plinth_id = engine.allocate_id();
        let plinth_poly = PolyMesh::from_primitive(PrimitiveType::Cube).expect("plinth poly");
        let plinth_mesh = plinth_poly.to_mesh_data(1);
        engine.nodes.push(SceneNode {
            id: plinth_id,
            parent_id: None,
            name: "Base Plinth".to_string(),
            kind: NodeKind::Mesh,
            transform: Transform {
                position: Vec3::new(0.0, -0.05, 0.0),
                rotation_euler: Vec3::ZERO,
                scale: Vec3::new(2.4, 0.1, 2.4),
            },
            mesh: Some(plinth_mesh),
            base_poly: Some(plinth_poly),
            modifiers: Vec::new(),
            material_id: Some(base_slate.id),
            material: MaterialParams {
                base_color: base_slate.base_color,
                metallic: base_slate.metallic,
                roughness: base_slate.roughness,
            },
            light: LightParams::default(),
            camera_node: CameraNodeParams::default(),
            visible: true,
        });

        engine.selected_ids.push(cube_id);
        engine.frame_all();
        engine
    }

    pub(crate) fn allocate_id(&mut self) -> NodeId {
        let id = self.next_node_id;
        self.next_node_id += 1;
        id
    }

    pub fn primary_selected_id(&self) -> Option<NodeId> {
        self.selected_ids.last().copied()
    }

    pub fn is_selected(&self, id: NodeId) -> bool {
        self.selected_ids.contains(&id)
    }

    pub fn find_node(&self, id: NodeId) -> Option<&SceneNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn find_node_mut(&mut self, id: NodeId) -> Option<&mut SceneNode> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    pub fn node_depth(&self, id: NodeId) -> usize {
        let mut depth = 0usize;
        let mut cur = self.find_node(id).and_then(|n| n.parent_id);
        while let Some(pid) = cur {
            depth += 1;
            if depth >= self.nodes.len() {
                break;
            }
            cur = self.find_node(pid).and_then(|n| n.parent_id);
        }
        depth
    }

    pub fn node_world_matrix(&self, id: NodeId) -> Mat4 {
        let mut chain = Vec::new();
        let mut cur = Some(id);
        let mut guard = 0usize;
        while let Some(nid) = cur {
            if let Some(node) = self.find_node(nid) {
                chain.push(node.transform.to_matrix());
                cur = node.parent_id;
                guard += 1;
                if guard > self.nodes.len() {
                    break;
                }
            } else {
                break;
            }
        }
        let mut world = Mat4::IDENTITY;
        for local in chain.into_iter().rev() {
            world = world.mul_mat4(local);
        }
        world
    }

    pub fn parent_world_matrix(&self, id: NodeId) -> Mat4 {
        match self.find_node(id).and_then(|n| n.parent_id) {
            Some(pid) => self.node_world_matrix(pid),
            None => Mat4::IDENTITY,
        }
    }

    pub fn would_create_cycle(&self, child_id: NodeId, new_parent_id: Option<NodeId>) -> bool {
        let mut cur = new_parent_id;
        let mut steps = 0usize;
        while let Some(pid) = cur {
            if pid == child_id {
                return true;
            }
            cur = self.find_node(pid).and_then(|n| n.parent_id);
            steps += 1;
            if steps > self.nodes.len() {
                return true;
            }
        }
        false
    }

    pub(crate) fn unique_name(&self, base: &str) -> String {
        if !self.nodes.iter().any(|n| n.name == base) {
            return base.to_string();
        }
        let mut idx = 2usize;
        loop {
            let candidate = format!("{base} {idx}");
            if !self.nodes.iter().any(|n| n.name == candidate) {
                return candidate;
            }
            idx += 1;
        }
    }

    pub fn select_node(&mut self, id: Option<NodeId>, additive: bool) -> bool {
        match id {
            None => {
                if !additive {
                    self.selected_ids.clear();
                    self.scene_revision += 1;
                }
                true
            }
            Some(target_id) => {
                if self.find_node(target_id).is_none() {
                    return false;
                }
                if additive {
                    if let Some(pos) = self.selected_ids.iter().position(|&x| x == target_id) {
                        self.selected_ids.remove(pos);
                    } else {
                        self.selected_ids.push(target_id);
                    }
                } else {
                    self.selected_ids.clear();
                    self.selected_ids.push(target_id);
                }
                self.scene_revision += 1;
                true
            }
        }
    }

    pub fn add_primitive(&mut self, prim: PrimitiveType) -> NodeId {
        let id = self.allocate_id();
        let kind = match prim {
            PrimitiveType::Empty => NodeKind::Empty,
            PrimitiveType::Camera => NodeKind::Camera,
            PrimitiveType::Light => NodeKind::Light,
            _ => NodeKind::Mesh,
        };
        let base_poly = PolyMesh::from_primitive(prim);
        let mesh = base_poly
            .as_ref()
            .map(|p| p.to_mesh_data(1))
            .or_else(|| build_primitive_mesh(prim));
        let default_pos = match prim {
            PrimitiveType::Light => Vec3::new(1.8, 2.5, 1.8),
            PrimitiveType::Camera => Vec3::new(0.0, 1.4, 3.6),
            _ => Vec3::new(0.0, 0.75, 0.0),
        };
        let palette = [
            [0.32, 0.62, 0.92, 1.0],
            [0.86, 0.45, 0.30, 1.0],
            [0.34, 0.76, 0.56, 1.0],
            [0.84, 0.68, 0.28, 1.0],
            [0.66, 0.46, 0.86, 1.0],
        ];
        let color = palette[(id as usize) % palette.len()];

        let node = SceneNode {
            id,
            parent_id: None,
            name: self.unique_name(prim.default_name()),
            kind,
            transform: Transform {
                position: default_pos,
                rotation_euler: Vec3::ZERO,
                scale: Vec3::ONE,
            },
            mesh,
            base_poly,
            modifiers: Vec::new(),
            material_id: self.materials.first().map(|m| m.id),
            material: MaterialParams {
                base_color: color,
                metallic: 0.08,
                roughness: 0.40,
            },
            light: LightParams::default(),
            camera_node: CameraNodeParams::default(),
            visible: true,
        };

        let cmd = AddNodesCommand {
            added: vec![node],
            prev_selected: self.selected_ids.clone(),
            next_selected: vec![id],
        };
        self.history
            .execute(Box::new(cmd), &mut self.nodes, &mut self.selected_ids);
        self.scene_revision += 1;
        id
    }

    pub fn duplicate_selected(&mut self) -> Vec<NodeId> {
        if self.selected_ids.is_empty() {
            return Vec::new();
        }

        // Collect selected nodes + all descendants in deterministic order
        let mut to_copy: HashSet<NodeId> = self.selected_ids.iter().copied().collect();
        let mut changed = true;
        while changed {
            changed = false;
            for node in &self.nodes {
                if let Some(pid) = node.parent_id {
                    if to_copy.contains(&pid) && !to_copy.contains(&node.id) {
                        to_copy.insert(node.id);
                        changed = true;
                    }
                }
            }
        }

        let ordered_originals: Vec<SceneNode> = self
            .nodes
            .iter()
            .filter(|n| to_copy.contains(&n.id))
            .cloned()
            .collect();

        let mut id_map = HashMap::new();
        for orig in &ordered_originals {
            id_map.insert(orig.id, self.allocate_id());
        }

        let mut clones = Vec::with_capacity(ordered_originals.len());
        let mut new_top_selected = Vec::new();
        for orig in &ordered_originals {
            let new_id = id_map[&orig.id];
            let new_parent = match orig.parent_id {
                Some(pid) if id_map.contains_key(&pid) => Some(id_map[&pid]),
                other => other,
            };
            let mut cloned = orig.clone();
            cloned.id = new_id;
            cloned.parent_id = new_parent;
            cloned.name = self.unique_name(&format!("{} Copy", orig.name));
            // Offset only roots of the duplicated selection slightly along +X for visual clarity
            if orig.parent_id.map_or(true, |pid| !id_map.contains_key(&pid)) {
                cloned.transform.position.x += 0.4;
                if self.selected_ids.contains(&orig.id) {
                    new_top_selected.push(new_id);
                }
            }
            clones.push(cloned);
        }
        if new_top_selected.is_empty() {
            new_top_selected = clones.iter().map(|c| c.id).collect();
        }

        let cmd = AddNodesCommand {
            added: clones,
            prev_selected: self.selected_ids.clone(),
            next_selected: new_top_selected.clone(),
        };
        self.history
            .execute(Box::new(cmd), &mut self.nodes, &mut self.selected_ids);
        self.scene_revision += 1;
        new_top_selected
    }

    pub fn delete_selected(&mut self) -> bool {
        if self.selected_ids.is_empty() {
            return false;
        }
        let mut to_delete: HashSet<NodeId> = self.selected_ids.iter().copied().collect();
        // Also collect descendants of deleted nodes so hierarchy stays consistent
        let mut changed = true;
        while changed {
            changed = false;
            for node in &self.nodes {
                if let Some(pid) = node.parent_id {
                    if to_delete.contains(&pid) && !to_delete.contains(&node.id) {
                        to_delete.insert(node.id);
                        changed = true;
                    }
                }
            }
        }

        let mut removed = Vec::new();
        for (idx, node) in self.nodes.iter().enumerate() {
            if to_delete.contains(&node.id) {
                removed.push((idx, node.clone()));
            }
        }
        if removed.is_empty() {
            return false;
        }

        let cmd = DeleteNodesCommand {
            removed,
            parent_updates: Vec::new(),
            prev_selected: self.selected_ids.clone(),
        };
        self.history
            .execute(Box::new(cmd), &mut self.nodes, &mut self.selected_ids);
        self.scene_revision += 1;
        true
    }

    pub fn rename_node(&mut self, id: NodeId, new_name: String) -> bool {
        let trimmed = new_name.trim().to_string();
        if trimmed.is_empty() {
            return false;
        }
        let Some(existing) = self.find_node(id) else {
            return false;
        };
        if existing.name == trimmed {
            return true;
        }
        let cmd = RenameNodeCommand {
            node_id: id,
            before: existing.name.clone(),
            after: trimmed,
        };
        self.history
            .execute(Box::new(cmd), &mut self.nodes, &mut self.selected_ids);
        self.scene_revision += 1;
        true
    }

    pub fn set_node_visibility(&mut self, id: NodeId, visible: bool) -> bool {
        let Some(existing) = self.find_node(id) else {
            return false;
        };
        if existing.visible == visible {
            return true;
        }
        let cmd = SetVisibilityCommand {
            node_id: id,
            before: existing.visible,
            after: visible,
        };
        self.history
            .execute(Box::new(cmd), &mut self.nodes, &mut self.selected_ids);
        self.scene_revision += 1;
        true
    }

    pub fn set_node_material(&mut self, id: NodeId, material: MaterialParams) -> bool {
        let Some(existing) = self.find_node(id).cloned() else {
            return false;
        };
        let clean = MaterialParams {
            base_color: [
                material.base_color[0].clamp(0.0, 1.0),
                material.base_color[1].clamp(0.0, 1.0),
                material.base_color[2].clamp(0.0, 1.0),
                material.base_color[3].clamp(0.0, 1.0),
            ],
            metallic: material.metallic.clamp(0.0, 1.0),
            roughness: material.roughness.clamp(0.04, 1.0),
        };
        if let Some(mid) = existing.material_id {
            if let Some(mut mat_spec) = self.find_material(mid).cloned() {
                mat_spec.base_color = clean.base_color;
                mat_spec.metallic = clean.metallic;
                mat_spec.roughness = clean.roughness;
                if mat_spec.base_color[3] < 0.995 {
                    mat_spec.alpha_mode = AlphaMode::Blend;
                }
                return self.update_material(mat_spec);
            }
        }
        if existing.material == clean {
            return true;
        }
        let cmd = SetMaterialCommand {
            node_id: id,
            before: existing.material,
            after: clean,
        };
        self.history
            .execute(Box::new(cmd), &mut self.nodes, &mut self.selected_ids);
        self.scene_revision += 1;
        true
    }

    pub fn set_node_light(&mut self, id: NodeId, light: LightParams) -> bool {
        let Some(existing) = self.find_node(id) else {
            return false;
        };
        let clean = LightParams {
            kind: light.kind,
            color: [
                light.color[0].clamp(0.0, 1.0),
                light.color[1].clamp(0.0, 1.0),
                light.color[2].clamp(0.0, 1.0),
            ],
            intensity: light.intensity.clamp(0.0, 500000.0),
            cast_shadows: light.cast_shadows,
            spot_inner_deg: light.spot_inner_deg.clamp(1.0, 80.0),
            spot_outer_deg: light.spot_outer_deg.clamp(light.spot_inner_deg.clamp(1.0, 80.0) + 1.0, 89.0),
            range: light.range.clamp(0.5, 120.0),
        };
        if existing.light == clean {
            return true;
        }
        let cmd = SetLightCommand {
            node_id: id,
            before: existing.light.clone(),
            after: clean,
        };
        self.history
            .execute(Box::new(cmd), &mut self.nodes, &mut self.selected_ids);
        self.scene_revision += 1;
        true
    }

    pub fn set_node_transform(&mut self, id: NodeId, new_transform: Transform) -> bool {
        let Some(existing) = self.find_node(id) else {
            return false;
        };
        let old_transform = existing.transform;
        if old_transform == new_transform {
            return true;
        }
        let cmd = SetTransformCommand::single(id, old_transform, new_transform);
        self.history
            .execute(Box::new(cmd), &mut self.nodes, &mut self.selected_ids);
        if self.animation.timeline.auto_key && !self.animation.timeline.playing {
            let frame = self.animation.timeline.current_frame.round() as i32;
            let _ = self.record_transform_autokey(id, old_transform, new_transform, frame);
        }
        self.scene_revision += 1;
        true
    }

    pub fn reparent_node(&mut self, child_id: NodeId, new_parent_id: Option<NodeId>) -> bool {
        if self.find_node(child_id).is_none() {
            return false;
        }
        if let Some(pid) = new_parent_id {
            if self.find_node(pid).is_none() || self.would_create_cycle(child_id, Some(pid)) {
                return false;
            }
        }
        let node = self.find_node(child_id).unwrap();
        if node.parent_id == new_parent_id {
            return true;
        }
        let old_parent = node.parent_id;
        let old_transform = node.transform;

        // Preserve world transform when changing parent
        let world_m = self.node_world_matrix(child_id);
        let new_parent_inv = match new_parent_id {
            Some(pid) => self
                .node_world_matrix(pid)
                .inverse()
                .unwrap_or(Mat4::IDENTITY),
            None => Mat4::IDENTITY,
        };
        let local_m = new_parent_inv.mul_mat4(world_m);
        let (position, rotation_euler, scale) = local_m.decompose_trs();
        let new_transform = Transform {
            position,
            rotation_euler,
            scale,
        };

        let cmd = ReparentNodesCommand {
            optional_created_group: None,
            changes: vec![ReparentChange {
                node_id: child_id,
                old_parent,
                new_parent: new_parent_id,
                old_transform,
                new_transform,
            }],
            prev_selected: self.selected_ids.clone(),
            next_selected: self.selected_ids.clone(),
        };
        self.history
            .execute(Box::new(cmd), &mut self.nodes, &mut self.selected_ids);
        self.scene_revision += 1;
        true
    }

    pub fn group_selected(&mut self) -> Option<NodeId> {
        if self.selected_ids.is_empty() {
            return None;
        }
        // Group top-most selected nodes around their mean world position
        let selected_set: HashSet<NodeId> = self.selected_ids.iter().copied().collect();
        let mut root_ids = Vec::new();
        for &id in &self.selected_ids {
            let mut has_selected_ancestor = false;
            let mut cur = self.find_node(id).and_then(|n| n.parent_id);
            while let Some(pid) = cur {
                if selected_set.contains(&pid) {
                    has_selected_ancestor = true;
                    break;
                }
                cur = self.find_node(pid).and_then(|n| n.parent_id);
            }
            if !has_selected_ancestor {
                root_ids.push(id);
            }
        }
        if root_ids.is_empty() {
            return None;
        }

        let mut center = Vec3::ZERO;
        for &id in &root_ids {
            center = center + self.node_world_matrix(id).transform_point(Vec3::ZERO);
        }
        center = center * (1.0 / root_ids.len() as f32);

        let group_id = self.allocate_id();
        let group_node = SceneNode {
            id: group_id,
            parent_id: None,
            name: self.unique_name("Group"),
            kind: NodeKind::Empty,
            transform: Transform {
                position: center,
                rotation_euler: Vec3::ZERO,
                scale: Vec3::ONE,
            },
            mesh: None,
            base_poly: None,
            modifiers: Vec::new(),
            material_id: None,
            material: MaterialParams::default(),
            light: LightParams::default(),
            camera_node: CameraNodeParams::default(),
            visible: true,
        };
        let group_inv = group_node
            .transform
            .to_matrix()
            .inverse()
            .unwrap_or(Mat4::IDENTITY);

        let mut changes = Vec::with_capacity(root_ids.len());
        for &id in &root_ids {
            let node = self.find_node(id).unwrap();
            let world_m = self.node_world_matrix(id);
            let local_m = group_inv.mul_mat4(world_m);
            let (position, rotation_euler, scale) = local_m.decompose_trs();
            changes.push(ReparentChange {
                node_id: id,
                old_parent: node.parent_id,
                new_parent: Some(group_id),
                old_transform: node.transform,
                new_transform: Transform {
                    position,
                    rotation_euler,
                    scale,
                },
            });
        }

        let cmd = ReparentNodesCommand {
            optional_created_group: Some(group_node),
            changes,
            prev_selected: self.selected_ids.clone(),
            next_selected: vec![group_id],
        };
        self.history
            .execute(Box::new(cmd), &mut self.nodes, &mut self.selected_ids);
        self.scene_revision += 1;
        Some(group_id)
    }

    pub fn unparent_selected(&mut self) -> bool {
        if self.selected_ids.is_empty() {
            return false;
        }
        let mut changes = Vec::new();
        for &id in &self.selected_ids {
            let Some(node) = self.find_node(id) else {
                continue;
            };
            if node.parent_id.is_none() {
                continue;
            }
            let world_m = self.node_world_matrix(id);
            let (position, rotation_euler, scale) = world_m.decompose_trs();
            changes.push(ReparentChange {
                node_id: id,
                old_parent: node.parent_id,
                new_parent: None,
                old_transform: node.transform,
                new_transform: Transform {
                    position,
                    rotation_euler,
                    scale,
                },
            });
        }
        if changes.is_empty() {
            return false;
        }
        let cmd = ReparentNodesCommand {
            optional_created_group: None,
            changes,
            prev_selected: self.selected_ids.clone(),
            next_selected: self.selected_ids.clone(),
        };
        self.history
            .execute(Box::new(cmd), &mut self.nodes, &mut self.selected_ids);
        self.scene_revision += 1;
        true
    }

    pub fn apply_transform_selected(&mut self) -> bool {
        let Some(id) = self.primary_selected_id() else {
            return false;
        };
        let Some(node) = self.find_node(id) else {
            return false;
        };
        let Some(old_mesh) = &node.mesh else {
            return false;
        };
        let old_transform = node.transform;
        let m = old_transform.to_matrix();
        let inv_m = m.inverse().unwrap_or(Mat4::IDENTITY);

        let mut new_positions = Vec::with_capacity(old_mesh.positions.len());
        for p in &old_mesh.positions {
            let wp = m.transform_point(Vec3::new(p[0], p[1], p[2]));
            new_positions.push([wp.x, wp.y, wp.z]);
        }

        // Normal matrix = transpose(inverse(M_3x3))
        let mut new_normals = Vec::with_capacity(old_mesh.normals.len());
        for n in &old_mesh.normals {
            let d = inv_m.data;
            let nx = d[0] * n[0] + d[1] * n[1] + d[2] * n[2];
            let ny = d[4] * n[0] + d[5] * n[1] + d[6] * n[2];
            let nz = d[8] * n[0] + d[9] * n[1] + d[10] * n[2];
            let nn = Vec3::new(nx, ny, nz).normalized_or(Vec3::UNIT_Y);
            new_normals.push([nn.x, nn.y, nn.z]);
        }

        let mut new_indices = old_mesh.indices.clone();
        // Flip winding if negative determinant (odd negative scales)
        let det3 = old_transform.scale.x * old_transform.scale.y * old_transform.scale.z;
        if det3 < 0.0 {
            for tri in new_indices.chunks_exact_mut(3) {
                tri.swap(1, 2);
            }
        }

        let (bounds_min, bounds_max) = MeshData::compute_bounds(&new_positions);
        let new_mesh = MeshData {
            positions: new_positions,
            normals: new_normals,
            uvs: old_mesh.uvs.clone(),
            indices: new_indices,
            bounds_min,
            bounds_max,
            version: old_mesh.version + 1,
        };

        let old_poly = node.base_poly.clone();
        let new_poly = old_poly.as_ref().map(|p| {
            let mut copy = p.clone();
            for v in &mut copy.vertices {
                *v = m.transform_point(*v);
            }
            if det3 < 0.0 {
                for f in &mut copy.faces {
                    f.reverse();
                }
            }
            copy
        });

        let cmd = BakeMeshTransformCommand {
            node_id: id,
            old_mesh: old_mesh.clone(),
            new_mesh,
            old_poly,
            new_poly,
            old_transform,
            new_transform: Transform::default(),
        };
        self.history
            .execute(Box::new(cmd), &mut self.nodes, &mut self.selected_ids);
        self.scene_revision += 1;
        true
    }

    pub fn origin_to_geometry_selected(&mut self) -> bool {
        let Some(id) = self.primary_selected_id() else {
            return false;
        };
        let Some(node) = self.find_node(id) else {
            return false;
        };
        let Some(old_mesh) = &node.mesh else {
            return false;
        };
        let local_center = (old_mesh.bounds_min + old_mesh.bounds_max) * 0.5;
        if local_center.length_squared() < 1e-8 {
            return true;
        }

        let new_positions: Vec<[f32; 3]> = old_mesh
            .positions
            .iter()
            .map(|p| [p[0] - local_center.x, p[1] - local_center.y, p[2] - local_center.z])
            .collect();
        let (bounds_min, bounds_max) = MeshData::compute_bounds(&new_positions);
        let new_mesh = MeshData {
            positions: new_positions,
            normals: old_mesh.normals.clone(),
            uvs: old_mesh.uvs.clone(),
            indices: old_mesh.indices.clone(),
            bounds_min,
            bounds_max,
            version: old_mesh.version + 1,
        };

        let mut new_transform = node.transform;
        let world_offset = node.transform.to_matrix().transform_vector(local_center);
        new_transform.position = new_transform.position + world_offset;

        let old_poly = node.base_poly.clone();
        let new_poly = old_poly.as_ref().map(|p| {
            let mut copy = p.clone();
            for v in &mut copy.vertices {
                *v = *v - local_center;
            }
            copy
        });

        let cmd = BakeMeshTransformCommand {
            node_id: id,
            old_mesh: old_mesh.clone(),
            new_mesh,
            old_poly,
            new_poly,
            old_transform: node.transform,
            new_transform,
        };
        self.history
            .execute(Box::new(cmd), &mut self.nodes, &mut self.selected_ids);
        self.scene_revision += 1;
        true
    }

    pub fn undo(&mut self) -> bool {
        self.gizmo.drag = None;
        self.gizmo.active_axis = GizmoAxis::None;
        if self.history.undo_all(
            &mut self.nodes,
            &mut self.selected_ids,
            &mut self.materials,
            &mut self.selected_material_id,
            &mut self.environment,
            &mut self.animation,
        ) {
            self.eval_animation_at_current_frame();
            self.sanitize_edit_state_after_history();
            self.rebuild_edit_overlay();
            self.environment_version += 1;
            self.scene_revision += 1;
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        self.gizmo.drag = None;
        self.gizmo.active_axis = GizmoAxis::None;
        if self.history.redo_all(
            &mut self.nodes,
            &mut self.selected_ids,
            &mut self.materials,
            &mut self.selected_material_id,
            &mut self.environment,
            &mut self.animation,
        ) {
            self.eval_animation_at_current_frame();
            self.sanitize_edit_state_after_history();
            self.rebuild_edit_overlay();
            self.environment_version += 1;
            self.scene_revision += 1;
            true
        } else {
            false
        }
    }

    pub fn world_bounds(&self) -> Option<(Vec3, Vec3)> {
        let mut has_any = false;
        let mut min_v = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
        let mut max_v = Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);

        for node in &self.nodes {
            if !node.visible {
                continue;
            }
            let m = self.node_world_matrix(node.id);
            if let Some(mesh) = &node.mesh {
                for p in &mesh.positions {
                    let wp = m.transform_point(Vec3::new(p[0], p[1], p[2]));
                    min_v = min_v.min(wp);
                    max_v = max_v.max(wp);
                    has_any = true;
                }
            } else {
                let wp = m.transform_point(Vec3::ZERO);
                min_v = min_v.min(wp - Vec3::new(0.25, 0.25, 0.25));
                max_v = max_v.max(wp + Vec3::new(0.25, 0.25, 0.25));
                has_any = true;
            }
        }

        if has_any {
            Some((min_v, max_v))
        } else {
            None
        }
    }

    pub fn node_world_bounds(&self, id: NodeId) -> Option<(Vec3, Vec3)> {
        let node = self.find_node(id)?;
        let m = self.node_world_matrix(id);
        if let Some(mesh) = &node.mesh {
            let mut min_v = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
            let mut max_v = Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
            for p in &mesh.positions {
                let wp = m.transform_point(Vec3::new(p[0], p[1], p[2]));
                min_v = min_v.min(wp);
                max_v = max_v.max(wp);
            }
            Some((min_v, max_v))
        } else {
            let wp = m.transform_point(Vec3::ZERO);
            Some((wp - Vec3::new(0.35, 0.35, 0.35), wp + Vec3::new(0.35, 0.35, 0.35)))
        }
    }

    pub fn frame_all(&mut self) {
        if let Some((min_v, max_v)) = self.world_bounds() {
            self.camera.frame_bounds(min_v, max_v);
        } else {
            self.camera
                .frame_bounds(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        }
    }

    pub fn focus_selected(&mut self) {
        if let Some(id) = self.primary_selected_id() {
            if let Some((min_v, max_v)) = self.node_world_bounds(id) {
                self.camera.frame_bounds(min_v, max_v);
                return;
            }
        }
        self.frame_all();
    }

    pub fn screen_ray(&self, ndc_x: f32, ndc_y: f32, aspect: f32) -> Ray {
        let (eye, right, up) = self.camera.basis();
        let forward = (self.camera.target - eye).normalized_or(Vec3::new(0.0, 0.0, -1.0));
        let safe_aspect = aspect.max(0.05);

        match self.camera.projection {
            ProjectionMode::Perspective => {
                let tan_half = (self.camera.fov_y_degrees * 0.5).to_radians().tan();
                let dir = (forward + right * (ndc_x * safe_aspect * tan_half) + up * (ndc_y * tan_half))
                    .normalized_or(forward);
                Ray { origin: eye, dir }
            }
            ProjectionMode::Orthographic => {
                let half_h = self.camera.visible_height_at_target() * 0.5;
                let half_w = half_h * safe_aspect;
                let origin = eye + right * (ndc_x * half_w) + up * (ndc_y * half_h);
                Ray {
                    origin,
                    dir: forward,
                }
            }
        }
    }

    pub fn gizmo_world_pose(&self) -> Option<(Vec3, [Vec3; 3], f32)> {
        if self.gizmo.mode == GizmoMode::None {
            return None;
        }
        if self.interaction_mode == InteractionMode::Edit {
            let nid = self.edit_node_id?;
            let node = self.find_node(nid)?;
            let poly = node.base_poly.as_ref()?;
            let verts = poly.selected_vertex_closure(
                self.edit_select_mode,
                &self.edit_sel_verts,
                &self.edit_sel_edges,
                &self.edit_sel_faces,
            );
            if verts.is_empty() {
                return None;
            }
            let mut sum = Vec3::ZERO;
            let mut count = 0usize;
            for vi in &verts {
                if let Some(&p) = poly.vertices.get(*vi) {
                    sum = sum + p;
                    count += 1;
                }
            }
            if count == 0 {
                return None;
            }
            let world_m = self.node_world_matrix(nid);
            let pivot = world_m.transform_point(sum * (1.0 / count as f32));
            let axes = match self.gizmo.space {
                GizmoSpace::World => [Vec3::UNIT_X, Vec3::UNIT_Y, Vec3::UNIT_Z],
                GizmoSpace::Local => world_m.basis_axes(),
            };
            let (eye, _, _) = self.camera.basis();
            let cam_dist = (eye - pivot).length().max(0.5);
            let scale = (cam_dist * 0.18).clamp(0.25, 18.0);
            return Some((pivot, axes, scale));
        }

        let primary_id = self.primary_selected_id()?;
        let _node = self.find_node(primary_id)?;
        let world_m = self.node_world_matrix(primary_id);
        let pivot = world_m.transform_point(Vec3::ZERO);
        let axes = match self.gizmo.space {
            GizmoSpace::World => [Vec3::UNIT_X, Vec3::UNIT_Y, Vec3::UNIT_Z],
            GizmoSpace::Local => world_m.basis_axes(),
        };
        let (eye, _, _) = self.camera.basis();
        let cam_dist = (eye - pivot).length().max(0.5);
        let scale = (cam_dist * 0.18).clamp(0.25, 18.0);
        Some((pivot, axes, scale))
    }

    pub fn pick_gizmo_axis(&self, ndc_x: f32, ndc_y: f32, aspect: f32) -> GizmoAxis {
        let Some((pivot, axes, scale)) = self.gizmo_world_pose() else {
            return GizmoAxis::None;
        };
        let ray = self.screen_ray(ndc_x, ndc_y, aspect);
        let hit_radius = scale * 0.16;

        // Center uniform scale / plane move handle near pivot
        if self.gizmo.mode == GizmoMode::Scale || self.gizmo.mode == GizmoMode::Move {
            let (dist_center, _) = ray.distance_to_segment(pivot, pivot + Vec3::new(0.001, 0.0, 0.0));
            if dist_center <= hit_radius * 1.15 {
                return GizmoAxis::Center;
            }
        }

        let mut best_axis = GizmoAxis::None;
        let mut best_dist = hit_radius;

        match self.gizmo.mode {
            GizmoMode::Move | GizmoMode::Scale => {
                let candidates = [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z];
                for (i, axis) in candidates.iter().enumerate() {
                    let a = pivot + axes[i] * (scale * 0.14);
                    let b = pivot + axes[i] * (scale * 1.05);
                    let (dist, _) = ray.distance_to_segment(a, b);
                    if dist < best_dist {
                        best_dist = dist;
                        best_axis = *axis;
                    }
                }
            }
            GizmoMode::Rotate => {
                let candidates = [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z];
                let ring_r = scale * 0.88;
                let segs = 24usize;
                for (i, axis) in candidates.iter().enumerate() {
                    let u = axes[(i + 1) % 3];
                    let v = axes[(i + 2) % 3];
                    for s in 0..segs {
                        let t0 = (s as f32 / segs as f32) * TAU;
                        let t1 = ((s + 1) as f32 / segs as f32) * TAU;
                        let p0 = pivot + (u * t0.cos() + v * t0.sin()) * ring_r;
                        let p1 = pivot + (u * t1.cos() + v * t1.sin()) * ring_r;
                        let (dist, _) = ray.distance_to_segment(p0, p1);
                        if dist < best_dist {
                            best_dist = dist;
                            best_axis = *axis;
                        }
                    }
                }
            }
            GizmoMode::None => {}
        }

        best_axis
    }

    pub fn begin_gizmo_drag(
        &mut self,
        axis: GizmoAxis,
        ndc_x: f32,
        ndc_y: f32,
        aspect: f32,
    ) -> bool {
        if axis == GizmoAxis::None {
            return false;
        }
        let Some((pivot, axes, scale)) = self.gizmo_world_pose() else {
            return false;
        };

        let axis_dir = match axis {
            GizmoAxis::X => axes[0],
            GizmoAxis::Y => axes[1],
            GizmoAxis::Z => axes[2],
            GizmoAxis::Center => Vec3::UNIT_Y,
            GizmoAxis::None => return false,
        };

        let ray = self.screen_ray(ndc_x, ndc_y, aspect);
        let (eye, _, _) = self.camera.basis();
        let view_dir = (pivot - eye).normalized_or(Vec3::new(0.0, 0.0, -1.0));
        let start_param = ray.closest_param_on_axis(pivot, axis_dir);
        let plane_normal = if self.gizmo.mode == GizmoMode::Rotate && axis != GizmoAxis::Center {
            axis_dir
        } else {
            view_dir
        };
        let start_plane_hit = ray.intersect_plane(pivot, plane_normal);

        if self.interaction_mode == InteractionMode::Edit {
            let Some(nid) = self.edit_node_id else {
                return false;
            };
            let Some(node) = self.find_node(nid) else {
                return false;
            };
            let Some(poly) = node.base_poly.clone() else {
                return false;
            };
            let Some(mesh) = node.mesh.clone() else {
                return false;
            };
            let verts = poly.selected_vertex_closure(
                self.edit_select_mode,
                &self.edit_sel_verts,
                &self.edit_sel_edges,
                &self.edit_sel_faces,
            );
            if verts.is_empty() {
                return false;
            }
            let init_verts: Vec<(usize, Vec3)> = verts
                .into_iter()
                .filter_map(|vi| poly.vertices.get(vi).copied().map(|p| (vi, p)))
                .collect();
            if init_verts.is_empty() {
                return false;
            }
            let world_m = self.node_world_matrix(nid);
            let world_inv = world_m.inverse().unwrap_or(Mat4::IDENTITY);
            let pivot_local = world_inv.transform_point(pivot);

            self.gizmo.active_axis = axis;
            self.gizmo.drag = Some(GizmoDragSession {
                axis,
                mode: self.gizmo.mode,
                pivot_world: pivot,
                axis_dir_world: axis_dir,
                scale_size: scale,
                start_param,
                start_plane_hit,
                targets: Vec::new(),
                edit_vertices: Some((nid, poly, mesh, world_m, world_inv, pivot_local, init_verts)),
            });
            return true;
        }

        // Top-most selected nodes only so parent+child don't double-apply
        let selected_set: HashSet<NodeId> = self.selected_ids.iter().copied().collect();
        let mut targets = Vec::new();
        for &id in &self.selected_ids {
            let mut has_sel_ancestor = false;
            let mut cur = self.find_node(id).and_then(|n| n.parent_id);
            while let Some(pid) = cur {
                if selected_set.contains(&pid) {
                    has_sel_ancestor = true;
                    break;
                }
                cur = self.find_node(pid).and_then(|n| n.parent_id);
            }
            if !has_sel_ancestor {
                if let Some(node) = self.find_node(id) {
                    let wm = self.node_world_matrix(id);
                    let pinv = self.parent_world_matrix(id).inverse().unwrap_or(Mat4::IDENTITY);
                    targets.push((id, node.transform, wm, pinv));
                }
            }
        }
        if targets.is_empty() {
            return false;
        }

        self.gizmo.active_axis = axis;
        self.gizmo.drag = Some(GizmoDragSession {
            axis,
            mode: self.gizmo.mode,
            pivot_world: pivot,
            axis_dir_world: axis_dir,
            scale_size: scale,
            start_param,
            start_plane_hit,
            targets,
            edit_vertices: None,
        });
        true
    }

    pub fn update_gizmo_drag(&mut self, ndc_x: f32, ndc_y: f32, aspect: f32) -> bool {
        let Some(drag) = self.gizmo.drag.clone() else {
            return false;
        };
        let ray = self.screen_ray(ndc_x, ndc_y, aspect);
        let (eye, _, _) = self.camera.basis();
        let view_dir = (drag.pivot_world - eye).normalized_or(Vec3::new(0.0, 0.0, -1.0));

        if let Some((nid, _init_poly, _init_mesh, _wm, world_inv, pivot_local, ref init_verts)) =
            drag.edit_vertices
        {
            match drag.mode {
                GizmoMode::Move => {
                    let world_delta = if drag.axis == GizmoAxis::Center {
                        let Some(start_hit) = drag.start_plane_hit else {
                            return false;
                        };
                        let Some(cur_hit) = ray.intersect_plane(drag.pivot_world, view_dir) else {
                            return false;
                        };
                        cur_hit - start_hit
                    } else {
                        let cur_param = ray.closest_param_on_axis(drag.pivot_world, drag.axis_dir_world);
                        drag.axis_dir_world * (cur_param - drag.start_param)
                    };
                    let local_delta = world_inv.transform_vector(world_delta);
                    if let Some(node) = self.find_node_mut(nid) {
                        if let Some(poly) = node.base_poly.as_mut() {
                            for &(vi, init_p) in init_verts {
                                if let Some(v) = poly.vertices.get_mut(vi) {
                                    *v = init_p + local_delta;
                                }
                            }
                        }
                        node.rebuild_evaluated_mesh(true);
                    }
                    self.rebuild_edit_overlay();
                    self.scene_revision += 1;
                    return true;
                }
                GizmoMode::Rotate => {
                    let plane_normal = drag.axis_dir_world;
                    let Some(start_hit) = drag.start_plane_hit else {
                        return false;
                    };
                    let Some(cur_hit) = ray.intersect_plane(drag.pivot_world, plane_normal) else {
                        return false;
                    };
                    let v0 = (start_hit - drag.pivot_world).normalized_or(Vec3::UNIT_X);
                    let v1 = (cur_hit - drag.pivot_world).normalized_or(Vec3::UNIT_X);
                    let cos_a = v0.dot(v1).clamp(-1.0, 1.0);
                    let sin_a = v0.cross(v1).dot(plane_normal);
                    let angle = sin_a.atan2(cos_a);
                    let local_axis = world_inv
                        .transform_vector(plane_normal)
                        .normalized_or(Vec3::UNIT_Y);
                    let (sa, ca) = angle.sin_cos();
                    if let Some(node) = self.find_node_mut(nid) {
                        if let Some(poly) = node.base_poly.as_mut() {
                            for &(vi, init_p) in init_verts {
                                if let Some(v) = poly.vertices.get_mut(vi) {
                                    let rel = init_p - pivot_local;
                                    let rot = rel * ca
                                        + local_axis.cross(rel) * sa
                                        + local_axis * (local_axis.dot(rel) * (1.0 - ca));
                                    *v = pivot_local + rot;
                                }
                            }
                        }
                        node.rebuild_evaluated_mesh(true);
                    }
                    self.rebuild_edit_overlay();
                    self.scene_revision += 1;
                    return true;
                }
                GizmoMode::Scale => {
                    let cur_param = ray.closest_param_on_axis(drag.pivot_world, drag.axis_dir_world);
                    let delta = (cur_param - drag.start_param) / drag.scale_size.max(0.25);
                    let factor = (1.0 + delta).clamp(0.05, 50.0);
                    let local_axis = world_inv
                        .transform_vector(drag.axis_dir_world)
                        .normalized_or(Vec3::UNIT_X);
                    if let Some(node) = self.find_node_mut(nid) {
                        if let Some(poly) = node.base_poly.as_mut() {
                            for &(vi, init_p) in init_verts {
                                if let Some(v) = poly.vertices.get_mut(vi) {
                                    let rel = init_p - pivot_local;
                                    let scaled = if drag.axis == GizmoAxis::Center {
                                        rel * factor
                                    } else {
                                        let proj = local_axis * local_axis.dot(rel);
                                        let perp = rel - proj;
                                        perp + proj * factor
                                    };
                                    *v = pivot_local + scaled;
                                }
                            }
                        }
                        node.rebuild_evaluated_mesh(true);
                    }
                    self.rebuild_edit_overlay();
                    self.scene_revision += 1;
                    return true;
                }
                GizmoMode::None => return false,
            }
        }

        match drag.mode {
            GizmoMode::Move => {
                let world_delta = if drag.axis == GizmoAxis::Center {
                    let Some(start_hit) = drag.start_plane_hit else {
                        return false;
                    };
                    let Some(cur_hit) = ray.intersect_plane(drag.pivot_world, view_dir) else {
                        return false;
                    };
                    cur_hit - start_hit
                } else {
                    let cur_param = ray.closest_param_on_axis(drag.pivot_world, drag.axis_dir_world);
                    drag.axis_dir_world * (cur_param - drag.start_param)
                };

                for (id, init_tr, _init_wm, pinv) in &drag.targets {
                    if let Some(node) = self.find_node_mut(*id) {
                        let local_delta = pinv.transform_vector(world_delta);
                        node.transform.position = init_tr.position + local_delta;
                    }
                }
                self.rebuild_edit_overlay();
                self.scene_revision += 1;
                true
            }
            GizmoMode::Rotate => {
                let plane_normal = drag.axis_dir_world;
                let Some(start_hit) = drag.start_plane_hit else {
                    return false;
                };
                let Some(cur_hit) = ray.intersect_plane(drag.pivot_world, plane_normal) else {
                    return false;
                };
                let v0 = (start_hit - drag.pivot_world).normalized_or(Vec3::UNIT_X);
                let v1 = (cur_hit - drag.pivot_world).normalized_or(Vec3::UNIT_X);
                let cos_a = v0.dot(v1).clamp(-1.0, 1.0);
                let sin_a = v0.cross(v1).dot(plane_normal);
                let angle = sin_a.atan2(cos_a);
                let delta_axis = match drag.axis {
                    GizmoAxis::X => Vec3::new(angle, 0.0, 0.0),
                    GizmoAxis::Y => Vec3::new(0.0, angle, 0.0),
                    GizmoAxis::Z => Vec3::new(0.0, 0.0, angle),
                    _ => Vec3::ZERO,
                };

                for (id, init_tr, _init_wm, _pinv) in &drag.targets {
                    if let Some(node) = self.find_node_mut(*id) {
                        node.transform.rotation_euler = init_tr.rotation_euler + delta_axis;
                    }
                }
                self.rebuild_edit_overlay();
                self.scene_revision += 1;
                true
            }
            GizmoMode::Scale => {
                let cur_param = ray.closest_param_on_axis(drag.pivot_world, drag.axis_dir_world);
                let delta = (cur_param - drag.start_param) / drag.scale_size.max(0.25);
                let factor = (1.0 + delta).clamp(0.05, 50.0);

                for (id, init_tr, _init_wm, _pinv) in &drag.targets {
                    if let Some(node) = self.find_node_mut(*id) {
                        let mut s = init_tr.scale;
                        match drag.axis {
                            GizmoAxis::X => s.x *= factor,
                            GizmoAxis::Y => s.y *= factor,
                            GizmoAxis::Z => s.z *= factor,
                            GizmoAxis::Center => s = s * factor,
                            GizmoAxis::None => {}
                        }
                        node.transform.scale = s;
                    }
                }
                self.rebuild_edit_overlay();
                self.scene_revision += 1;
                true
            }
            GizmoMode::None => false,
        }
    }

    pub fn end_gizmo_drag(&mut self) -> bool {
        let Some(drag) = self.gizmo.drag.take() else {
            self.gizmo.active_axis = GizmoAxis::None;
            return false;
        };
        self.gizmo.active_axis = GizmoAxis::None;

        if let Some((nid, old_poly, old_mesh, _, _, _, _)) = drag.edit_vertices {
            let Some(node) = self.find_node(nid) else {
                return false;
            };
            let (Some(new_poly), Some(new_mesh)) = (node.base_poly.clone(), node.mesh.clone()) else {
                return false;
            };
            if new_poly == old_poly {
                return false;
            }
            let cmd = SetPolyMeshCommand {
                node_id: nid,
                old_poly,
                new_poly,
                old_mesh,
                new_mesh,
            };
            self.history.push_executed(Box::new(cmd));
            self.rebuild_edit_overlay();
            self.scene_revision += 1;
            return true;
        }

        let mut changes = Vec::new();
        for (id, before, _, _) in drag.targets {
            if let Some(node) = self.find_node(id) {
                if node.transform != before {
                    changes.push((id, before, node.transform));
                }
            }
        }
        if changes.is_empty() {
            return false;
        }
        let auto_key = self.animation.timeline.auto_key && !self.animation.timeline.playing;
        let frame = self.animation.timeline.current_frame.round() as i32;
        let cmd = SetTransformCommand {
            changes: changes.clone(),
        };
        self.history.push_executed(Box::new(cmd));
        if auto_key {
            for (id, before, after) in changes {
                let _ = self.record_transform_autokey(id, before, after, frame);
            }
        }
        self.rebuild_edit_overlay();
        self.scene_revision += 1;
        true
    }

    pub fn pick_node_at_ndc(
        &mut self,
        ndc_x: f32,
        ndc_y: f32,
        aspect: f32,
        additive: bool,
    ) -> Option<NodeId> {
        if self.interaction_mode == InteractionMode::Edit {
            let hit = self.edit_pick_at_ndc(ndc_x, ndc_y, aspect, additive);
            return if hit { self.edit_node_id } else { None };
        }
        let ray = self.screen_ray(ndc_x, ndc_y, aspect);
        let mut best_hit: Option<(NodeId, f32)> = None;

        for node in &self.nodes {
            if !node.visible {
                continue;
            }
            let world_m = self.node_world_matrix(node.id);
            let Some(inv_m) = world_m.inverse() else {
                continue;
            };
            let local_origin = inv_m.transform_point(ray.origin);
            let local_dir = inv_m.transform_vector(ray.dir);
            if local_dir.length_squared() < 1e-10 {
                continue;
            }
            let local_ray = Ray {
                origin: local_origin,
                dir: local_dir,
            };

            let mut local_t: Option<f32> = None;
            if let Some(mesh) = &node.mesh {
                if local_ray
                    .intersect_aabb(mesh.bounds_min, mesh.bounds_max)
                    .is_some()
                {
                    for tri in mesh.indices.chunks_exact(3) {
                        let v0 = Vec3::from(mesh.positions[tri[0] as usize]);
                        let v1 = Vec3::from(mesh.positions[tri[1] as usize]);
                        let v2 = Vec3::from(mesh.positions[tri[2] as usize]);
                        if let Some(t) = local_ray.intersect_triangle(v0, v1, v2) {
                            if local_t.map_or(true, |best| t < best) {
                                local_t = Some(t);
                            }
                        }
                    }
                }
            } else {
                local_t = local_ray.intersect_aabb(
                    Vec3::new(-0.35, -0.35, -0.35),
                    Vec3::new(0.35, 0.35, 0.35),
                );
            }

            if let Some(lt) = local_t {
                let hit_world = world_m.transform_point(local_origin + local_dir * lt);
                let world_dist = (hit_world - ray.origin).length();
                if best_hit.map_or(true, |(_, bd)| world_dist < bd) {
                    best_hit = Some((node.id, world_dist));
                }
            }
        }

        let hit_id = best_hit.map(|(id, _)| id);
        self.select_node(hit_id, additive);
        hit_id
    }

    pub fn camera_snapshot(&self) -> CameraSnapshot {
        if let Some(cam_id) = self.active_camera_node_id {
            if let Some(cam_node) = self.find_node(cam_id) {
                if cam_node.kind == NodeKind::Camera {
                    let world = self.node_world_matrix(cam_id);
                    let eye = world.transform_point(Vec3::ZERO);
                    let forward = world
                        .transform_vector(Vec3::new(0.0, 0.0, -1.0))
                        .normalized_or(Vec3::new(0.0, 0.0, -1.0));
                    let up = world
                        .transform_vector(Vec3::new(0.0, 1.0, 0.0))
                        .normalized_or(Vec3::UNIT_Y);
                    let target = eye + forward * 5.0;
                    return CameraSnapshot {
                        eye: [eye.x, eye.y, eye.z],
                        target: [target.x, target.y, target.z],
                        up: [up.x, up.y, up.z],
                        projection: ProjectionMode::Perspective as u32,
                        fov_y_degrees: cam_node.camera_node.fov_y_degrees.clamp(15.0, 120.0),
                        ortho_half_height: 3.0,
                        near_plane: cam_node.camera_node.near_plane.max(0.01),
                        far_plane: cam_node.camera_node.far_plane.max(10.0),
                        distance: 5.0,
                        yaw_radians: cam_node.transform.rotation_euler.y,
                        pitch_radians: cam_node.transform.rotation_euler.x,
                    };
                }
            }
        }
        self.camera.snapshot()
    }

    pub fn set_projection(&mut self, mode: ProjectionMode) {
        self.camera.set_projection(mode);
    }

    pub fn set_standard_view(&mut self, view: StandardView) {
        self.camera.set_standard_view(view);
    }

    // -------------------------------------------------------------------------
    // M3: Edit Mode, Topology Operations, Overlay & Non-Destructive Modifiers
    // -------------------------------------------------------------------------

    pub fn active_mesh_node_id(&self) -> Option<NodeId> {
        if let Some(id) = self.edit_node_id {
            if self
                .find_node(id)
                .map_or(false, |n| n.kind == NodeKind::Mesh)
            {
                return Some(id);
            }
        }
        if let Some(pid) = self.primary_selected_id() {
            if self
                .find_node(pid)
                .map_or(false, |n| n.kind == NodeKind::Mesh)
            {
                return Some(pid);
            }
        }
        self.selected_ids.iter().rev().copied().find(|&id| {
            self.find_node(id)
                .map_or(false, |n| n.kind == NodeKind::Mesh)
        })
    }

    fn sanitize_edit_state_after_history(&mut self) {
        if let Some(cam_id) = self.active_camera_node_id {
            if !self
                .find_node(cam_id)
                .map_or(false, |n| n.kind == NodeKind::Camera)
            {
                self.active_camera_node_id = None;
            }
        }
        if self.interaction_mode == InteractionMode::Edit {
            let valid_poly = self
                .edit_node_id
                .and_then(|id| self.find_node(id))
                .filter(|n| n.kind == NodeKind::Mesh)
                .and_then(|n| n.base_poly.as_ref());
            let Some(poly) = valid_poly else {
                self.interaction_mode = InteractionMode::Object;
                self.edit_node_id = None;
                self.edit_sel_verts.clear();
                self.edit_sel_edges.clear();
                self.edit_sel_faces.clear();
                return;
            };
            let vc = poly.vertices.len();
            let fc = poly.faces.len();
            self.edit_sel_verts.retain(|&v| v < vc);
            self.edit_sel_edges.retain(|&(a, b)| a < vc && b < vc);
            self.edit_sel_faces.retain(|&f| f < fc);
        }
    }

    pub fn set_interaction_mode(&mut self, mode: InteractionMode) -> bool {
        self.gizmo.drag = None;
        self.gizmo.active_axis = GizmoAxis::None;
        if mode == self.interaction_mode {
            return true;
        }
        match mode {
            InteractionMode::Edit => {
                let Some(nid) = self.active_mesh_node_id() else {
                    return false;
                };
                if let Some(node) = self.find_node_mut(nid) {
                    node.ensure_base_poly();
                    node.rebuild_evaluated_mesh(true);
                }
                self.interaction_mode = InteractionMode::Edit;
                self.edit_node_id = Some(nid);
                self.edit_sel_verts.clear();
                self.edit_sel_edges.clear();
                self.edit_sel_faces.clear();
                self.rebuild_edit_overlay();
                self.scene_revision += 1;
                true
            }
            InteractionMode::Object => {
                if let Some(nid) = self.edit_node_id {
                    if let Some(node) = self.find_node_mut(nid) {
                        node.rebuild_evaluated_mesh(false);
                    }
                }
                self.interaction_mode = InteractionMode::Object;
                self.edit_node_id = None;
                self.edit_sel_verts.clear();
                self.edit_sel_edges.clear();
                self.edit_sel_faces.clear();
                self.rebuild_edit_overlay();
                self.scene_revision += 1;
                true
            }
        }
    }

    pub fn set_edit_select_mode(&mut self, mode: EditSelectMode) {
        if self.edit_select_mode != mode {
            self.edit_select_mode = mode;
            self.gizmo.drag = None;
            self.rebuild_edit_overlay();
            self.scene_revision += 1;
        }
    }

    pub fn set_face_orientation(&mut self, enabled: bool) {
        if self.show_face_orientation != enabled {
            self.show_face_orientation = enabled;
            self.rebuild_edit_overlay();
            self.scene_revision += 1;
        }
    }

    pub fn edit_clear_or_select_all(&mut self) -> bool {
        if self.interaction_mode != InteractionMode::Edit {
            return false;
        }
        let Some(nid) = self.edit_node_id else {
            return false;
        };
        let Some(poly) = self.find_node(nid).and_then(|n| n.base_poly.as_ref()) else {
            return false;
        };
        let has_any = match self.edit_select_mode {
            EditSelectMode::Vertex => !self.edit_sel_verts.is_empty(),
            EditSelectMode::Edge => !self.edit_sel_edges.is_empty(),
            EditSelectMode::Face => !self.edit_sel_faces.is_empty(),
        };
        if has_any {
            self.edit_sel_verts.clear();
            self.edit_sel_edges.clear();
            self.edit_sel_faces.clear();
        } else {
            match self.edit_select_mode {
                EditSelectMode::Vertex => {
                    self.edit_sel_verts = (0..poly.vertices.len()).collect();
                }
                EditSelectMode::Edge => {
                    self.edit_sel_edges = poly.unique_edges().into_iter().collect();
                }
                EditSelectMode::Face => {
                    self.edit_sel_faces = (0..poly.faces.len()).collect();
                }
            }
        }
        self.rebuild_edit_overlay();
        self.scene_revision += 1;
        true
    }

    pub fn edit_selection_counts(&self) -> (usize, usize, usize, usize) {
        let Some(nid) = self.edit_node_id else {
            return (0, 0, 0, 0);
        };
        let Some(poly) = self.find_node(nid).and_then(|n| n.base_poly.as_ref()) else {
            return (0, 0, 0, 0);
        };
        let closure = poly.selected_vertex_closure(
            self.edit_select_mode,
            &self.edit_sel_verts,
            &self.edit_sel_edges,
            &self.edit_sel_faces,
        );
        (
            closure.len(),
            self.edit_sel_verts.len(),
            self.edit_sel_edges.len(),
            self.edit_sel_faces.len(),
        )
    }

    pub fn edit_selection_centroid_world(&self) -> Option<Vec3> {
        let nid = self.edit_node_id?;
        let poly = self.find_node(nid)?.base_poly.as_ref()?;
        let closure = poly.selected_vertex_closure(
            self.edit_select_mode,
            &self.edit_sel_verts,
            &self.edit_sel_edges,
            &self.edit_sel_faces,
        );
        if closure.is_empty() {
            return None;
        }
        let mut sum = Vec3::ZERO;
        let mut c = 0usize;
        for vi in closure {
            if let Some(&p) = poly.vertices.get(vi) {
                sum = sum + p;
                c += 1;
            }
        }
        if c == 0 {
            return None;
        }
        Some(self.node_world_matrix(nid).transform_point(sum * (1.0 / c as f32)))
    }

    pub fn edit_pick_at_ndc(&mut self, ndc_x: f32, ndc_y: f32, aspect: f32, additive: bool) -> bool {
        if self.interaction_mode != InteractionMode::Edit {
            return false;
        }
        let Some(nid) = self.edit_node_id else {
            return false;
        };
        let Some(node) = self.find_node(nid) else {
            return false;
        };
        let Some(poly) = node.base_poly.as_ref() else {
            return false;
        };
        let world_m = self.node_world_matrix(nid);
        let Some(inv_m) = world_m.inverse() else {
            return false;
        };

        let ray = self.screen_ray(ndc_x, ndc_y, aspect);
        let local_origin = inv_m.transform_point(ray.origin);
        let local_dir = inv_m.transform_vector(ray.dir);
        if local_dir.length_squared() < 1e-10 {
            return false;
        }
        let local_ray = Ray {
            origin: local_origin,
            dir: local_dir,
        };

        let (mapped_mesh, tri_to_face) = poly.to_mesh_data_mapped(0);
        let mut best_face: Option<(usize, f32)> = None;
        for (ti, tri) in mapped_mesh.indices.chunks_exact(3).enumerate() {
            let v0 = Vec3::from(mapped_mesh.positions[tri[0] as usize]);
            let v1 = Vec3::from(mapped_mesh.positions[tri[1] as usize]);
            let v2 = Vec3::from(mapped_mesh.positions[tri[2] as usize]);
            if let Some(t) = local_ray.intersect_triangle(v0, v1, v2) {
                if best_face.map_or(true, |(_, bt)| t < bt) {
                    best_face = Some((tri_to_face[ti], t));
                }
            }
        }
        let max_surface_t = best_face.map(|(_, t)| t);

        let (eye, _, _) = self.camera.basis();
        let node_center = world_m.transform_point(Vec3::ZERO);
        let cam_dist = (eye - node_center).length().max(0.5);

        match self.edit_select_mode {
            EditSelectMode::Face => {
                if let Some((f_idx, _)) = best_face {
                    if additive {
                        if !self.edit_sel_faces.remove(&f_idx) {
                            self.edit_sel_faces.insert(f_idx);
                        }
                    } else {
                        self.edit_sel_faces.clear();
                        self.edit_sel_faces.insert(f_idx);
                    }
                    self.rebuild_edit_overlay();
                    self.scene_revision += 1;
                    return true;
                }
                if !additive && !self.edit_sel_faces.is_empty() {
                    self.edit_sel_faces.clear();
                    self.rebuild_edit_overlay();
                    self.scene_revision += 1;
                }
                false
            }
            EditSelectMode::Edge => {
                let thresh = cam_dist * 0.055;
                let norm_dir = ray.dir.normalized_or(Vec3::new(0.0, 0.0, -1.0));
                let mut best_edge: Option<((usize, usize), f32)> = None;
                for (a, b) in poly.unique_edges() {
                    let wa = world_m.transform_point(poly.vertices[a]);
                    let wb = world_m.transform_point(poly.vertices[b]);
                    let (dist, t_on_ray) = ray.distance_to_segment(wa, wb);
                    if dist <= thresh {
                        if let Some(st) = max_surface_t {
                            let hit_world = world_m.transform_point(local_origin + local_dir * st);
                            let surf_w_dist = (hit_world - ray.origin).dot(norm_dir);
                            if t_on_ray > surf_w_dist + 0.08 {
                                continue;
                            }
                        }
                        let score = dist + t_on_ray * 0.001;
                        if best_edge.map_or(true, |(_, bs)| score < bs) {
                            best_edge = Some(((a, b), score));
                        }
                    }
                }
                if let Some((edge, _)) = best_edge {
                    if additive {
                        if !self.edit_sel_edges.remove(&edge) {
                            self.edit_sel_edges.insert(edge);
                        }
                    } else {
                        self.edit_sel_edges.clear();
                        self.edit_sel_edges.insert(edge);
                    }
                    self.rebuild_edit_overlay();
                    self.scene_revision += 1;
                    return true;
                }
                if !additive && !self.edit_sel_edges.is_empty() {
                    self.edit_sel_edges.clear();
                    self.rebuild_edit_overlay();
                    self.scene_revision += 1;
                }
                false
            }
            EditSelectMode::Vertex => {
                let thresh = cam_dist * 0.065;
                let norm_dir = ray.dir.normalized_or(Vec3::new(0.0, 0.0, -1.0));
                let mut best_vert: Option<(usize, f32)> = None;
                for (vi, &lp) in poly.vertices.iter().enumerate() {
                    let wp = world_m.transform_point(lp);
                    let to_p = wp - ray.origin;
                    let t_on_ray = to_p.dot(norm_dir).max(0.0);
                    let closest = ray.origin + norm_dir * t_on_ray;
                    let dist = (wp - closest).length();
                    if dist <= thresh {
                        if let Some(st) = max_surface_t {
                            let hit_world = world_m.transform_point(local_origin + local_dir * st);
                            let surf_w_dist = (hit_world - ray.origin).dot(norm_dir);
                            if t_on_ray > surf_w_dist + 0.08 {
                                continue;
                            }
                        }
                        let score = dist + t_on_ray * 0.001;
                        if best_vert.map_or(true, |(_, bs)| score < bs) {
                            best_vert = Some((vi, score));
                        }
                    }
                }
                if let Some((vi, _)) = best_vert {
                    if additive {
                        if !self.edit_sel_verts.remove(&vi) {
                            self.edit_sel_verts.insert(vi);
                        }
                    } else {
                        self.edit_sel_verts.clear();
                        self.edit_sel_verts.insert(vi);
                    }
                    self.rebuild_edit_overlay();
                    self.scene_revision += 1;
                    return true;
                }
                if !additive && !self.edit_sel_verts.is_empty() {
                    self.edit_sel_verts.clear();
                    self.rebuild_edit_overlay();
                    self.scene_revision += 1;
                }
                false
            }
        }
    }

    fn apply_poly_edit<F>(&mut self, op: F) -> bool
    where
        F: FnOnce(
            &mut PolyMesh,
            EditSelectMode,
            &mut BTreeSet<usize>,
            &mut BTreeSet<(usize, usize)>,
            &mut BTreeSet<usize>,
        ) -> bool,
    {
        if self.interaction_mode != InteractionMode::Edit {
            return false;
        }
        let Some(nid) = self.edit_node_id else {
            return false;
        };
        let Some(node) = self.find_node(nid) else {
            return false;
        };
        let (Some(old_poly), Some(old_mesh)) = (node.base_poly.clone(), node.mesh.clone()) else {
            return false;
        };

        let mut new_poly = old_poly.clone();
        let mut next_verts = self.edit_sel_verts.clone();
        let mut next_edges = self.edit_sel_edges.clone();
        let mut next_faces = self.edit_sel_faces.clone();

        if !op(
            &mut new_poly,
            self.edit_select_mode,
            &mut next_verts,
            &mut next_edges,
            &mut next_faces,
        ) {
            return false;
        }

        let new_mesh = new_poly.to_mesh_data(old_mesh.version + 1);
        let cmd = SetPolyMeshCommand {
            node_id: nid,
            old_poly,
            new_poly,
            old_mesh,
            new_mesh,
        };
        self.history
            .execute(Box::new(cmd), &mut self.nodes, &mut self.selected_ids);
        self.edit_sel_verts = next_verts;
        self.edit_sel_edges = next_edges;
        self.edit_sel_faces = next_faces;
        self.sanitize_edit_state_after_history();
        self.rebuild_edit_overlay();
        self.scene_revision += 1;
        true
    }

    pub fn edit_extrude(&mut self, distance: f32) -> bool {
        self.apply_poly_edit(|poly, mode, sv, se, sf| {
            poly.extrude(mode, distance, sv, se, sf).is_ok()
        })
    }

    pub fn edit_inset(&mut self, factor: f32) -> bool {
        self.apply_poly_edit(|poly, _mode, sv, se, sf| {
            poly.inset(factor, sv, se, sf).is_ok()
        })
    }

    pub fn edit_bevel(&mut self, width: f32) -> bool {
        self.apply_poly_edit(|poly, mode, sv, se, sf| {
            poly.bevel(mode, width, sv, se, sf).is_ok()
        })
    }

    pub fn edit_loop_cut(&mut self) -> bool {
        self.apply_poly_edit(|poly, _mode, sv, se, sf| {
            poly.loop_cut(sv, se, sf).is_ok()
        })
    }

    pub fn edit_subdivide(&mut self) -> bool {
        self.apply_poly_edit(|poly, _mode, sv, se, sf| {
            poly.subdivide(sv, se, sf).is_ok()
        })
    }

    pub fn edit_merge_selected(&mut self) -> bool {
        self.apply_poly_edit(|poly, mode, sv, se, sf| {
            poly.merge_selected(mode, sv, se, sf).is_ok()
        })
    }

    pub fn edit_delete_selected(&mut self) -> bool {
        self.apply_poly_edit(|poly, mode, sv, se, sf| {
            poly.delete_selected(mode, sv, se, sf).is_ok()
        })
    }

    pub fn edit_dissolve_edges(&mut self) -> bool {
        self.apply_poly_edit(|poly, _mode, sv, se, sf| {
            poly.dissolve_edges(sv, se, sf).is_ok()
        })
    }

    pub fn edit_recalculate_normals(&mut self) -> bool {
        self.apply_poly_edit(|poly, _mode, _sv, _se, _sf| {
            poly.recalculate_normals();
            true
        })
    }

    pub fn edit_flip_normals(&mut self) -> bool {
        self.apply_poly_edit(|poly, mode, _sv, _se, sf| {
            let target: BTreeSet<usize> = if mode == EditSelectMode::Face && !sf.is_empty() {
                sf.clone()
            } else {
                BTreeSet::new()
            };
            poly.flip_normals(&target);
            true
        })
    }

    pub fn edit_set_smooth_shading(&mut self, smooth: bool) -> bool {
        let Some(nid) = self.active_mesh_node_id() else {
            return false;
        };
        if let Some(node) = self.find_node_mut(nid) {
            node.ensure_base_poly();
        }
        let Some(node) = self.find_node(nid) else {
            return false;
        };
        let (Some(old_poly), Some(old_mesh)) = (node.base_poly.clone(), node.mesh.clone()) else {
            return false;
        };
        if old_poly.smooth_shading == smooth {
            return true;
        }
        let mut new_poly = old_poly.clone();
        new_poly.smooth_shading = smooth;
        let eval = if self.interaction_mode == InteractionMode::Edit || node.modifiers.is_empty() {
            new_poly.clone()
        } else {
            evaluate_modifier_stack(&new_poly, &node.modifiers)
        };
        let new_mesh = eval.to_mesh_data(old_mesh.version + 1);
        let cmd = SetPolyMeshCommand {
            node_id: nid,
            old_poly,
            new_poly,
            old_mesh,
            new_mesh,
        };
        self.history
            .execute(Box::new(cmd), &mut self.nodes, &mut self.selected_ids);
        self.rebuild_edit_overlay();
        self.scene_revision += 1;
        true
    }

    pub fn edit_translate_selected(&mut self, world_delta: Vec3) -> bool {
        if self.interaction_mode != InteractionMode::Edit {
            return false;
        }
        let Some(nid) = self.edit_node_id else {
            return false;
        };
        let world_inv = self
            .node_world_matrix(nid)
            .inverse()
            .unwrap_or(Mat4::IDENTITY);
        let local_delta = world_inv.transform_vector(world_delta);
        if local_delta.length_squared() < 1e-12 {
            return false;
        }
        self.apply_poly_edit(|poly, mode, sv, se, sf| {
            let closure = poly.selected_vertex_closure(mode, sv, se, sf);
            if closure.is_empty() {
                return false;
            }
            for vi in closure {
                if let Some(v) = poly.vertices.get_mut(vi) {
                    *v = *v + local_delta;
                }
            }
            true
        })
    }

    // -------------------------------------------------------------------------
    // M3: Modifier Stack (MD-7)
    // -------------------------------------------------------------------------

    fn commit_modifiers_change(
        &mut self,
        nid: NodeId,
        new_modifiers: Vec<ModifierSpec>,
        baked_poly: Option<PolyMesh>,
    ) -> bool {
        if let Some(node) = self.find_node_mut(nid) {
            node.ensure_base_poly();
        }
        let Some(node) = self.find_node(nid) else {
            return false;
        };
        let (Some(old_base), Some(old_mesh)) = (node.base_poly.clone(), node.mesh.clone()) else {
            return false;
        };
        let old_modifiers = node.modifiers.clone();
        let next_base = baked_poly.as_ref().unwrap_or(&old_base);
        let edit_active =
            self.interaction_mode == InteractionMode::Edit && self.edit_node_id == Some(nid);
        let eval = if edit_active || new_modifiers.is_empty() {
            next_base.clone()
        } else {
            evaluate_modifier_stack(next_base, &new_modifiers)
        };
        let new_mesh = eval.to_mesh_data(old_mesh.version + 1);
        let cmd = SetModifiersCommand {
            node_id: nid,
            old_modifiers,
            new_modifiers,
            old_poly: baked_poly.as_ref().map(|_| old_base),
            new_poly: baked_poly,
            old_mesh,
            new_mesh,
        };
        self.history
            .execute(Box::new(cmd), &mut self.nodes, &mut self.selected_ids);
        self.sanitize_edit_state_after_history();
        self.rebuild_edit_overlay();
        self.scene_revision += 1;
        true
    }

    pub fn add_modifier(&mut self, spec: ModifierSpec) -> bool {
        let Some(nid) = self.active_mesh_node_id() else {
            return false;
        };
        let Some(mut mods) = self.find_node(nid).map(|n| n.modifiers.clone()) else {
            return false;
        };
        mods.push(spec);
        self.commit_modifiers_change(nid, mods, None)
    }

    pub fn remove_modifier(&mut self, index: usize) -> bool {
        let Some(nid) = self.active_mesh_node_id() else {
            return false;
        };
        let Some(mut mods) = self.find_node(nid).map(|n| n.modifiers.clone()) else {
            return false;
        };
        if index >= mods.len() {
            return false;
        }
        mods.remove(index);
        self.commit_modifiers_change(nid, mods, None)
    }

    pub fn move_modifier(&mut self, index: usize, delta: i32) -> bool {
        let Some(nid) = self.active_mesh_node_id() else {
            return false;
        };
        let Some(mut mods) = self.find_node(nid).map(|n| n.modifiers.clone()) else {
            return false;
        };
        if index >= mods.len() {
            return false;
        }
        let target = index as isize + delta as isize;
        if target < 0 || (target as usize) >= mods.len() || target as usize == index {
            return false;
        }
        mods.swap(index, target as usize);
        self.commit_modifiers_change(nid, mods, None)
    }

    pub fn update_modifier(&mut self, index: usize, spec: ModifierSpec) -> bool {
        let Some(nid) = self.active_mesh_node_id() else {
            return false;
        };
        let Some(mut mods) = self.find_node(nid).map(|n| n.modifiers.clone()) else {
            return false;
        };
        if index >= mods.len() {
            return false;
        }
        if mods[index] == spec {
            return true;
        }
        mods[index] = spec;
        self.commit_modifiers_change(nid, mods, None)
    }

    pub fn apply_modifier_stack(&mut self) -> bool {
        let Some(nid) = self.active_mesh_node_id() else {
            return false;
        };
        if let Some(node) = self.find_node_mut(nid) {
            node.ensure_base_poly();
        }
        let Some(node) = self.find_node(nid) else {
            return false;
        };
        if node.modifiers.is_empty() {
            return false;
        }
        let Some(base) = node.base_poly.as_ref() else {
            return false;
        };
        let baked = evaluate_modifier_stack(base, &node.modifiers);
        self.commit_modifiers_change(nid, Vec::new(), Some(baked))
    }

    pub fn rebuild_edit_overlay(&mut self) {
        self.edit_overlay_lines.clear();
        self.edit_overlay_tris.clear();

        if self.show_face_orientation {
            for node in &self.nodes {
                if !node.visible || node.kind != NodeKind::Mesh {
                    continue;
                }
                let Some(mesh) = &node.mesh else {
                    continue;
                };
                let wm = self.node_world_matrix(node.id);
                for tri in mesh.indices.chunks_exact(3) {
                    let v0 = wm.transform_point(Vec3::from(mesh.positions[tri[0] as usize]));
                    let v1 = wm.transform_point(Vec3::from(mesh.positions[tri[1] as usize]));
                    let v2 = wm.transform_point(Vec3::from(mesh.positions[tri[2] as usize]));
                    let fn_w = (v1 - v0).cross(v2 - v0).normalized_or(Vec3::UNIT_Y);
                    let f0 = v0 + fn_w * 0.0010;
                    let f1 = v1 + fn_w * 0.0010;
                    let f2 = v2 + fn_w * 0.0010;
                    self.edit_overlay_tris.push(EditOverlayTri {
                        v0: [f0.x, f0.y, f0.z],
                        v1: [f1.x, f1.y, f1.z],
                        v2: [f2.x, f2.y, f2.z],
                        color: [0.16, 0.50, 0.96, 0.44],
                    });
                    // Back-face reversed winding with slight inward nudge so flipped faces paint red
                    let b0 = v0 - fn_w * 0.0010;
                    let b1 = v2 - fn_w * 0.0010;
                    let b2 = v1 - fn_w * 0.0010;
                    self.edit_overlay_tris.push(EditOverlayTri {
                        v0: [b0.x, b0.y, b0.z],
                        v1: [b1.x, b1.y, b1.z],
                        v2: [b2.x, b2.y, b2.z],
                        color: [0.92, 0.22, 0.22, 0.58],
                    });
                }
            }
        }

        if self.interaction_mode != InteractionMode::Edit {
            return;
        }
        let Some(nid) = self.edit_node_id else {
            return;
        };
        let Some(poly) = self.find_node(nid).and_then(|n| n.base_poly.clone()) else {
            return;
        };
        let wm = self.node_world_matrix(nid);
        let (eye, cam_right, cam_up) = self.camera.basis();

        // Highlight selected faces as translucent orange quads/triangles
        let (mapped_mesh, tri_to_face) = poly.to_mesh_data_mapped(0);
        if self.edit_select_mode == EditSelectMode::Face && !self.edit_sel_faces.is_empty() {
            for (ti, tri) in mapped_mesh.indices.chunks_exact(3).enumerate() {
                let f_idx = tri_to_face[ti];
                if !self.edit_sel_faces.contains(&f_idx) {
                    continue;
                }
                let v0 = wm.transform_point(Vec3::from(mapped_mesh.positions[tri[0] as usize]));
                let v1 = wm.transform_point(Vec3::from(mapped_mesh.positions[tri[1] as usize]));
                let v2 = wm.transform_point(Vec3::from(mapped_mesh.positions[tri[2] as usize]));
                let fn_w = (v1 - v0).cross(v2 - v0).normalized_or(Vec3::UNIT_Y);
                let p0 = v0 + fn_w * 0.0016;
                let p1 = v1 + fn_w * 0.0016;
                let p2 = v2 + fn_w * 0.0016;
                self.edit_overlay_tris.push(EditOverlayTri {
                    v0: [p0.x, p0.y, p0.z],
                    v1: [p1.x, p1.y, p1.z],
                    v2: [p2.x, p2.y, p2.z],
                    color: [1.0, 0.54, 0.12, 0.36],
                });
            }
        }

        let mut face_selected_edges: HashSet<(usize, usize)> = HashSet::new();
        if self.edit_select_mode == EditSelectMode::Face {
            for &fi in &self.edit_sel_faces {
                if let Some(f) = poly.faces.get(fi) {
                    let n = f.len();
                    for i in 0..n {
                        let a = f[i];
                        let b = f[(i + 1) % n];
                        face_selected_edges.insert((a.min(b), a.max(b)));
                    }
                }
            }
        }

        // Accumulate smooth vertex normals in local space so wireframe edges lift cleanly off all faces
        let mut center_local = Vec3::ZERO;
        if !poly.vertices.is_empty() {
            for &p in &poly.vertices {
                center_local = center_local + p;
            }
            center_local = center_local * (1.0 / poly.vertices.len() as f32);
        }

        let mut vert_normals_local = vec![Vec3::ZERO; poly.vertices.len()];
        for face in &poly.faces {
            if face.len() < 3 {
                continue;
            }
            let mut fn_acc = Vec3::ZERO;
            let p0 = poly.vertices[face[0]];
            for i in 1..(face.len() - 1) {
                let p1 = poly.vertices[face[i]];
                let p2 = poly.vertices[face[i + 1]];
                fn_acc = fn_acc + (p1 - p0).cross(p2 - p0);
            }
            let fn_dir = fn_acc.normalized_or(Vec3::ZERO);
            for &vi in face {
                if vi < vert_normals_local.len() {
                    vert_normals_local[vi] = vert_normals_local[vi] + fn_dir;
                }
            }
        }

        let world_verts: Vec<Vec3> = poly
            .vertices
            .iter()
            .enumerate()
            .map(|(vi, &v)| {
                let fallback = (v - center_local).normalized_or(Vec3::UNIT_Y);
                let n_local = vert_normals_local[vi].normalized_or(fallback);
                let wp0 = wm.transform_point(v);
                let wp1 = wm.transform_point(v + n_local);
                let n_world = (wp1 - wp0).normalized_or(Vec3::UNIT_Y);
                let dist = (eye - wp0).length();
                let bias = (dist * 0.0018).clamp(0.0050, 0.022);
                wp0 + n_world * bias
            })
            .collect();

        for (a, b) in poly.unique_edges() {
            let is_sel = match self.edit_select_mode {
                EditSelectMode::Edge => self.edit_sel_edges.contains(&(a, b)),
                EditSelectMode::Vertex => {
                    self.edit_sel_verts.contains(&a) && self.edit_sel_verts.contains(&b)
                }
                EditSelectMode::Face => face_selected_edges.contains(&(a, b)),
            };
            let color = if is_sel {
                [1.0, 0.60, 0.12, 1.0]
            } else {
                [0.14, 0.18, 0.25, 1.0]
            };
            let pa = world_verts[a];
            let pb = world_verts[b];
            self.edit_overlay_lines.push(EditOverlayLine {
                a: [pa.x, pa.y, pa.z],
                b: [pb.x, pb.y, pb.z],
                color,
            });
        }

        // Draw vertex handles in Vertex select mode, and highlighted verts in all Edit modes
        for (vi, &wp) in world_verts.iter().enumerate() {
            let is_sel = match self.edit_select_mode {
                EditSelectMode::Vertex => self.edit_sel_verts.contains(&vi),
                EditSelectMode::Edge => self.edit_sel_edges.iter().any(|&(a, b)| a == vi || b == vi),
                EditSelectMode::Face => false,
            };
            if self.edit_select_mode != EditSelectMode::Vertex && !is_sel {
                continue;
            }
            let r = ((eye - wp).length() * 0.009).clamp(0.012, 0.075)
                * if is_sel { 1.45 } else { 0.95 };
            let color = if is_sel {
                [1.0, 0.72, 0.15, 1.0]
            } else {
                [0.14, 0.18, 0.25, 0.95]
            };
            let top = wp + cam_up * r;
            let bot = wp - cam_up * r;
            let right = wp + cam_right * r;
            let left = wp - cam_right * r;
            let pts = [(top, right), (right, bot), (bot, left), (left, top)];
            for (p0, p1) in pts {
                self.edit_overlay_lines.push(EditOverlayLine {
                    a: [p0.x, p0.y, p0.z],
                    b: [p1.x, p1.y, p1.z],
                    color,
                });
            }
        }
    }

    // -------------------------------------------------------------------------
    // M4 Look Development: Material Library, Textures, Environment & Light Rigs
    // -------------------------------------------------------------------------

    pub fn find_material(&self, mat_id: MaterialId) -> Option<&MaterialSpec> {
        self.materials.iter().find(|m| m.id == mat_id)
    }

    pub fn resolved_node_material(&self, node: &SceneNode) -> MaterialSpec {
        if let Some(mid) = node.material_id {
            if let Some(mat) = self.find_material(mid) {
                return mat.clone();
            }
        }
        MaterialSpec {
            id: 0,
            name: format!("{} Inline", node.name),
            base_color: node.material.base_color,
            metallic: node.material.metallic,
            roughness: node.material.roughness,
            ..MaterialSpec::studio_default(0, "Inline")
        }
    }

    pub(crate) fn unique_material_name(&self, base: &str) -> String {
        if !self.materials.iter().any(|m| m.name == base) {
            return base.to_string();
        }
        for i in 2..1000 {
            let cand = format!("{base} {i}");
            if !self.materials.iter().any(|m| m.name == cand) {
                return cand;
            }
        }
        format!("{base}_mat")
    }

    fn node_links_for_materials(&self, mats: &[MaterialSpec]) -> Vec<(NodeId, Option<MaterialId>, MaterialParams, Option<MaterialId>, MaterialParams)> {
        let mut changes = Vec::new();
        for node in &self.nodes {
            if node.kind != NodeKind::Mesh {
                continue;
            }
            if let Some(mid) = node.material_id {
                if let Some(m) = mats.iter().find(|mm| mm.id == mid) {
                    let synced = MaterialParams {
                        base_color: m.base_color,
                        metallic: m.metallic,
                        roughness: m.roughness,
                    };
                    changes.push((
                        node.id,
                        node.material_id,
                        node.material.clone(),
                        Some(mid),
                        synced,
                    ));
                }
            }
        }
        changes
    }

    pub fn select_material(&mut self, mat_id: MaterialId) -> bool {
        if self.find_material(mat_id).is_some() {
            self.selected_material_id = Some(mat_id);
            self.scene_revision += 1;
            true
        } else {
            false
        }
    }

    pub fn create_material_from_preset(&mut self, preset_idx: u32, assign_selected: bool) -> MaterialId {
        let id = self.next_material_id;
        self.next_material_id += 1;
        let mut mat = MaterialSpec::from_preset(id, preset_idx);
        mat.name = self.unique_material_name(&mat.name);

        let old_materials = self.materials.clone();
        let old_selected_material = self.selected_material_id;
        let mut new_materials = old_materials.clone();
        new_materials.push(mat.clone());

        let mut node_changes = Vec::new();
        if assign_selected {
            let synced = MaterialParams {
                base_color: mat.base_color,
                metallic: mat.metallic,
                roughness: mat.roughness,
            };
            for &nid in &self.selected_ids {
                if let Some(n) = self.find_node(nid) {
                    if n.kind == NodeKind::Mesh {
                        node_changes.push((nid, n.material_id, n.material.clone(), Some(id), synced.clone()));
                    }
                }
            }
        }

        let cmd = SetMaterialLibraryCommand {
            old_materials,
            new_materials,
            old_selected_material,
            new_selected_material: Some(id),
            node_changes,
        };
        self.history.execute_lookdev(
            Box::new(cmd),
            &mut self.nodes,
            &mut self.selected_ids,
            &mut self.materials,
            &mut self.selected_material_id,
            &mut self.environment,
        );
        self.scene_revision += 1;
        id
    }

    pub fn duplicate_material(&mut self, mat_id: MaterialId) -> Option<MaterialId> {
        let src = self.find_material(mat_id)?.clone();
        let new_id = self.next_material_id;
        self.next_material_id += 1;
        let mut copy = src;
        copy.id = new_id;
        copy.name = self.unique_material_name(&format!("{} Copy", copy.name));

        let old_materials = self.materials.clone();
        let mut new_materials = old_materials.clone();
        new_materials.push(copy);

        let cmd = SetMaterialLibraryCommand {
            old_materials,
            new_materials,
            old_selected_material: self.selected_material_id,
            new_selected_material: Some(new_id),
            node_changes: Vec::new(),
        };
        self.history.execute_lookdev(
            Box::new(cmd),
            &mut self.nodes,
            &mut self.selected_ids,
            &mut self.materials,
            &mut self.selected_material_id,
            &mut self.environment,
        );
        self.scene_revision += 1;
        Some(new_id)
    }

    pub fn rename_material(&mut self, mat_id: MaterialId, new_name: &str) -> bool {
        let trimmed = new_name.trim();
        if trimmed.is_empty() {
            return false;
        }
        let Some(idx) = self.materials.iter().position(|m| m.id == mat_id) else {
            return false;
        };
        if self.materials[idx].name == trimmed {
            return true;
        }
        let old_materials = self.materials.clone();
        let mut new_materials = old_materials.clone();
        new_materials[idx].name = trimmed.to_string();

        let cmd = SetMaterialLibraryCommand {
            old_materials,
            new_materials,
            old_selected_material: self.selected_material_id,
            new_selected_material: self.selected_material_id,
            node_changes: Vec::new(),
        };
        self.history.execute_lookdev(
            Box::new(cmd),
            &mut self.nodes,
            &mut self.selected_ids,
            &mut self.materials,
            &mut self.selected_material_id,
            &mut self.environment,
        );
        self.scene_revision += 1;
        true
    }

    pub fn delete_material(&mut self, mat_id: MaterialId) -> bool {
        if self.materials.len() <= 1 || self.find_material(mat_id).is_none() {
            return false;
        }
        let old_materials = self.materials.clone();
        let new_materials: Vec<MaterialSpec> = old_materials
            .iter()
            .filter(|m| m.id != mat_id)
            .cloned()
            .collect();
        let fallback = new_materials[0].clone();
        let fallback_params = MaterialParams {
            base_color: fallback.base_color,
            metallic: fallback.metallic,
            roughness: fallback.roughness,
        };
        let mut node_changes = Vec::new();
        for n in &self.nodes {
            if n.kind == NodeKind::Mesh && n.material_id == Some(mat_id) {
                node_changes.push((
                    n.id,
                    n.material_id,
                    n.material.clone(),
                    Some(fallback.id),
                    fallback_params.clone(),
                ));
            }
        }
        let new_sel = if self.selected_material_id == Some(mat_id) {
            Some(fallback.id)
        } else {
            self.selected_material_id
        };
        let cmd = SetMaterialLibraryCommand {
            old_materials,
            new_materials,
            old_selected_material: self.selected_material_id,
            new_selected_material: new_sel,
            node_changes,
        };
        self.history.execute_lookdev(
            Box::new(cmd),
            &mut self.nodes,
            &mut self.selected_ids,
            &mut self.materials,
            &mut self.selected_material_id,
            &mut self.environment,
        );
        self.scene_revision += 1;
        true
    }

    pub fn update_material(&mut self, updated: MaterialSpec) -> bool {
        let Some(idx) = self.materials.iter().position(|m| m.id == updated.id) else {
            return false;
        };
        let clean = updated.sanitized();
        if self.materials[idx] == clean {
            return true;
        }
        let old_materials = self.materials.clone();
        let mut new_materials = old_materials.clone();
        new_materials[idx] = clean;
        let node_changes = self.node_links_for_materials(&new_materials);
        let cmd = SetMaterialLibraryCommand {
            old_materials,
            new_materials,
            old_selected_material: self.selected_material_id,
            new_selected_material: self.selected_material_id,
            node_changes,
        };
        self.history.execute_lookdev(
            Box::new(cmd),
            &mut self.nodes,
            &mut self.selected_ids,
            &mut self.materials,
            &mut self.selected_material_id,
            &mut self.environment,
        );
        self.scene_revision += 1;
        true
    }

    pub fn assign_material_to_selected(&mut self, mat_id: MaterialId) -> bool {
        let Some(mat) = self.find_material(mat_id).cloned() else {
            return false;
        };
        let synced = MaterialParams {
            base_color: mat.base_color,
            metallic: mat.metallic,
            roughness: mat.roughness,
        };
        let mut node_changes = Vec::new();
        for &nid in &self.selected_ids {
            if let Some(n) = self.find_node(nid) {
                if n.kind == NodeKind::Mesh {
                    node_changes.push((nid, n.material_id, n.material.clone(), Some(mat_id), synced.clone()));
                }
            }
        }
        if node_changes.is_empty() {
            self.selected_material_id = Some(mat_id);
            self.scene_revision += 1;
            return true;
        }
        let cmd = SetMaterialLibraryCommand {
            old_materials: self.materials.clone(),
            new_materials: self.materials.clone(),
            old_selected_material: self.selected_material_id,
            new_selected_material: Some(mat_id),
            node_changes,
        };
        self.history.execute_lookdev(
            Box::new(cmd),
            &mut self.nodes,
            &mut self.selected_ids,
            &mut self.materials,
            &mut self.selected_material_id,
            &mut self.environment,
        );
        self.scene_revision += 1;
        true
    }

    pub fn find_texture(&self, tex_id: TextureId) -> Option<&TextureAsset> {
        self.textures.iter().find(|t| t.id == tex_id)
    }

    pub fn register_rgba_texture(
        &mut self,
        name: &str,
        source_uri: &str,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    ) -> Result<TextureId, String> {
        let expected = (width as usize)
            .checked_mul(height as usize)
            .and_then(|px| px.checked_mul(4))
            .ok_or_else(|| "Texture dimensions overflow".to_string())?;
        if width == 0 || height == 0 || rgba.len() != expected {
            return Err("Invalid RGBA texture buffer dimensions".to_string());
        }
        let id = self.next_texture_id;
        self.next_texture_id += 1;
        let clean_name = if name.trim().is_empty() {
            format!("Texture {id}")
        } else {
            name.trim().to_string()
        };
        self.textures.push(TextureAsset {
            id,
            name: clean_name,
            source_uri: source_uri.to_string(),
            width,
            height,
            rgba,
            status: TextureStatus::Loaded,
            version: 1,
        });
        self.scene_revision += 1;
        Ok(id)
    }

    pub fn register_missing_texture(&mut self, name: &str, source_uri: &str) -> TextureId {
        let id = self.next_texture_id;
        self.next_texture_id += 1;
        self.textures.push(TextureAsset {
            id,
            name: if name.trim().is_empty() {
                format!("Missing Texture {id}")
            } else {
                name.trim().to_string()
            },
            source_uri: source_uri.to_string(),
            width: 64,
            height: 64,
            rgba: Vec::new(),
            status: TextureStatus::Missing,
            version: 1,
        });
        self.scene_revision += 1;
        id
    }

    pub fn mark_texture_missing(&mut self, tex_id: TextureId, missing: bool) -> bool {
        let Some(t) = self.textures.iter_mut().find(|t| t.id == tex_id) else {
            return false;
        };
        if missing {
            t.status = TextureStatus::Missing;
        } else if !t.rgba.is_empty() {
            t.status = if t.source_uri.starts_with("nova://builtin/") {
                TextureStatus::Builtin
            } else {
                TextureStatus::Loaded
            };
        } else {
            return false;
        }
        t.version += 1;
        self.scene_revision += 1;
        true
    }

    pub fn missing_texture_warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        for tex in &self.textures {
            if tex.status == TextureStatus::Missing {
                let used_by: Vec<&str> = self
                    .materials
                    .iter()
                    .filter(|m| m.referenced_textures().contains(&Some(tex.id)))
                    .map(|m| m.name.as_str())
                    .collect();
                if used_by.is_empty() {
                    warnings.push(format!("Missing texture '{}': {}", tex.name, tex.source_uri));
                } else {
                    warnings.push(format!(
                        "Missing texture '{}' ({}) used by {}",
                        tex.name,
                        tex.source_uri,
                        used_by.join(", ")
                    ));
                }
            }
        }
        warnings
    }

    pub fn set_environment(&mut self, env: EnvironmentSpec) -> bool {
        let clean = env.sanitized();
        if self.environment == clean {
            return true;
        }
        let cmd = SetEnvironmentCommand {
            before: self.environment.clone(),
            after: clean,
        };
        self.history.execute_lookdev(
            Box::new(cmd),
            &mut self.nodes,
            &mut self.selected_ids,
            &mut self.materials,
            &mut self.selected_material_id,
            &mut self.environment,
        );
        self.environment_version += 1;
        self.scene_revision += 1;
        true
    }

    pub fn apply_lighting_rig(&mut self, rig_idx: u32) -> bool {
        fn euler_aim_at(pos: Vec3, target: Vec3) -> Vec3 {
            let dir = (target - pos).normalized_or(Vec3::new(0.0, -1.0, 0.0));
            let yaw = (-dir.x).atan2(-dir.z);
            let pitch = dir.y.clamp(-0.999, 0.999).asin();
            Vec3::new(pitch, yaw, 0.0)
        }
        let target = Vec3::new(0.0, 0.75, 0.0);
        let removed_lights: Vec<(usize, SceneNode)> = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.kind == NodeKind::Light)
            .map(|(i, n)| (i, n.clone()))
            .collect();

        let mut make_light = |name: &str, pos: Vec3, light: LightParams| -> SceneNode {
            let id = self.allocate_id();
            SceneNode {
                id,
                parent_id: None,
                name: name.to_string(),
                kind: NodeKind::Light,
                transform: Transform {
                    position: pos,
                    rotation_euler: euler_aim_at(pos, target),
                    scale: Vec3::ONE,
                },
                mesh: None,
                base_poly: None,
                modifiers: Vec::new(),
                material_id: None,
                material: MaterialParams::default(),
                light,
                camera_node: CameraNodeParams::default(),
                visible: true,
            }
        };

        let (added_lights, new_env) = match rig_idx {
            // 0: 3-Point Studio (Key Directional + Warm Fill Point + Cool Rim Spot)
            0 => (
                vec![
                    make_light(
                        "Key Sun",
                        Vec3::new(3.2, 4.4, 3.0),
                        LightParams {
                            kind: LightKind::Directional,
                            color: [1.0, 0.97, 0.92],
                            intensity: 95000.0,
                            cast_shadows: true,
                            spot_inner_deg: 20.0,
                            spot_outer_deg: 40.0,
                            range: 20.0,
                        },
                    ),
                    make_light(
                        "Warm Fill Light",
                        Vec3::new(-3.0, 2.2, 2.6),
                        LightParams {
                            kind: LightKind::Point,
                            color: [1.0, 0.80, 0.60],
                            intensity: 38000.0,
                            cast_shadows: false,
                            spot_inner_deg: 20.0,
                            spot_outer_deg: 40.0,
                            range: 14.0,
                        },
                    ),
                    make_light(
                        "Cool Rim Spot",
                        Vec3::new(-1.8, 3.4, -3.4),
                        LightParams {
                            kind: LightKind::Spot,
                            color: [0.56, 0.78, 1.0],
                            intensity: 85000.0,
                            cast_shadows: true,
                            spot_inner_deg: 16.0,
                            spot_outer_deg: 36.0,
                            range: 18.0,
                        },
                    ),
                ],
                EnvironmentSpec {
                    preset: EnvironmentPreset::StudioSoft,
                    rotation_deg: 25.0,
                    intensity: 1.0,
                    show_skybox: self.environment.show_skybox,
                    bloom_enabled: self.environment.bloom_enabled,
                    bloom_strength: self.environment.bloom_strength,
                    exposure: 1.0,
                },
            ),
            // 1: Product Turntable
            1 => (
                vec![
                    make_light(
                        "Overhead Soft Key",
                        Vec3::new(0.8, 5.0, 1.8),
                        LightParams {
                            kind: LightKind::Spot,
                            color: [1.0, 0.99, 0.98],
                            intensity: 110000.0,
                            cast_shadows: true,
                            spot_inner_deg: 28.0,
                            spot_outer_deg: 55.0,
                            range: 20.0,
                        },
                    ),
                    make_light(
                        "Side Accent Fill",
                        Vec3::new(-3.4, 1.8, -1.6),
                        LightParams {
                            kind: LightKind::Point,
                            color: [0.85, 0.92, 1.0],
                            intensity: 42000.0,
                            cast_shadows: false,
                            spot_inner_deg: 20.0,
                            spot_outer_deg: 40.0,
                            range: 14.0,
                        },
                    ),
                ],
                EnvironmentSpec {
                    preset: EnvironmentPreset::StudioContrast,
                    rotation_deg: 65.0,
                    intensity: 1.15,
                    show_skybox: self.environment.show_skybox,
                    bloom_enabled: self.environment.bloom_enabled,
                    bloom_strength: self.environment.bloom_strength,
                    exposure: 1.0,
                },
            ),
            // 2: Dramatic Rim
            2 => (
                vec![
                    make_light(
                        "Hero Spot",
                        Vec3::new(2.8, 3.8, -2.8),
                        LightParams {
                            kind: LightKind::Spot,
                            color: [1.0, 0.72, 0.42],
                            intensity: 135000.0,
                            cast_shadows: true,
                            spot_inner_deg: 14.0,
                            spot_outer_deg: 30.0,
                            range: 18.0,
                        },
                    ),
                    make_light(
                        "Cyan Edge Rim",
                        Vec3::new(-3.2, 2.2, 2.5),
                        LightParams {
                            kind: LightKind::Spot,
                            color: [0.35, 0.85, 1.0],
                            intensity: 95000.0,
                            cast_shadows: false,
                            spot_inner_deg: 18.0,
                            spot_outer_deg: 36.0,
                            range: 16.0,
                        },
                    ),
                ],
                EnvironmentSpec {
                    preset: EnvironmentPreset::NeonNight,
                    rotation_deg: 140.0,
                    intensity: 0.55,
                    show_skybox: self.environment.show_skybox,
                    bloom_enabled: true,
                    bloom_strength: 0.22,
                    exposure: 1.1,
                },
            ),
            // 3: Outdoor Sun & Sky
            _ => (
                vec![make_light(
                    "Golden Sun",
                    Vec3::new(4.5, 5.5, 2.2),
                    LightParams {
                        kind: LightKind::Directional,
                        color: [1.0, 0.90, 0.76],
                        intensity: 115000.0,
                        cast_shadows: true,
                        spot_inner_deg: 20.0,
                        spot_outer_deg: 40.0,
                        range: 30.0,
                    },
                )],
                EnvironmentSpec {
                    preset: EnvironmentPreset::OutdoorDaylight,
                    rotation_deg: 45.0,
                    intensity: 1.35,
                    show_skybox: true,
                    bloom_enabled: true,
                    bloom_strength: 0.14,
                    exposure: 1.0,
                },
            ),
        };

        let first_light_id = added_lights.first().map(|n| n.id);
        let new_selected = first_light_id.map_or_else(|| self.selected_ids.clone(), |id| vec![id]);
        let cmd = ApplyLightingRigCommand {
            removed_lights,
            added_lights,
            old_selected: self.selected_ids.clone(),
            new_selected,
            old_env: self.environment.clone(),
            new_env,
        };
        self.history.execute_lookdev(
            Box::new(cmd),
            &mut self.nodes,
            &mut self.selected_ids,
            &mut self.materials,
            &mut self.selected_material_id,
            &mut self.environment,
        );
        self.environment_version += 1;
        self.scene_revision += 1;
        true
    }

    // -------------------------------------------------------------------------
    // M5: Keyframe Animation Engine, Curves & Active Camera Evaluation (AN-1..AN-7)
    // -------------------------------------------------------------------------

    pub fn eval_animation_at_current_frame(&mut self) -> bool {
        if self.animation.tracks.is_empty() {
            return false;
        }
        let frame = self.animation.timeline.current_frame;
        let mut changed = false;

        for track in &self.animation.tracks {
            let Some(val) = track.evaluate_at(frame) else {
                continue;
            };
            match track.target_kind {
                AnimTargetKind::Node => {
                    if let Some(node) = self.nodes.iter_mut().find(|n| n.id == track.target_id) {
                        match track.channel {
                            AnimChannelKind::LocX => {
                                if (node.transform.position.x - val).abs() > 1e-6 {
                                    node.transform.position.x = val;
                                    changed = true;
                                }
                            }
                            AnimChannelKind::LocY => {
                                if (node.transform.position.y - val).abs() > 1e-6 {
                                    node.transform.position.y = val;
                                    changed = true;
                                }
                            }
                            AnimChannelKind::LocZ => {
                                if (node.transform.position.z - val).abs() > 1e-6 {
                                    node.transform.position.z = val;
                                    changed = true;
                                }
                            }
                            AnimChannelKind::RotX => {
                                if (node.transform.rotation_euler.x - val).abs() > 1e-6 {
                                    node.transform.rotation_euler.x = val;
                                    changed = true;
                                }
                            }
                            AnimChannelKind::RotY => {
                                if (node.transform.rotation_euler.y - val).abs() > 1e-6 {
                                    node.transform.rotation_euler.y = val;
                                    changed = true;
                                }
                            }
                            AnimChannelKind::RotZ => {
                                if (node.transform.rotation_euler.z - val).abs() > 1e-6 {
                                    node.transform.rotation_euler.z = val;
                                    changed = true;
                                }
                            }
                            AnimChannelKind::ScaleX => {
                                if (node.transform.scale.x - val).abs() > 1e-6 {
                                    node.transform.scale.x = val;
                                    changed = true;
                                }
                            }
                            AnimChannelKind::ScaleY => {
                                if (node.transform.scale.y - val).abs() > 1e-6 {
                                    node.transform.scale.y = val;
                                    changed = true;
                                }
                            }
                            AnimChannelKind::ScaleZ => {
                                if (node.transform.scale.z - val).abs() > 1e-6 {
                                    node.transform.scale.z = val;
                                    changed = true;
                                }
                            }
                            AnimChannelKind::LightIntensity => {
                                if (node.light.intensity - val).abs() > 1e-4 {
                                    node.light.intensity = val;
                                    changed = true;
                                }
                            }
                            AnimChannelKind::LightColorR => {
                                if (node.light.color[0] - val).abs() > 1e-5 {
                                    node.light.color[0] = val;
                                    changed = true;
                                }
                            }
                            AnimChannelKind::LightColorG => {
                                if (node.light.color[1] - val).abs() > 1e-5 {
                                    node.light.color[1] = val;
                                    changed = true;
                                }
                            }
                            AnimChannelKind::LightColorB => {
                                if (node.light.color[2] - val).abs() > 1e-5 {
                                    node.light.color[2] = val;
                                    changed = true;
                                }
                            }
                            AnimChannelKind::LightSpotOuterDeg => {
                                if (node.light.spot_outer_deg - val).abs() > 1e-4 {
                                    node.light.spot_outer_deg = val;
                                    node.light.spot_inner_deg =
                                        node.light.spot_inner_deg.min(val - 1.0).max(2.0);
                                    changed = true;
                                }
                            }
                            AnimChannelKind::LightRange => {
                                if (node.light.range - val).abs() > 1e-4 {
                                    node.light.range = val;
                                    changed = true;
                                }
                            }
                            AnimChannelKind::CameraFovDeg => {
                                if (node.camera_node.fov_y_degrees - val).abs() > 1e-4 {
                                    node.camera_node.fov_y_degrees = val;
                                    changed = true;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                AnimTargetKind::Material => {
                    if let Some(mat) = self.materials.iter_mut().find(|m| m.id == track.target_id) {
                        let mat_id = mat.id;
                        let mut mat_changed = false;
                        match track.channel {
                            AnimChannelKind::MatColorR => {
                                if (mat.base_color[0] - val).abs() > 1e-5 {
                                    mat.base_color[0] = val;
                                    mat_changed = true;
                                }
                            }
                            AnimChannelKind::MatColorG => {
                                if (mat.base_color[1] - val).abs() > 1e-5 {
                                    mat.base_color[1] = val;
                                    mat_changed = true;
                                }
                            }
                            AnimChannelKind::MatColorB => {
                                if (mat.base_color[2] - val).abs() > 1e-5 {
                                    mat.base_color[2] = val;
                                    mat_changed = true;
                                }
                            }
                            AnimChannelKind::MatOpacity => {
                                if (mat.base_color[3] - val).abs() > 1e-5 {
                                    mat.base_color[3] = val;
                                    mat_changed = true;
                                }
                            }
                            AnimChannelKind::MatMetallic => {
                                if (mat.metallic - val).abs() > 1e-5 {
                                    mat.metallic = val;
                                    mat_changed = true;
                                }
                            }
                            AnimChannelKind::MatRoughness => {
                                if (mat.roughness - val).abs() > 1e-5 {
                                    mat.roughness = val;
                                    mat_changed = true;
                                }
                            }
                            AnimChannelKind::MatEmissiveIntensity => {
                                if (mat.emissive_intensity - val).abs() > 1e-5 {
                                    mat.emissive_intensity = val;
                                    mat_changed = true;
                                }
                            }
                            _ => {}
                        }
                        if mat_changed {
                            let synced = MaterialParams {
                                base_color: mat.base_color,
                                metallic: mat.metallic,
                                roughness: mat.roughness,
                            };
                            for n in &mut self.nodes {
                                if n.material_id == Some(mat_id) {
                                    n.material = synced.clone();
                                }
                            }
                            changed = true;
                        }
                    }
                }
            }
        }

        if changed {
            self.scene_revision += 1;
        }
        changed
    }

    pub fn anim_tick(&mut self, frame_time_nanos: u64) -> bool {
        let moved = self.animation.tick_playback(frame_time_nanos);
        if moved {
            self.eval_animation_at_current_frame();
        }
        moved
    }

    pub fn anim_set_playing(&mut self, playing: bool) {
        if playing && self.animation.timeline.current_frame >= self.animation.timeline.end_frame as f32 {
            self.animation.timeline.current_frame = self.animation.timeline.start_frame as f32;
            self.eval_animation_at_current_frame();
        }
        self.animation.timeline.playing = playing;
        self.animation.timeline.last_tick_nanos = None;
    }

    pub fn anim_set_frame(&mut self, frame: f32) -> bool {
        let s = self.animation.timeline.start_frame as f32;
        let e = self.animation.timeline.end_frame.max(self.animation.timeline.start_frame + 1) as f32;
        let next = if frame.is_finite() { frame.clamp(s, e) } else { s };
        self.animation.timeline.current_frame = next;
        let eval_changed = self.eval_animation_at_current_frame();
        if !eval_changed {
            self.scene_revision += 1;
        }
        true
    }

    pub fn anim_set_timeline(
        &mut self,
        start: i32,
        end: i32,
        fps: f32,
        looping: bool,
        auto_key: bool,
    ) {
        let old_anim = self.animation.clone();
        let mut new_anim = old_anim.clone();
        new_anim.set_timeline_range(start, end, fps, looping, auto_key);
        let cmd = SetAnimationStateCommand { old_anim, new_anim };
        self.history.execute_all(
            Box::new(cmd),
            &mut self.nodes,
            &mut self.selected_ids,
            &mut self.materials,
            &mut self.selected_material_id,
            &mut self.environment,
            &mut self.animation,
        );
        self.eval_animation_at_current_frame();
        self.scene_revision += 1;
    }

    pub fn anim_insert_keyframe(
        &mut self,
        target_kind: AnimTargetKind,
        target_id: u64,
        channel: AnimChannelKind,
        frame: i32,
        value: f32,
        interpolation: InterpolationMode,
    ) -> (TrackId, KeyframeId) {
        let old_anim = self.animation.clone();
        let mut new_anim = old_anim.clone();
        let ids = new_anim.insert_or_update_keyframe(
            target_kind,
            target_id,
            channel,
            frame,
            value,
            interpolation,
        );
        let cmd = SetAnimationStateCommand { old_anim, new_anim };
        self.history.execute_all(
            Box::new(cmd),
            &mut self.nodes,
            &mut self.selected_ids,
            &mut self.materials,
            &mut self.selected_material_id,
            &mut self.environment,
            &mut self.animation,
        );
        self.eval_animation_at_current_frame();
        self.scene_revision += 1;
        ids
    }

    pub fn anim_insert_node_trs_keys(
        &mut self,
        node_id: NodeId,
        frame: i32,
        mask: u32,
        interpolation: InterpolationMode,
    ) -> usize {
        let Some(node) = self.find_node(node_id).cloned() else {
            return 0;
        };
        let old_anim = self.animation.clone();
        let mut new_anim = old_anim.clone();
        let mut count = 0usize;
        if (mask & 1) != 0 {
            new_anim.insert_or_update_keyframe(AnimTargetKind::Node, node_id, AnimChannelKind::LocX, frame, node.transform.position.x, interpolation);
            new_anim.insert_or_update_keyframe(AnimTargetKind::Node, node_id, AnimChannelKind::LocY, frame, node.transform.position.y, interpolation);
            new_anim.insert_or_update_keyframe(AnimTargetKind::Node, node_id, AnimChannelKind::LocZ, frame, node.transform.position.z, interpolation);
            count += 3;
        }
        if (mask & 2) != 0 {
            new_anim.insert_or_update_keyframe(AnimTargetKind::Node, node_id, AnimChannelKind::RotX, frame, node.transform.rotation_euler.x, interpolation);
            new_anim.insert_or_update_keyframe(AnimTargetKind::Node, node_id, AnimChannelKind::RotY, frame, node.transform.rotation_euler.y, interpolation);
            new_anim.insert_or_update_keyframe(AnimTargetKind::Node, node_id, AnimChannelKind::RotZ, frame, node.transform.rotation_euler.z, interpolation);
            count += 3;
        }
        if (mask & 4) != 0 {
            new_anim.insert_or_update_keyframe(AnimTargetKind::Node, node_id, AnimChannelKind::ScaleX, frame, node.transform.scale.x, interpolation);
            new_anim.insert_or_update_keyframe(AnimTargetKind::Node, node_id, AnimChannelKind::ScaleY, frame, node.transform.scale.y, interpolation);
            new_anim.insert_or_update_keyframe(AnimTargetKind::Node, node_id, AnimChannelKind::ScaleZ, frame, node.transform.scale.z, interpolation);
            count += 3;
        }
        if count == 0 {
            return 0;
        }
        let cmd = SetAnimationStateCommand { old_anim, new_anim };
        self.history.execute_all(
            Box::new(cmd),
            &mut self.nodes,
            &mut self.selected_ids,
            &mut self.materials,
            &mut self.selected_material_id,
            &mut self.environment,
            &mut self.animation,
        );
        self.scene_revision += 1;
        count
    }

    pub fn anim_insert_light_keys(
        &mut self,
        node_id: NodeId,
        frame: i32,
        interpolation: InterpolationMode,
    ) -> usize {
        let Some(node) = self.find_node(node_id).cloned() else {
            return 0;
        };
        if node.kind != NodeKind::Light {
            return 0;
        }
        let old_anim = self.animation.clone();
        let mut new_anim = old_anim.clone();
        new_anim.insert_or_update_keyframe(AnimTargetKind::Node, node_id, AnimChannelKind::LightIntensity, frame, node.light.intensity, interpolation);
        new_anim.insert_or_update_keyframe(AnimTargetKind::Node, node_id, AnimChannelKind::LightColorR, frame, node.light.color[0], interpolation);
        new_anim.insert_or_update_keyframe(AnimTargetKind::Node, node_id, AnimChannelKind::LightColorG, frame, node.light.color[1], interpolation);
        new_anim.insert_or_update_keyframe(AnimTargetKind::Node, node_id, AnimChannelKind::LightColorB, frame, node.light.color[2], interpolation);
        new_anim.insert_or_update_keyframe(AnimTargetKind::Node, node_id, AnimChannelKind::LightSpotOuterDeg, frame, node.light.spot_outer_deg, interpolation);
        new_anim.insert_or_update_keyframe(AnimTargetKind::Node, node_id, AnimChannelKind::LightRange, frame, node.light.range, interpolation);
        let cmd = SetAnimationStateCommand { old_anim, new_anim };
        self.history.execute_all(
            Box::new(cmd),
            &mut self.nodes,
            &mut self.selected_ids,
            &mut self.materials,
            &mut self.selected_material_id,
            &mut self.environment,
            &mut self.animation,
        );
        self.scene_revision += 1;
        6
    }

    pub fn anim_insert_camera_keys(
        &mut self,
        node_id: NodeId,
        frame: i32,
        interpolation: InterpolationMode,
    ) -> usize {
        let Some(node) = self.find_node(node_id).cloned() else {
            return 0;
        };
        if node.kind != NodeKind::Camera {
            return 0;
        }
        let old_anim = self.animation.clone();
        let mut new_anim = old_anim.clone();
        new_anim.insert_or_update_keyframe(AnimTargetKind::Node, node_id, AnimChannelKind::LocX, frame, node.transform.position.x, interpolation);
        new_anim.insert_or_update_keyframe(AnimTargetKind::Node, node_id, AnimChannelKind::LocY, frame, node.transform.position.y, interpolation);
        new_anim.insert_or_update_keyframe(AnimTargetKind::Node, node_id, AnimChannelKind::LocZ, frame, node.transform.position.z, interpolation);
        new_anim.insert_or_update_keyframe(AnimTargetKind::Node, node_id, AnimChannelKind::RotX, frame, node.transform.rotation_euler.x, interpolation);
        new_anim.insert_or_update_keyframe(AnimTargetKind::Node, node_id, AnimChannelKind::RotY, frame, node.transform.rotation_euler.y, interpolation);
        new_anim.insert_or_update_keyframe(AnimTargetKind::Node, node_id, AnimChannelKind::RotZ, frame, node.transform.rotation_euler.z, interpolation);
        new_anim.insert_or_update_keyframe(AnimTargetKind::Node, node_id, AnimChannelKind::CameraFovDeg, frame, node.camera_node.fov_y_degrees, interpolation);
        let cmd = SetAnimationStateCommand { old_anim, new_anim };
        self.history.execute_all(
            Box::new(cmd),
            &mut self.nodes,
            &mut self.selected_ids,
            &mut self.materials,
            &mut self.selected_material_id,
            &mut self.environment,
            &mut self.animation,
        );
        self.scene_revision += 1;
        7
    }

    pub fn anim_insert_material_keys(
        &mut self,
        mat_id: MaterialId,
        frame: i32,
        interpolation: InterpolationMode,
    ) -> usize {
        let Some(mat) = self.find_material(mat_id).cloned() else {
            return 0;
        };
        let old_anim = self.animation.clone();
        let mut new_anim = old_anim.clone();
        new_anim.insert_or_update_keyframe(AnimTargetKind::Material, mat_id, AnimChannelKind::MatColorR, frame, mat.base_color[0], interpolation);
        new_anim.insert_or_update_keyframe(AnimTargetKind::Material, mat_id, AnimChannelKind::MatColorG, frame, mat.base_color[1], interpolation);
        new_anim.insert_or_update_keyframe(AnimTargetKind::Material, mat_id, AnimChannelKind::MatColorB, frame, mat.base_color[2], interpolation);
        new_anim.insert_or_update_keyframe(AnimTargetKind::Material, mat_id, AnimChannelKind::MatOpacity, frame, mat.base_color[3], interpolation);
        new_anim.insert_or_update_keyframe(AnimTargetKind::Material, mat_id, AnimChannelKind::MatMetallic, frame, mat.metallic, interpolation);
        new_anim.insert_or_update_keyframe(AnimTargetKind::Material, mat_id, AnimChannelKind::MatRoughness, frame, mat.roughness, interpolation);
        new_anim.insert_or_update_keyframe(AnimTargetKind::Material, mat_id, AnimChannelKind::MatEmissiveIntensity, frame, mat.emissive_intensity, interpolation);
        let cmd = SetAnimationStateCommand { old_anim, new_anim };
        self.history.execute_all(
            Box::new(cmd),
            &mut self.nodes,
            &mut self.selected_ids,
            &mut self.materials,
            &mut self.selected_material_id,
            &mut self.environment,
            &mut self.animation,
        );
        self.scene_revision += 1;
        7
    }

    pub fn anim_update_keyframe(
        &mut self,
        track_id: TrackId,
        key_id: KeyframeId,
        frame: i32,
        value: f32,
        interpolation: InterpolationMode,
        in_handle: [f32; 2],
        out_handle: [f32; 2],
    ) -> bool {
        let old_anim = self.animation.clone();
        let mut new_anim = old_anim.clone();
        if !new_anim.update_keyframe(track_id, key_id, frame, value, interpolation, in_handle, out_handle) {
            return false;
        }
        let cmd = SetAnimationStateCommand { old_anim, new_anim };
        self.history.execute_all(
            Box::new(cmd),
            &mut self.nodes,
            &mut self.selected_ids,
            &mut self.materials,
            &mut self.selected_material_id,
            &mut self.environment,
            &mut self.animation,
        );
        self.eval_animation_at_current_frame();
        self.scene_revision += 1;
        true
    }

    pub fn anim_delete_keyframe(&mut self, track_id: TrackId, key_id: KeyframeId) -> bool {
        let old_anim = self.animation.clone();
        let mut new_anim = old_anim.clone();
        if !new_anim.delete_keyframe(track_id, key_id) {
            return false;
        }
        let cmd = SetAnimationStateCommand { old_anim, new_anim };
        self.history.execute_all(
            Box::new(cmd),
            &mut self.nodes,
            &mut self.selected_ids,
            &mut self.materials,
            &mut self.selected_material_id,
            &mut self.environment,
            &mut self.animation,
        );
        self.eval_animation_at_current_frame();
        self.scene_revision += 1;
        true
    }

    pub fn anim_step_keyframe(&mut self, direction: i32) -> bool {
        if let Some(target_f) = self
            .animation
            .step_keyframe_from(self.animation.timeline.current_frame, direction)
        {
            self.anim_set_frame(target_f as f32)
        } else {
            false
        }
    }

    pub fn set_camera_node_fov(&mut self, node_id: NodeId, fov_deg: f32) -> bool {
        let Some(existing) = self.find_node(node_id) else {
            return false;
        };
        if existing.kind != NodeKind::Camera {
            return false;
        }
        let old_cam = existing.camera_node.clone();
        let mut new_cam = old_cam.clone();
        new_cam.fov_y_degrees = fov_deg.clamp(15.0, 120.0);
        if (old_cam.fov_y_degrees - new_cam.fov_y_degrees).abs() < 1e-4 {
            return true;
        }
        let cmd = SetCameraNodeCommand {
            node_id,
            old_cam,
            new_cam: new_cam.clone(),
        };
        self.history
            .execute(Box::new(cmd), &mut self.nodes, &mut self.selected_ids);
        if self.animation.timeline.auto_key && !self.animation.timeline.playing {
            let f = self.animation.timeline.current_frame.round() as i32;
            self.animation.insert_or_update_keyframe(
                AnimTargetKind::Node,
                node_id,
                AnimChannelKind::CameraFovDeg,
                f,
                new_cam.fov_y_degrees,
                InterpolationMode::Bezier,
            );
        }
        self.scene_revision += 1;
        true
    }

    pub fn set_active_camera_node(&mut self, cam_node_id: Option<NodeId>) -> bool {
        if let Some(cid) = cam_node_id {
            let Some(n) = self.find_node(cid) else {
                return false;
            };
            if n.kind != NodeKind::Camera {
                return false;
            }
        }
        self.active_camera_node_id = cam_node_id;
        self.scene_revision += 1;
        true
    }

    fn record_transform_autokey(
        &mut self,
        node_id: NodeId,
        before: Transform,
        after: Transform,
        frame: i32,
    ) -> usize {
        let mut count = 0usize;
        let interp = InterpolationMode::Bezier;
        if (before.position.x - after.position.x).abs() > 1e-5 {
            self.animation.insert_or_update_keyframe(AnimTargetKind::Node, node_id, AnimChannelKind::LocX, frame, after.position.x, interp);
            count += 1;
        }
        if (before.position.y - after.position.y).abs() > 1e-5 {
            self.animation.insert_or_update_keyframe(AnimTargetKind::Node, node_id, AnimChannelKind::LocY, frame, after.position.y, interp);
            count += 1;
        }
        if (before.position.z - after.position.z).abs() > 1e-5 {
            self.animation.insert_or_update_keyframe(AnimTargetKind::Node, node_id, AnimChannelKind::LocZ, frame, after.position.z, interp);
            count += 1;
        }
        if (before.rotation_euler.x - after.rotation_euler.x).abs() > 1e-5 {
            self.animation.insert_or_update_keyframe(AnimTargetKind::Node, node_id, AnimChannelKind::RotX, frame, after.rotation_euler.x, interp);
            count += 1;
        }
        if (before.rotation_euler.y - after.rotation_euler.y).abs() > 1e-5 {
            self.animation.insert_or_update_keyframe(AnimTargetKind::Node, node_id, AnimChannelKind::RotY, frame, after.rotation_euler.y, interp);
            count += 1;
        }
        if (before.rotation_euler.z - after.rotation_euler.z).abs() > 1e-5 {
            self.animation.insert_or_update_keyframe(AnimTargetKind::Node, node_id, AnimChannelKind::RotZ, frame, after.rotation_euler.z, interp);
            count += 1;
        }
        if (before.scale.x - after.scale.x).abs() > 1e-5 {
            self.animation.insert_or_update_keyframe(AnimTargetKind::Node, node_id, AnimChannelKind::ScaleX, frame, after.scale.x, interp);
            count += 1;
        }
        if (before.scale.y - after.scale.y).abs() > 1e-5 {
            self.animation.insert_or_update_keyframe(AnimTargetKind::Node, node_id, AnimChannelKind::ScaleY, frame, after.scale.y, interp);
            count += 1;
        }
        if (before.scale.z - after.scale.z).abs() > 1e-5 {
            self.animation.insert_or_update_keyframe(AnimTargetKind::Node, node_id, AnimChannelKind::ScaleZ, frame, after.scale.z, interp);
            count += 1;
        }
        count
    }
}

impl From<[f32; 3]> for Vec3 {
    fn from(v: [f32; 3]) -> Self {
        Self::new(v[0], v[1], v[2])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serialize::{deserialize_project, serialize_project};

    #[test]
    fn starter_scene_has_valid_mesh_and_bounds() {
        let state = NovaEngineState::new_with_starter_scene();
        assert_eq!(state.nodes.len(), 2);
        let cube = &state.nodes[0];
        let mesh = cube.mesh.as_ref().unwrap();
        assert_eq!(mesh.positions.len(), 24);
        assert_eq!(mesh.normals.len(), 24);
        assert_eq!(mesh.indices.len(), 36);
        assert!(state.world_bounds().is_some());
    }

    #[test]
    fn all_primitives_generate_valid_nodes_and_undo_redo() {
        let mut state = NovaEngineState::new_empty();
        for p in 0..=10u32 {
            let prim = PrimitiveType::from_u32(p).unwrap();
            let id = state.add_primitive(prim);
            let node = state.find_node(id).unwrap();
            if p <= 7 {
                let mesh = node.mesh.as_ref().expect("mesh primitive must have mesh");
                assert!(!mesh.positions.is_empty());
                assert_eq!(mesh.positions.len(), mesh.normals.len());
                assert_eq!(mesh.positions.len(), mesh.uvs.len());
                assert!(mesh.indices.len() >= 3);
                for &idx in &mesh.indices {
                    assert!((idx as usize) < mesh.positions.len());
                }
            } else {
                assert!(node.mesh.is_none());
            }
        }
        assert_eq!(state.nodes.len(), 11);
        assert!(state.undo());
        assert_eq!(state.nodes.len(), 10);
        assert!(state.redo());
        assert_eq!(state.nodes.len(), 11);
    }

    #[test]
    fn transform_undo_redo_restores_exact_state() {
        let mut state = NovaEngineState::new_with_starter_scene();
        let id = state.nodes[0].id;
        let orig = state.nodes[0].transform;
        let modified = Transform {
            position: Vec3::new(3.0, 1.5, -2.0),
            rotation_euler: Vec3::new(0.1, 0.5, 0.0),
            scale: Vec3::new(2.0, 2.0, 2.0),
        };

        assert!(state.set_node_transform(id, modified));
        assert_eq!(state.find_node(id).unwrap().transform, modified);

        assert!(state.undo());
        assert_eq!(state.find_node(id).unwrap().transform, orig);

        assert!(state.redo());
        assert_eq!(state.find_node(id).unwrap().transform, modified);
    }

    #[test]
    fn hierarchy_group_reparent_cycle_guard_and_delete_undo() {
        let mut state = NovaEngineState::new_with_starter_scene();
        let c1 = state.nodes[0].id;
        let c2 = state.nodes[1].id;
        assert!(state.select_node(Some(c1), false));
        assert!(state.select_node(Some(c2), true));
        assert_eq!(state.selected_ids.len(), 2);

        let group_id = state.group_selected().expect("group created");
        assert_eq!(state.find_node(c1).unwrap().parent_id, Some(group_id));
        assert_eq!(state.find_node(c2).unwrap().parent_id, Some(group_id));
        // Cycle prevention: group cannot be parented under its child
        assert!(!state.reparent_node(group_id, Some(c1)));

        // Delete group (removes group + children) then undo restores everything cleanly
        assert!(state.select_node(Some(group_id), false));
        assert!(state.delete_selected());
        assert_eq!(state.nodes.len(), 0);

        assert!(state.undo());
        assert_eq!(state.nodes.len(), 3);
        assert_eq!(state.find_node(c1).unwrap().parent_id, Some(group_id));
    }

    #[test]
    fn apply_transform_and_origin_to_geometry_undo_cleanly() {
        let mut state = NovaEngineState::new_with_starter_scene();
        let id = state.nodes[0].id;
        assert!(state.select_node(Some(id), false));
        let before_world = state.node_world_bounds(id).unwrap();

        assert!(state.apply_transform_selected());
        assert_eq!(state.find_node(id).unwrap().transform, Transform::default());
        let after_world = state.node_world_bounds(id).unwrap();
        assert!((before_world.0.y - after_world.0.y).abs() < 1e-4);

        assert!(state.origin_to_geometry_selected());
        assert!((state.find_node(id).unwrap().transform.position.y - 0.75).abs() < 1e-4);

        assert!(state.undo());
        assert_eq!(state.find_node(id).unwrap().transform, Transform::default());
        assert!(state.undo());
        assert!((state.find_node(id).unwrap().transform.position.y - 0.75).abs() < 1e-4);
    }

    #[test]
    fn ray_pick_and_gizmo_drag_work_and_undo() {
        let mut state = NovaEngineState::new_with_starter_scene();
        state.camera.set_standard_view(StandardView::Front);
        state.camera.target = Vec3::new(0.0, 0.75, 0.0);
        // Center screen ray should hit Studio Cube
        let hit = state.pick_node_at_ndc(0.0, 0.0, 1.5, false);
        assert_eq!(hit, Some(state.nodes[0].id));

        state.gizmo.mode = GizmoMode::Move;
        assert!(state.begin_gizmo_drag(GizmoAxis::X, 0.0, 0.0, 1.5));
        assert!(state.update_gizmo_drag(0.25, 0.0, 1.5));
        let moved_x = state.nodes[0].transform.position.x;
        assert!(moved_x > 0.1);
        assert!(state.end_gizmo_drag());

        assert!(state.undo());
        assert!(state.nodes[0].transform.position.x.abs() < 1e-5);
    }

    #[test]
    fn serialize_and_deserialize_round_trip_and_reject_corrupt() {
        let mut state = NovaEngineState::new_with_starter_scene();
        let sphere_id = state.add_primitive(PrimitiveType::UvSphere);
        let light_id = state.add_primitive(PrimitiveType::Light);
        assert!(state.reparent_node(sphere_id, Some(state.nodes[0].id)));
        assert!(state.rename_node(light_id, "Key Light \"1\"".to_string()));

        let json = serialize_project(&state);
        let restored = deserialize_project(&json).expect("valid project should load");
        assert_eq!(restored.nodes.len(), state.nodes.len());
        assert_eq!(restored.find_node(sphere_id).unwrap().parent_id, Some(state.nodes[0].id));
        assert_eq!(restored.find_node(light_id).unwrap().name, "Key Light \"1\"");

        assert!(deserialize_project("{\"format\":\"other\",\"version\":1}").is_err());
        assert!(deserialize_project("not json at all").is_err());
    }

    #[test]
    fn m3_edit_mode_mesh_ops_gizmo_and_undo() {
        let mut state = NovaEngineState::new_with_starter_scene();
        let cube_id = state.nodes[0].id;
        assert!(state.select_node(Some(cube_id), false));
        assert!(state.set_interaction_mode(InteractionMode::Edit));
        assert_eq!(state.interaction_mode, InteractionMode::Edit);
        assert!(!state.edit_overlay_lines.is_empty());

        // Front view pick center face & extrude
        state.camera.set_standard_view(StandardView::Front);
        state.camera.target = Vec3::new(0.0, 0.75, 0.0);
        state.set_edit_select_mode(EditSelectMode::Face);
        assert!(state.edit_pick_at_ndc(0.0, 0.0, 1.5, false));
        assert_eq!(state.edit_sel_faces.len(), 1);
        let init_face_count = state.find_node(cube_id).unwrap().base_poly.as_ref().unwrap().faces.len();
        assert_eq!(init_face_count, 6);

        assert!(state.edit_extrude(0.4));
        assert_eq!(
            state.find_node(cube_id).unwrap().base_poly.as_ref().unwrap().faces.len(),
            10
        );
        assert!(state.edit_inset(0.25));
        assert_eq!(
            state.find_node(cube_id).unwrap().base_poly.as_ref().unwrap().faces.len(),
            14
        );

        // Edit gizmo move on selected face vertices
        let before_centroid = state.edit_selection_centroid_world().unwrap();
        state.gizmo.mode = GizmoMode::Move;
        assert!(state.begin_gizmo_drag(GizmoAxis::Y, 0.0, 0.0, 1.5));
        assert!(state.update_gizmo_drag(0.0, 0.25, 1.5));
        assert!(state.end_gizmo_drag());
        let after_centroid = state.edit_selection_centroid_world().unwrap();
        assert!((after_centroid.y - before_centroid.y).abs() > 0.02);

        // Undo vertex gizmo move, inset, extrude -> back to 6 faces
        assert!(state.undo());
        assert!(state.undo());
        assert!(state.undo());
        assert_eq!(
            state.find_node(cube_id).unwrap().base_poly.as_ref().unwrap().faces.len(),
            6
        );

        // Test subdivide, bevel, normals, smooth shading, face orientation
        assert!(state.edit_subdivide());
        assert!(state.find_node(cube_id).unwrap().base_poly.as_ref().unwrap().faces.len() > 6);
        assert!(state.edit_flip_normals());
        assert!(state.edit_recalculate_normals());
        assert!(state.edit_set_smooth_shading(true));
        assert!(state.find_node(cube_id).unwrap().base_poly.as_ref().unwrap().smooth_shading);

        state.set_face_orientation(true);
        assert!(!state.edit_overlay_tris.is_empty());

        // Switch back to Object mode & verify clean exit
        assert!(state.set_interaction_mode(InteractionMode::Object));
        assert_eq!(state.interaction_mode, InteractionMode::Object);
    }

    #[test]
    fn m3_loop_cut_bevel_merge_delete_dissolve() {
        let mut state = NovaEngineState::new_with_starter_scene();
        assert!(state.set_interaction_mode(InteractionMode::Edit));

        // Edge mode: select one edge -> loop cut & bevel
        state.set_edit_select_mode(EditSelectMode::Edge);
        state.edit_sel_edges.insert((0, 1));
        assert!(state.edit_loop_cut());
        assert!(state.find_node(state.nodes[0].id).unwrap().base_poly.as_ref().unwrap().faces.len() > 6);

        assert!(state.undo());
        state.edit_sel_edges.clear();
        state.edit_sel_edges.insert((0, 1));
        assert!(state.edit_bevel(0.15));
        assert!(state.find_node(state.nodes[0].id).unwrap().base_poly.as_ref().unwrap().faces.len() >= 7);

        // Edge dissolve
        assert!(state.undo());
        assert!(state.edit_loop_cut());
        assert!(!state.edit_sel_edges.is_empty());
        let after_loop_faces = state.find_node(state.nodes[0].id).unwrap().base_poly.as_ref().unwrap().faces.len();
        assert!(state.edit_dissolve_edges());
        assert!(state.find_node(state.nodes[0].id).unwrap().base_poly.as_ref().unwrap().faces.len() < after_loop_faces);

        // Merge & delete vertices
        state.set_edit_select_mode(EditSelectMode::Vertex);
        state.edit_sel_verts.insert(0);
        state.edit_sel_verts.insert(1);
        assert!(state.edit_merge_selected());
        assert_eq!(state.edit_sel_verts.len(), 1);
        assert!(state.edit_delete_selected());
    }

    #[test]
    fn m3_modifiers_stack_apply_undo_and_serialize_roundtrip() {
        let mut state = NovaEngineState::new_with_starter_scene();
        let cube_id = state.nodes[0].id;
        assert!(state.select_node(Some(cube_id), false));

        assert!(state.add_modifier(ModifierSpec::Mirror {
            axis: 0,
            merge: true,
            enabled: true,
        }));
        assert!(state.add_modifier(ModifierSpec::Subdivision {
            levels: 1,
            enabled: true,
        }));
        assert!(state.add_modifier(ModifierSpec::Solidify {
            thickness: 0.08,
            enabled: false,
        }));
        assert!(state.add_modifier(ModifierSpec::Array {
            count: 2,
            offset: [2.0, 0.0, 0.0],
            enabled: true,
        }));
        assert_eq!(state.find_node(cube_id).unwrap().modifiers.len(), 4);

        let mod_tri_count = state.find_node(cube_id).unwrap().mesh.as_ref().unwrap().indices.len() / 3;
        assert!(mod_tri_count > 12);

        // Reorder and serialize round-trip
        assert!(state.move_modifier(1, -1));
        let json = serialize_project(&state);
        let restored = deserialize_project(&json).expect("m3 project round-trip");
        let r_cube = restored.find_node(cube_id).unwrap();
        assert_eq!(r_cube.modifiers.len(), 4);
        assert!(r_cube.base_poly.is_some());
        assert_eq!(r_cube.mesh.as_ref().unwrap().indices.len() / 3, mod_tri_count);

        // Apply modifier stack & undo
        assert!(state.apply_modifier_stack());
        assert!(state.find_node(cube_id).unwrap().modifiers.is_empty());
        assert!(state.find_node(cube_id).unwrap().base_poly.as_ref().unwrap().faces.len() > 6);
        assert!(state.undo());
        assert_eq!(state.find_node(cube_id).unwrap().modifiers.len(), 4);
        assert_eq!(state.find_node(cube_id).unwrap().base_poly.as_ref().unwrap().faces.len(), 6);
    }

    #[test]
    fn m4_material_library_crud_assign_undo_and_roundtrip() {
        let mut state = NovaEngineState::new_with_starter_scene();
        let cube_id = state.nodes[0].id;
        assert!(state.select_node(Some(cube_id), false));
        let initial_mats = state.materials.len();
        assert_eq!(initial_mats, 2);

        // Create Brick Masonry preset (preset 6) and assign to selected cube
        let brick_id = state.create_material_from_preset(6, true);
        assert_eq!(state.materials.len(), initial_mats + 1);
        assert_eq!(state.find_node(cube_id).unwrap().material_id, Some(brick_id));
        let resolved = state.resolved_node_material(state.find_node(cube_id).unwrap());
        assert!(resolved.albedo_tex_id.is_some());
        assert!(resolved.normal_tex_id.is_some());
        assert!((resolved.uv_scale[0] - 2.0).abs() < 1e-4);

        // Rename & duplicate & update glass alpha
        assert!(state.rename_material(brick_id, "Red Masonry"));
        assert_eq!(state.find_material(brick_id).unwrap().name, "Red Masonry");
        let dup_id = state.duplicate_material(brick_id).expect("duplicate");
        let mut dup = state.find_material(dup_id).unwrap().clone();
        dup.base_color = [0.4, 0.8, 1.0, 0.45];
        dup.emissive = [0.1, 0.7, 1.0];
        dup.emissive_intensity = 2.0;
        assert!(state.update_material(dup));
        assert!(state.assign_material_to_selected(dup_id));
        let updated_node = state.resolved_node_material(state.find_node(cube_id).unwrap());
        assert_eq!(updated_node.alpha_mode, AlphaMode::Blend);
        assert!((updated_node.emissive_intensity - 2.0).abs() < 1e-4);

        // Delete duplicate reverts node fallback cleanly, and undo restores it
        assert!(state.delete_material(dup_id));
        assert!(state.find_material(dup_id).is_none());
        assert!(state.undo());
        assert!(state.find_material(dup_id).is_some());
        assert!(state.redo());
        assert!(state.find_material(dup_id).is_none());

        // Serialize / deserialize preserves custom materials, assignments & UVs
        let json = serialize_project(&state);
        let restored = deserialize_project(&json).expect("m4 serialize");
        let r_brick = restored.find_material(brick_id).expect("brick kept");
        assert_eq!(r_brick.name, "Red Masonry");
        assert!(r_brick.normal_tex_id.is_some());
        assert!(!restored.nodes[0].mesh.as_ref().unwrap().uvs.is_empty());
    }

    #[test]
    fn m4_missing_texture_fallback_env_and_lighting_rigs() {
        let mut state = NovaEngineState::new_with_starter_scene();
        // Register 2x2 custom RGBA texture + attach to Brick Masonry
        let rgba = vec![
            255, 0, 0, 255, 0, 255, 0, 255,
            0, 0, 255, 255, 255, 255, 0, 255,
        ];
        let tex_id = state
            .register_rgba_texture("Custom Check", "content://test/check.png", 2, 2, rgba.clone())
            .expect("register rgba");
        let brick_id = state.create_material_from_preset(6, true);
        let mut mat = state.find_material(brick_id).unwrap().clone();
        mat.albedo_tex_id = Some(tex_id);
        assert!(state.update_material(mat));

        assert!(state.missing_texture_warnings().is_empty());
        assert!(state.mark_texture_missing(tex_id, true));
        let warnings = state.missing_texture_warnings();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("Custom Check"));
        let fallback_tex = state.find_texture(tex_id).unwrap();
        assert_eq!(fallback_tex.status, TextureStatus::Missing);
        assert_eq!(fallback_tex.effective_dimensions(), (64, 64));
        assert_eq!(fallback_tex.effective_rgba().len(), 64 * 64 * 4);

        // Save/load preserves custom texture bytes (when loaded) and missing flag
        assert!(state.mark_texture_missing(tex_id, false));
        let json = serialize_project(&state);
        let restored = deserialize_project(&json).expect("texture roundtrip");
        let r_tex = restored.find_texture(tex_id).expect("custom tex restored");
        assert_eq!(r_tex.status, TextureStatus::Loaded);
        assert_eq!(r_tex.rgba, rgba);

        // Environment + Lighting Rigs + Undo
        let mut env = state.environment.clone();
        env.preset = EnvironmentPreset::GoldenSunset;
        env.rotation_deg = 135.0;
        env.show_skybox = true;
        assert!(state.set_environment(env));
        assert_eq!(state.environment.preset, EnvironmentPreset::GoldenSunset);
        assert!(state.apply_lighting_rig(2)); // Dramatic Rim
        let light_count = state.nodes.iter().filter(|n| n.kind == NodeKind::Light).count();
        assert!(light_count >= 2);
        assert!(state.undo());
        assert_eq!(
            state.nodes.iter().filter(|n| n.kind == NodeKind::Light).count(),
            0
        );
    }

    #[test]
    fn m4_backward_compatible_with_m2_m3_json() {
        let legacy_json = r#"{
          "format": "nova3d",
          "version": 1,
          "next_node_id": 3,
          "show_grid": true,
          "show_axes": true,
          "shading_mode": 0,
          "selected_ids": [],
          "camera": {
            "target": [0.0, 0.0, 0.0],
            "distance": 5.0,
            "yaw": 0.6,
            "pitch": 0.4,
            "projection": 0,
            "fov_y_degrees": 45.0,
            "near_plane": 0.05,
            "far_plane": 1000.0
          },
          "nodes": [
            {
              "id": 1,
              "parent_id": null,
              "name": "Legacy Cube",
              "kind": 0,
              "visible": true,
              "transform": { "position": [0, 0, 0], "rotation": [0, 0, 0], "scale": [1, 1, 1] },
              "material": { "base_color": [0.9, 0.2, 0.3, 1.0], "metallic": 0.1, "roughness": 0.4 },
              "light": { "kind": 0, "color": [1, 1, 1], "intensity": 3.0, "cast_shadows": true },
              "camera_node": { "fov_y_degrees": 45.0, "near_plane": 0.05, "far_plane": 100.0 },
              "mesh": null
            }
          ]
        }"#;
        let state = deserialize_project(legacy_json).expect("legacy M2/M3 load");
        assert!(!state.materials.is_empty());
        assert!(!state.textures.is_empty());
        let resolved = state.resolved_node_material(&state.nodes[0]);
        assert!((resolved.base_color[0] - 0.9).abs() < 1e-4);
    }

    #[test]
    fn m5_keyframe_interpolation_curves_and_bezier() {
        let mut anim = AnimationState::default();
        let (_, k1) = anim.insert_or_update_keyframe(
            AnimTargetKind::Node,
            1,
            AnimChannelKind::LocX,
            1,
            0.0,
            InterpolationMode::Linear,
        );
        anim.insert_or_update_keyframe(
            AnimTargetKind::Node,
            1,
            AnimChannelKind::LocX,
            11,
            10.0,
            InterpolationMode::Linear,
        );
        let track = &anim.tracks[0];
        assert!((track.evaluate_at(6.0).unwrap() - 5.0).abs() < 1e-4);

        // Constant step holds key1 value until key2
        assert!(anim.update_keyframe(
            anim.tracks[0].id,
            k1,
            1,
            0.0,
            InterpolationMode::Constant,
            [-3.0, 0.0],
            [3.0, 0.0],
        ));
        assert!((anim.tracks[0].evaluate_at(9.5).unwrap() - 0.0).abs() < 1e-4);
        assert!((anim.tracks[0].evaluate_at(11.0).unwrap() - 10.0).abs() < 1e-4);

        // EaseInOut slower near start/end, exact midpoint 5.0
        assert!(anim.update_keyframe(
            anim.tracks[0].id,
            k1,
            1,
            0.0,
            InterpolationMode::EaseInOut,
            [-3.0, 0.0],
            [3.0, 0.0],
        ));
        assert!((anim.tracks[0].evaluate_at(6.0).unwrap() - 5.0).abs() < 1e-3);
        assert!(anim.tracks[0].evaluate_at(3.5).unwrap() < 2.5);

        // Bezier custom tangent overshoot/boost
        assert!(anim.update_keyframe(
            anim.tracks[0].id,
            k1,
            1,
            0.0,
            InterpolationMode::Bezier,
            [-3.0, 0.0],
            [4.0, 6.0],
        ));
        let bez_mid = anim.tracks[0].evaluate_at(6.0).unwrap();
        assert!(bez_mid > 5.0 && bez_mid < 12.0, "got {bez_mid}");
    }

    #[test]
    fn m5_deterministic_playback_and_material_light_camera_eval() {
        let mut state = NovaEngineState::new_with_starter_scene();
        let cube_id = state.nodes[0].id;
        let light_id = state.add_primitive(PrimitiveType::Light);
        let cam_id = state.add_primitive(PrimitiveType::Camera);
        let mat_id = state.materials[0].id;

        state.anim_insert_keyframe(
            AnimTargetKind::Node,
            cube_id,
            AnimChannelKind::LocY,
            1,
            1.0,
            InterpolationMode::Linear,
        );
        state.anim_insert_keyframe(
            AnimTargetKind::Node,
            cube_id,
            AnimChannelKind::LocY,
            25,
            5.0,
            InterpolationMode::Linear,
        );
        state.anim_insert_keyframe(
            AnimTargetKind::Node,
            light_id,
            AnimChannelKind::LightIntensity,
            1,
            20000.0,
            InterpolationMode::Linear,
        );
        state.anim_insert_keyframe(
            AnimTargetKind::Node,
            light_id,
            AnimChannelKind::LightIntensity,
            25,
            100000.0,
            InterpolationMode::Linear,
        );
        state.anim_insert_keyframe(
            AnimTargetKind::Node,
            cam_id,
            AnimChannelKind::CameraFovDeg,
            1,
            40.0,
            InterpolationMode::Linear,
        );
        state.anim_insert_keyframe(
            AnimTargetKind::Node,
            cam_id,
            AnimChannelKind::CameraFovDeg,
            25,
            70.0,
            InterpolationMode::Linear,
        );
        state.anim_insert_keyframe(
            AnimTargetKind::Material,
            mat_id,
            AnimChannelKind::MatRoughness,
            1,
            0.2,
            InterpolationMode::Linear,
        );
        state.anim_insert_keyframe(
            AnimTargetKind::Material,
            mat_id,
            AnimChannelKind::MatRoughness,
            25,
            0.8,
            InterpolationMode::Linear,
        );

        state.anim_set_timeline(1, 25, 24.0, true, false);
        state.anim_set_playing(true);
        // Tick 0 initializes timestamp, tick +0.5s (= +12 frames @ 24fps) moves frame 1 -> 13 (exact midpoint)
        let _ = state.anim_tick(1_000_000_000);
        // Clamp dt max is 0.1s per call, so step 5 frames of 0.1s = 0.5s total
        for i in 1..=5 {
            state.anim_tick(1_000_000_000 + i * 100_000_000);
        }
        assert!((state.animation.timeline.current_frame - 13.0).abs() < 1e-3);
        assert!((state.find_node(cube_id).unwrap().transform.position.y - 3.0).abs() < 1e-2);
        assert!((state.find_node(light_id).unwrap().light.intensity - 60000.0).abs() < 150.0);
        assert!((state.find_material(mat_id).unwrap().roughness - 0.5).abs() < 1e-2);

        state.set_active_camera_node(Some(cam_id));
        let snap = state.camera_snapshot();
        assert!((snap.fov_y_degrees - 55.0).abs() < 0.2);
    }

    #[test]
    fn m5_animation_undo_redo_and_serialize_roundtrip() {
        let mut state = NovaEngineState::new_with_starter_scene();
        let cube_id = state.nodes[0].id;
        let cam_id = state.add_primitive(PrimitiveType::Camera);
        state.set_active_camera_node(Some(cam_id));
        state.anim_set_timeline(1, 60, 30.0, true, true);
        let (tid, kid) = state.anim_insert_keyframe(
            AnimTargetKind::Node,
            cube_id,
            AnimChannelKind::RotY,
            15,
            1.25,
            InterpolationMode::EaseOut,
        );
        assert_eq!(state.animation.total_keyframes(), 1);

        let json = serialize_project(&state);
        let restored = deserialize_project(&json).expect("M5 animation roundtrip");
        assert_eq!(restored.animation.timeline.end_frame, 60);
        assert!((restored.animation.timeline.fps - 30.0).abs() < 1e-4);
        assert!(restored.animation.timeline.auto_key);
        assert_eq!(restored.active_camera_node_id, Some(cam_id));
        assert_eq!(restored.animation.total_keyframes(), 1);
        assert_eq!(
            restored.animation.tracks[0].keyframes[0].interpolation,
            InterpolationMode::EaseOut
        );

        assert!(state.anim_delete_keyframe(tid, kid));
        assert_eq!(state.animation.total_keyframes(), 0);
        assert!(state.undo());
        assert_eq!(state.animation.total_keyframes(), 1);
        assert!(state.redo());
        assert_eq!(state.animation.total_keyframes(), 0);
    }
}
