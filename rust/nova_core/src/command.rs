//! Command-based undo/redo history covering M2 object, transform, hierarchy, and property edits (UR-1, UR-2, UR-3).

use std::collections::VecDeque;

use crate::animation::AnimationState;
use crate::lookdev::{EnvironmentSpec, MaterialId, MaterialSpec};
use crate::poly::{ModifierSpec, PolyMesh};
use crate::scene::{CameraNodeParams, LightParams, MaterialParams, MeshData, NodeId, SceneNode, Transform};

pub trait Command: std::fmt::Debug {
    fn apply(&self, nodes: &mut Vec<SceneNode>, selected: &mut Vec<NodeId>);
    fn revert(&self, nodes: &mut Vec<SceneNode>, selected: &mut Vec<NodeId>);

    fn apply_lookdev(
        &self,
        _materials: &mut Vec<MaterialSpec>,
        _selected_material: &mut Option<MaterialId>,
        _environment: &mut EnvironmentSpec,
    ) {
    }

    fn revert_lookdev(
        &self,
        _materials: &mut Vec<MaterialSpec>,
        _selected_material: &mut Option<MaterialId>,
        _environment: &mut EnvironmentSpec,
    ) {
    }

    fn apply_animation(&self, _anim: &mut AnimationState) {}

    fn revert_animation(&self, _anim: &mut AnimationState) {}
}

#[derive(Debug, Clone)]
pub struct SetTransformCommand {
    pub changes: Vec<(NodeId, Transform, Transform)>,
}

impl SetTransformCommand {
    pub fn single(node_id: NodeId, before: Transform, after: Transform) -> Self {
        Self {
            changes: vec![(node_id, before, after)],
        }
    }
}

impl Command for SetTransformCommand {
    fn apply(&self, nodes: &mut Vec<SceneNode>, _selected: &mut Vec<NodeId>) {
        for (id, _, after) in &self.changes {
            if let Some(node) = nodes.iter_mut().find(|n| n.id == *id) {
                node.transform = *after;
            }
        }
    }

