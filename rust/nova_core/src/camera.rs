//! Viewport camera model owned by the Rust core.
//! Supports spherical orbit, screen-space pan, dolly/ortho zoom,
//! perspective/orthographic projection, standard views, and framing bounds.

use std::f32::consts::{FRAC_PI_2, PI};

use crate::math::Vec3;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionMode {
    Perspective = 0,
    Orthographic = 1,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandardView {
    Home = 0,
    Front = 1,
    Back = 2,
    Left = 3,
    Right = 4,
    Top = 5,
    Bottom = 6,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraSnapshot {
    pub eye: [f32; 3],
    pub target: [f32; 3],
    pub up: [f32; 3],
    pub projection: u32,
    pub fov_y_degrees: f32,
    pub ortho_half_height: f32,
    pub near_plane: f32,
    pub far_plane: f32,
    pub distance: f32,
    pub yaw_radians: f32,
    pub pitch_radians: f32,
}

#[derive(Debug, Clone)]
pub struct ViewportCamera {
    pub target: Vec3,
    pub distance: f32,
    /// Azimuth in radians around +Y axis. 0 looks toward -Z from +Z.
    pub yaw: f32,
    /// Elevation in radians from XZ plane. Positive moves camera up (+Y).
    pub pitch: f32,
    pub projection: ProjectionMode,
    pub fov_y_degrees: f32,
    pub near_plane: f32,
    pub far_plane: f32,
}

impl Default for ViewportCamera {
    fn default() -> Self {
        Self {
            target: Vec3::new(0.0, 0.5, 0.0),
            distance: 5.5,
            yaw: 0.65,
            pitch: 0.42,
            projection: ProjectionMode::Perspective,
            fov_y_degrees: 45.0,
            near_plane: 0.05,
            far_plane: 500.0,
        }
    }
}

impl ViewportCamera {
    const MIN_DISTANCE: f32 = 0.25;
    const MAX_DISTANCE: f32 = 250.0;
    const MAX_PITCH: f32 = FRAC_PI_2 - 0.005;

    pub fn orbit(&mut self, delta_yaw_rad: f32, delta_pitch_rad: f32) {
        if !delta_yaw_rad.is_finite() || !delta_pitch_rad.is_finite() {
            return;
        }
        self.yaw = (self.yaw + delta_yaw_rad).rem_euclid(2.0 * PI);
        self.pitch = (self.pitch + delta_pitch_rad).clamp(-Self::MAX_PITCH, Self::MAX_PITCH);
    }

    /// Pans the camera target in the camera's local right/up plane.
    /// `norm_dx` and `norm_dy` are normalized by viewport height (e.g. pixel_delta / viewport_height).
    pub fn pan(&mut self, norm_dx: f32, norm_dy: f32) {
        if !norm_dx.is_finite() || !norm_dy.is_finite() {
            return;
        }
        let (eye, right, up) = self.basis();
        let _ = eye;
        let visible_height = self.visible_height_at_target();
        self.target = self.target - right * (norm_dx * visible_height) + up * (norm_dy * visible_height);
    }

    /// Zooms by a multiplicative factor (> 1.0 zooms out, < 1.0 zooms in; e.g. 1 / pinch_scale).
    pub fn zoom_by_factor(&mut self, factor: f32) {
        if !factor.is_finite() || factor <= 0.0 {
            return;
        }
        self.distance = (self.distance * factor).clamp(Self::MIN_DISTANCE, Self::MAX_DISTANCE);
    }

    pub fn set_projection(&mut self, mode: ProjectionMode) {
        self.projection = mode;
    }

    pub fn set_standard_view(&mut self, view: StandardView) {
        match view {
            StandardView::Home => {
                self.yaw = 0.65;
                self.pitch = 0.42;
            }
            StandardView::Front => {
                self.yaw = 0.0;
                self.pitch = 0.0;
            }
            StandardView::Back => {
                self.yaw = PI;
                self.pitch = 0.0;
            }
            StandardView::Right => {
                self.yaw = FRAC_PI_2;
                self.pitch = 0.0;
            }
            StandardView::Left => {
                self.yaw = -FRAC_PI_2;
                self.pitch = 0.0;
            }
            StandardView::Top => {
                self.yaw = 0.0;
                self.pitch = Self::MAX_PITCH;
            }
            StandardView::Bottom => {
                self.yaw = 0.0;
                self.pitch = -Self::MAX_PITCH;
            }
        }
    }

    pub fn frame_bounds(&mut self, min: Vec3, max: Vec3) {
        let center = (min + max) * 0.5;
        let extent = max - min;
        let radius = (extent.length() * 0.5).max(0.5);
        let half_fov = (self.fov_y_degrees * 0.5).to_radians().max(0.05);
        let dist = (radius / half_fov.sin()) * 1.35;
        self.target = center;
        self.distance = dist.clamp(Self::MIN_DISTANCE, Self::MAX_DISTANCE);
    }

    pub fn visible_height_at_target(&self) -> f32 {
        let half_fov = (self.fov_y_degrees * 0.5).to_radians();
        2.0 * self.distance * half_fov.tan()
    }

    pub fn basis(&self) -> (Vec3, Vec3, Vec3) {
        let cos_p = self.pitch.cos();
        let sin_p = self.pitch.sin();
        let cos_y = self.yaw.cos();
        let sin_y = self.yaw.sin();

        // Spherical offset from target to eye: yaw=0 => +Z, yaw=+PI/2 => +X
        let offset = Vec3::new(
            self.distance * cos_p * sin_y,
            self.distance * sin_p,
            self.distance * cos_p * cos_y,
        );
        let eye = self.target + offset;
        let forward = (self.target - eye).normalized_or(Vec3::new(0.0, 0.0, -1.0));
        let world_up = Vec3::UNIT_Y;
        let right = forward.cross(world_up).normalized_or(Vec3::UNIT_X);
        let up = right.cross(forward).normalized_or(Vec3::UNIT_Y);
        (eye, right, up)
    }

    pub fn snapshot(&self) -> CameraSnapshot {
        let (eye, _right, up) = self.basis();
        let ortho_half_height = self.visible_height_at_target() * 0.5;
        CameraSnapshot {
            eye: [eye.x, eye.y, eye.z],
            target: [self.target.x, self.target.y, self.target.z],
            up: [up.x, up.y, up.z],
            projection: self.projection as u32,
            fov_y_degrees: self.fov_y_degrees,
            ortho_half_height,
            near_plane: self.near_plane,
            far_plane: self.far_plane,
            distance: self.distance,
            yaw_radians: self.yaw,
            pitch_radians: self.pitch,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orbit_clamps_pitch_and_wraps_yaw() {
        let mut cam = ViewportCamera::default();
        cam.orbit(2.0 * PI + 0.5, 10.0);
        assert!(cam.pitch <= ViewportCamera::MAX_PITCH);
        assert!(cam.yaw >= 0.0 && cam.yaw < 2.0 * PI);
    }

    #[test]
    fn zoom_clamps_distance() {
        let mut cam = ViewportCamera::default();
        cam.zoom_by_factor(0.0001);
        assert!((cam.distance - ViewportCamera::MIN_DISTANCE).abs() < 1e-5);
        cam.zoom_by_factor(100000.0);
        assert!((cam.distance - ViewportCamera::MAX_DISTANCE).abs() < 1e-5);
    }

    #[test]
    fn frame_bounds_centers_target() {
        let mut cam = ViewportCamera::default();
        cam.frame_bounds(Vec3::new(1.0, 2.0, 3.0), Vec3::new(3.0, 4.0, 5.0));
        assert!((cam.target.x - 2.0).abs() < 1e-5);
        assert!((cam.target.y - 3.0).abs() < 1e-5);
        assert!((cam.target.z - 4.0).abs() < 1e-5);
    }
}
