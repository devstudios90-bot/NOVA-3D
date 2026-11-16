#ifndef NOVA_CORE_H
#define NOVA_CORE_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct NovaCoreHandle NovaCoreHandle;

typedef struct NovaCameraSnapshot {
    float eye[3];
    float target[3];
    float up[3];
    uint32_t projection; /* 0 = Perspective, 1 = Orthographic */
    float fov_y_degrees;
    float ortho_half_height;
    float near_plane;
    float far_plane;
    float distance;
    float yaw_radians;
    float pitch_radians;
} NovaCameraSnapshot;

typedef struct NovaNodeMeta {
    uint64_t id;
    uint64_t parent_id;
    uint32_t depth;
    uint32_t kind;
    bool visible;
    bool selected;
    bool primary_selected;
    bool has_mesh;
    uint64_t mesh_version;
    float model_matrix[16];
    float base_color[4];
    float metallic;
    float roughness;
    float emissive[3];
    float emissive_intensity;
    float ao_strength;
    uint32_t alpha_mode;
    float uv_scale[2];
    float uv_offset[2];
    uint64_t material_id;
    uint64_t albedo_tex_id;
    uint64_t normal_tex_id;
    uint64_t roughness_tex_id;
    uint64_t ao_tex_id;
    uint64_t emissive_tex_id;
    float bounds_min[3];
    float bounds_max[3];
    uint32_t light_kind;
    float light_color[3];
    float light_intensity;
    bool light_cast_shadows;
    float light_spot_inner;
    float light_spot_outer;
    float light_range;
} NovaNodeMeta;

typedef struct NovaNodeDetails {
    uint64_t id;
    uint64_t parent_id;
    uint32_t kind;
    bool visible;
    bool selected;
    bool primary_selected;
    bool has_mesh;
    float position[3];
    float rotation_euler[3];
    float scale[3];
    float base_color[4];
    float metallic;
    float roughness;
    uint64_t material_id;
    uint32_t light_kind;
    float light_color[3];
    float light_intensity;
    bool light_cast_shadows;
    float light_spot_inner;
    float light_spot_outer;
    float light_range;
} NovaNodeDetails;

typedef struct NovaMaterialInfo {
    uint64_t id;
    float base_color[4];
    float metallic;
    float roughness;
    float emissive[3];
    float emissive_intensity;
    float ao_strength;
    uint32_t alpha_mode; /* 0 = Opaque, 1 = Blend */
    uint64_t albedo_tex_id;
    uint64_t normal_tex_id;
    uint64_t roughness_tex_id;
    uint64_t ao_tex_id;
    uint64_t emissive_tex_id;
    float uv_scale[2];
    float uv_offset[2];
} NovaMaterialInfo;

typedef struct NovaTextureMeta {
    uint64_t id;
    uint32_t width;
    uint32_t height;
    uint32_t status; /* 0 = Builtin, 1 = Loaded, 2 = Missing */
    uint64_t version;
} NovaTextureMeta;

typedef struct NovaEnvironmentInfo {
    uint32_t preset; /* 0..5 */
    float rotation_deg;
    float intensity;
    bool show_skybox;
    bool bloom_enabled;
    float bloom_strength;
    float exposure;
    uint64_t version;
    bool material_preview_sphere;
} NovaEnvironmentInfo;

typedef struct NovaTimelineInfo {
    int32_t start_frame;
    int32_t end_frame;
    float fps;
    float current_frame;
    bool playing;
    bool looping;
    bool auto_key;
    uint32_t track_count;
    uint64_t active_camera_node_id;
} NovaTimelineInfo;

typedef struct NovaAnimTrackMeta {
    uint64_t track_id;
    uint32_t target_kind; /* 0 = Node, 1 = Material */
    uint64_t target_id;
    uint32_t channel; /* 0..22 */
    uint32_t keyframe_count;
} NovaAnimTrackMeta;

typedef struct NovaKeyframeMeta {
    uint64_t id;
    int32_t frame;
    float value;
    uint32_t interpolation; /* 0=Linear,1=Constant,2=Bezier,3=EaseIn,4=EaseOut,5=EaseInOut */
    float in_handle[2];
    float out_handle[2];
} NovaKeyframeMeta;

typedef struct NovaGizmoSnapshot {
    bool visible;
    uint32_t mode;        /* 0=None, 1=Move, 2=Rotate, 3=Scale */
    uint32_t space;       /* 0=World, 1=Local */
    uint32_t active_axis; /* 0=None, 1=X, 2=Y, 3=Z, 4=Center */
    float pivot[3];
    float axis_x[3];
    float axis_y[3];
    float axis_z[3];
    float scale;
} NovaGizmoSnapshot;

