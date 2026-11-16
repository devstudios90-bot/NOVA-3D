# Project NOVA 3D — v1.0 Test & Verification Report

**Date:** 2026-11-12  
**Milestone Scope:** M1–M6 (`v1.0` Definition of Done)  
**Test Device:** Android physical handset `5c00b8d4` (`arm64-v8a`, Qualcomm Adreno 613 Vulkan, `720x1600` display, `5407 MiB` total RAM, hardware H.264 encoder `c2.qti.avc.encoder` max `1920x1920`)

---

## 1. Automated Unit Tests (`rust/nova_core`)

Executed via:
```powershell
cargo test --manifest-path "d:\My Apps\Nova 3d\rust\nova_core\Cargo.toml"
```
**Result:** **25 passed; 0 failed; 0 ignored** (`0` doc-tests).

| Module | Test Cases Verified | Result |
|---|---|---|
| `camera` | `frame_bounds_centers_target`, `orbit_clamps_pitch_and_wraps_yaw`, `zoom_clamps_distance` | Pass (3/3) |
| `ffi` | `ffi_lifecycle_and_camera_snapshot` | Pass (1/1) |
| `scene` / `poly` / `lookdev` / `animation` | Starter scene & bounds, all 11 primitives + Undo/Redo, transform Undo/Redo, Apply Transform & Origin to Geometry, hierarchy group/reparent cycle guard & delete Undo, ray picking & gizmo drags, Edit Mode ops (Extrude/Inset/Bevel/Loop Cut/Subdivide/Merge/Delete/Dissolve), Modifier stack (`Mirror`/`Subdivision`/`Solidify`/`Array`), PBR Material Library & `.nova3d` M2/M3 backward compatibility, Missing texture fallback/Environment rigs/Spot cones, M5 keyframe/Bezier interpolation & deterministic evaluation + `.nova3d` round-trip | Pass (17/17) |
| `io_formats` | `png_encode_decode_roundtrip`, `obj_export_import_roundtrip_preserves_mesh_and_reports_warnings`, `obj_rejects_external_mtl_silently_dropped_warning_and_oob_index`, `glb_and_gltf_roundtrip_hierarchy_pbr_camera_and_textures`, `gltf_import_warns_on_skins_morphs_and_rejects_corrupt_glb` | Pass (4/4) |

---

## 2. Multi-ABI Native & Android Build Verification

Executed via:
```powershell
cd "d:\My Apps\Nova 3d\android"
.\gradlew.bat assembleDebug --no-daemon --console=plain
```
**Result:** `BUILD SUCCESSFUL` (`compileDebugKotlin`, `buildRustCore`, and `buildCMakeDebug` cleanly compiled `arm64-v8a`, `armeabi-v7a`, and `x86_64` into `android/app/build/outputs/apk/debug/app-debug.apk`).

---

## 3. Physical Device Verification Matrix (`5c00b8d4`)

### Milestone 6 & v1.0 End-to-End Checks (Latest Build)
1. **Principle 1 Shell & Navigation:** Top navigation bar contains `Home`, `Projects`, `Workspace`, `Materials`, `Animation`, `Render` (no `"M6"` pending badge), and `Settings`, plus `Vulkan` backend status indicator (`59–60 FPS`).
2. **Offscreen Still Render (`Render` tab):**
   - Rendered `1920x1080` lossless PNG (`studio_starter_1920x1080_*.png`, `1.0M`) via `SwapChain::CONFIG_READABLE` + `renderer_->readPixels` + `engine_->flushAndWait()`.
   - Visual inspection confirmed upright Vulkan readback orientation, clean PBR lighting & AO contact shadow on `Base Plinth`, and complete exclusion of viewport grid, coordinate axes, selection outlines, and gizmos (`offscreenView_` mask `(0x03, 0x01)`).
3. **Device Capability Probe & 4K Pre-Flight Guard (`RN-7`):**
   - `Render` and `Settings` accurately surfaced live telemetry (`2099–2300 MiB` free RAM, `1993 / 5407 MiB` in `Settings`, `c2.qti.avc.encoder`, max `1920x1920`).
   - Selecting `4K UHD (3840x2160)` with `Animation MP4` displayed the amber pre-flight notice (`Device H.264 encoder (c2.qti.avc.encoder, max 1920x1920) does not support 4K; downscaled safely to 1080p FHD`) without OOM or codec crashes.
