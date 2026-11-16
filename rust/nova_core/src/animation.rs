//! Keyframe animation, F-Curves, interpolation, and deterministic timeline evaluation (AN-1..AN-7).
//! Single source of truth owned by Rust core; UI renders derived curves/keyframes and sends edits.

pub type TrackId = u64;
pub type KeyframeId = u64;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InterpolationMode {
    Linear = 0,
    Constant = 1,
    Bezier = 2,
    EaseIn = 3,
    EaseOut = 4,
    EaseInOut = 5,
}

impl InterpolationMode {
    pub fn from_u32(v: u32) -> Option<Self> {
        match v {
            0 => Some(Self::Linear),
            1 => Some(Self::Constant),
            2 => Some(Self::Bezier),
            3 => Some(Self::EaseIn),
            4 => Some(Self::EaseOut),
            5 => Some(Self::EaseInOut),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Linear => "Linear",
            Self::Constant => "Constant",
            Self::Bezier => "Bezier",
            Self::EaseIn => "Ease In",
            Self::EaseOut => "Ease Out",
            Self::EaseInOut => "Ease In-Out",
        }
    }
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AnimTargetKind {
    Node = 0,
    Material = 1,
    // AN-7 architectural extension point (v1.x armatures / skeletal animation):
    // Bone = 2,
}

impl AnimTargetKind {
    pub fn from_u32(v: u32) -> Option<Self> {
        match v {
            0 => Some(Self::Node),
            1 => Some(Self::Material),
            _ => None,
        }
    }
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AnimChannelKind {
    LocX = 0,
    LocY = 1,
    LocZ = 2,
    RotX = 3,
    RotY = 4,
    RotZ = 5,
    ScaleX = 6,
    ScaleY = 7,
    ScaleZ = 8,
    LightIntensity = 9,
    LightColorR = 10,
    LightColorG = 11,
    LightColorB = 12,
    LightSpotOuterDeg = 13,
    LightRange = 14,
    CameraFovDeg = 15,
    MatColorR = 16,
    MatColorG = 17,
    MatColorB = 18,
    MatOpacity = 19,
    MatMetallic = 20,
    MatRoughness = 21,
    MatEmissiveIntensity = 22,
}

impl AnimChannelKind {
    pub fn from_u32(v: u32) -> Option<Self> {
        match v {
            0 => Some(Self::LocX),
            1 => Some(Self::LocY),
            2 => Some(Self::LocZ),
            3 => Some(Self::RotX),
            4 => Some(Self::RotY),
            5 => Some(Self::RotZ),
            6 => Some(Self::ScaleX),
            7 => Some(Self::ScaleY),
            8 => Some(Self::ScaleZ),
            9 => Some(Self::LightIntensity),
            10 => Some(Self::LightColorR),
            11 => Some(Self::LightColorG),
            12 => Some(Self::LightColorB),
            13 => Some(Self::LightSpotOuterDeg),
            14 => Some(Self::LightRange),
            15 => Some(Self::CameraFovDeg),
            16 => Some(Self::MatColorR),
            17 => Some(Self::MatColorG),
            18 => Some(Self::MatColorB),
            19 => Some(Self::MatOpacity),
            20 => Some(Self::MatMetallic),
            21 => Some(Self::MatRoughness),
            22 => Some(Self::MatEmissiveIntensity),
            _ => None,
        }
    }

    pub fn target_kind(self) -> AnimTargetKind {
        match self as u32 {
            16..=22 => AnimTargetKind::Material,
            _ => AnimTargetKind::Node,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::LocX => "Location X",
            Self::LocY => "Location Y",
            Self::LocZ => "Location Z",
            Self::RotX => "Rotation X",
            Self::RotY => "Rotation Y",
            Self::RotZ => "Rotation Z",
            Self::ScaleX => "Scale X",
            Self::ScaleY => "Scale Y",
            Self::ScaleZ => "Scale Z",
            Self::LightIntensity => "Light Intensity",
            Self::LightColorR => "Light Color R",
            Self::LightColorG => "Light Color G",
            Self::LightColorB => "Light Color B",
            Self::LightSpotOuterDeg => "Spot Cone",
            Self::LightRange => "Light Range",
            Self::CameraFovDeg => "Camera FOV",
            Self::MatColorR => "Base Color R",
            Self::MatColorG => "Base Color G",
            Self::MatColorB => "Base Color B",
            Self::MatOpacity => "Opacity",
            Self::MatMetallic => "Metallic",
            Self::MatRoughness => "Roughness",
            Self::MatEmissiveIntensity => "Emissive Power",
        }
    }