typedef struct NovaMeshView {
    const float* positions_ptr;
    const float* normals_ptr;
    const float* uvs_ptr;
    size_t vertex_count;
    const uint32_t* indices_ptr;
    size_t index_count;
    float bounds_min[3];
    float bounds_max[3];
} NovaMeshView;

typedef struct NovaEditState {
    uint32_t interaction_mode; /* 0=Object, 1=Edit */
    uint64_t edit_node_id;
    uint32_t edit_select_mode; /* 0=Vertex, 1=Edge, 2=Face */
    bool show_face_orientation;
    bool smooth_shading;
    uint32_t total_verts;
    uint32_t total_edges;
    uint32_t total_faces;
    uint32_t selected_vert_closure;
    uint32_t selected_verts;
    uint32_t selected_edges;
    uint32_t selected_faces;
    bool has_centroid;
    float centroid_world[3];
    uint32_t modifier_count;
} NovaEditState;

typedef struct NovaModifierInfo {
    uint32_t kind; /* 0=Mirror, 1=Subdiv, 2=Solidify, 3=Array */
    bool enabled;
    uint32_t axis;
    bool merge;
    uint32_t levels;
    float thickness;
    uint32_t count;
    float offset[3];
} NovaModifierInfo;

typedef struct NovaEditOverlayLine {
    float a[3];
    float b[3];
    float color[4];
} NovaEditOverlayLine;

typedef struct NovaEditOverlayTri {
    float v0[3];
    float v1[3];
    float v2[3];
    float color[4];
} NovaEditOverlayTri;

typedef struct NovaEditOverlayView {
    const NovaEditOverlayLine* lines_ptr;
    size_t line_count;
    const NovaEditOverlayTri* tris_ptr;
    size_t tri_count;
} NovaEditOverlayView;

NovaCoreHandle* nova_core_create(void);
void nova_core_destroy(NovaCoreHandle* handle);
size_t nova_core_last_error(char* out_buf, size_t buf_len);

int32_t nova_core_camera_orbit(NovaCoreHandle* handle, float delta_yaw_rad, float delta_pitch_rad);
int32_t nova_core_camera_pan(NovaCoreHandle* handle, float norm_dx, float norm_dy);
int32_t nova_core_camera_zoom(NovaCoreHandle* handle, float factor);
int32_t nova_core_camera_set_projection(NovaCoreHandle* handle, uint32_t projection_mode);
int32_t nova_core_camera_set_standard_view(NovaCoreHandle* handle, uint32_t standard_view);
int32_t nova_core_camera_frame_all(NovaCoreHandle* handle);
int32_t nova_core_camera_focus_selected(NovaCoreHandle* handle);
int32_t nova_core_camera_get_snapshot(const NovaCoreHandle* handle, NovaCameraSnapshot* out_snapshot);

uint64_t nova_core_scene_revision(const NovaCoreHandle* handle);
size_t nova_core_scene_node_count(const NovaCoreHandle* handle);
int32_t nova_core_scene_get_node_meta(const NovaCoreHandle* handle, size_t index, NovaNodeMeta* out_meta);
int32_t nova_core_scene_get_node_details(const NovaCoreHandle* handle, uint64_t node_id, NovaNodeDetails* out_details);
int32_t nova_core_scene_get_node_name(const NovaCoreHandle* handle, uint64_t node_id, char* out_buf, size_t buf_len);
int32_t nova_core_scene_get_node_mesh(const NovaCoreHandle* handle, uint64_t node_id, NovaMeshView* out_mesh);
int32_t nova_core_scene_select_node(NovaCoreHandle* handle, uint64_t node_id);
int32_t nova_core_scene_select_node_ext(NovaCoreHandle* handle, uint64_t node_id, bool additive);
int32_t nova_core_scene_pick_at_ndc(NovaCoreHandle* handle, float ndc_x, float ndc_y, float aspect, bool additive, uint64_t* out_id);
int32_t nova_core_scene_set_grid_axes(NovaCoreHandle* handle, bool show_grid, bool show_axes);
int32_t nova_core_scene_get_grid_axes(const NovaCoreHandle* handle, bool* out_show_grid, bool* out_show_axes);
int32_t nova_core_set_shading_mode(NovaCoreHandle* handle, uint32_t shading_mode);
uint32_t nova_core_get_shading_mode(const NovaCoreHandle* handle);