4. **Offscreen MP4 Video Render (`MediaCodec` + `MediaMuxer`):**
   - Shortened frame range (`1..20` @ `24 fps`) and executed `Render MP4 Video (1920x1080)`.
   - Produced valid H.264 `.mp4` clip (`studio_starter_1920x1080_24fps_*.mp4`, `127K`) inside `files/renders/`, listed cleanly under `Saved Renders (2)`.
5. **3D Asset Exchange (`FM-5` – `FM-7`):**
   - Verified `+ Add ▾ -> Import 3D (GLB / glTF / OBJ)` and `Projects -> Import 3D / Export GLB / Export OBJ` actions.
   - Imported `nova_pyramid_test.obj` via Android SAF from Downloads: `VerificationPyramid` appeared in `Outliner (3)` and live Vulkan viewport, and triggered the `3D Format Conversion Notes` dialog warning about skipped external `mtllib studio.mtl`.
   - Exported binary glTF (`studio_starter.glb`, `5.2K`) cleanly via Android SAF.
6. **Settings & Adaptive Quality:**
   - Verified `Adaptive Viewport Resolution` switch toggled cleanly (`true` <-> `false`) alongside backend, node count, memory, encoder, and `Last Native Error: None` readouts.
7. **Stability / Logcat Check:**
   - Monitored `adb logcat -d -v time *:E` across the entire session: zero `FATAL`, `AndroidRuntime`, JNI, Rust panic, or `NovaFilament` error entries.

### Milestones 1–5 (Previously Verified on `5c00b8d4`)
- **M1:** Workspace shell, `Home`, `Projects`, `Workspace`, 3-axis orbit/pan/zoom camera, `Studio Starter` scene.
- **M2:** All 11 primitives (`Cube`..`Light`), multi-selection/group/reparent/duplicate/delete, World & Local `Move`/`Rotate`/`Scale` gizmos, transactional `.nova3d` save/open/crash recovery.
- **M3:** `Object <-> Edit` mesh mode (`Vertex`/`Edge`/`Face`), `Extrude`, `Inset`, `Bevel`, `Loop Cut`, `Subdivide`, `Merge`, `Delete`, `Dissolve`, `Apply Transform`, non-destructive `Mirror`/`Subdivision`/`Solidify`/`Array` modifiers.
- **M4:** PBR `Materials` tab + live `Sphere`/`Scene` shaderball, procedural & imported textures, missing-texture checker recovery, 6 HDR environments + skybox + bloom, `World` lighting rigs, 4 viewport `Shade:` modes.
- **M5:** `Animation` tab (`Dope Sheet`, `Graph Editor` with Bezier handles, `▶ Play`/`Loop`/`● Auto-Key`), Workspace collapsible timeline strip, Camera lens/FOV & `Active Cam View`, and `.nova3d` track round-trip.

---

## 4. Known v1.0 Limitations (Per PRD Section 16 Scope)
- **Hardware Video Encoder Resolution Ceiling:** H.264 MP4 encodes honor each device's hardware `MediaCodec` capability (`1920x1920` max on `5c00b8d4` `c2.qti.avc.encoder`); `2K`/`4K` video targets safely downscale to `1080p` rather than failing on unsupported hardware profiles. (Still image PNG/JPEG renders support higher targets within free RAM limits.)
- **External OBJ `.mtl` & glTF Armatures/Morphs (`FM-7`):** SAF content URIs provide standalone stream access, so external `.mtl` sidecars, `.gltf` external `.bin`/image URI references, and skeletal `skins` / morph `targets` emit explicit user-visible conversion warnings (`3D Format Conversion Notes`) rather than silent failure; self-contained `.glb` bundles preserve full PBR textures cleanly.
- **Skeletal Bone Rigging (`AN-7`):** Track/target model carries the documented `Bone` extension marker for post-v1.0 rigging without breaking `.nova3d` v1 schema compatibility.
