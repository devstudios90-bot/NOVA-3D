//! Self-contained, deterministic 3D math primitives for NOVA 3D core.
//! Uses column-major 4x4 matrices matching OpenGL / Filament conventions.

use std::ops::{Add, Mul, Neg, Sub};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0 };
    pub const ONE: Self = Self { x: 1.0, y: 1.0, z: 1.0 };
    pub const UNIT_X: Self = Self { x: 1.0, y: 0.0, z: 0.0 };
    pub const UNIT_Y: Self = Self { x: 0.0, y: 1.0, z: 0.0 };
    pub const UNIT_Z: Self = Self { x: 0.0, y: 0.0, z: 1.0 };

    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    #[inline]
    pub fn dot(self, rhs: Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    #[inline]
    pub fn cross(self, rhs: Self) -> Self {
        Self {
            x: self.y * rhs.z - self.z * rhs.y,
            y: self.z * rhs.x - self.x * rhs.z,
            z: self.x * rhs.y - self.y * rhs.x,
        }
    }

    #[inline]
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    #[inline]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    #[inline]
    pub fn normalized_or(self, fallback: Self) -> Self {
        let len = self.length();
        if len > 1e-6 {
            Self {
                x: self.x / len,
                y: self.y / len,
                z: self.z / len,
            }
        } else {
            fallback
        }
    }

    #[inline]
    pub fn min(self, rhs: Self) -> Self {
        Self {
            x: self.x.min(rhs.x),
            y: self.y.min(rhs.y),
            z: self.z.min(rhs.z),
        }
    }

    #[inline]
    pub fn max(self, rhs: Self) -> Self {
        Self {
            x: self.x.max(rhs.x),
            y: self.y.max(rhs.y),
            z: self.z.max(rhs.z),
        }
    }

    #[inline]
    pub const fn to_array(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }

    #[inline]
    pub fn lerp(self, rhs: Self, t: f32) -> Self {
        Self::new(
            self.x + (rhs.x - self.x) * t,
            self.y + (rhs.y - self.y) * t,
            self.z + (rhs.z - self.z) * t,
        )
    }
}

impl Add for Vec3 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Sub for Vec3 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;
    #[inline]
    fn mul(self, s: f32) -> Self {
        Self::new(self.x * s, self.y * s, self.z * s)
    }
}

impl Neg for Vec3 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Default for Quat {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Quat {
    pub const IDENTITY: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 1.0,
    };

    /// Constructs a quaternion from XYZ intrinsic/extrinsic Euler angles in radians (YXZ order).
    pub fn from_euler_xyz(rx: f32, ry: f32, rz: f32) -> Self {
        let (sx, cx) = (rx * 0.5).sin_cos();
        let (sy, cy) = (ry * 0.5).sin_cos();
        let (sz, cz) = (rz * 0.5).sin_cos();

        Self {
            x: sx * cy * cz + cx * sy * sz,
            y: cx * sy * cz - sx * cy * sz,
            z: cx * cy * sz - sx * sy * cz,
            w: cx * cy * cz + sx * sy * sz,
        }
        .normalized()
    }

    pub fn normalized(self) -> Self {
        let len = (self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w).sqrt();
        if len > 1e-6 {
            Self {
                x: self.x / len,
                y: self.y / len,
                z: self.z / len,
                w: self.w / len,
            }
        } else {
            Self::IDENTITY
        }
    }
}

/// Column-major 4x4 matrix (`cols[col][row]`, stored flat as 16 floats).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat4 {
    pub data: [f32; 16],
}

impl Default for Mat4 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Mat4 {
    pub const IDENTITY: Self = Self {
        data: [
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0, //
        ],
    };