    pub fn clamp_value(self, v: f32) -> f32 {
        let f = if v.is_finite() { v } else { 0.0 };
        match self {
            Self::ScaleX | Self::ScaleY | Self::ScaleZ => f.clamp(0.01, 100.0),
            Self::LightIntensity => f.clamp(0.0, 300_000.0),
            Self::LightColorR | Self::LightColorG | Self::LightColorB => f.clamp(0.0, 1.0),
            Self::LightSpotOuterDeg => f.clamp(4.0, 88.0),
            Self::LightRange => f.clamp(1.0, 80.0),
            Self::CameraFovDeg => f.clamp(15.0, 120.0),
            Self::MatColorR | Self::MatColorG | Self::MatColorB | Self::MatOpacity | Self::MatMetallic => {
                f.clamp(0.0, 1.0)
            }
            Self::MatRoughness => f.clamp(0.04, 1.0),
            Self::MatEmissiveIntensity => f.clamp(0.0, 12.0),
            _ => f.clamp(-10_000.0, 10_000.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Keyframe {
    pub id: KeyframeId,
    pub frame: i32,
    pub value: f32,
    pub interpolation: InterpolationMode,
    /// Relative tangent offsets from this keyframe: `(dx_frames, dy_value)`.
    pub in_handle: [f32; 2],
    pub out_handle: [f32; 2],
}

impl Keyframe {
    pub fn new(id: KeyframeId, frame: i32, value: f32, interpolation: InterpolationMode) -> Self {
        Self {
            id,
            frame,
            value,
            interpolation,
            in_handle: [-4.0, 0.0],
            out_handle: [4.0, 0.0],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimTrack {
    pub id: TrackId,
    pub target_kind: AnimTargetKind,
    pub target_id: u64,
    pub channel: AnimChannelKind,
    pub keyframes: Vec<Keyframe>,
}

impl AnimTrack {
    pub fn sort_keyframes(&mut self) {
        self.keyframes.sort_by_key(|k| k.frame);
    }

    pub fn evaluate_at(&self, frame: f32) -> Option<f32> {
        let len = self.keyframes.len();
        if len == 0 {
            return None;
        }
        let first = &self.keyframes[0];
        if len == 1 || frame <= first.frame as f32 {
            return Some(self.channel.clamp_value(first.value));
        }
        let last = &self.keyframes[len - 1];
        if frame >= last.frame as f32 {
            return Some(self.channel.clamp_value(last.value));
        }

        // Find segment [k0, k1] with k0.frame <= frame <= k1.frame
        let idx = self
            .keyframes
            .partition_point(|k| (k.frame as f32) <= frame)
            .saturating_sub(1)
            .min(len - 2);
        let k0 = &self.keyframes[idx];
        let k1 = &self.keyframes[idx + 1];

        let t0 = k0.frame as f32;
        let t1 = k1.frame as f32;
        let span = (t1 - t0).max(1e-5);
        let u = ((frame - t0) / span).clamp(0.0, 1.0);

        let raw = match k0.interpolation {
            InterpolationMode::Constant => k0.value,
            InterpolationMode::Linear => k0.value + (k1.value - k0.value) * u,
            InterpolationMode::EaseIn => {
                let e = u * u * u;
                k0.value + (k1.value - k0.value) * e
            }
            InterpolationMode::EaseOut => {
                let inv = 1.0 - u;
                let e = 1.0 - inv * inv * inv;
                k0.value + (k1.value - k0.value) * e
            }
            InterpolationMode::EaseInOut => {
                let e = if u < 0.5 {
                    4.0 * u * u * u
                } else {
                    1.0 - ((-2.0 * u + 2.0).powi(3)) * 0.5
                };
                k0.value + (k1.value - k0.value) * e
            }
            InterpolationMode::Bezier => eval_cubic_bezier_segment(k0, k1, frame),
        };
        Some(self.channel.clamp_value(raw))
    }
}

/// Evaluates a monotonic cubic Bezier F-Curve segment `(k0, k1)` at `frame`.
/// Control points:
///   P0 = (t0, v0)
///   P1 = (t0 + clamp(k0.out_handle[0], 0..span), v0 + k0.out_handle[1])
///   P2 = (t1 + clamp(k1.in_handle[0], -span..0), v1 + k1.in_handle[1])
///   P3 = (t1, v1)
pub fn eval_cubic_bezier_segment(k0: &Keyframe, k1: &Keyframe, frame: f32) -> f32 {
    let x0 = k0.frame as f32;
    let x3 = k1.frame as f32;
    let span = (x3 - x0).max(1e-5);

    // Clamp handle X offsets within the segment so X(t) remains monotonic.
    let out_dx = if k0.out_handle[0].is_finite() && k0.out_handle[0] > 0.0 {
        k0.out_handle[0].min(span * 0.99)
    } else {
        span * 0.35
    };
    let in_dx = if k1.in_handle[0].is_finite() && k1.in_handle[0] < 0.0 {
        k1.in_handle[0].max(-span * 0.99)
    } else {
        -span * 0.35
    };

    let x1 = x0 + out_dx;
    let x2 = x3 + in_dx;

    let y0 = k0.value;
    let y3 = k1.value;
    let y1 = y0 + if k0.out_handle[1].is_finite() { k0.out_handle[1] } else { 0.0 };
    let y2 = y3 + if k1.in_handle[1].is_finite() { k1.in_handle[1] } else { 0.0 };

    let bez_x = |t: f32| -> f32 {
        let omt = 1.0 - t;
        omt * omt * omt * x0
            + 3.0 * omt * omt * t * x1
            + 3.0 * omt * t * t * x2
            + t * t * t * x3
    };
    let bez_dx = |t: f32| -> f32 {
        let omt = 1.0 - t;
        3.0 * omt * omt * (x1 - x0) + 6.0 * omt * t * (x2 - x1) + 3.0 * t * t * (x3 - x2)
    };
    let bez_y = |t: f32| -> f32 {
        let omt = 1.0 - t;
        omt * omt * omt * y0
            + 3.0 * omt * omt * t * y1
            + 3.0 * omt * t * t * y2
            + t * t * t * y3
    };

    // Bisect + Newton solve for X(t) == frame
    let mut lo = 0.0f32;
    let mut hi = 1.0f32;
    let mut t = ((frame - x0) / span).clamp(0.0, 1.0);
    for _ in 0..8 {
        let x = bez_x(t) - frame;
        if x.abs() < 1e-4 {
            return bez_y(t);
        }
        if x < 0.0 {
            lo = lo.max(t);
        } else {
            hi = hi.min(t);
        }
        let d = bez_dx(t);
        if d.abs() > 1e-4 {
            let next = t - x / d;
            if next > lo && next < hi {
                t = next;
                continue;
            }
        }
        t = 0.5 * (lo + hi);
    }
    for _ in 0..10 {
        let x = bez_x(t) - frame;
        if x < 0.0 {
            lo = t;
        } else {
            hi = t;
        }
        t = 0.5 * (lo + hi);
    }
    bez_y(t)
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimelineSettings {
    pub start_frame: i32,
    pub end_frame: i32,
    pub fps: f32,
    pub current_frame: f32,
    pub looping: bool,
    pub auto_key: bool,
    pub playing: bool,
    pub last_tick_nanos: Option<u64>,
}

impl Default for TimelineSettings {
    fn default() -> Self {
        Self {
            start_frame: 1,
            end_frame: 120,
            fps: 24.0,
            current_frame: 1.0,
            looping: true,
            auto_key: false,
            playing: false,
            last_tick_nanos: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimationState {
    pub timeline: TimelineSettings,
    pub tracks: Vec<AnimTrack>,
    pub next_track_id: TrackId,
    pub next_keyframe_id: KeyframeId,
}

impl Default for AnimationState {
    fn default() -> Self {
        Self {
            timeline: TimelineSettings::default(),
            tracks: Vec::new(),
            next_track_id: 1,
            next_keyframe_id: 1,
        }
    }
}

impl AnimationState {
    pub fn total_keyframes(&self) -> usize {
        self.tracks.iter().map(|t| t.keyframes.len()).sum()
    }

    pub fn set_timeline_range(&mut self, start: i32, end: i32, fps: f32, looping: bool, auto_key: bool) {
        let s = start.clamp(0, 99_999);
        let e = end.max(s + 1).clamp(1, 100_000);
        self.timeline.start_frame = s;
        self.timeline.end_frame = e;
        self.timeline.fps = fps.clamp(1.0, 120.0);
        self.timeline.looping = looping;
        self.timeline.auto_key = auto_key;
        self.timeline.current_frame = self
            .timeline
            .current_frame
            .clamp(s as f32, e as f32);
    }

    pub fn insert_or_update_keyframe(
        &mut self,
        target_kind: AnimTargetKind,
        target_id: u64,
        channel: AnimChannelKind,
        frame: i32,
        value: f32,
        interpolation: InterpolationMode,
    ) -> (TrackId, KeyframeId) {
        let clamped_val = channel.clamp_value(value);
        let clamped_frame = frame.clamp(0, 100_000);

        let track_idx = if let Some(pos) = self.tracks.iter().position(|t| {
            t.target_kind == target_kind && t.target_id == target_id && t.channel == channel
        }) {
            pos
        } else {
            let tid = self.next_track_id;
            self.next_track_id += 1;
            self.tracks.push(AnimTrack {
                id: tid,
                target_kind,
                target_id,
                channel,
                keyframes: Vec::new(),
            });
            self.tracks.len() - 1
        };

        let next_kf_id = self.next_keyframe_id;
        let track = &mut self.tracks[track_idx];
        let track_id = track.id;

        if let Some(existing) = track.keyframes.iter_mut().find(|k| k.frame == clamped_frame) {
            existing.value = clamped_val;
            existing.interpolation = interpolation;
            return (track_id, existing.id);
        }

        self.next_keyframe_id = next_kf_id + 1;
        let kf = Keyframe::new(next_kf_id, clamped_frame, clamped_val, interpolation);
        track.keyframes.push(kf);
        track.sort_keyframes();
        (track_id, next_kf_id)
    }

    pub fn update_keyframe(
        &mut self,
        track_id: TrackId,
        key_id: KeyframeId,
        frame: i32,
        value: f32,
        interpolation: InterpolationMode,
        in_handle: [f32; 2],
        out_handle: [f32; 2],
    ) -> bool {
        let Some(track) = self.tracks.iter_mut().find(|t| t.id == track_id) else {
            return false;
        };
        let clamped_frame = frame.clamp(0, 100_000);
        let clamped_val = track.channel.clamp_value(value);
        let in_dx = if in_handle[0].is_finite() {
            in_handle[0].clamp(-200.0, -0.05)
        } else {
            -4.0
        };
        let in_dy = if in_handle[1].is_finite() { in_handle[1].clamp(-10_000.0, 10_000.0) } else { 0.0 };
        let out_dx = if out_handle[0].is_finite() {
            out_handle[0].clamp(0.05, 200.0)
        } else {
            4.0
        };
        let out_dy = if out_handle[1].is_finite() { out_handle[1].clamp(-10_000.0, 10_000.0) } else { 0.0 };

        // Remove any conflicting keyframe at target frame with different id so times remain unique
        track.keyframes.retain(|k| k.id == key_id || k.frame != clamped_frame);

        let Some(kf) = track.keyframes.iter_mut().find(|k| k.id == key_id) else {
            return false;
        };
        kf.frame = clamped_frame;
        kf.value = clamped_val;
        kf.interpolation = interpolation;
        kf.in_handle = [in_dx, in_dy];
        kf.out_handle = [out_dx, out_dy];
        track.sort_keyframes();
        true
    }

    pub fn delete_keyframe(&mut self, track_id: TrackId, key_id: KeyframeId) -> bool {
        let Some(pos) = self.tracks.iter().position(|t| t.id == track_id) else {
            return false;
        };
        let before = self.tracks[pos].keyframes.len();
        self.tracks[pos].keyframes.retain(|k| k.id != key_id);
        let removed = self.tracks[pos].keyframes.len() != before;
        if self.tracks[pos].keyframes.is_empty() {
            self.tracks.remove(pos);
        }
        removed
    }

    pub fn all_keyframe_frames_sorted(&self) -> Vec<i32> {
        let mut frames: Vec<i32> = self
            .tracks
            .iter()
            .flat_map(|t| t.keyframes.iter().map(|k| k.frame))
            .collect();
        frames.sort_unstable();
        frames.dedup();
        frames
    }

    pub fn step_keyframe_from(&self, current: f32, direction: i32) -> Option<i32> {
        let frames = self.all_keyframe_frames_sorted();
        if frames.is_empty() {
            return None;
        }
        if direction >= 0 {
            let threshold = current.floor() as i32;
            frames.iter().copied().find(|&f| f > threshold)
        } else {
            let threshold = current.ceil() as i32;
            frames.iter().copied().rev().find(|&f| f < threshold)
        }
    }

    /// Advances playback deterministically given a monotonic nanosecond clock sample.
    /// Returns true if the playhead moved.
    pub fn tick_playback(&mut self, frame_time_nanos: u64) -> bool {
        if !self.timeline.playing {
            self.timeline.last_tick_nanos = None;
            return false;
        }
        let prev_nanos = match self.timeline.last_tick_nanos {
            Some(prev) => prev,
            None => {
                self.timeline.last_tick_nanos = Some(frame_time_nanos);
                return false;
            }
        };
        self.timeline.last_tick_nanos = Some(frame_time_nanos);

        let dt_secs = if frame_time_nanos > prev_nanos {
            ((frame_time_nanos - prev_nanos) as f64 * 1e-9).clamp(0.0, 0.1) as f32
        } else {
            0.0
        };
        if dt_secs <= 0.0 {
            return false;
        }

        let start = self.timeline.start_frame as f32;
        let end = self.timeline.end_frame.max(self.timeline.start_frame + 1) as f32;
        let span = (end - start).max(1.0);
        let delta_frames = dt_secs * self.timeline.fps.clamp(1.0, 120.0);
        let mut next = self.timeline.current_frame + delta_frames;

        if next > end {
            if self.timeline.looping {
                next = start + ((next - start) % span);
            } else {
                next = end;
                self.timeline.playing = false;
                self.timeline.last_tick_nanos = None;
            }
        }
        self.timeline.current_frame = next.clamp(start, end);
        true
    }
}
