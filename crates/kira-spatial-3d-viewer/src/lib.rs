use std::fmt;
use std::path::Path;

use winit::dpi::PhysicalSize;
use winit::event::{ElementState, Event, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::WindowBuilder;

pub mod camera;
pub mod loader;
pub mod renderer;

use camera::{OrbitCamera, ViewPreset};
use loader::{compute_bounding_box, load_mesh_prefix, load_polyline_layers};
use renderer::Renderer;

#[derive(Debug)]
pub enum ViewerError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Data(String),
    Surface(wgpu::SurfaceError),
    EventLoop(String),
}

impl fmt::Display for ViewerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::Json(e) => write!(f, "json error: {e}"),
            Self::Data(e) => write!(f, "data error: {e}"),
            Self::Surface(e) => write!(f, "surface error: {e}"),
            Self::EventLoop(e) => write!(f, "event loop error: {e}"),
        }
    }
}

impl std::error::Error for ViewerError {}

impl From<std::io::Error> for ViewerError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for ViewerError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

pub fn run_viewer(mesh_prefix: &Path, polylines_path: Option<&Path>) -> Result<(), ViewerError> {
    let mesh = load_mesh_prefix(mesh_prefix)?;
    let (line_layers, line_idx) = if let Some(p) = polylines_path {
        load_polyline_layers(p)?
    } else {
        (Vec::new(), 0)
    };

    let bbox = compute_bounding_box(&mesh.positions);
    let mut camera = OrbitCamera::from_bbox(bbox);
    let mut base_z_exaggeration = suggested_z_exaggeration(bbox);
    let mut sensitivity = SensitivityMode::Normal;
    let mut use_colormap = true;

    let event_loop = EventLoop::new().map_err(|e| ViewerError::EventLoop(e.to_string()))?;
    let window = WindowBuilder::new()
        .with_title("kira-spatial-3d-viewer")
        .with_inner_size(PhysicalSize::new(1280, 720))
        .build(&event_loop)
        .map_err(|e| ViewerError::EventLoop(e.to_string()))?;

    let mut renderer = pollster::block_on(Renderer::new(&window, &mesh, &line_layers, line_idx))?;
    update_window_title(
        &window,
        sensitivity,
        base_z_exaggeration,
        renderer.line_levels(),
        renderer.active_line(),
        renderer.lines_visible(),
        renderer.show_vectors(),
        use_colormap,
    );
    renderer.update_uniforms(
        camera.view_proj(renderer.aspect()),
        mesh.normals.is_some(),
        bbox.min[2],
        bbox.max[2],
        effective_z(base_z_exaggeration, sensitivity),
        use_colormap,
        sensitivity.gamma(),
    );

    let mut mouse_down = false;
    let mut last_cursor: Option<(f64, f64)> = None;

    event_loop
        .run(|event, elwt| {
            elwt.set_control_flow(ControlFlow::Poll);

            match event {
                Event::WindowEvent { event, window_id } if window_id == window.id() => {
                    match event {
                        WindowEvent::CloseRequested => elwt.exit(),
                        WindowEvent::Resized(size) => {
                            renderer.resize(size);
                            renderer.update_uniforms(
                                camera.view_proj(renderer.aspect()),
                                mesh.normals.is_some(),
                                bbox.min[2],
                                bbox.max[2],
                                effective_z(base_z_exaggeration, sensitivity),
                                use_colormap,
                                sensitivity.gamma(),
                            );
                        }
                        WindowEvent::MouseInput {
                            state,
                            button: MouseButton::Left,
                            ..
                        } => {
                            mouse_down = state == ElementState::Pressed;
                            if !mouse_down {
                                last_cursor = None;
                            }
                        }
                        WindowEvent::CursorMoved { position, .. } => {
                            if mouse_down {
                                if let Some((lx, ly)) = last_cursor {
                                    camera
                                        .orbit((position.x - lx) as f32, (position.y - ly) as f32);
                                    renderer.update_uniforms(
                                        camera.view_proj(renderer.aspect()),
                                        mesh.normals.is_some(),
                                        bbox.min[2],
                                        bbox.max[2],
                                        effective_z(base_z_exaggeration, sensitivity),
                                        use_colormap,
                                        sensitivity.gamma(),
                                    );
                                }
                                last_cursor = Some((position.x, position.y));
                            }
                        }
                        WindowEvent::MouseWheel { delta, .. } => {
                            let dy = match delta {
                                MouseScrollDelta::LineDelta(_, y) => y,
                                MouseScrollDelta::PixelDelta(p) => p.y as f32 * 0.05,
                            };
                            camera.zoom(dy);
                            renderer.update_uniforms(
                                camera.view_proj(renderer.aspect()),
                                mesh.normals.is_some(),
                                bbox.min[2],
                                bbox.max[2],
                                effective_z(base_z_exaggeration, sensitivity),
                                use_colormap,
                                sensitivity.gamma(),
                            );
                        }
                        WindowEvent::KeyboardInput { event, .. } => {
                            if event.state == ElementState::Pressed && !event.repeat {
                                let preset = match event.physical_key {
                                    PhysicalKey::Code(KeyCode::Digit1) => Some(ViewPreset::Top),
                                    PhysicalKey::Code(KeyCode::Digit2) => Some(ViewPreset::Oblique),
                                    PhysicalKey::Code(KeyCode::Digit3) => Some(ViewPreset::Side),
                                    _ => None,
                                };
                                if let Some(preset) = preset {
                                    camera.set_preset(preset);
                                    renderer.update_uniforms(
                                        camera.view_proj(renderer.aspect()),
                                        mesh.normals.is_some(),
                                        bbox.min[2],
                                        bbox.max[2],
                                        effective_z(base_z_exaggeration, sensitivity),
                                        use_colormap,
                                        sensitivity.gamma(),
                                    );
                                    update_window_title(
                                        &window,
                                        sensitivity,
                                        base_z_exaggeration,
                                        renderer.line_levels(),
                                        renderer.active_line(),
                                        renderer.lines_visible(),
                                        renderer.show_vectors(),
                                        use_colormap,
                                    );
                                    return;
                                }

                                match event.physical_key {
                                    PhysicalKey::Code(KeyCode::Equal)
                                    | PhysicalKey::Code(KeyCode::NumpadAdd) => {
                                        base_z_exaggeration =
                                            (base_z_exaggeration * 1.25).clamp(1.0, 512.0);
                                        eprintln!(
                                            "z_exaggeration={:.3}",
                                            effective_z(base_z_exaggeration, sensitivity)
                                        );
                                        renderer.update_uniforms(
                                            camera.view_proj(renderer.aspect()),
                                            mesh.normals.is_some(),
                                            bbox.min[2],
                                            bbox.max[2],
                                            effective_z(base_z_exaggeration, sensitivity),
                                            use_colormap,
                                            sensitivity.gamma(),
                                        );
                                    }
                                    PhysicalKey::Code(KeyCode::Minus)
                                    | PhysicalKey::Code(KeyCode::NumpadSubtract) => {
                                        base_z_exaggeration =
                                            (base_z_exaggeration / 1.25).clamp(1.0, 512.0);
                                        eprintln!(
                                            "z_exaggeration={:.3}",
                                            effective_z(base_z_exaggeration, sensitivity)
                                        );
                                        renderer.update_uniforms(
                                            camera.view_proj(renderer.aspect()),
                                            mesh.normals.is_some(),
                                            bbox.min[2],
                                            bbox.max[2],
                                            effective_z(base_z_exaggeration, sensitivity),
                                            use_colormap,
                                            sensitivity.gamma(),
                                        );
                                    }
                                    PhysicalKey::Code(KeyCode::KeyC) => {
                                        use_colormap = !use_colormap;
                                        eprintln!("colormap={use_colormap}");
                                        renderer.update_uniforms(
                                            camera.view_proj(renderer.aspect()),
                                            mesh.normals.is_some(),
                                            bbox.min[2],
                                            bbox.max[2],
                                            effective_z(base_z_exaggeration, sensitivity),
                                            use_colormap,
                                            sensitivity.gamma(),
                                        );
                                    }
                                    PhysicalKey::Code(KeyCode::KeyQ) => {
                                        sensitivity = sensitivity.prev();
                                        eprintln!("sensitivity={}", sensitivity.label());
                                        renderer.update_uniforms(
                                            camera.view_proj(renderer.aspect()),
                                            mesh.normals.is_some(),
                                            bbox.min[2],
                                            bbox.max[2],
                                            effective_z(base_z_exaggeration, sensitivity),
                                            use_colormap,
                                            sensitivity.gamma(),
                                        );
                                    }
                                    PhysicalKey::Code(KeyCode::KeyE) => {
                                        sensitivity = sensitivity.next();
                                        eprintln!("sensitivity={}", sensitivity.label());
                                        renderer.update_uniforms(
                                            camera.view_proj(renderer.aspect()),
                                            mesh.normals.is_some(),
                                            bbox.min[2],
                                            bbox.max[2],
                                            effective_z(base_z_exaggeration, sensitivity),
                                            use_colormap,
                                            sensitivity.gamma(),
                                        );
                                    }
                                    PhysicalKey::Code(KeyCode::PageUp)
                                    | PhysicalKey::Code(KeyCode::Period) => {
                                        if !renderer.line_levels().is_empty() {
                                            let n = renderer.line_levels().len();
                                            let next = (renderer.active_line() + 1) % n;
                                            renderer.set_active_line(next);
                                        }
                                    }
                                    PhysicalKey::Code(KeyCode::PageDown)
                                    | PhysicalKey::Code(KeyCode::Comma) => {
                                        if !renderer.line_levels().is_empty() {
                                            let n = renderer.line_levels().len();
                                            let prev = (renderer.active_line() + n - 1) % n;
                                            renderer.set_active_line(prev);
                                        }
                                    }
                                    PhysicalKey::Code(KeyCode::KeyL) => {
                                        renderer.toggle_lines_visible();
                                    }
                                    PhysicalKey::Code(KeyCode::KeyV) => {
                                        renderer.toggle_show_vectors();
                                    }
                                    _ => {}
                                }
                                update_window_title(
                                    &window,
                                    sensitivity,
                                    base_z_exaggeration,
                                    renderer.line_levels(),
                                    renderer.active_line(),
                                    renderer.lines_visible(),
                                    renderer.show_vectors(),
                                    use_colormap,
                                );
                            }
                        }
                        WindowEvent::RedrawRequested => {
                            if let Err(err) = renderer.render() {
                                eprintln!("{err}");
                            }
                        }
                        _ => {}
                    }
                }
                Event::AboutToWait => {
                    window.request_redraw();
                }
                _ => {}
            }
        })
        .map_err(|e| ViewerError::EventLoop(e.to_string()))
}