    pub fn from_trs(t: Vec3, r: Quat, s: Vec3) -> Self {
        let x2 = r.x + r.x;
        let y2 = r.y + r.y;
        let z2 = r.z + r.z;

        let xx = r.x * x2;
        let xy = r.x * y2;
        let xz = r.x * z2;
        let yy = r.y * y2;
        let yz = r.y * z2;
        let zz = r.z * z2;
        let wx = r.w * x2;
        let wy = r.w * y2;
        let wz = r.w * z2;

        Self {
            data: [
                (1.0 - (yy + zz)) * s.x,
                (xy + wz) * s.x,
                (xz - wy) * s.x,
                0.0,
                (xy - wz) * s.y,
                (1.0 - (xx + zz)) * s.y,
                (yz + wx) * s.y,
                0.0,
                (xz + wy) * s.z,
                (yz - wx) * s.z,
                (1.0 - (xx + yy)) * s.z,
                0.0,
                t.x,
                t.y,
                t.z,
                1.0,
            ],
        }
    }

    pub fn mul_mat4(self, rhs: Self) -> Self {
        let mut out = [0.0f32; 16];
        for col in 0..4 {
            for row in 0..4 {
                let mut sum = 0.0f32;
                for k in 0..4 {
                    sum += self.data[k * 4 + row] * rhs.data[col * 4 + k];
                }
                out[col * 4 + row] = sum;
            }
        }
        Self { data: out }
    }

    pub fn transform_point(self, p: Vec3) -> Vec3 {
        let x = self.data[0] * p.x + self.data[4] * p.y + self.data[8] * p.z + self.data[12];
        let y = self.data[1] * p.x + self.data[5] * p.y + self.data[9] * p.z + self.data[13];
        let z = self.data[2] * p.x + self.data[6] * p.y + self.data[10] * p.z + self.data[14];
        Vec3::new(x, y, z)
    }

    pub fn transform_vector(self, v: Vec3) -> Vec3 {
        let x = self.data[0] * v.x + self.data[4] * v.y + self.data[8] * v.z;
        let y = self.data[1] * v.x + self.data[5] * v.y + self.data[9] * v.z;
        let z = self.data[2] * v.x + self.data[6] * v.y + self.data[10] * v.z;
        Vec3::new(x, y, z)
    }

    /// Computes full 4x4 inverse; returns None if singular.
    pub fn inverse(self) -> Option<Self> {
        let m = self.data;
        let a2323 = m[10] * m[15] - m[14] * m[11];
        let a1323 = m[6] * m[15] - m[14] * m[7];
        let a1223 = m[6] * m[11] - m[10] * m[7];
        let a0323 = m[2] * m[15] - m[14] * m[3];
        let a0223 = m[2] * m[11] - m[10] * m[3];
        let a0123 = m[2] * m[7] - m[6] * m[3];
        let a2313 = m[9] * m[15] - m[13] * m[11];
        let a1313 = m[5] * m[15] - m[13] * m[7];
        let a1213 = m[5] * m[11] - m[9] * m[7];
        let a2312 = m[9] * m[14] - m[13] * m[10];
        let a1312 = m[5] * m[14] - m[13] * m[6];
        let a1212 = m[5] * m[10] - m[9] * m[6];
        let a0313 = m[1] * m[15] - m[13] * m[3];
        let a0213 = m[1] * m[11] - m[9] * m[3];
        let a0312 = m[1] * m[14] - m[13] * m[2];
        let a0212 = m[1] * m[10] - m[9] * m[2];
        let a0113 = m[1] * m[7] - m[5] * m[3];
        let a0112 = m[1] * m[6] - m[5] * m[2];

        let det = m[0] * (m[5] * a2323 - m[9] * a1323 + m[13] * a1223)
            - m[4] * (m[1] * a2323 - m[9] * a0323 + m[13] * a0223)
            + m[8] * (m[1] * a1323 - m[5] * a0323 + m[13] * a0123)
            - m[12] * (m[1] * a1223 - m[5] * a0223 + m[9] * a0123);

        if !det.is_finite() || det.abs() < 1e-10 {
            return None;
        }
        let inv_det = 1.0 / det;

        Some(Self {
            data: [
                inv_det * (m[5] * a2323 - m[9] * a1323 + m[13] * a1223),
                inv_det * -(m[1] * a2323 - m[9] * a0323 + m[13] * a0223),
                inv_det * (m[1] * a1323 - m[5] * a0323 + m[13] * a0123),
                inv_det * -(m[1] * a1223 - m[5] * a0223 + m[9] * a0123),
                inv_det * -(m[4] * a2323 - m[8] * a1323 + m[12] * a1223),
                inv_det * (m[0] * a2323 - m[8] * a0323 + m[12] * a0223),
                inv_det * -(m[0] * a1323 - m[4] * a0323 + m[12] * a0123),
                inv_det * (m[0] * a1223 - m[4] * a0223 + m[8] * a0123),
                inv_det * (m[4] * a2313 - m[8] * a1313 + m[12] * a1213),
                inv_det * -(m[0] * a2313 - m[8] * a0313 + m[12] * a0213),
                inv_det * (m[0] * a1313 - m[4] * a0313 + m[12] * a0113),
                inv_det * -(m[0] * a1213 - m[4] * a0213 + m[8] * a0113),
                inv_det * -(m[4] * a2312 - m[8] * a1312 + m[12] * a1212),
                inv_det * (m[0] * a2312 - m[8] * a0312 + m[12] * a0212),
                inv_det * -(m[0] * a1312 - m[4] * a0312 + m[12] * a0112),
                inv_det * (m[0] * a1212 - m[4] * a0212 + m[8] * a0112),
            ],
        })
    }

