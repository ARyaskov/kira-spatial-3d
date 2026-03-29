use crate::loader::BoundingBox;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewPreset {
    Top,
    Oblique,
    Side,
}

#[derive(Debug, Clone)]
pub struct OrbitCamera {
    pub target: [f32; 3],
    pub distance: f32,
    pub home_distance: f32,
    pub min_distance: f32,
    pub max_distance: f32,
    pub zoom_speed: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub fovy_radians: f32,
    pub z_near: f32,
    pub z_far: f32,
}

impl OrbitCamera {
    pub fn from_bbox(b: BoundingBox) -> Self {
        let radius = b.radius.max(1e-3);
        let distance = (2.0 * radius).max(1e-3);
        Self {
            target: b.center,
            distance,
            home_distance: distance,
            min_distance: (0.03 * radius).max(1e-3),
            max_distance: (80.0 * radius).max(10.0),
            zoom_speed: 0.18,
            yaw: 0.7,
            pitch: 0.5,
            fovy_radians: 45.0_f32.to_radians(),
            z_near: 0.01,
            z_far: (120.0 * radius).max(100.0),
        }
    }

    pub fn orbit(&mut self, dx: f32, dy: f32) {
        self.yaw += dx * 0.01;
        self.pitch = (self.pitch + dy * 0.01).clamp(-1.5, 1.5);
    }

    pub fn zoom(&mut self, delta: f32) {
        let factor = (-delta * self.zoom_speed).exp();
        self.distance = (self.distance * factor).clamp(self.min_distance, self.max_distance);
    }

    pub fn set_preset(&mut self, preset: ViewPreset) {
        match preset {
            ViewPreset::Top => {
                self.yaw = 0.0;
                self.pitch = 1.45;
            }
            ViewPreset::Oblique => {
                self.yaw = 0.7;
                self.pitch = 0.5;
            }
            ViewPreset::Side => {
                self.yaw = 0.0;
                self.pitch = 0.0;
            }
        }
        self.distance = self.home_distance;
    }

    pub fn view_proj(&self, aspect: f32) -> [[f32; 4]; 4] {
        let eye = self.eye_position();
        let view = look_at_rh(eye, self.target, [0.0, 1.0, 0.0]);
        let proj = perspective_rh_zo(self.fovy_radians, aspect, self.z_near, self.z_far);
        mul_mat4(proj, view)
    }

    fn eye_position(&self) -> [f32; 3] {
        let cp = self.pitch.cos();
        let sp = self.pitch.sin();
        let cy = self.yaw.cos();
        let sy = self.yaw.sin();

        [
            self.target[0] + self.distance * cp * cy,
            self.target[1] + self.distance * sp,
            self.target[2] + self.distance * cp * sy,
        ]
    }
}

// Right-handed perspective matrix with depth range 0..1 (WebGPU/WGSL NDC).
// Matrices are stored column-major to match WGSL `mat4x4<f32>` layout.
fn perspective_rh_zo(fovy: f32, aspect: f32, z_near: f32, z_far: f32) -> [[f32; 4]; 4] {
    let f = 1.0 / (0.5 * fovy).tan();
    let nf = 1.0 / (z_near - z_far);
    [
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, z_far * nf, -1.0],
        [0.0, 0.0, z_far * z_near * nf, 0.0],
    ]
}

fn look_at_rh(eye: [f32; 3], center: [f32; 3], up: [f32; 3]) -> [[f32; 4]; 4] {
    let f = normalize(sub(center, eye));
    let s = normalize(cross(f, up));
    let u = cross(s, f);

    [
        [s[0], s[1], s[2], 0.0],
        [u[0], u[1], u[2], 0.0],
        [-f[0], -f[1], -f[2], 0.0],
        [-dot(s, eye), -dot(u, eye), dot(f, eye), 1.0],
    ]
}

// Column-major multiplication: out = a * b
fn mul_mat4(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0.0; 4]; 4];
    for c in 0..4 {
        for r in 0..4 {
            out[c][r] =
                a[0][r] * b[c][0] + a[1][r] * b[c][1] + a[2][r] * b[c][2] + a[3][r] * b[c][3];
        }
    }
    out
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let l2 = dot(v, v);
    if l2 <= 0.0 || !l2.is_finite() {
        return [0.0, 0.0, 1.0];
    }
    let inv = l2.sqrt().recip();
    [v[0] * inv, v[1] * inv, v[2] * inv]
}
