//! Narrow C-ABI FFI surface for Project NOVA 3D Rust core.
//! Every entry point validates pointers, catches panics, and records honest errors.

use std::ffi::{c_char, CStr};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Mutex;

use crate::animation::{AnimChannelKind, AnimTargetKind, InterpolationMode};
use crate::camera::{CameraSnapshot, ProjectionMode, StandardView};
use crate::error::{clear_last_error, set_last_error, take_last_error, NovaStatus};
use crate::io_formats::{export_gltf, export_obj, import_gltf_or_glb, import_obj};
use crate::lookdev::{
    AlphaMode, EnvironmentPreset, EnvironmentSpec, MaterialId, MaterialSpec, TextureId,
};
use crate::math::Vec3;
use crate::poly::{EditSelectMode, InteractionMode, ModifierSpec};
use crate::primitives::PrimitiveType;
use crate::scene::{
    EditOverlayLine, EditOverlayTri, GizmoAxis, GizmoMode, GizmoSpace, LightKind, LightParams,
    MaterialParams, NodeId, NovaEngineState, Transform,
};
use crate::serialize::{deserialize_project, serialize_project};

pub struct NovaCoreHandle {
    pub(crate) state: NovaEngineState,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NovaNodeMeta {
    pub id: u64,
    pub parent_id: u64,
    pub depth: u32,
    pub kind: u32,
    pub visible: bool,
    pub selected: bool,
    pub primary_selected: bool,
    pub has_mesh: bool,
    pub mesh_version: u64,
    pub model_matrix: [f32; 16],
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub emissive: [f32; 3],
    pub emissive_intensity: f32,
    pub ao_strength: f32,
    pub alpha_mode: u32,
    pub uv_scale: [f32; 2],
    pub uv_offset: [f32; 2],
    pub material_id: u64,
    pub albedo_tex_id: u64,
    pub normal_tex_id: u64,
    pub roughness_tex_id: u64,
    pub ao_tex_id: u64,
    pub emissive_tex_id: u64,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    pub light_kind: u32,
    pub light_color: [f32; 3],
    pub light_intensity: f32,
    pub light_cast_shadows: bool,
    pub light_spot_inner: f32,
    pub light_spot_outer: f32,
    pub light_range: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NovaNodeDetails {
    pub id: u64,
    pub parent_id: u64,
    pub kind: u32,
    pub visible: bool,
    pub selected: bool,
    pub primary_selected: bool,
    pub has_mesh: bool,
    pub position: [f32; 3],
    pub rotation_euler: [f32; 3],
    pub scale: [f32; 3],
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub material_id: u64,
    pub light_kind: u32,
    pub light_color: [f32; 3],
    pub light_intensity: f32,
    pub light_cast_shadows: bool,
    pub light_spot_inner: f32,
    pub light_spot_outer: f32,
    pub light_range: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NovaMaterialInfo {
    pub id: u64,
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub emissive: [f32; 3],
    pub emissive_intensity: f32,
    pub ao_strength: f32,
    pub alpha_mode: u32,
    pub albedo_tex_id: u64,
    pub normal_tex_id: u64,
    pub roughness_tex_id: u64,
    pub ao_tex_id: u64,
    pub emissive_tex_id: u64,
    pub uv_scale: [f32; 2],
    pub uv_offset: [f32; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NovaTextureMeta {
    pub id: u64,
    pub width: u32,
    pub height: u32,
    pub status: u32,
    pub version: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NovaEnvironmentInfo {
    pub preset: u32,
    pub rotation_deg: f32,
    pub intensity: f32,
    pub show_skybox: bool,
    pub bloom_enabled: bool,
    pub bloom_strength: f32,
    pub exposure: f32,
    pub version: u64,
    pub material_preview_sphere: bool,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NovaTimelineInfo {
    pub start_frame: i32,
    pub end_frame: i32,
    pub fps: f32,
    pub current_frame: f32,
    pub playing: bool,
    pub looping: bool,
    pub auto_key: bool,
    pub track_count: u32,
    pub active_camera_node_id: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NovaAnimTrackMeta {
    pub track_id: u64,
    pub target_kind: u32,
    pub target_id: u64,
    pub channel: u32,
    pub keyframe_count: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NovaKeyframeMeta {
    pub id: u64,
    pub frame: i32,
    pub value: f32,
    pub interpolation: u32,
    pub in_handle: [f32; 2],
    pub out_handle: [f32; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NovaGizmoSnapshot {
    pub visible: bool,
    pub mode: u32,
    pub space: u32,
    pub active_axis: u32,
    pub pivot: [f32; 3],
    pub axis_x: [f32; 3],
    pub axis_y: [f32; 3],
    pub axis_z: [f32; 3],
    pub scale: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NovaMeshView {
    pub positions_ptr: *const f32,
    pub normals_ptr: *const f32,
    pub uvs_ptr: *const f32,
    pub vertex_count: usize,
    pub indices_ptr: *const u32,
    pub index_count: usize,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NovaEditState {
    pub interaction_mode: u32,
    pub edit_node_id: u64,
    pub edit_select_mode: u32,
    pub show_face_orientation: bool,
    pub smooth_shading: bool,
    pub total_verts: u32,
    pub total_edges: u32,
    pub total_faces: u32,
    pub selected_vert_closure: u32,
    pub selected_verts: u32,
    pub selected_edges: u32,
    pub selected_faces: u32,
    pub has_centroid: bool,
    pub centroid_world: [f32; 3],
    pub modifier_count: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NovaModifierInfo {
    pub kind: u32,
    pub enabled: bool,
    pub axis: u32,
    pub merge: bool,
    pub levels: u32,
    pub thickness: f32,
    pub count: u32,
    pub offset: [f32; 3],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NovaEditOverlayView {
    pub lines_ptr: *const EditOverlayLine,
    pub line_count: usize,
    pub tris_ptr: *const EditOverlayTri,
    pub tri_count: usize,
}

fn guard_ffi<F>(f: F) -> i32
where
    F: FnOnce() -> Result<(), (NovaStatus, String)>,
{
    clear_last_error();
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(())) => NovaStatus::Ok as i32,
        Ok(Err((code, msg))) => {
            set_last_error(msg);
            code as i32
        }
        Err(_) => {
            set_last_error("Rust core panic captured at FFI boundary");
            NovaStatus::PanicCaptured as i32
        }
    }
}

unsafe fn ref_mut<'a>(
    handle: *mut NovaCoreHandle,
) -> Result<&'a mut NovaEngineState, (NovaStatus, String)> {
    if handle.is_null() {
        return Err((NovaStatus::InvalidArgument, "Null NovaCoreHandle".to_string()));
    }
    Ok(&mut (*handle).state)
}

unsafe fn ref_const<'a>(
    handle: *const NovaCoreHandle,
) -> Result<&'a NovaEngineState, (NovaStatus, String)> {
    if handle.is_null() {
        return Err((NovaStatus::InvalidArgument, "Null NovaCoreHandle".to_string()));
    }
    Ok(&(*handle).state)
}

#[no_mangle]
pub extern "C" fn nova_core_create() -> *mut NovaCoreHandle {
    clear_last_error();
    match catch_unwind(|| {
        Box::into_raw(Box::new(NovaCoreHandle {
            state: NovaEngineState::new_with_starter_scene(),
        }))
    }) {
        Ok(ptr) => ptr,
        Err(_) => {
            set_last_error("Panic during nova_core_create");
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_destroy(handle: *mut NovaCoreHandle) {
    if !handle.is_null() {
        let _ = catch_unwind(AssertUnwindSafe(|| {
            drop(Box::from_raw(handle));
        }));
    }
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_last_error(out_buf: *mut c_char, buf_len: usize) -> usize {
    let msg = take_last_error();
    let bytes = msg.as_bytes();
    if out_buf.is_null() || buf_len == 0 {
        return bytes.len();
    }
    let copy_len = bytes.len().min(buf_len - 1);
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_buf as *mut u8, copy_len);
    *out_buf.add(copy_len) = 0;
    copy_len
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_camera_orbit(
    handle: *mut NovaCoreHandle,
    delta_yaw_rad: f32,
    delta_pitch_rad: f32,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        state.camera.orbit(delta_yaw_rad, delta_pitch_rad);
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_camera_pan(
    handle: *mut NovaCoreHandle,
    norm_dx: f32,
    norm_dy: f32,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        state.camera.pan(norm_dx, norm_dy);
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_camera_zoom(handle: *mut NovaCoreHandle, factor: f32) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        state.camera.zoom_by_factor(factor);
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_camera_set_projection(
    handle: *mut NovaCoreHandle,
    projection_mode: u32,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        let mode = match projection_mode {
            0 => ProjectionMode::Perspective,
            1 => ProjectionMode::Orthographic,
            _ => {
                return Err((
                    NovaStatus::InvalidArgument,
                    format!("Unknown projection mode: {projection_mode}"),
                ))
            }
        };
        state.set_projection(mode);
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_camera_set_standard_view(
    handle: *mut NovaCoreHandle,
    standard_view: u32,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        let view = match standard_view {
            0 => StandardView::Home,
            1 => StandardView::Front,
            2 => StandardView::Back,
            3 => StandardView::Left,
            4 => StandardView::Right,
            5 => StandardView::Top,
            6 => StandardView::Bottom,
            _ => {
                return Err((
                    NovaStatus::InvalidArgument,
                    format!("Unknown standard view: {standard_view}"),
                ))
            }
        };
        state.set_standard_view(view);
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_camera_frame_all(handle: *mut NovaCoreHandle) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        state.frame_all();
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_camera_focus_selected(handle: *mut NovaCoreHandle) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        state.focus_selected();
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_camera_get_snapshot(
    handle: *const NovaCoreHandle,
    out_snapshot: *mut CameraSnapshot,
) -> i32 {
    guard_ffi(|| {
        if out_snapshot.is_null() {
            return Err((NovaStatus::InvalidArgument, "Null out_snapshot".to_string()));
        }
        let state = ref_const(handle)?;
        *out_snapshot = state.camera_snapshot();
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_scene_revision(handle: *const NovaCoreHandle) -> u64 {
    if handle.is_null() {
        return 0;
    }
    (*handle).state.scene_revision
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_scene_node_count(handle: *const NovaCoreHandle) -> usize {
    if handle.is_null() {
        return 0;
    }
    (*handle).state.nodes.len()
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_scene_get_node_meta(
    handle: *const NovaCoreHandle,
    index: usize,
    out_meta: *mut NovaNodeMeta,
) -> i32 {
    guard_ffi(|| {
        if out_meta.is_null() {
            return Err((NovaStatus::InvalidArgument, "Null out_meta".to_string()));
        }
        let state = ref_const(handle)?;
        let node = state.nodes.get(index).ok_or_else(|| {
            (
                NovaStatus::NotFound,
                format!("Node index {index} out of range (len {})", state.nodes.len()),
            )
        })?;
        let (bmin, bmax) = match &node.mesh {
            Some(m) => (m.bounds_min, m.bounds_max),
            None => (Vec3::new(-0.35, -0.35, -0.35), Vec3::new(0.35, 0.35, 0.35)),
        };
        let resolved_mat = state.resolved_node_material(node);
        *out_meta = NovaNodeMeta {
            id: node.id,
            parent_id: node.parent_id.unwrap_or(0),
            depth: state.node_depth(node.id) as u32,
            kind: node.kind as u32,
            visible: node.visible,
            selected: state.is_selected(node.id),
            primary_selected: state.primary_selected_id() == Some(node.id),
            has_mesh: node.mesh.is_some(),
            mesh_version: node.mesh.as_ref().map_or(0, |m| m.version),
            model_matrix: state.node_world_matrix(node.id).data,
            base_color: resolved_mat.base_color,
            metallic: resolved_mat.metallic,
            roughness: resolved_mat.roughness,
            emissive: resolved_mat.emissive,
            emissive_intensity: resolved_mat.emissive_intensity,
            ao_strength: resolved_mat.ao_strength,
            alpha_mode: resolved_mat.alpha_mode as u32,
            uv_scale: resolved_mat.uv_scale,
            uv_offset: resolved_mat.uv_offset,
            material_id: node.material_id.unwrap_or(0),
            albedo_tex_id: resolved_mat.albedo_tex_id.unwrap_or(0),
            normal_tex_id: resolved_mat.normal_tex_id.unwrap_or(0),
            roughness_tex_id: resolved_mat.roughness_tex_id.unwrap_or(0),
            ao_tex_id: resolved_mat.ao_tex_id.unwrap_or(0),
            emissive_tex_id: resolved_mat.emissive_tex_id.unwrap_or(0),
            bounds_min: [bmin.x, bmin.y, bmin.z],
            bounds_max: [bmax.x, bmax.y, bmax.z],
            light_kind: node.light.kind as u32,
            light_color: node.light.color,
            light_intensity: node.light.intensity,
            light_cast_shadows: node.light.cast_shadows,
            light_spot_inner: node.light.spot_inner_deg,
            light_spot_outer: node.light.spot_outer_deg,
            light_range: node.light.range,
        };
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_scene_get_node_details(
    handle: *const NovaCoreHandle,
    node_id: NodeId,
    out_details: *mut NovaNodeDetails,
) -> i32 {
    guard_ffi(|| {
        if out_details.is_null() {
            return Err((NovaStatus::InvalidArgument, "Null out_details".to_string()));
        }
        let state = ref_const(handle)?;
        let node = state.find_node(node_id).ok_or_else(|| {
            (NovaStatus::NotFound, format!("Node id {node_id} not found"))
        })?;
        let resolved_mat = state.resolved_node_material(node);
        *out_details = NovaNodeDetails {
            id: node.id,
            parent_id: node.parent_id.unwrap_or(0),
            kind: node.kind as u32,
            visible: node.visible,
            selected: state.is_selected(node.id),
            primary_selected: state.primary_selected_id() == Some(node.id),
            has_mesh: node.mesh.is_some(),
            position: [node.transform.position.x, node.transform.position.y, node.transform.position.z],
            rotation_euler: [
                node.transform.rotation_euler.x,
                node.transform.rotation_euler.y,
                node.transform.rotation_euler.z,
            ],
            scale: [node.transform.scale.x, node.transform.scale.y, node.transform.scale.z],
            base_color: resolved_mat.base_color,
            metallic: resolved_mat.metallic,
            roughness: resolved_mat.roughness,
            material_id: node.material_id.unwrap_or(0),
            light_kind: node.light.kind as u32,
            light_color: node.light.color,
            light_intensity: node.light.intensity,
            light_cast_shadows: node.light.cast_shadows,
            light_spot_inner: node.light.spot_inner_deg,
            light_spot_outer: node.light.spot_outer_deg,
            light_range: node.light.range,
        };
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_scene_get_node_name(
    handle: *const NovaCoreHandle,
    node_id: NodeId,
    out_buf: *mut c_char,
    buf_len: usize,
) -> i32 {
    guard_ffi(|| {
        let state = ref_const(handle)?;
        let node = state.find_node(node_id).ok_or_else(|| {
            (NovaStatus::NotFound, format!("Node id {node_id} not found"))
        })?;
        if out_buf.is_null() || buf_len == 0 {
            return Err((NovaStatus::InvalidArgument, "Invalid output buffer".to_string()));
        }
        let bytes = node.name.as_bytes();
        let copy_len = bytes.len().min(buf_len - 1);
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_buf as *mut u8, copy_len);
        *out_buf.add(copy_len) = 0;
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_scene_get_node_mesh(
    handle: *const NovaCoreHandle,
    node_id: NodeId,
    out_mesh: *mut NovaMeshView,
) -> i32 {
    guard_ffi(|| {
        if out_mesh.is_null() {
            return Err((NovaStatus::InvalidArgument, "Null out_mesh".to_string()));
        }
        let state = ref_const(handle)?;
        let node = state.find_node(node_id).ok_or_else(|| {
            (NovaStatus::NotFound, format!("Node id {node_id} not found"))
        })?;
        let mesh = node.mesh.as_ref().ok_or_else(|| {
            (NovaStatus::NotFound, format!("Node id {node_id} has no mesh"))
        })?;
        *out_mesh = NovaMeshView {
            positions_ptr: mesh.positions.as_ptr() as *const f32,
            normals_ptr: mesh.normals.as_ptr() as *const f32,
            uvs_ptr: mesh.uvs.as_ptr() as *const f32,
            vertex_count: mesh.positions.len(),
            indices_ptr: mesh.indices.as_ptr(),
            index_count: mesh.indices.len(),
            bounds_min: [mesh.bounds_min.x, mesh.bounds_min.y, mesh.bounds_min.z],
            bounds_max: [mesh.bounds_max.x, mesh.bounds_max.y, mesh.bounds_max.z],
        };
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_scene_select_node(
    handle: *mut NovaCoreHandle,
    node_id: u64,
) -> i32 {
    nova_core_scene_select_node_ext(handle, node_id, false)
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_scene_select_node_ext(
    handle: *mut NovaCoreHandle,
    node_id: u64,
    additive: bool,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        let target = if node_id == 0 { None } else { Some(node_id) };
        if state.select_node(target, additive) {
            Ok(())
        } else {
            Err((NovaStatus::NotFound, format!("Node id {node_id} not found")))
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_scene_pick_at_ndc(
    handle: *mut NovaCoreHandle,
    ndc_x: f32,
    ndc_y: f32,
    aspect: f32,
    additive: bool,
    out_id: *mut u64,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        let hit = state.pick_node_at_ndc(ndc_x, ndc_y, aspect, additive);
        if !out_id.is_null() {
            *out_id = hit.unwrap_or(0);
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_scene_set_grid_axes(
    handle: *mut NovaCoreHandle,
    show_grid: bool,
    show_axes: bool,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        state.show_grid = show_grid;
        state.show_axes = show_axes;
        state.scene_revision += 1;
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_scene_get_grid_axes(
    handle: *const NovaCoreHandle,
    out_show_grid: *mut bool,
    out_show_axes: *mut bool,
) -> i32 {
    guard_ffi(|| {
        let state = ref_const(handle)?;
        if !out_show_grid.is_null() {
            *out_show_grid = state.show_grid;
        }
        if !out_show_axes.is_null() {
            *out_show_axes = state.show_axes;
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_set_shading_mode(
    handle: *mut NovaCoreHandle,
    shading_mode: u32,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        state.shading_mode = shading_mode.min(3);
        state.scene_revision += 1;
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_get_shading_mode(handle: *const NovaCoreHandle) -> u32 {
    if handle.is_null() {
        return 0;
    }
    (*handle).state.shading_mode
}

fn replace_scene_state(state: &mut NovaEngineState, mut next: NovaEngineState) {
    let next_rev = state.scene_revision.wrapping_add(1).max(1);
    let max_mesh_ver = state
        .nodes
        .iter()
        .filter_map(|n| n.mesh.as_ref().map(|m| m.version))
        .max()
        .unwrap_or(0)
        .max(state.scene_revision);
    next.scene_revision = next_rev;
    next.environment_version = state.environment_version.wrapping_add(1).max(1);
    for node in &mut next.nodes {
        if let Some(mesh) = &mut node.mesh {
            mesh.version = mesh.version.wrapping_add(max_mesh_ver).max(max_mesh_ver + 1);
        }
    }
    next.rebuild_edit_overlay();
    *state = next;
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_scene_reset_empty(handle: *mut NovaCoreHandle) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        replace_scene_state(state, NovaEngineState::new_empty());
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_scene_reset_starter(handle: *mut NovaCoreHandle) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        replace_scene_state(state, NovaEngineState::new_with_starter_scene());
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_scene_add_primitive(
    handle: *mut NovaCoreHandle,
    primitive_type: u32,
    out_id: *mut u64,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        let prim = PrimitiveType::from_u32(primitive_type).ok_or_else(|| {
            (
                NovaStatus::InvalidArgument,
                format!("Unknown primitive type: {primitive_type}"),
            )
        })?;
        let id = state.add_primitive(prim);
        if !out_id.is_null() {
            *out_id = id;
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_scene_duplicate_selected(handle: *mut NovaCoreHandle) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        if state.duplicate_selected().is_empty() {
            return Err((NovaStatus::NotFound, "No selected object to duplicate".to_string()));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_scene_delete_selected(handle: *mut NovaCoreHandle) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        if !state.delete_selected() {
            return Err((NovaStatus::NotFound, "No selected object to delete".to_string()));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_scene_rename_node(
    handle: *mut NovaCoreHandle,
    node_id: u64,
    name_utf8: *const c_char,
) -> i32 {
    guard_ffi(|| {
        if name_utf8.is_null() {
            return Err((NovaStatus::InvalidArgument, "Null name string".to_string()));
        }
        let state = ref_mut(handle)?;
        let cstr = CStr::from_ptr(name_utf8);
        let name = cstr
            .to_str()
            .map_err(|_| (NovaStatus::InvalidArgument, "Invalid UTF-8 name".to_string()))?;
        if !state.rename_node(node_id, name.to_string()) {
            return Err((
                NovaStatus::InvalidArgument,
                format!("Unable to rename node {node_id}"),
            ));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_scene_reparent_node(
    handle: *mut NovaCoreHandle,
    child_id: u64,
    new_parent_id: u64,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        let parent = if new_parent_id == 0 {
            None
        } else {
            Some(new_parent_id)
        };
        if !state.reparent_node(child_id, parent) {
            return Err((
                NovaStatus::InvalidArgument,
                "Reparent rejected (missing node or hierarchy cycle)".to_string(),
            ));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_scene_group_selected(
    handle: *mut NovaCoreHandle,
    out_group_id: *mut u64,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        let gid = state
            .group_selected()
            .ok_or_else(|| (NovaStatus::NotFound, "Select at least one object to group".to_string()))?;
        if !out_group_id.is_null() {
            *out_group_id = gid;
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_scene_unparent_selected(handle: *mut NovaCoreHandle) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        if !state.unparent_selected() {
            return Err((
                NovaStatus::InvalidArgument,
                "Selected object has no parent to unparent".to_string(),
            ));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_scene_apply_transform_selected(
    handle: *mut NovaCoreHandle,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        if !state.apply_transform_selected() {
            return Err((
                NovaStatus::InvalidArgument,
                "Apply transform requires a selected mesh object".to_string(),
            ));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_scene_origin_to_geometry_selected(
    handle: *mut NovaCoreHandle,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        if !state.origin_to_geometry_selected() {
            return Err((
                NovaStatus::InvalidArgument,
                "Origin edit requires a selected mesh object".to_string(),
            ));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_scene_set_visibility(
    handle: *mut NovaCoreHandle,
    node_id: u64,
    visible: bool,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        if !state.set_node_visibility(node_id, visible) {
            return Err((NovaStatus::NotFound, format!("Node id {node_id} not found")));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_scene_set_transform(
    handle: *mut NovaCoreHandle,
    node_id: u64,
    position: *const f32,
    rotation_euler: *const f32,
    scale: *const f32,
) -> i32 {
    guard_ffi(|| {
        if position.is_null() || rotation_euler.is_null() || scale.is_null() {
            return Err((NovaStatus::InvalidArgument, "Null transform array".to_string()));
        }
        let state = ref_mut(handle)?;
        let pos = std::slice::from_raw_parts(position, 3);
        let rot = std::slice::from_raw_parts(rotation_euler, 3);
        let scl = std::slice::from_raw_parts(scale, 3);
        let tr = Transform {
            position: Vec3::new(pos[0], pos[1], pos[2]),
            rotation_euler: Vec3::new(rot[0], rot[1], rot[2]),
            scale: Vec3::new(scl[0], scl[1], scl[2]),
        };
        if !state.set_node_transform(node_id, tr) {
            return Err((NovaStatus::NotFound, format!("Node id {node_id} not found")));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_scene_set_material(
    handle: *mut NovaCoreHandle,
    node_id: u64,
    base_color: *const f32,
    metallic: f32,
    roughness: f32,
) -> i32 {
    guard_ffi(|| {
        if base_color.is_null() {
            return Err((NovaStatus::InvalidArgument, "Null base_color".to_string()));
        }
        let state = ref_mut(handle)?;
        let c = std::slice::from_raw_parts(base_color, 4);
        let mat = MaterialParams {
            base_color: [c[0], c[1], c[2], c[3]],
            metallic,
            roughness,
        };
        if !state.set_node_material(node_id, mat) {
            return Err((NovaStatus::NotFound, format!("Node id {node_id} not found")));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_scene_set_light(
    handle: *mut NovaCoreHandle,
    node_id: u64,
    kind: u32,
    color: *const f32,
    intensity: f32,
    cast_shadows: bool,
) -> i32 {
    guard_ffi(|| {
        if color.is_null() {
            return Err((NovaStatus::InvalidArgument, "Null light color".to_string()));
        }
        let state = ref_mut(handle)?;
        let c = std::slice::from_raw_parts(color, 3);
        let existing = state
            .find_node(node_id)
            .map(|n| n.light.clone())
            .unwrap_or_default();
        let light = LightParams {
            kind: match kind {
                0 => LightKind::Directional,
                2 => LightKind::Spot,
                _ => LightKind::Point,
            },
            color: [c[0].clamp(0.0, 1.0), c[1].clamp(0.0, 1.0), c[2].clamp(0.0, 1.0)],
            intensity: intensity.max(0.0),
            cast_shadows,
            spot_inner_deg: existing.spot_inner_deg,
            spot_outer_deg: existing.spot_outer_deg,
            range: existing.range,
        };
        if !state.set_node_light(node_id, light) {
            return Err((NovaStatus::NotFound, format!("Node id {node_id} not found")));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_scene_set_light_ex(
    handle: *mut NovaCoreHandle,
    node_id: u64,
    kind: u32,
    color: *const f32,
    intensity: f32,
    cast_shadows: bool,
    spot_inner_deg: f32,
    spot_outer_deg: f32,
    range: f32,
) -> i32 {
    guard_ffi(|| {
        if color.is_null() {
            return Err((NovaStatus::InvalidArgument, "Null light color".to_string()));
        }
        let state = ref_mut(handle)?;
        let c = std::slice::from_raw_parts(color, 3);
        let inner = spot_inner_deg.clamp(1.0, 88.0);
        let outer = spot_outer_deg.clamp(inner + 1.0, 89.5);
        let light = LightParams {
            kind: match kind {
                0 => LightKind::Directional,
                2 => LightKind::Spot,
                _ => LightKind::Point,
            },
            color: [c[0].clamp(0.0, 1.0), c[1].clamp(0.0, 1.0), c[2].clamp(0.0, 1.0)],
            intensity: intensity.max(0.0),
            cast_shadows,
            spot_inner_deg: inner,
            spot_outer_deg: outer,
            range: range.clamp(0.5, 200.0),
        };
        if !state.set_node_light(node_id, light) {
            return Err((NovaStatus::NotFound, format!("Node id {node_id} not found")));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_gizmo_set_mode(
    handle: *mut NovaCoreHandle,
    mode: u32,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        state.gizmo.mode = match mode {
            0 => GizmoMode::None,
            1 => GizmoMode::Move,
            2 => GizmoMode::Rotate,
            3 => GizmoMode::Scale,
            _ => {
                return Err((
                    NovaStatus::InvalidArgument,
                    format!("Unknown gizmo mode: {mode}"),
                ))
            }
        };
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_gizmo_set_space(
    handle: *mut NovaCoreHandle,
    space: u32,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        state.gizmo.space = if space == 1 {
            GizmoSpace::Local
        } else {
            GizmoSpace::World
        };
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_gizmo_get_snapshot(
    handle: *const NovaCoreHandle,
    out_snap: *mut NovaGizmoSnapshot,
) -> i32 {
    guard_ffi(|| {
        if out_snap.is_null() {
            return Err((NovaStatus::InvalidArgument, "Null out_snap".to_string()));
        }
        let state = ref_const(handle)?;
        if let Some((pivot, axes, scale)) = state.gizmo_world_pose() {
            *out_snap = NovaGizmoSnapshot {
                visible: true,
                mode: state.gizmo.mode as u32,
                space: state.gizmo.space as u32,
                active_axis: state.gizmo.active_axis as u32,
                pivot: [pivot.x, pivot.y, pivot.z],
                axis_x: [axes[0].x, axes[0].y, axes[0].z],
                axis_y: [axes[1].x, axes[1].y, axes[1].z],
                axis_z: [axes[2].x, axes[2].y, axes[2].z],
                scale,
            };
        } else {
            *out_snap = NovaGizmoSnapshot {
                visible: false,
                mode: state.gizmo.mode as u32,
                space: state.gizmo.space as u32,
                active_axis: state.gizmo.active_axis as u32,
                pivot: [0.0; 3],
                axis_x: [1.0, 0.0, 0.0],
                axis_y: [0.0, 1.0, 0.0],
                axis_z: [0.0, 0.0, 1.0],
                scale: 1.0,
            };
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_gizmo_pick_axis(
    handle: *const NovaCoreHandle,
    ndc_x: f32,
    ndc_y: f32,
    aspect: f32,
) -> u32 {
    if handle.is_null() {
        return 0;
    }
    (*handle).state.pick_gizmo_axis(ndc_x, ndc_y, aspect) as u32
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_gizmo_begin_drag(
    handle: *mut NovaCoreHandle,
    axis: u32,
    ndc_x: f32,
    ndc_y: f32,
    aspect: f32,
) -> bool {
    if handle.is_null() {
        return false;
    }
    let ga = match axis {
        1 => GizmoAxis::X,
        2 => GizmoAxis::Y,
        3 => GizmoAxis::Z,
        4 => GizmoAxis::Center,
        _ => return false,
    };
    (*handle).state.begin_gizmo_drag(ga, ndc_x, ndc_y, aspect)
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_gizmo_update_drag(
    handle: *mut NovaCoreHandle,
    ndc_x: f32,
    ndc_y: f32,
    aspect: f32,
) -> bool {
    if handle.is_null() {
        return false;
    }
    (*handle).state.update_gizmo_drag(ndc_x, ndc_y, aspect)
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_gizmo_end_drag(handle: *mut NovaCoreHandle) -> bool {
    if handle.is_null() {
        return false;
    }
    (*handle).state.end_gizmo_drag()
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_undo(handle: *mut NovaCoreHandle) -> bool {
    if handle.is_null() {
        return false;
    }
    (*handle).state.undo()
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_redo(handle: *mut NovaCoreHandle) -> bool {
    if handle.is_null() {
        return false;
    }
    (*handle).state.redo()
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_can_undo(handle: *const NovaCoreHandle) -> bool {
    if handle.is_null() {
        return false;
    }
    (*handle).state.history.can_undo()
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_can_redo(handle: *const NovaCoreHandle) -> bool {
    if handle.is_null() {
        return false;
    }
    (*handle).state.history.can_redo()
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_project_serialize(
    handle: *const NovaCoreHandle,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    guard_ffi(|| {
        if out_ptr.is_null() || out_len.is_null() {
            return Err((NovaStatus::InvalidArgument, "Null output pointer".to_string()));
        }
        let state = ref_const(handle)?;
        let json = serialize_project(state);
        let mut bytes = json.into_bytes().into_boxed_slice();
        let len = bytes.len();
        let ptr = bytes.as_mut_ptr();
        std::mem::forget(bytes);
        *out_ptr = ptr;
        *out_len = len;
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_free_buffer(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len > 0 {
        let slice = std::ptr::slice_from_raw_parts_mut(ptr, len);
        drop(Box::from_raw(slice));
    }
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_project_deserialize(
    handle: *mut NovaCoreHandle,
    json_ptr: *const u8,
    json_len: usize,
) -> i32 {
    guard_ffi(|| {
        if json_ptr.is_null() || json_len == 0 {
            return Err((NovaStatus::InvalidArgument, "Empty project payload".to_string()));
        }
        let state = ref_mut(handle)?;
        let bytes = std::slice::from_raw_parts(json_ptr, json_len);
        let s = std::str::from_utf8(bytes)
            .map_err(|_| (NovaStatus::InvalidArgument, "Project payload is not valid UTF-8".to_string()))?;
        let restored = deserialize_project(s)
            .map_err(|e| (NovaStatus::InvalidArgument, e))?;
        replace_scene_state(state, restored);
        Ok(())
    })
}

// -----------------------------------------------------------------------------
// M3: Edit Mode, Modeling Ops, Overlay & Modifier Stack FFI
// -----------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn nova_core_set_interaction_mode(
    handle: *mut NovaCoreHandle,
    mode: u32,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        let target = match mode {
            0 => InteractionMode::Object,
            1 => InteractionMode::Edit,
            _ => {
                return Err((
                    NovaStatus::InvalidArgument,
                    format!("Unknown interaction mode {mode}"),
                ))
            }
        };
        if !state.set_interaction_mode(target) {
            return Err((
                NovaStatus::InvalidArgument,
                "Select a mesh object before entering Edit Mode".to_string(),
            ));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_get_interaction_mode(handle: *const NovaCoreHandle) -> u32 {
    catch_unwind(AssertUnwindSafe(|| {
        if let Ok(state) = ref_const(handle) {
            state.interaction_mode as u32
        } else {
            0
        }
    }))
    .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_edit_set_select_mode(
    handle: *mut NovaCoreHandle,
    select_mode: u32,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        let mode = match select_mode {
            0 => EditSelectMode::Vertex,
            1 => EditSelectMode::Edge,
            2 => EditSelectMode::Face,
            _ => {
                return Err((
                    NovaStatus::InvalidArgument,
                    format!("Unknown edit select mode {select_mode}"),
                ))
            }
        };
        state.set_edit_select_mode(mode);
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_edit_get_state(
    handle: *const NovaCoreHandle,
    out_state: *mut NovaEditState,
) -> i32 {
    guard_ffi(|| {
        if out_state.is_null() {
            return Err((NovaStatus::InvalidArgument, "Null out_state pointer".to_string()));
        }
        let state = ref_const(handle)?;
        let active_id = state.active_mesh_node_id();
        let active_node = active_id.and_then(|id| state.find_node(id));
        let poly_opt = active_node.and_then(|n| n.base_poly.as_ref());
        let (total_v, total_e, total_f, smooth) = if let Some(p) = poly_opt {
            (
                p.vertices.len() as u32,
                p.unique_edges().len() as u32,
                p.faces.len() as u32,
                p.smooth_shading,
            )
        } else {
            (0, 0, 0, false)
        };
        let (closure_cnt, sv_cnt, se_cnt, sf_cnt) = state.edit_selection_counts();
        let centroid = state.edit_selection_centroid_world();
        let mod_cnt = active_node.map_or(0, |n| n.modifiers.len() as u32);
        *out_state = NovaEditState {
            interaction_mode: state.interaction_mode as u32,
            edit_node_id: state.edit_node_id.unwrap_or(0),
            edit_select_mode: state.edit_select_mode as u32,
            show_face_orientation: state.show_face_orientation,
            smooth_shading: smooth,
            total_verts: total_v,
            total_edges: total_e,
            total_faces: total_f,
            selected_vert_closure: closure_cnt as u32,
            selected_verts: sv_cnt as u32,
            selected_edges: se_cnt as u32,
            selected_faces: sf_cnt as u32,
            has_centroid: centroid.is_some(),
            centroid_world: centroid.map_or([0.0; 3], |c| c.to_array()),
            modifier_count: mod_cnt,
        };
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_edit_pick_at_ndc(
    handle: *mut NovaCoreHandle,
    ndc_x: f32,
    ndc_y: f32,
    aspect: f32,
    additive: bool,
    out_hit: *mut bool,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        let hit = state.edit_pick_at_ndc(ndc_x, ndc_y, aspect, additive);
        if !out_hit.is_null() {
            *out_hit = hit;
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_edit_clear_or_select_all(handle: *mut NovaCoreHandle) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        state.edit_clear_or_select_all();
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_edit_set_face_orientation(
    handle: *mut NovaCoreHandle,
    enabled: bool,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        state.set_face_orientation(enabled);
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_edit_extrude(
    handle: *mut NovaCoreHandle,
    distance: f32,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        if !state.edit_extrude(distance) {
            return Err((NovaStatus::InvalidArgument, "Select geometry to extrude".to_string()));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_edit_inset(handle: *mut NovaCoreHandle, factor: f32) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        if !state.edit_inset(factor) {
            return Err((NovaStatus::InvalidArgument, "Select face(s) to inset".to_string()));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_edit_bevel(handle: *mut NovaCoreHandle, width: f32) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        if !state.edit_bevel(width) {
            return Err((NovaStatus::InvalidArgument, "Select edge(s), vert(s), or face(s) to bevel".to_string()));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_edit_loop_cut(handle: *mut NovaCoreHandle) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        if !state.edit_loop_cut() {
            return Err((NovaStatus::InvalidArgument, "Select edge or quad ring for loop cut".to_string()));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_edit_subdivide(handle: *mut NovaCoreHandle) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        if !state.edit_subdivide() {
            return Err((NovaStatus::InvalidArgument, "No editable mesh faces found".to_string()));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_edit_merge(handle: *mut NovaCoreHandle) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        if !state.edit_merge_selected() {
            return Err((NovaStatus::InvalidArgument, "Select at least 2 vertices to merge".to_string()));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_edit_delete(handle: *mut NovaCoreHandle) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        if !state.edit_delete_selected() {
            return Err((NovaStatus::InvalidArgument, "Select vertices, edges, or faces to delete".to_string()));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_edit_dissolve_edges(handle: *mut NovaCoreHandle) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        if !state.edit_dissolve_edges() {
            return Err((NovaStatus::InvalidArgument, "Select interior edge(s) shared by two faces to dissolve".to_string()));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_edit_recalculate_normals(
    handle: *mut NovaCoreHandle,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        if !state.edit_recalculate_normals() {
            return Err((NovaStatus::InvalidArgument, "Cannot recalculate normals".to_string()));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_edit_flip_normals(handle: *mut NovaCoreHandle) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        if !state.edit_flip_normals() {
            return Err((NovaStatus::InvalidArgument, "Cannot flip normals".to_string()));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_edit_set_smooth_shading(
    handle: *mut NovaCoreHandle,
    smooth: bool,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        if !state.edit_set_smooth_shading(smooth) {
            return Err((NovaStatus::InvalidArgument, "Select a mesh node to change shading".to_string()));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_edit_translate_selected(
    handle: *mut NovaCoreHandle,
    dx: f32,
    dy: f32,
    dz: f32,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        if !state.edit_translate_selected(Vec3::new(dx, dy, dz)) {
            return Err((NovaStatus::InvalidArgument, "Select elements in Edit Mode to move".to_string()));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_edit_get_overlay(
    handle: *const NovaCoreHandle,
    out_overlay: *mut NovaEditOverlayView,
) -> i32 {
    guard_ffi(|| {
        if out_overlay.is_null() {
            return Err((NovaStatus::InvalidArgument, "Null out_overlay pointer".to_string()));
        }
        let state = ref_const(handle)?;
        *out_overlay = NovaEditOverlayView {
            lines_ptr: if state.edit_overlay_lines.is_empty() {
                std::ptr::null()
            } else {
                state.edit_overlay_lines.as_ptr()
            },
            line_count: state.edit_overlay_lines.len(),
            tris_ptr: if state.edit_overlay_tris.is_empty() {
                std::ptr::null()
            } else {
                state.edit_overlay_tris.as_ptr()
            },
            tri_count: state.edit_overlay_tris.len(),
        };
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_modifier_count(handle: *const NovaCoreHandle) -> usize {
    catch_unwind(AssertUnwindSafe(|| {
        if let Ok(state) = ref_const(handle) {
            state
                .active_mesh_node_id()
                .and_then(|id| state.find_node(id))
                .map_or(0, |n| n.modifiers.len())
        } else {
            0
        }
    }))
    .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_modifier_get(
    handle: *const NovaCoreHandle,
    index: usize,
    out_mod: *mut NovaModifierInfo,
) -> i32 {
    guard_ffi(|| {
        if out_mod.is_null() {
            return Err((NovaStatus::InvalidArgument, "Null out_mod pointer".to_string()));
        }
        let state = ref_const(handle)?;
        let nid = state
            .active_mesh_node_id()
            .ok_or((NovaStatus::NotFound, "No active mesh node".to_string()))?;
        let node = state
            .find_node(nid)
            .ok_or((NovaStatus::NotFound, "Active mesh node not found".to_string()))?;
        let spec = node
            .modifiers
            .get(index)
            .ok_or((NovaStatus::NotFound, "Modifier index out of bounds".to_string()))?;
        *out_mod = match *spec {
            ModifierSpec::Mirror { axis, merge, enabled } => NovaModifierInfo {
                kind: 0,
                enabled,
                axis: axis as u32,
                merge,
                levels: 1,
                thickness: 0.05,
                count: 2,
                offset: [1.5, 0.0, 0.0],
            },
            ModifierSpec::Subdivision { levels, enabled } => NovaModifierInfo {
                kind: 1,
                enabled,
                axis: 0,
                merge: true,
                levels: levels as u32,
                thickness: 0.05,
                count: 2,
                offset: [1.5, 0.0, 0.0],
            },
            ModifierSpec::Solidify { thickness, enabled } => NovaModifierInfo {
                kind: 2,
                enabled,
                axis: 0,
                merge: true,
                levels: 1,
                thickness,
                count: 2,
                offset: [1.5, 0.0, 0.0],
            },
            ModifierSpec::Array { count, offset, enabled } => NovaModifierInfo {
                kind: 3,
                enabled,
                axis: 0,
                merge: true,
                levels: 1,
                thickness: 0.05,
                count,
                offset,
            },
        };
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_modifier_add(
    handle: *mut NovaCoreHandle,
    kind: u32,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        let spec = match kind {
            0 => ModifierSpec::Mirror {
                axis: 0,
                merge: true,
                enabled: true,
            },
            1 => ModifierSpec::Subdivision {
                levels: 1,
                enabled: true,
            },
            2 => ModifierSpec::Solidify {
                thickness: 0.08,
                enabled: true,
            },
            3 => ModifierSpec::Array {
                count: 3,
                offset: [1.6, 0.0, 0.0],
                enabled: true,
            },
            _ => {
                return Err((
                    NovaStatus::InvalidArgument,
                    format!("Unknown modifier kind {kind}"),
                ))
            }
        };
        if !state.add_modifier(spec) {
            return Err((NovaStatus::InvalidArgument, "Select a mesh node first".to_string()));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_modifier_remove(
    handle: *mut NovaCoreHandle,
    index: usize,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        if !state.remove_modifier(index) {
            return Err((NovaStatus::NotFound, "Modifier index invalid".to_string()));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_modifier_move(
    handle: *mut NovaCoreHandle,
    index: usize,
    delta: i32,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        if !state.move_modifier(index, delta) {
            return Err((NovaStatus::InvalidArgument, "Cannot move modifier".to_string()));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_modifier_update(
    handle: *mut NovaCoreHandle,
    index: usize,
    mod_info: *const NovaModifierInfo,
) -> i32 {
    guard_ffi(|| {
        if mod_info.is_null() {
            return Err((NovaStatus::InvalidArgument, "Null mod_info pointer".to_string()));
        }
        let state = ref_mut(handle)?;
        let info = &*mod_info;
        let spec = match info.kind {
            0 => ModifierSpec::Mirror {
                axis: (info.axis.min(2)) as u8,
                merge: info.merge,
                enabled: info.enabled,
            },
            1 => ModifierSpec::Subdivision {
                levels: (info.levels.clamp(1, 3)) as u8,
                enabled: info.enabled,
            },
            2 => ModifierSpec::Solidify {
                thickness: info.thickness.clamp(-2.0, 2.0),
                enabled: info.enabled,
            },
            3 => ModifierSpec::Array {
                count: info.count.clamp(1, 16),
                offset: info.offset,
                enabled: info.enabled,
            },
            k => {
                return Err((
                    NovaStatus::InvalidArgument,
                    format!("Unknown modifier kind {k}"),
                ))
            }
        };
        if !state.update_modifier(index, spec) {
            return Err((NovaStatus::NotFound, "Modifier index invalid".to_string()));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_modifier_apply_all(handle: *mut NovaCoreHandle) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        if !state.apply_modifier_stack() {
            return Err((NovaStatus::InvalidArgument, "No modifiers to apply".to_string()));
        }
        Ok(())
    })
}

fn mat_to_ffi(m: &MaterialSpec) -> NovaMaterialInfo {
    NovaMaterialInfo {
        id: m.id,
        base_color: m.base_color,
        metallic: m.metallic,
        roughness: m.roughness,
        emissive: m.emissive,
        emissive_intensity: m.emissive_intensity,
        ao_strength: m.ao_strength,
        alpha_mode: m.alpha_mode as u32,
        albedo_tex_id: m.albedo_tex_id.unwrap_or(0),
        normal_tex_id: m.normal_tex_id.unwrap_or(0),
        roughness_tex_id: m.roughness_tex_id.unwrap_or(0),
        ao_tex_id: m.ao_tex_id.unwrap_or(0),
        emissive_tex_id: m.emissive_tex_id.unwrap_or(0),
        uv_scale: m.uv_scale,
        uv_offset: m.uv_offset,
    }
}

fn copy_str_to_c_buf(text: &str, out_buf: *mut c_char, buf_len: usize) -> Result<(), (NovaStatus, String)> {
    if out_buf.is_null() || buf_len == 0 {
        return Err((NovaStatus::InvalidArgument, "Invalid output buffer".to_string()));
    }
    let bytes = text.as_bytes();
    let copy_len = bytes.len().min(buf_len - 1);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_buf as *mut u8, copy_len);
        *out_buf.add(copy_len) = 0;
    }
    Ok(())
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_material_count(handle: *const NovaCoreHandle) -> usize {
    if handle.is_null() {
        return 0;
    }
    (*handle).state.materials.len()
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_material_get_by_index(
    handle: *const NovaCoreHandle,
    index: usize,
    out_info: *mut NovaMaterialInfo,
) -> i32 {
    guard_ffi(|| {
        if out_info.is_null() {
            return Err((NovaStatus::InvalidArgument, "Null out_info".to_string()));
        }
        let state = ref_const(handle)?;
        let mat = state.materials.get(index).ok_or_else(|| {
            (NovaStatus::NotFound, format!("Material index {index} out of range"))
        })?;
        *out_info = mat_to_ffi(mat);
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_material_get(
    handle: *const NovaCoreHandle,
    mat_id: MaterialId,
    out_info: *mut NovaMaterialInfo,
) -> i32 {
    guard_ffi(|| {
        if out_info.is_null() {
            return Err((NovaStatus::InvalidArgument, "Null out_info".to_string()));
        }
        let state = ref_const(handle)?;
        let mat = state.find_material(mat_id).ok_or_else(|| {
            (NovaStatus::NotFound, format!("Material {mat_id} not found"))
        })?;
        *out_info = mat_to_ffi(mat);
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_material_get_name(
    handle: *const NovaCoreHandle,
    mat_id: MaterialId,
    out_buf: *mut c_char,
    buf_len: usize,
) -> i32 {
    guard_ffi(|| {
        let state = ref_const(handle)?;
        let mat = state.find_material(mat_id).ok_or_else(|| {
            (NovaStatus::NotFound, format!("Material {mat_id} not found"))
        })?;
        copy_str_to_c_buf(&mat.name, out_buf, buf_len)
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_material_selected_id(handle: *const NovaCoreHandle) -> u64 {
    if handle.is_null() {
        return 0;
    }
    (*handle).state.selected_material_id.unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_material_select(
    handle: *mut NovaCoreHandle,
    mat_id: MaterialId,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        if !state.select_material(mat_id) {
            return Err((NovaStatus::NotFound, format!("Material {mat_id} not found")));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_material_create_preset(
    handle: *mut NovaCoreHandle,
    preset_idx: u32,
    assign_selected: bool,
    out_id: *mut u64,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        let id = state.create_material_from_preset(preset_idx, assign_selected);
        if !out_id.is_null() {
            *out_id = id;
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_material_duplicate(
    handle: *mut NovaCoreHandle,
    mat_id: MaterialId,
    out_id: *mut u64,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        let id = state.duplicate_material(mat_id).ok_or_else(|| {
            (NovaStatus::NotFound, format!("Material {mat_id} not found"))
        })?;
        if !out_id.is_null() {
            *out_id = id;
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_material_rename(
    handle: *mut NovaCoreHandle,
    mat_id: MaterialId,
    name_utf8: *const c_char,
) -> i32 {
    guard_ffi(|| {
        if name_utf8.is_null() {
            return Err((NovaStatus::InvalidArgument, "Null name_utf8".to_string()));
        }
        let state = ref_mut(handle)?;
        let name = CStr::from_ptr(name_utf8)
            .to_str()
            .map_err(|_| (NovaStatus::InvalidArgument, "Invalid UTF-8".to_string()))?;
        if !state.rename_material(mat_id, name) {
            return Err((NovaStatus::InvalidArgument, format!("Cannot rename material {mat_id}")));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_material_delete(
    handle: *mut NovaCoreHandle,
    mat_id: MaterialId,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        if !state.delete_material(mat_id) {
            return Err((
                NovaStatus::InvalidArgument,
                "Cannot delete default or last material".to_string(),
            ));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_material_update(
    handle: *mut NovaCoreHandle,
    info: *const NovaMaterialInfo,
) -> i32 {
    guard_ffi(|| {
        if info.is_null() {
            return Err((NovaStatus::InvalidArgument, "Null material info".to_string()));
        }
        let state = ref_mut(handle)?;
        let m = &*info;
        let existing_name = state
            .find_material(m.id)
            .map(|x| x.name.clone())
            .ok_or_else(|| (NovaStatus::NotFound, format!("Material {} not found", m.id)))?;
        let spec = MaterialSpec {
            id: m.id,
            name: existing_name,
            base_color: m.base_color,
            metallic: m.metallic,
            roughness: m.roughness,
            emissive: m.emissive,
            emissive_intensity: m.emissive_intensity,
            ao_strength: m.ao_strength,
            alpha_mode: if m.alpha_mode == 1 {
                AlphaMode::Blend
            } else {
                AlphaMode::Opaque
            },
            albedo_tex_id: (m.albedo_tex_id > 0).then_some(m.albedo_tex_id),
            normal_tex_id: (m.normal_tex_id > 0).then_some(m.normal_tex_id),
            roughness_tex_id: (m.roughness_tex_id > 0).then_some(m.roughness_tex_id),
            ao_tex_id: (m.ao_tex_id > 0).then_some(m.ao_tex_id),
            emissive_tex_id: (m.emissive_tex_id > 0).then_some(m.emissive_tex_id),
            uv_scale: m.uv_scale,
            uv_offset: m.uv_offset,
        };
        if !state.update_material(spec) {
            return Err((NovaStatus::NotFound, format!("Material {} not found", m.id)));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_material_assign_to_selected(
    handle: *mut NovaCoreHandle,
    mat_id: MaterialId,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        if !state.assign_material_to_selected(mat_id) {
            return Err((
                NovaStatus::InvalidArgument,
                "Select at least one mesh object and a valid material".to_string(),
            ));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_texture_count(handle: *const NovaCoreHandle) -> usize {
    if handle.is_null() {
        return 0;
    }
    (*handle).state.textures.len()
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_texture_get_meta_by_index(
    handle: *const NovaCoreHandle,
    index: usize,
    out_meta: *mut NovaTextureMeta,
) -> i32 {
    guard_ffi(|| {
        if out_meta.is_null() {
            return Err((NovaStatus::InvalidArgument, "Null out_meta".to_string()));
        }
        let state = ref_const(handle)?;
        let tex = state.textures.get(index).ok_or_else(|| {
            (NovaStatus::NotFound, format!("Texture index {index} out of range"))
        })?;
        let (w, h) = tex.effective_dimensions();
        *out_meta = NovaTextureMeta {
            id: tex.id,
            width: w,
            height: h,
            status: tex.status as u32,
            version: tex.version,
        };
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_texture_get_meta(
    handle: *const NovaCoreHandle,
    tex_id: TextureId,
    out_meta: *mut NovaTextureMeta,
) -> i32 {
    guard_ffi(|| {
        if out_meta.is_null() {
            return Err((NovaStatus::InvalidArgument, "Null out_meta".to_string()));
        }
        let state = ref_const(handle)?;
        let tex = state.find_texture(tex_id).ok_or_else(|| {
            (NovaStatus::NotFound, format!("Texture {tex_id} not found"))
        })?;
        let (w, h) = tex.effective_dimensions();
        *out_meta = NovaTextureMeta {
            id: tex.id,
            width: w,
            height: h,
            status: tex.status as u32,
            version: tex.version,
        };
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_texture_get_name(
    handle: *const NovaCoreHandle,
    tex_id: TextureId,
    out_buf: *mut c_char,
    buf_len: usize,
) -> i32 {
    guard_ffi(|| {
        let state = ref_const(handle)?;
        let tex = state.find_texture(tex_id).ok_or_else(|| {
            (NovaStatus::NotFound, format!("Texture {tex_id} not found"))
        })?;
        copy_str_to_c_buf(&tex.name, out_buf, buf_len)
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_texture_copy_rgba(
    handle: *const NovaCoreHandle,
    tex_id: TextureId,
    out_buf: *mut u8,
    buf_len: usize,
    out_written: *mut usize,
) -> i32 {
    guard_ffi(|| {
        let state = ref_const(handle)?;
        let tex = state.find_texture(tex_id).ok_or_else(|| {
            (NovaStatus::NotFound, format!("Texture {tex_id} not found"))
        })?;
        let rgba = tex.effective_rgba();
        if !out_written.is_null() {
            *out_written = rgba.len();
        }
        if !out_buf.is_null() {
            if buf_len < rgba.len() {
                return Err((NovaStatus::InvalidArgument, "Buffer too small for RGBA".to_string()));
            }
            std::ptr::copy_nonoverlapping(rgba.as_ptr(), out_buf, rgba.len());
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_texture_register_rgba(
    handle: *mut NovaCoreHandle,
    name_utf8: *const c_char,
    source_uri_utf8: *const c_char,
    width: u32,
    height: u32,
    rgba_ptr: *const u8,
    rgba_len: usize,
    out_id: *mut u64,
) -> i32 {
    guard_ffi(|| {
        if rgba_ptr.is_null() || name_utf8.is_null() || source_uri_utf8.is_null() {
            return Err((NovaStatus::InvalidArgument, "Null texture registration pointer".to_string()));
        }
        let state = ref_mut(handle)?;
        let name = CStr::from_ptr(name_utf8)
            .to_str()
            .map_err(|_| (NovaStatus::InvalidArgument, "Invalid UTF-8 name".to_string()))?;
        let uri = CStr::from_ptr(source_uri_utf8)
            .to_str()
            .map_err(|_| (NovaStatus::InvalidArgument, "Invalid UTF-8 source_uri".to_string()))?;
        let bytes = std::slice::from_raw_parts(rgba_ptr, rgba_len).to_vec();
        let id = state
            .register_rgba_texture(name, uri, width, height, bytes)
            .map_err(|e| (NovaStatus::InvalidArgument, e))?;
        if !out_id.is_null() {
            *out_id = id;
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_texture_register_missing(
    handle: *mut NovaCoreHandle,
    name_utf8: *const c_char,
    source_uri_utf8: *const c_char,
    out_id: *mut u64,
) -> i32 {
    guard_ffi(|| {
        if name_utf8.is_null() || source_uri_utf8.is_null() {
            return Err((NovaStatus::InvalidArgument, "Null string pointer".to_string()));
        }
        let state = ref_mut(handle)?;
        let name = CStr::from_ptr(name_utf8)
            .to_str()
            .map_err(|_| (NovaStatus::InvalidArgument, "Invalid UTF-8 name".to_string()))?;
        let uri = CStr::from_ptr(source_uri_utf8)
            .to_str()
            .map_err(|_| (NovaStatus::InvalidArgument, "Invalid UTF-8 source_uri".to_string()))?;
        let id = state.register_missing_texture(name, uri);
        if !out_id.is_null() {
            *out_id = id;
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_texture_mark_missing(
    handle: *mut NovaCoreHandle,
    tex_id: TextureId,
    missing: bool,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        if !state.mark_texture_missing(tex_id, missing) {
            return Err((NovaStatus::InvalidArgument, format!("Cannot toggle missing state for texture {tex_id}")));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_texture_missing_warning_count(handle: *const NovaCoreHandle) -> usize {
    if handle.is_null() {
        return 0;
    }
    (*handle).state.missing_texture_warnings().len()
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_texture_get_missing_warning(
    handle: *const NovaCoreHandle,
    index: usize,
    out_buf: *mut c_char,
    buf_len: usize,
) -> i32 {
    guard_ffi(|| {
        let state = ref_const(handle)?;
        let warnings = state.missing_texture_warnings();
        let w = warnings.get(index).ok_or_else(|| {
            (NovaStatus::NotFound, format!("Warning index {index} out of range"))
        })?;
        copy_str_to_c_buf(w, out_buf, buf_len)
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_env_get(
    handle: *const NovaCoreHandle,
    out_env: *mut NovaEnvironmentInfo,
) -> i32 {
    guard_ffi(|| {
        if out_env.is_null() {
            return Err((NovaStatus::InvalidArgument, "Null out_env".to_string()));
        }
        let state = ref_const(handle)?;
        let e = &state.environment;
        *out_env = NovaEnvironmentInfo {
            preset: e.preset as u32,
            rotation_deg: e.rotation_deg,
            intensity: e.intensity,
            show_skybox: e.show_skybox,
            bloom_enabled: e.bloom_enabled,
            bloom_strength: e.bloom_strength,
            exposure: e.exposure,
            version: state.environment_version,
            material_preview_sphere: state.material_preview_sphere,
        };
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_env_set(
    handle: *mut NovaCoreHandle,
    env: *const NovaEnvironmentInfo,
) -> i32 {
    guard_ffi(|| {
        if env.is_null() {
            return Err((NovaStatus::InvalidArgument, "Null env".to_string()));
        }
        let state = ref_mut(handle)?;
        let e = &*env;
        state.set_environment(EnvironmentSpec {
            preset: EnvironmentPreset::from_u32(e.preset),
            rotation_deg: e.rotation_deg,
            intensity: e.intensity,
            show_skybox: e.show_skybox,
            bloom_enabled: e.bloom_enabled,
            bloom_strength: e.bloom_strength,
            exposure: e.exposure,
        });
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_lighting_apply_rig(
    handle: *mut NovaCoreHandle,
    rig_idx: u32,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        if !state.apply_lighting_rig(rig_idx) {
            return Err((NovaStatus::InvalidArgument, format!("Invalid lighting rig {rig_idx}")));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_material_preview_sphere_set(
    handle: *mut NovaCoreHandle,
    enabled: bool,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        if state.material_preview_sphere != enabled {
            state.material_preview_sphere = enabled;
            state.environment_version += 1;
            state.scene_revision += 1;
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_material_preview_sphere_get(handle: *const NovaCoreHandle) -> bool {
    if handle.is_null() {
        return false;
    }
    (*handle).state.material_preview_sphere
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_anim_tick(
    handle: *mut NovaCoreHandle,
    frame_time_nanos: u64,
) -> bool {
    if handle.is_null() {
        return false;
    }
    (*handle).state.anim_tick(frame_time_nanos)
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_anim_get_timeline(
    handle: *const NovaCoreHandle,
    out_info: *mut NovaTimelineInfo,
) -> i32 {
    guard_ffi(|| {
        let state = ref_const(handle)?;
        if out_info.is_null() {
            return Err((NovaStatus::InvalidArgument, "out_info is null".into()));
        }
        let tl = &state.animation.timeline;
        out_info.write(NovaTimelineInfo {
            start_frame: tl.start_frame,
            end_frame: tl.end_frame,
            fps: tl.fps,
            current_frame: tl.current_frame,
            playing: tl.playing,
            looping: tl.looping,
            auto_key: tl.auto_key,
            track_count: state.animation.tracks.len() as u32,
            active_camera_node_id: state.active_camera_node_id.unwrap_or(0),
        });
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_anim_set_playing(
    handle: *mut NovaCoreHandle,
    playing: bool,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        state.anim_set_playing(playing);
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_anim_set_frame(
    handle: *mut NovaCoreHandle,
    frame: f32,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        state.anim_set_frame(frame);
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_anim_set_timeline(
    handle: *mut NovaCoreHandle,
    start_frame: i32,
    end_frame: i32,
    fps: f32,
    looping: bool,
    auto_key: bool,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        state.anim_set_timeline(start_frame, end_frame, fps, looping, auto_key);
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_anim_insert_keyframe(
    handle: *mut NovaCoreHandle,
    target_kind: u32,
    target_id: u64,
    channel: u32,
    frame: i32,
    value: f32,
    interpolation: u32,
    out_track_id: *mut u64,
    out_key_id: *mut u64,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        let tk = AnimTargetKind::from_u32(target_kind)
            .ok_or_else(|| (NovaStatus::InvalidArgument, format!("Invalid target kind {target_kind}")))?;
        let ch = AnimChannelKind::from_u32(channel)
            .ok_or_else(|| (NovaStatus::InvalidArgument, format!("Invalid anim channel {channel}")))?;
        let interp = InterpolationMode::from_u32(interpolation)
            .ok_or_else(|| (NovaStatus::InvalidArgument, format!("Invalid interpolation {interpolation}")))?;
        let (tid, kid) = state.anim_insert_keyframe(tk, target_id, ch, frame, value, interp);
        if !out_track_id.is_null() {
            out_track_id.write(tid);
        }
        if !out_key_id.is_null() {
            out_key_id.write(kid);
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_anim_insert_node_trs_keys(
    handle: *mut NovaCoreHandle,
    node_id: u64,
    frame: i32,
    mask: u32,
    interpolation: u32,
    out_count: *mut u32,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        let interp = InterpolationMode::from_u32(interpolation)
            .ok_or_else(|| (NovaStatus::InvalidArgument, format!("Invalid interpolation {interpolation}")))?;
        let count = state.anim_insert_node_trs_keys(node_id, frame, mask, interp);
        if !out_count.is_null() {
            out_count.write(count as u32);
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_anim_insert_light_keys(
    handle: *mut NovaCoreHandle,
    node_id: u64,
    frame: i32,
    interpolation: u32,
    out_count: *mut u32,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        let interp = InterpolationMode::from_u32(interpolation)
            .ok_or_else(|| (NovaStatus::InvalidArgument, format!("Invalid interpolation {interpolation}")))?;
        let count = state.anim_insert_light_keys(node_id, frame, interp);
        if !out_count.is_null() {
            out_count.write(count as u32);
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_anim_insert_camera_keys(
    handle: *mut NovaCoreHandle,
    node_id: u64,
    frame: i32,
    interpolation: u32,
    out_count: *mut u32,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        let interp = InterpolationMode::from_u32(interpolation)
            .ok_or_else(|| (NovaStatus::InvalidArgument, format!("Invalid interpolation {interpolation}")))?;
        let count = state.anim_insert_camera_keys(node_id, frame, interp);
        if !out_count.is_null() {
            out_count.write(count as u32);
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_anim_insert_material_keys(
    handle: *mut NovaCoreHandle,
    mat_id: u64,
    frame: i32,
    interpolation: u32,
    out_count: *mut u32,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        let interp = InterpolationMode::from_u32(interpolation)
            .ok_or_else(|| (NovaStatus::InvalidArgument, format!("Invalid interpolation {interpolation}")))?;
        let count = state.anim_insert_material_keys(mat_id, frame, interp);
        if !out_count.is_null() {
            out_count.write(count as u32);
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_anim_update_keyframe(
    handle: *mut NovaCoreHandle,
    track_id: u64,
    key_id: u64,
    frame: i32,
    value: f32,
    interpolation: u32,
    in_dx: f32,
    in_dy: f32,
    out_dx: f32,
    out_dy: f32,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        let interp = InterpolationMode::from_u32(interpolation)
            .ok_or_else(|| (NovaStatus::InvalidArgument, format!("Invalid interpolation {interpolation}")))?;
        if !state.anim_update_keyframe(
            track_id,
            key_id,
            frame,
            value,
            interp,
            [in_dx, in_dy],
            [out_dx, out_dy],
        ) {
            return Err((NovaStatus::NotFound, "Keyframe not found".into()));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_anim_delete_keyframe(
    handle: *mut NovaCoreHandle,
    track_id: u64,
    key_id: u64,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        if !state.anim_delete_keyframe(track_id, key_id) {
            return Err((NovaStatus::NotFound, "Keyframe not found".into()));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_anim_step_keyframe(
    handle: *mut NovaCoreHandle,
    direction: i32,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        state.anim_step_keyframe(direction);
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_anim_track_count(handle: *const NovaCoreHandle) -> u32 {
    if handle.is_null() {
        return 0;
    }
    (*handle).state.animation.tracks.len() as u32
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_anim_track_get(
    handle: *const NovaCoreHandle,
    index: u32,
    out_meta: *mut NovaAnimTrackMeta,
) -> i32 {
    guard_ffi(|| {
        let state = ref_const(handle)?;
        if out_meta.is_null() {
            return Err((NovaStatus::InvalidArgument, "out_meta is null".into()));
        }
        let track = state
            .animation
            .tracks
            .get(index as usize)
            .ok_or_else(|| (NovaStatus::NotFound, format!("Track index {index} out of bounds")))?;
        out_meta.write(NovaAnimTrackMeta {
            track_id: track.id,
            target_kind: track.target_kind as u32,
            target_id: track.target_id,
            channel: track.channel as u32,
            keyframe_count: track.keyframes.len() as u32,
        });
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_anim_track_keyframe_get(
    handle: *const NovaCoreHandle,
    track_index: u32,
    key_index: u32,
    out_key: *mut NovaKeyframeMeta,
) -> i32 {
    guard_ffi(|| {
        let state = ref_const(handle)?;
        if out_key.is_null() {
            return Err((NovaStatus::InvalidArgument, "out_key is null".into()));
        }
        let track = state
            .animation
            .tracks
            .get(track_index as usize)
            .ok_or_else(|| (NovaStatus::NotFound, "Track out of bounds".into()))?;
        let kf = track
            .keyframes
            .get(key_index as usize)
            .ok_or_else(|| (NovaStatus::NotFound, "Keyframe out of bounds".into()))?;
        out_key.write(NovaKeyframeMeta {
            id: kf.id,
            frame: kf.frame,
            value: kf.value,
            interpolation: kf.interpolation as u32,
            in_handle: kf.in_handle,
            out_handle: kf.out_handle,
        });
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_camera_get_fov(
    handle: *const NovaCoreHandle,
    node_id: u64,
    out_fov: *mut f32,
) -> i32 {
    guard_ffi(|| {
        let state = ref_const(handle)?;
        if out_fov.is_null() {
            return Err((NovaStatus::InvalidArgument, "out_fov is null".into()));
        }
        let node = state
            .find_node(node_id)
            .ok_or_else(|| (NovaStatus::NotFound, format!("Node {node_id} not found")))?;
        out_fov.write(node.camera_node.fov_y_degrees);
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_camera_set_fov(
    handle: *mut NovaCoreHandle,
    node_id: u64,
    fov_deg: f32,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        if !state.set_camera_node_fov(node_id, fov_deg) {
            return Err((NovaStatus::InvalidArgument, "Not a camera node".into()));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_camera_set_active_node(
    handle: *mut NovaCoreHandle,
    cam_node_id: u64,
) -> i32 {
    guard_ffi(|| {
        let state = ref_mut(handle)?;
        let target = if cam_node_id == 0 {
            None
        } else {
            Some(cam_node_id)
        };
        if !state.set_active_camera_node(target) {
            return Err((NovaStatus::InvalidArgument, "Not a valid camera node".into()));
        }
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_camera_get_active_node(handle: *const NovaCoreHandle) -> u64 {
    if handle.is_null() {
        return 0;
    }
    (*handle).state.active_camera_node_id.unwrap_or(0)
}

static LAST_IO_WARNINGS: Mutex<String> = Mutex::new(String::new());

fn store_io_warnings(warnings: &[String]) {
    if let Ok(mut guard) = LAST_IO_WARNINGS.lock() {
        *guard = warnings.join("\n");
    }
}

unsafe fn boxed_output(bytes: Vec<u8>, out_ptr: *mut *mut u8, out_len: *mut usize) {
    let mut boxed = bytes.into_boxed_slice();
    let len = boxed.len();
    let ptr = boxed.as_mut_ptr();
    std::mem::forget(boxed);
    *out_ptr = ptr;
    *out_len = len;
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_io_take_warnings(out_buf: *mut c_char, buf_len: usize) -> usize {
    let msg = LAST_IO_WARNINGS
        .lock()
        .map(|mut g| std::mem::take(&mut *g))
        .unwrap_or_default();
    let bytes = msg.as_bytes();
    if out_buf.is_null() || buf_len == 0 {
        return bytes.len();
    }
    let copy_len = bytes.len().min(buf_len - 1);
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_buf as *mut u8, copy_len);
    *out_buf.add(copy_len) = 0;
    copy_len
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_import_obj(
    handle: *mut NovaCoreHandle,
    bytes_ptr: *const u8,
    bytes_len: usize,
    name_utf8: *const c_char,
) -> i32 {
    guard_ffi(|| {
        if bytes_ptr.is_null() || bytes_len == 0 {
            return Err((NovaStatus::InvalidArgument, "Empty OBJ payload".into()));
        }
        let state = ref_mut(handle)?;
        let bytes = std::slice::from_raw_parts(bytes_ptr, bytes_len);
        let name = if name_utf8.is_null() {
            "Imported OBJ"
        } else {
            CStr::from_ptr(name_utf8).to_str().unwrap_or("Imported OBJ")
        };
        let rep = import_obj(state, bytes, name)
            .map_err(|e| (NovaStatus::InvalidArgument, e))?;
        store_io_warnings(&rep.warnings);
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_import_gltf(
    handle: *mut NovaCoreHandle,
    bytes_ptr: *const u8,
    bytes_len: usize,
    name_utf8: *const c_char,
) -> i32 {
    guard_ffi(|| {
        if bytes_ptr.is_null() || bytes_len == 0 {
            return Err((NovaStatus::InvalidArgument, "Empty glTF/GLB payload".into()));
        }
        let state = ref_mut(handle)?;
        let bytes = std::slice::from_raw_parts(bytes_ptr, bytes_len);
        let name = if name_utf8.is_null() {
            "Imported glTF"
        } else {
            CStr::from_ptr(name_utf8).to_str().unwrap_or("Imported glTF")
        };
        let rep = import_gltf_or_glb(state, bytes, name)
            .map_err(|e| (NovaStatus::InvalidArgument, e))?;
        store_io_warnings(&rep.warnings);
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_export_obj(
    handle: *const NovaCoreHandle,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    guard_ffi(|| {
        if out_ptr.is_null() || out_len.is_null() {
            return Err((NovaStatus::InvalidArgument, "Null output pointer".into()));
        }
        let state = ref_const(handle)?;
        let (bytes, rep) = export_obj(state).map_err(|e| (NovaStatus::InvalidArgument, e))?;
        store_io_warnings(&rep.warnings);
        boxed_output(bytes, out_ptr, out_len);
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn nova_core_export_gltf(
    handle: *const NovaCoreHandle,
    as_glb: bool,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    guard_ffi(|| {
        if out_ptr.is_null() || out_len.is_null() {
            return Err((NovaStatus::InvalidArgument, "Null output pointer".into()));
        }
        let state = ref_const(handle)?;
        let (bytes, rep) =
            export_gltf(state, as_glb).map_err(|e| (NovaStatus::InvalidArgument, e))?;
        store_io_warnings(&rep.warnings);
        boxed_output(bytes, out_ptr, out_len);
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffi_lifecycle_and_camera_snapshot() {
        unsafe {
            let handle = nova_core_create();
            assert!(!handle.is_null());

            assert_eq!(nova_core_scene_node_count(handle), 2);
            let mut snap = std::mem::zeroed::<CameraSnapshot>();
            assert_eq!(nova_core_camera_get_snapshot(handle, &mut snap), 0);
            assert_eq!(snap.projection, 0);

            assert_eq!(nova_core_camera_set_projection(handle, 1), 0);
            assert_eq!(nova_core_camera_get_snapshot(handle, &mut snap), 0);
            assert_eq!(snap.projection, 1);

            let mut new_id = 0u64;
            assert_eq!(nova_core_scene_add_primitive(handle, 1, &mut new_id), 0);
            assert!(new_id > 0);
            assert_eq!(nova_core_scene_node_count(handle), 3);

            let mut buf_ptr: *mut u8 = std::ptr::null_mut();
            let mut buf_len: usize = 0;
            assert_eq!(nova_core_project_serialize(handle, &mut buf_ptr, &mut buf_len), 0);
            assert!(!buf_ptr.is_null() && buf_len > 20);
            assert_eq!(nova_core_scene_reset_empty(handle), 0);
            assert_eq!(nova_core_scene_node_count(handle), 0);
            assert_eq!(nova_core_project_deserialize(handle, buf_ptr, buf_len), 0);
            assert_eq!(nova_core_scene_node_count(handle), 3);
            nova_core_free_buffer(buf_ptr, buf_len);

            assert_ne!(nova_core_camera_set_projection(handle, 99), 0);
            let mut err_buf = [0 as c_char; 128];
            let err_len = nova_core_last_error(err_buf.as_mut_ptr(), err_buf.len());
            assert!(err_len > 0);

            // GLB FFI export -> import -> warnings check
            let mut light_id = 0u64;
            assert_eq!(nova_core_scene_add_primitive(handle, 10, &mut light_id), 0);
            let mut glb_ptr: *mut u8 = std::ptr::null_mut();
            let mut glb_len: usize = 0;
            assert_eq!(nova_core_export_gltf(handle, true, &mut glb_ptr, &mut glb_len), 0);
            assert!(!glb_ptr.is_null() && glb_len > 64);
            let mut warn_buf = [0 as c_char; 512];
            let warn_len = nova_core_io_take_warnings(warn_buf.as_mut_ptr(), warn_buf.len());
            assert!(warn_len > 0);
            let cname = std::ffi::CString::new("FFI_Asset").unwrap();
            assert_eq!(nova_core_import_gltf(handle, glb_ptr, glb_len, cname.as_ptr()), 0);
            nova_core_free_buffer(glb_ptr, glb_len);
            assert!(nova_core_scene_node_count(handle) > 4);

            nova_core_destroy(handle);
        }
    }
}