    fn revert(&self, nodes: &mut Vec<SceneNode>, _selected: &mut Vec<NodeId>) {
        for (id, before, _) in &self.changes {
            if let Some(node) = nodes.iter_mut().find(|n| n.id == *id) {
                node.transform = *before;
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct AddNodesCommand {
    pub added: Vec<SceneNode>,
    pub prev_selected: Vec<NodeId>,
    pub next_selected: Vec<NodeId>,
}

impl Command for AddNodesCommand {
    fn apply(&self, nodes: &mut Vec<SceneNode>, selected: &mut Vec<NodeId>) {
        for item in &self.added {
            if !nodes.iter().any(|n| n.id == item.id) {
                nodes.push(item.clone());
            }
        }
        *selected = self.next_selected.clone();
    }

    fn revert(&self, nodes: &mut Vec<SceneNode>, selected: &mut Vec<NodeId>) {
        nodes.retain(|n| !self.added.iter().any(|a| a.id == n.id));
        *selected = self.prev_selected.clone();
    }
}

#[derive(Debug, Clone)]
pub struct DeleteNodesCommand {
    /// Original (index, node) pairs sorted in ascending index order for exact restoration.
    pub removed: Vec<(usize, SceneNode)>,
    /// Any surviving child nodes whose parent_id was cleared/reassigned on delete: (child_id, old_parent, new_parent).
    pub parent_updates: Vec<(NodeId, Option<NodeId>, Option<NodeId>)>,
    pub prev_selected: Vec<NodeId>,
}

impl Command for DeleteNodesCommand {
    fn apply(&self, nodes: &mut Vec<SceneNode>, selected: &mut Vec<NodeId>) {
        for (child_id, _, new_parent) in &self.parent_updates {
            if let Some(node) = nodes.iter_mut().find(|n| n.id == *child_id) {
                node.parent_id = *new_parent;
            }
        }
        nodes.retain(|n| !self.removed.iter().any(|(_, r)| r.id == n.id));
        selected.retain(|id| nodes.iter().any(|n| n.id == *id));
    }

    fn revert(&self, nodes: &mut Vec<SceneNode>, selected: &mut Vec<NodeId>) {
        for (idx, node) in &self.removed {
            if !nodes.iter().any(|n| n.id == node.id) {
                let insert_at = (*idx).min(nodes.len());
                nodes.insert(insert_at, node.clone());
            }
        }
        for (child_id, old_parent, _) in &self.parent_updates {
            if let Some(node) = nodes.iter_mut().find(|n| n.id == *child_id) {
                node.parent_id = *old_parent;
            }
        }
        *selected = self.prev_selected.clone();
    }
}

#[derive(Debug, Clone)]
pub struct RenameNodeCommand {
    pub node_id: NodeId,
    pub before: String,
    pub after: String,
}

impl Command for RenameNodeCommand {
    fn apply(&self, nodes: &mut Vec<SceneNode>, _selected: &mut Vec<NodeId>) {
        if let Some(node) = nodes.iter_mut().find(|n| n.id == self.node_id) {
            node.name = self.after.clone();
        }
    }

    fn revert(&self, nodes: &mut Vec<SceneNode>, _selected: &mut Vec<NodeId>) {
        if let Some(node) = nodes.iter_mut().find(|n| n.id == self.node_id) {
            node.name = self.before.clone();
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReparentChange {
    pub node_id: NodeId,
    pub old_parent: Option<NodeId>,
    pub new_parent: Option<NodeId>,
    pub old_transform: Transform,
    pub new_transform: Transform,
}

#[derive(Debug, Clone)]
pub struct ReparentNodesCommand {
    pub optional_created_group: Option<SceneNode>,
    pub changes: Vec<ReparentChange>,
    pub prev_selected: Vec<NodeId>,
    pub next_selected: Vec<NodeId>,
}

impl Command for ReparentNodesCommand {
    fn apply(&self, nodes: &mut Vec<SceneNode>, selected: &mut Vec<NodeId>) {
        if let Some(group) = &self.optional_created_group {
            if !nodes.iter().any(|n| n.id == group.id) {
                nodes.push(group.clone());
            }
        }
        for c in &self.changes {
            if let Some(node) = nodes.iter_mut().find(|n| n.id == c.node_id) {
                node.parent_id = c.new_parent;
                node.transform = c.new_transform;
            }
        }
        *selected = self.next_selected.clone();
    }

    fn revert(&self, nodes: &mut Vec<SceneNode>, selected: &mut Vec<NodeId>) {
        for c in &self.changes {
            if let Some(node) = nodes.iter_mut().find(|n| n.id == c.node_id) {
                node.parent_id = c.old_parent;
                node.transform = c.old_transform;
            }
        }
        if let Some(group) = &self.optional_created_group {
            nodes.retain(|n| n.id != group.id);
        }
        *selected = self.prev_selected.clone();
    }
}

#[derive(Debug, Clone)]
pub struct BakeMeshTransformCommand {
    pub node_id: NodeId,
    pub old_mesh: MeshData,
    pub new_mesh: MeshData,
    pub old_poly: Option<PolyMesh>,
    pub new_poly: Option<PolyMesh>,
    pub old_transform: Transform,
    pub new_transform: Transform,
}

impl Command for BakeMeshTransformCommand {
    fn apply(&self, nodes: &mut Vec<SceneNode>, _selected: &mut Vec<NodeId>) {
        if let Some(node) = nodes.iter_mut().find(|n| n.id == self.node_id) {
            node.mesh = Some(self.new_mesh.clone());
            node.base_poly = self.new_poly.clone();
            node.transform = self.new_transform;
        }
    }

    fn revert(&self, nodes: &mut Vec<SceneNode>, _selected: &mut Vec<NodeId>) {
        if let Some(node) = nodes.iter_mut().find(|n| n.id == self.node_id) {
            node.mesh = Some(self.old_mesh.clone());
            node.base_poly = self.old_poly.clone();
            node.transform = self.old_transform;
        }
    }
}

#[derive(Debug, Clone)]
pub struct SetVisibilityCommand {
    pub node_id: NodeId,
    pub before: bool,
    pub after: bool,
}

impl Command for SetVisibilityCommand {
    fn apply(&self, nodes: &mut Vec<SceneNode>, _selected: &mut Vec<NodeId>) {
        if let Some(node) = nodes.iter_mut().find(|n| n.id == self.node_id) {
            node.visible = self.after;
        }
    }

    fn revert(&self, nodes: &mut Vec<SceneNode>, _selected: &mut Vec<NodeId>) {
        if let Some(node) = nodes.iter_mut().find(|n| n.id == self.node_id) {
            node.visible = self.before;
        }
    }
}

#[derive(Debug, Clone)]
pub struct SetMaterialCommand {
    pub node_id: NodeId,
    pub before: MaterialParams,
    pub after: MaterialParams,
}

impl Command for SetMaterialCommand {
    fn apply(&self, nodes: &mut Vec<SceneNode>, _selected: &mut Vec<NodeId>) {
        if let Some(node) = nodes.iter_mut().find(|n| n.id == self.node_id) {
            node.material = self.after.clone();
        }
    }

    fn revert(&self, nodes: &mut Vec<SceneNode>, _selected: &mut Vec<NodeId>) {
        if let Some(node) = nodes.iter_mut().find(|n| n.id == self.node_id) {
            node.material = self.before.clone();
        }
    }
}

#[derive(Debug, Clone)]
pub struct SetLightCommand {
    pub node_id: NodeId,
    pub before: LightParams,
    pub after: LightParams,
}

impl Command for SetLightCommand {
    fn apply(&self, nodes: &mut Vec<SceneNode>, _selected: &mut Vec<NodeId>) {
        if let Some(node) = nodes.iter_mut().find(|n| n.id == self.node_id) {
            node.light = self.after.clone();
        }
    }

    fn revert(&self, nodes: &mut Vec<SceneNode>, _selected: &mut Vec<NodeId>) {
        if let Some(node) = nodes.iter_mut().find(|n| n.id == self.node_id) {
            node.light = self.before.clone();
        }
    }
}

#[derive(Debug, Clone)]
pub struct SetPolyMeshCommand {
    pub node_id: NodeId,
    pub old_poly: PolyMesh,
    pub new_poly: PolyMesh,
    pub old_mesh: MeshData,
    pub new_mesh: MeshData,
}

impl Command for SetPolyMeshCommand {
    fn apply(&self, nodes: &mut Vec<SceneNode>, _selected: &mut Vec<NodeId>) {
        if let Some(node) = nodes.iter_mut().find(|n| n.id == self.node_id) {
            node.base_poly = Some(self.new_poly.clone());
            node.mesh = Some(self.new_mesh.clone());
        }
    }

    fn revert(&self, nodes: &mut Vec<SceneNode>, _selected: &mut Vec<NodeId>) {
        if let Some(node) = nodes.iter_mut().find(|n| n.id == self.node_id) {
            node.base_poly = Some(self.old_poly.clone());
            node.mesh = Some(self.old_mesh.clone());
        }
    }
}

#[derive(Debug, Clone)]
pub struct SetModifiersCommand {
    pub node_id: NodeId,
    pub old_modifiers: Vec<ModifierSpec>,
    pub new_modifiers: Vec<ModifierSpec>,
    pub old_poly: Option<PolyMesh>,
    pub new_poly: Option<PolyMesh>,
    pub old_mesh: MeshData,
    pub new_mesh: MeshData,
}

impl Command for SetModifiersCommand {
    fn apply(&self, nodes: &mut Vec<SceneNode>, _selected: &mut Vec<NodeId>) {
        if let Some(node) = nodes.iter_mut().find(|n| n.id == self.node_id) {
            node.modifiers = self.new_modifiers.clone();
            if let Some(poly) = &self.new_poly {
                node.base_poly = Some(poly.clone());
            }
            node.mesh = Some(self.new_mesh.clone());
        }
    }

    fn revert(&self, nodes: &mut Vec<SceneNode>, _selected: &mut Vec<NodeId>) {
        if let Some(node) = nodes.iter_mut().find(|n| n.id == self.node_id) {
            node.modifiers = self.old_modifiers.clone();
            if let Some(poly) = &self.old_poly {
                node.base_poly = Some(poly.clone());
            }
            node.mesh = Some(self.old_mesh.clone());
        }
    }
}

#[derive(Debug, Clone)]
pub struct SetMaterialLibraryCommand {
    pub old_materials: Vec<MaterialSpec>,
    pub new_materials: Vec<MaterialSpec>,
    pub old_selected_material: Option<MaterialId>,
    pub new_selected_material: Option<MaterialId>,
    pub node_changes: Vec<(NodeId, Option<MaterialId>, MaterialParams, Option<MaterialId>, MaterialParams)>,
}

impl Command for SetMaterialLibraryCommand {
    fn apply(&self, nodes: &mut Vec<SceneNode>, _selected: &mut Vec<NodeId>) {
        for (nid, _, _, new_mid, new_params) in &self.node_changes {
            if let Some(n) = nodes.iter_mut().find(|n| n.id == *nid) {
                n.material_id = *new_mid;
                n.material = new_params.clone();
            }
        }
    }

    fn revert(&self, nodes: &mut Vec<SceneNode>, _selected: &mut Vec<NodeId>) {
        for (nid, old_mid, old_params, _, _) in &self.node_changes {
            if let Some(n) = nodes.iter_mut().find(|n| n.id == *nid) {
                n.material_id = *old_mid;
                n.material = old_params.clone();
            }
        }
    }

    fn apply_lookdev(
        &self,
        materials: &mut Vec<MaterialSpec>,
        selected_material: &mut Option<MaterialId>,
        _environment: &mut EnvironmentSpec,
    ) {
        *materials = self.new_materials.clone();
        *selected_material = self.new_selected_material;
    }

    fn revert_lookdev(
        &self,
        materials: &mut Vec<MaterialSpec>,
        selected_material: &mut Option<MaterialId>,
        _environment: &mut EnvironmentSpec,
    ) {
        *materials = self.old_materials.clone();
        *selected_material = self.old_selected_material;
    }
}

#[derive(Debug, Clone)]
pub struct SetEnvironmentCommand {
    pub before: EnvironmentSpec,
    pub after: EnvironmentSpec,
}

impl Command for SetEnvironmentCommand {
    fn apply(&self, _nodes: &mut Vec<SceneNode>, _selected: &mut Vec<NodeId>) {}
    fn revert(&self, _nodes: &mut Vec<SceneNode>, _selected: &mut Vec<NodeId>) {}

    fn apply_lookdev(
        &self,
        _materials: &mut Vec<MaterialSpec>,
        _selected_material: &mut Option<MaterialId>,
        environment: &mut EnvironmentSpec,
    ) {
        *environment = self.after.clone();
    }

    fn revert_lookdev(
        &self,
        _materials: &mut Vec<MaterialSpec>,
        _selected_material: &mut Option<MaterialId>,
        environment: &mut EnvironmentSpec,
    ) {
        *environment = self.before.clone();
    }
}

#[derive(Debug, Clone)]
pub struct ApplyLightingRigCommand {
    pub removed_lights: Vec<(usize, SceneNode)>,
    pub added_lights: Vec<SceneNode>,
    pub old_selected: Vec<NodeId>,
    pub new_selected: Vec<NodeId>,
    pub old_env: EnvironmentSpec,
    pub new_env: EnvironmentSpec,
}

impl Command for ApplyLightingRigCommand {
    fn apply(&self, nodes: &mut Vec<SceneNode>, selected: &mut Vec<NodeId>) {
        let rem_ids: Vec<NodeId> = self.removed_lights.iter().map(|(_, n)| n.id).collect();
        nodes.retain(|n| !rem_ids.contains(&n.id));
        nodes.extend(self.added_lights.clone());
        *selected = self.new_selected.clone();
    }

    fn revert(&self, nodes: &mut Vec<SceneNode>, selected: &mut Vec<NodeId>) {
        let add_ids: Vec<NodeId> = self.added_lights.iter().map(|n| n.id).collect();
        nodes.retain(|n| !add_ids.contains(&n.id));
        let mut sorted = self.removed_lights.clone();
        sorted.sort_by_key(|(idx, _)| *idx);
        for (idx, n) in sorted {
            let pos = idx.min(nodes.len());
            nodes.insert(pos, n);
        }
        *selected = self.old_selected.clone();
    }

    fn apply_lookdev(
        &self,
        _materials: &mut Vec<MaterialSpec>,
        _selected_material: &mut Option<MaterialId>,
        environment: &mut EnvironmentSpec,
    ) {
        *environment = self.new_env.clone();
    }

    fn revert_lookdev(
        &self,
        _materials: &mut Vec<MaterialSpec>,
        _selected_material: &mut Option<MaterialId>,
        environment: &mut EnvironmentSpec,
    ) {
        *environment = self.old_env.clone();
    }
}

#[derive(Debug, Clone)]
pub struct SetCameraNodeCommand {
    pub node_id: NodeId,
    pub old_cam: CameraNodeParams,
    pub new_cam: CameraNodeParams,
}

impl Command for SetCameraNodeCommand {
    fn apply(&self, nodes: &mut Vec<SceneNode>, _selected: &mut Vec<NodeId>) {
        if let Some(node) = nodes.iter_mut().find(|n| n.id == self.node_id) {
            node.camera_node = self.new_cam.clone();
        }
    }

    fn revert(&self, nodes: &mut Vec<SceneNode>, _selected: &mut Vec<NodeId>) {
        if let Some(node) = nodes.iter_mut().find(|n| n.id == self.node_id) {
            node.camera_node = self.old_cam.clone();
        }
    }
}

#[derive(Debug, Clone)]
pub struct SetAnimationStateCommand {
    pub old_anim: AnimationState,
    pub new_anim: AnimationState,
}

impl Command for SetAnimationStateCommand {
    fn apply(&self, _nodes: &mut Vec<SceneNode>, _selected: &mut Vec<NodeId>) {}
    fn revert(&self, _nodes: &mut Vec<SceneNode>, _selected: &mut Vec<NodeId>) {}

    fn apply_animation(&self, anim: &mut AnimationState) {
        *anim = self.new_anim.clone();
    }

    fn revert_animation(&self, anim: &mut AnimationState) {
        *anim = self.old_anim.clone();
    }
}

#[derive(Debug)]
pub struct CommandHistory {
    undo_stack: VecDeque<Box<dyn Command>>,
    redo_stack: Vec<Box<dyn Command>>,
    capacity: usize,
}

impl CommandHistory {
    pub fn new(capacity: usize) -> Self {
        Self {
            undo_stack: VecDeque::with_capacity(capacity.max(1)),
            redo_stack: Vec::new(),
            capacity: capacity.max(1),
        }
    }

    pub fn execute(
        &mut self,
        cmd: Box<dyn Command>,
        nodes: &mut Vec<SceneNode>,
        selected: &mut Vec<NodeId>,
    ) {
        cmd.apply(nodes, selected);
        self.push_executed(cmd);
    }

    pub fn execute_lookdev(
        &mut self,
        cmd: Box<dyn Command>,
        nodes: &mut Vec<SceneNode>,
        selected: &mut Vec<NodeId>,
        materials: &mut Vec<MaterialSpec>,
        selected_material: &mut Option<MaterialId>,
        environment: &mut EnvironmentSpec,
    ) {
        cmd.apply(nodes, selected);
        cmd.apply_lookdev(materials, selected_material, environment);
        self.push_executed(cmd);
    }

    pub fn execute_all(
        &mut self,
        cmd: Box<dyn Command>,
        nodes: &mut Vec<SceneNode>,
        selected: &mut Vec<NodeId>,
        materials: &mut Vec<MaterialSpec>,
        selected_material: &mut Option<MaterialId>,
        environment: &mut EnvironmentSpec,
        anim: &mut AnimationState,
    ) {
        cmd.apply(nodes, selected);
        cmd.apply_lookdev(materials, selected_material, environment);
        cmd.apply_animation(anim);
        self.push_executed(cmd);
    }

    /// Pushes an already-applied command (used for interactive drag gestures).
    pub fn push_executed(&mut self, cmd: Box<dyn Command>) {
        self.redo_stack.clear();
        if self.undo_stack.len() >= self.capacity {
            self.undo_stack.pop_front();
        }
        self.undo_stack.push_back(cmd);
    }

    pub fn undo(&mut self, nodes: &mut Vec<SceneNode>, selected: &mut Vec<NodeId>) -> bool {
        if let Some(cmd) = self.undo_stack.pop_back() {
            cmd.revert(nodes, selected);
            self.redo_stack.push(cmd);
            true
        } else {
            false
        }
    }

    pub fn undo_lookdev(
        &mut self,
        nodes: &mut Vec<SceneNode>,
        selected: &mut Vec<NodeId>,
        materials: &mut Vec<MaterialSpec>,
        selected_material: &mut Option<MaterialId>,
        environment: &mut EnvironmentSpec,
    ) -> bool {
        if let Some(cmd) = self.undo_stack.pop_back() {
            cmd.revert(nodes, selected);
            cmd.revert_lookdev(materials, selected_material, environment);
            self.redo_stack.push(cmd);
            true
        } else {
            false
        }
    }

    pub fn undo_all(
        &mut self,
        nodes: &mut Vec<SceneNode>,
        selected: &mut Vec<NodeId>,
        materials: &mut Vec<MaterialSpec>,
        selected_material: &mut Option<MaterialId>,
        environment: &mut EnvironmentSpec,
        anim: &mut AnimationState,
    ) -> bool {
        if let Some(cmd) = self.undo_stack.pop_back() {
            cmd.revert(nodes, selected);
            cmd.revert_lookdev(materials, selected_material, environment);
            cmd.revert_animation(anim);
            self.redo_stack.push(cmd);
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self, nodes: &mut Vec<SceneNode>, selected: &mut Vec<NodeId>) -> bool {
        if let Some(cmd) = self.redo_stack.pop() {
            cmd.apply(nodes, selected);
            self.undo_stack.push_back(cmd);
            true
        } else {
            false
        }
    }

    pub fn redo_lookdev(
        &mut self,
        nodes: &mut Vec<SceneNode>,
        selected: &mut Vec<NodeId>,
        materials: &mut Vec<MaterialSpec>,
        selected_material: &mut Option<MaterialId>,
        environment: &mut EnvironmentSpec,
    ) -> bool {
        if let Some(cmd) = self.redo_stack.pop() {
            cmd.apply(nodes, selected);
            cmd.apply_lookdev(materials, selected_material, environment);
            self.undo_stack.push_back(cmd);
            true
        } else {
            false
        }
    }

    pub fn redo_all(
        &mut self,
        nodes: &mut Vec<SceneNode>,
        selected: &mut Vec<NodeId>,
        materials: &mut Vec<MaterialSpec>,
        selected_material: &mut Option<MaterialId>,
        environment: &mut EnvironmentSpec,
        anim: &mut AnimationState,
    ) -> bool {
        if let Some(cmd) = self.redo_stack.pop() {
            cmd.apply(nodes, selected);
            cmd.apply_lookdev(materials, selected_material, environment);
            cmd.apply_animation(anim);
            self.undo_stack.push_back(cmd);
            true
        } else {
            false
        }
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}