int32_t nova_core_scene_reset_empty(NovaCoreHandle* handle);
int32_t nova_core_scene_reset_starter(NovaCoreHandle* handle);
int32_t nova_core_scene_add_primitive(NovaCoreHandle* handle, uint32_t primitive_type, uint64_t* out_id);
int32_t nova_core_scene_duplicate_selected(NovaCoreHandle* handle);
int32_t nova_core_scene_delete_selected(NovaCoreHandle* handle);
int32_t nova_core_scene_rename_node(NovaCoreHandle* handle, uint64_t node_id, const char* name_utf8);
int32_t nova_core_scene_reparent_node(NovaCoreHandle* handle, uint64_t child_id, uint64_t new_parent_id);
int32_t nova_core_scene_group_selected(NovaCoreHandle* handle, uint64_t* out_group_id);
int32_t nova_core_scene_unparent_selected(NovaCoreHandle* handle);
int32_t nova_core_scene_apply_transform_selected(NovaCoreHandle* handle);
int32_t nova_core_scene_origin_to_geometry_selected(NovaCoreHandle* handle);
int32_t nova_core_scene_set_visibility(NovaCoreHandle* handle, uint64_t node_id, bool visible);
int32_t nova_core_scene_set_transform(NovaCoreHandle* handle, uint64_t node_id, const float* position, const float* rotation_euler, const float* scale);
int32_t nova_core_scene_set_material(NovaCoreHandle* handle, uint64_t node_id, const float* base_color, float metallic, float roughness);
int32_t nova_core_scene_set_light(NovaCoreHandle* handle, uint64_t node_id, uint32_t kind, const float* color, float intensity, bool cast_shadows);
int32_t nova_core_scene_set_light_ex(NovaCoreHandle* handle, uint64_t node_id, uint32_t kind, const float* color, float intensity, bool cast_shadows, float spot_inner_deg, float spot_outer_deg, float range);

int32_t nova_core_gizmo_set_mode(NovaCoreHandle* handle, uint32_t mode);
int32_t nova_core_gizmo_set_space(NovaCoreHandle* handle, uint32_t space);
int32_t nova_core_gizmo_get_snapshot(const NovaCoreHandle* handle, NovaGizmoSnapshot* out_snap);
uint32_t nova_core_gizmo_pick_axis(const NovaCoreHandle* handle, float ndc_x, float ndc_y, float aspect);
bool nova_core_gizmo_begin_drag(NovaCoreHandle* handle, uint32_t axis, float ndc_x, float ndc_y, float aspect);
bool nova_core_gizmo_update_drag(NovaCoreHandle* handle, float ndc_x, float ndc_y, float aspect);
bool nova_core_gizmo_end_drag(NovaCoreHandle* handle);

bool nova_core_undo(NovaCoreHandle* handle);
bool nova_core_redo(NovaCoreHandle* handle);
bool nova_core_can_undo(const NovaCoreHandle* handle);
bool nova_core_can_redo(const NovaCoreHandle* handle);

int32_t nova_core_project_serialize(const NovaCoreHandle* handle, uint8_t** out_ptr, size_t* out_len);
void nova_core_free_buffer(uint8_t* ptr, size_t len);
int32_t nova_core_project_deserialize(NovaCoreHandle* handle, const uint8_t* json_ptr, size_t json_len);

/* M3: Edit Mode, Topology Operations, Overlay & Modifiers */
int32_t nova_core_set_interaction_mode(NovaCoreHandle* handle, uint32_t mode);
uint32_t nova_core_get_interaction_mode(const NovaCoreHandle* handle);
int32_t nova_core_edit_set_select_mode(NovaCoreHandle* handle, uint32_t select_mode);
int32_t nova_core_edit_get_state(const NovaCoreHandle* handle, NovaEditState* out_state);
int32_t nova_core_edit_pick_at_ndc(NovaCoreHandle* handle, float ndc_x, float ndc_y, float aspect, bool additive, bool* out_hit);
int32_t nova_core_edit_clear_or_select_all(NovaCoreHandle* handle);
int32_t nova_core_edit_set_face_orientation(NovaCoreHandle* handle, bool enabled);
int32_t nova_core_edit_extrude(NovaCoreHandle* handle, float distance);
int32_t nova_core_edit_inset(NovaCoreHandle* handle, float factor);
int32_t nova_core_edit_bevel(NovaCoreHandle* handle, float width);
int32_t nova_core_edit_loop_cut(NovaCoreHandle* handle);
int32_t nova_core_edit_subdivide(NovaCoreHandle* handle);
int32_t nova_core_edit_merge(NovaCoreHandle* handle);
int32_t nova_core_edit_delete(NovaCoreHandle* handle);
int32_t nova_core_edit_dissolve_edges(NovaCoreHandle* handle);
int32_t nova_core_edit_recalculate_normals(NovaCoreHandle* handle);
int32_t nova_core_edit_flip_normals(NovaCoreHandle* handle);
int32_t nova_core_edit_set_smooth_shading(NovaCoreHandle* handle, bool smooth);
int32_t nova_core_edit_translate_selected(NovaCoreHandle* handle, float dx, float dy, float dz);
int32_t nova_core_edit_get_overlay(const NovaCoreHandle* handle, NovaEditOverlayView* out_overlay);