    /// Extracts (translation, Euler YXZ angles in radians matching `Quat::from_euler_xyz`, scale).
    pub fn decompose_trs(self) -> (Vec3, Vec3, Vec3) {
        let t = Vec3::new(self.data[12], self.data[13], self.data[14]);
        let col0 = Vec3::new(self.data[0], self.data[1], self.data[2]);
        let col1 = Vec3::new(self.data[4], self.data[5], self.data[6]);
        let col2 = Vec3::new(self.data[8], self.data[9], self.data[10]);

        let sx = col0.length().max(1e-5);
        let sy = col1.length().max(1e-5);
        let sz = col2.length().max(1e-5);

        let r0 = col0 * (1.0 / sx);
        let r1 = col1 * (1.0 / sy);
        let r2 = col2 * (1.0 / sz);

        // Determinant check for negative scale parity
        let det3 = r0.dot(r1.cross(r2));
        let (sx, r0) = if det3 < 0.0 { (-sx, -r0) } else { (sx, r0) };

        // Rotation matrix R = Ry * Rx * Rz -> column 2 is [sinY*cosX, -sinX, cosY*cosX], etc.
        let sin_x = (-r2.y).clamp(-1.0, 1.0);
        let rx = sin_x.asin();
        let cos_x = rx.cos();
        let (ry, rz) = if cos_x.abs() > 1e-4 {
            (r2.x.atan2(r2.z), r0.y.atan2(r1.y))
        } else {
            ((-r0.z).atan2(r0.x), 0.0)
        };

        (t, Vec3::new(rx, ry, rz), Vec3::new(sx, sy, sz))
    }