fn suggested_z_exaggeration(bbox: loader::BoundingBox) -> f32 {
    let dx = (bbox.max[0] - bbox.min[0]).abs();
    let dy = (bbox.max[1] - bbox.min[1]).abs();
    let dz = (bbox.max[2] - bbox.min[2]).abs().max(1e-6);
    let xy = dx.max(dy).max(1.0);
    ((xy / dz) * 0.05).clamp(1.0, 128.0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SensitivityMode {
    Low,
    Normal,
    High,
}

impl SensitivityMode {
    fn gamma(self) -> f32 {
        match self {
            Self::Low => 1.35,
            Self::Normal => 1.0,
            Self::High => 0.72,
        }
    }

    fn z_mult(self) -> f32 {
        match self {
            Self::Low => 0.8,
            Self::Normal => 1.0,
            Self::High => 1.6,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::High => "high",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Low => Self::Normal,
            Self::Normal => Self::High,
            Self::High => Self::Low,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::Low => Self::High,
            Self::Normal => Self::Low,
            Self::High => Self::Normal,
        }
    }
}

fn effective_z(base: f32, sensitivity: SensitivityMode) -> f32 {
    (base * sensitivity.z_mult()).clamp(1.0, 512.0)
}

fn update_window_title(
    window: &winit::window::Window,
    sensitivity: SensitivityMode,
    base_z: f32,
    line_levels: &[f32],
    active_line: usize,
    lines_visible: bool,
    show_vectors: bool,
    use_colormap: bool,
) {
    let level_text = if line_levels.is_empty() {
        "off".to_string()
    } else {
        let idx = active_line.min(line_levels.len() - 1);
        format!(
            "{:.3} ({}/{})",
            line_levels[idx],
            idx + 1,
            line_levels.len()
        )
    };
    let title = format!(
        "kira-spatial-3d-viewer | View[1/2/3] Sens[Q/E]={} Z[+/-]={:.2} Level[,/.]={} Lines[L]={} Vectors[V]={} Cmap[C]={}",
        sensitivity.label(),
        effective_z(base_z, sensitivity),
        level_text,
        if lines_visible { "on" } else { "off" },
        if show_vectors { "on" } else { "off" },
        if use_colormap { "on" } else { "off" },
    );
    window.set_title(&title);
}