size_t nova_core_modifier_count(const NovaCoreHandle* handle);
int32_t nova_core_modifier_get(const NovaCoreHandle* handle, size_t index, NovaModifierInfo* out_mod);
int32_t nova_core_modifier_add(NovaCoreHandle* handle, uint32_t kind);
int32_t nova_core_modifier_remove(NovaCoreHandle* handle, size_t index);
int32_t nova_core_modifier_move(NovaCoreHandle* handle, size_t index, int32_t delta);
int32_t nova_core_modifier_update(NovaCoreHandle* handle, size_t index, const NovaModifierInfo* mod_info);
int32_t nova_core_modifier_apply_all(NovaCoreHandle* handle);

/* M4: Look Development — Materials, Textures, HDR Environment & Lighting Rigs */
size_t nova_core_material_count(const NovaCoreHandle* handle);
int32_t nova_core_material_get_by_index(const NovaCoreHandle* handle, size_t index, NovaMaterialInfo* out_info);
int32_t nova_core_material_get(const NovaCoreHandle* handle, uint64_t mat_id, NovaMaterialInfo* out_info);
int32_t nova_core_material_get_name(const NovaCoreHandle* handle, uint64_t mat_id, char* out_buf, size_t buf_len);
uint64_t nova_core_material_selected_id(const NovaCoreHandle* handle);
int32_t nova_core_material_select(NovaCoreHandle* handle, uint64_t mat_id);
int32_t nova_core_material_create_preset(NovaCoreHandle* handle, uint32_t preset_idx, bool assign_selected, uint64_t* out_id);
int32_t nova_core_material_duplicate(NovaCoreHandle* handle, uint64_t mat_id, uint64_t* out_id);
int32_t nova_core_material_rename(NovaCoreHandle* handle, uint64_t mat_id, const char* name_utf8);
int32_t nova_core_material_delete(NovaCoreHandle* handle, uint64_t mat_id);
int32_t nova_core_material_update(NovaCoreHandle* handle, const NovaMaterialInfo* info);
int32_t nova_core_material_assign_to_selected(NovaCoreHandle* handle, uint64_t mat_id);

size_t nova_core_texture_count(const NovaCoreHandle* handle);
int32_t nova_core_texture_get_meta_by_index(const NovaCoreHandle* handle, size_t index, NovaTextureMeta* out_meta);
int32_t nova_core_texture_get_meta(const NovaCoreHandle* handle, uint64_t tex_id, NovaTextureMeta* out_meta);
int32_t nova_core_texture_get_name(const NovaCoreHandle* handle, uint64_t tex_id, char* out_buf, size_t buf_len);
int32_t nova_core_texture_copy_rgba(const NovaCoreHandle* handle, uint64_t tex_id, uint8_t* out_buf, size_t buf_len, size_t* out_written);
int32_t nova_core_texture_register_rgba(NovaCoreHandle* handle, const char* name_utf8, const char* source_uri_utf8, uint32_t width, uint32_t height, const uint8_t* rgba_ptr, size_t rgba_len, uint64_t* out_id);
int32_t nova_core_texture_register_missing(NovaCoreHandle* handle, const char* name_utf8, const char* source_uri_utf8, uint64_t* out_id);
int32_t nova_core_texture_mark_missing(NovaCoreHandle* handle, uint64_t tex_id, bool missing);
size_t nova_core_texture_missing_warning_count(const NovaCoreHandle* handle);
int32_t nova_core_texture_get_missing_warning(const NovaCoreHandle* handle, size_t index, char* out_buf, size_t buf_len);