    pub fn basis_axes(self) -> [Vec3; 3] {
        [
            Vec3::new(self.data[0], self.data[1], self.data[2]).normalized_or(Vec3::UNIT_X),
            Vec3::new(self.data[4], self.data[5], self.data[6]).normalized_or(Vec3::UNIT_Y),
            Vec3::new(self.data[8], self.data[9], self.data[10]).normalized_or(Vec3::UNIT_Z),
        ]
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Ray {
    pub origin: Vec3,
    pub dir: Vec3,
}

impl Ray {
    pub fn intersect_aabb(self, min_v: Vec3, max_v: Vec3) -> Option<f32> {
        let mut tmin = 0.0_f32;
        let mut tmax = f32::INFINITY;
        let orig = [self.origin.x, self.origin.y, self.origin.z];
        let dir = [self.dir.x, self.dir.y, self.dir.z];
        let bmin = [min_v.x, min_v.y, min_v.z];
        let bmax = [max_v.x, max_v.y, max_v.z];

        for i in 0..3 {
            if dir[i].abs() < 1e-7 {
                if orig[i] < bmin[i] || orig[i] > bmax[i] {
                    return None;
                }
            } else {
                let inv_d = 1.0 / dir[i];
                let mut t1 = (bmin[i] - orig[i]) * inv_d;
                let mut t2 = (bmax[i] - orig[i]) * inv_d;
                if t1 > t2 {
                    std::mem::swap(&mut t1, &mut t2);
                }
                tmin = tmin.max(t1);
                tmax = tmax.min(t2);
                if tmin > tmax {
                    return None;
                }
            }
        }
        Some(tmin)
    }

    pub fn intersect_triangle(self, v0: Vec3, v1: Vec3, v2: Vec3) -> Option<f32> {
        let e1 = v1 - v0;
        let e2 = v2 - v0;
        let p = self.dir.cross(e2);
        let det = e1.dot(p);
        if det.abs() < 1e-7 {
            return None;
        }
        let inv_det = 1.0 / det;
        let tvec = self.origin - v0;
        let u = tvec.dot(p) * inv_det;
        if !(0.0..=1.0).contains(&u) {
            return None;
        }
        let q = tvec.cross(e1);
        let v = self.dir.dot(q) * inv_det;
        if v < 0.0 || u + v > 1.0 {
            return None;
        }
        let t = e2.dot(q) * inv_det;
        if t > 1e-4 {
            Some(t)
        } else {
            None
        }
    }

    pub fn intersect_plane(self, plane_origin: Vec3, plane_normal: Vec3) -> Option<Vec3> {
        let denom = self.dir.dot(plane_normal);
        if denom.abs() < 1e-6 {
            return None;
        }
        let t = (plane_origin - self.origin).dot(plane_normal) / denom;
        if t > 0.0 {
            Some(self.origin + self.dir * t)
        } else {
            None
        }
    }

    /// Returns scalar `s` along `axis_origin + s * axis_dir` closest to this ray.
    pub fn closest_param_on_axis(self, axis_origin: Vec3, axis_dir: Vec3) -> f32 {
        let u = self.dir;
        let v = axis_dir.normalized_or(Vec3::UNIT_X);
        let w = self.origin - axis_origin;
        let a = u.dot(u);
        let b = u.dot(v);
        let c = v.dot(v);
        let d = u.dot(w);
        let e = v.dot(w);
        let denom = a * c - b * b;
        if denom.abs() < 1e-6 {
            e / c.max(1e-6)
        } else {
            (a * e - b * d) / denom
        }
    }

    /// Closest (distance, ray_t) between forward ray (`t >= 0`) and finite segment `[a, b]`.
    pub fn distance_to_segment(self, a: Vec3, b: Vec3) -> (f32, f32) {
        let d1 = self.dir;
        let d2 = b - a;
        let r = self.origin - a;
        let a1 = d1.dot(d1);
        let e = d2.dot(d2);
        let f = d2.dot(r);

        if e <= 1e-7 {
            let t = (-d1.dot(r) / a1.max(1e-6)).max(0.0);
            let p1 = self.origin + d1 * t;
            return ((p1 - a).length(), t);
        }

        let b12 = d1.dot(d2);
        let c = d1.dot(r);
        let denom = a1 * e - b12 * b12;

        let mut s = if denom.abs() > 1e-6 {
            ((b12 * f - c * e) / denom).max(0.0)
        } else {
            0.0
        };
        let mut t = (b12 * s + f) / e;
        if t < 0.0 {
            t = 0.0;
            s = (-c / a1.max(1e-6)).max(0.0);
        } else if t > 1.0 {
            t = 1.0;
            s = ((b12 - c) / a1.max(1e-6)).max(0.0);
        }
        let p_ray = self.origin + d1 * s;
        let p_seg = a + d2 * t;
        ((p_ray - p_seg).length(), s)
    }
}