int32_t nova_core_env_get(const NovaCoreHandle* handle, NovaEnvironmentInfo* out_env);
int32_t nova_core_env_set(NovaCoreHandle* handle, const NovaEnvironmentInfo* env);
int32_t nova_core_lighting_apply_rig(NovaCoreHandle* handle, uint32_t rig_idx);
int32_t nova_core_material_preview_sphere_set(NovaCoreHandle* handle, bool enabled);
bool nova_core_material_preview_sphere_get(const NovaCoreHandle* handle);

bool nova_core_anim_tick(NovaCoreHandle* handle, uint64_t frame_time_nanos);
int32_t nova_core_anim_get_timeline(const NovaCoreHandle* handle, NovaTimelineInfo* out_info);
int32_t nova_core_anim_set_playing(NovaCoreHandle* handle, bool playing);
int32_t nova_core_anim_set_frame(NovaCoreHandle* handle, float frame);
int32_t nova_core_anim_set_timeline(NovaCoreHandle* handle,
                                    int32_t start_frame,
                                    int32_t end_frame,
                                    float fps,
                                    bool looping,
                                    bool auto_key);
int32_t nova_core_anim_insert_keyframe(NovaCoreHandle* handle,
                                       uint32_t target_kind,
                                       uint64_t target_id,
                                       uint32_t channel,
                                       int32_t frame,
                                       float value,
                                       uint32_t interpolation,
                                       uint64_t* out_track_id,
                                       uint64_t* out_key_id);
int32_t nova_core_anim_insert_node_trs_keys(NovaCoreHandle* handle,
                                            uint64_t node_id,
                                            int32_t frame,
                                            uint32_t mask,
                                            uint32_t interpolation,
                                            uint32_t* out_count);
int32_t nova_core_anim_insert_light_keys(NovaCoreHandle* handle,
                                         uint64_t node_id,
                                         int32_t frame,
                                         uint32_t interpolation,
                                         uint32_t* out_count);
int32_t nova_core_anim_insert_camera_keys(NovaCoreHandle* handle,
                                          uint64_t node_id,
                                          int32_t frame,
                                          uint32_t interpolation,
                                          uint32_t* out_count);
int32_t nova_core_anim_insert_material_keys(NovaCoreHandle* handle,
                                            uint64_t mat_id,
                                            int32_t frame,
                                            uint32_t interpolation,
                                            uint32_t* out_count);
int32_t nova_core_anim_update_keyframe(NovaCoreHandle* handle,
                                       uint64_t track_id,
                                       uint64_t key_id,
                                       int32_t frame,
                                       float value,
                                       uint32_t interpolation,
                                       float in_dx,
                                       float in_dy,
                                       float out_dx,
                                       float out_dy);
int32_t nova_core_anim_delete_keyframe(NovaCoreHandle* handle, uint64_t track_id, uint64_t key_id);
int32_t nova_core_anim_step_keyframe(NovaCoreHandle* handle, int32_t direction);
uint32_t nova_core_anim_track_count(const NovaCoreHandle* handle);
int32_t nova_core_anim_track_get(const NovaCoreHandle* handle, uint32_t index, NovaAnimTrackMeta* out_meta);
int32_t nova_core_anim_track_keyframe_get(const NovaCoreHandle* handle,
                                          uint32_t track_index,
                                          uint32_t key_index,
                                          NovaKeyframeMeta* out_key);
int32_t nova_core_camera_get_fov(const NovaCoreHandle* handle, uint64_t node_id, float* out_fov);
int32_t nova_core_camera_set_fov(NovaCoreHandle* handle, uint64_t node_id, float fov_deg);
int32_t nova_core_camera_set_active_node(NovaCoreHandle* handle, uint64_t cam_node_id);
uint64_t nova_core_camera_get_active_node(const NovaCoreHandle* handle);

/* 3D format import/export (OBJ, glTF 2.0, GLB) */
uintptr_t nova_core_io_take_warnings(char* out_buf, uintptr_t buf_len);
int32_t nova_core_import_obj(NovaCoreHandle* handle,
                             const uint8_t* bytes_ptr,
                             uintptr_t bytes_len,
                             const char* name_utf8);
int32_t nova_core_import_gltf(NovaCoreHandle* handle,
                              const uint8_t* bytes_ptr,
                              uintptr_t bytes_len,
                              const char* name_utf8);
int32_t nova_core_export_obj(const NovaCoreHandle* handle,
                             uint8_t** out_ptr,
                             uintptr_t* out_len);
int32_t nova_core_export_gltf(const NovaCoreHandle* handle,
                              bool as_glb,
                              uint8_t** out_ptr,
                              uintptr_t* out_len);

#ifdef __cplusplus
}
#endif

#endif /* NOVA_CORE_H */
