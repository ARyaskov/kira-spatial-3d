use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::ViewerError;
use crate::loader::{MeshData, PolylineLayer};

pub struct Renderer<'w> {
    surface: wgpu::Surface<'w>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: PhysicalSize<u32>,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,

    uniform_buf: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,

    surface_pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,

    position_buf: wgpu::Buffer,
    normal_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
    index_count: u32,

    line_bufs: Vec<wgpu::Buffer>,
    line_counts: Vec<u32>,
    line_levels: Vec<f32>,
    active_line: usize,
    lines_visible: bool,
    show_vectors: bool,
}

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    mvp: [[f32; 4]; 4],
    light_dir: [f32; 4],
    z_params: [f32; 4], // z_min, z_max, z_exaggeration, gamma
    flags: [u32; 4],    // has_normals, use_colormap, _, _
}

impl<'w> Renderer<'w> {
    pub async fn new(
        window: &'w Window,
        mesh: &MeshData,
        lines: &[PolylineLayer],
        active_line: usize,
    ) -> Result<Self, ViewerError> {
        let size = window.inner_size();
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(window)
            .map_err(|e| ViewerError::Data(format!("surface creation failed: {e}")))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| ViewerError::Data("no suitable GPU adapter found".to_string()))?;
        let required_limits = adapter.limits();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("kira-spatial-3d-viewer device"),
                    required_features: wgpu::Features::empty(),
                    required_limits,
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .map_err(|e| ViewerError::Data(format!("request_device failed: {e}")))?;

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);
        let (depth_texture, depth_view) = Self::create_depth_target(&device, &config);

        let has_normals = mesh.normals.is_some();
        let uniforms = Uniforms {
            mvp: [[0.0; 4]; 4],
            light_dir: [0.5, 0.8, 0.3, 0.0],
            z_params: [mesh.bbox.min[2], mesh.bbox.max[2], 1.0, 0.0],
            flags: [u32::from(has_normals), 1, 0, 0],
        };
        let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("uniform buffer"),
            contents: bytemuck::bytes_of(&uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("uniform bind layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("uniform bind group"),
            layout: &bind_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("surface pipeline layout"),
            bind_group_layouts: &[&bind_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("surface shader"),
            source: wgpu::ShaderSource::Wgsl(SURFACE_WGSL.into()),
        });

        let surface_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("surface pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<[f32; 3]>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x3,
                        }],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<[f32; 3]>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x3,
                        }],
                    },
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let line_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("line shader"),
            source: wgpu::ShaderSource::Wgsl(LINE_WGSL.into()),
        });

        let line_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("line pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &line_shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<[f32; 3]>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float32x3,
                    }],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &line_shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let position_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("position buffer"),
            contents: bytemuck::cast_slice(&mesh.positions),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let normals_owned;
        let normal_slice: &[[f32; 3]] = if let Some(ns) = &mesh.normals {
            ns
        } else {
            normals_owned = vec![[0.0_f32, 0.0, 0.0]; mesh.positions.len()];
            &normals_owned
        };
        let normal_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("normal buffer"),
            contents: bytemuck::cast_slice(normal_slice),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("index buffer"),
            contents: bytemuck::cast_slice(&mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let mut line_bufs = Vec::<wgpu::Buffer>::with_capacity(lines.len());
        let mut line_counts = Vec::<u32>::with_capacity(lines.len());
        let mut line_levels = Vec::<f32>::with_capacity(lines.len());
        for layer in lines {
            let b = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("line buffer"),
                contents: bytemuck::cast_slice(&layer.points),
                usage: wgpu::BufferUsages::VERTEX,
            });
            line_bufs.push(b);
            line_counts.push(layer.points.len() as u32);
            line_levels.push(layer.level);
        }

        Ok(Self {
            surface,
            device,
            queue,
            config,
            size,
            depth_texture,
            depth_view,
            uniform_buf,
            uniform_bind_group,
            surface_pipeline,
            line_pipeline,
            position_buf,
            normal_buf,
            index_buf,
            index_count: mesh.indices.len() as u32,
            line_bufs,
            line_counts,
            line_levels,
            active_line: active_line.min(lines.len().saturating_sub(1)),
            lines_visible: true,
            show_vectors: true,
        })
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.size = new_size;
        self.config.width = new_size.width.max(1);
        self.config.height = new_size.height.max(1);
        self.surface.configure(&self.device, &self.config);
        let (depth_texture, depth_view) = Self::create_depth_target(&self.device, &self.config);
        self.depth_texture = depth_texture;
        self.depth_view = depth_view;
    }

    pub fn update_uniforms(
        &self,
        mvp: [[f32; 4]; 4],
        has_normals: bool,
        z_min: f32,
        z_max: f32,
        z_exaggeration: f32,
        use_colormap: bool,
        gamma: f32,
    ) {
        let u = Uniforms {
            mvp,
            light_dir: [0.5, 0.8, 0.3, 0.0],
            z_params: [
                z_min,
                z_max,
                z_exaggeration.max(1e-3),
                gamma.clamp(0.2, 4.0),
            ],
            flags: [u32::from(has_normals), u32::from(use_colormap), 0, 0],
        };
        self.queue
            .write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(&u));
    }

    pub fn render(&mut self) -> Result<(), ViewerError> {
        let frame = match self.surface.get_current_texture() {
            Ok(v) => v,
            Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                self.surface.configure(&self.device, &self.config);
                self.surface
                    .get_current_texture()
                    .map_err(ViewerError::Surface)?
            }
            Err(e) => return Err(ViewerError::Surface(e)),
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.08,
                            g: 0.09,
                            b: 0.1,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            if self.index_count > 0 {
                pass.set_pipeline(&self.surface_pipeline);
                pass.set_vertex_buffer(0, self.position_buf.slice(..));
                pass.set_vertex_buffer(1, self.normal_buf.slice(..));
                pass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..self.index_count, 0, 0..1);
            }

            if self.show_vectors && self.lines_visible && !self.line_bufs.is_empty() {
                let idx = self.active_line.min(self.line_bufs.len() - 1);
                if self.line_counts[idx] > 0 {
                    pass.set_pipeline(&self.line_pipeline);
                    pass.set_vertex_buffer(0, self.line_bufs[idx].slice(..));
                    pass.draw(0..self.line_counts[idx], 0..1);
                }
            }
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
        Ok(())
    }

    pub fn aspect(&self) -> f32 {
        self.config.width as f32 / self.config.height.max(1) as f32
    }

    pub fn set_active_line(&mut self, idx: usize) {
        if !self.line_bufs.is_empty() {
            self.active_line = idx.min(self.line_bufs.len() - 1);
        }
    }

    pub fn line_levels(&self) -> &[f32] {
        &self.line_levels
    }

    pub fn active_line(&self) -> usize {
        self.active_line
    }

    pub fn toggle_lines_visible(&mut self) {
        self.lines_visible = !self.lines_visible;
    }

    pub fn lines_visible(&self) -> bool {
        self.lines_visible
    }

    pub fn toggle_show_vectors(&mut self) {
        self.show_vectors = !self.show_vectors;
    }

    pub fn show_vectors(&self) -> bool {
        self.show_vectors
    }

    fn create_depth_target(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth texture"),
            size: wgpu::Extent3d {
                width: config.width.max(1),
                height: config.height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
        (depth_texture, depth_view)
    }
}

const SURFACE_WGSL: &str = r#"
struct Uniforms {
    mvp: mat4x4<f32>,
    light_dir: vec4<f32>,
    z_params: vec4<f32>,
    flags: vec4<u32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VsIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) height01: f32,
};

fn apply_height_exaggeration(position: vec3<f32>) -> vec3<f32> {
    let z_min = uniforms.z_params.x;
    let z_max = uniforms.z_params.y;
    let z_scale = uniforms.z_params.z;
    let z_mid = 0.5 * (z_min + z_max);
    let z = z_mid + (position.z - z_mid) * z_scale;
    return vec3<f32>(position.x, position.y, z);
}

fn normalize_height01(z: f32) -> f32 {
    let z_min = uniforms.z_params.x;
    let z_max = uniforms.z_params.y;
    let denom = max(z_max - z_min, 1e-6);
    return clamp((z - z_min) / denom, 0.0, 1.0);
}

@vertex
fn vs_main(v: VsIn) -> VsOut {
    var o: VsOut;
    let p = apply_height_exaggeration(v.position);
    o.clip = uniforms.mvp * vec4<f32>(p, 1.0);
    o.world_pos = p;
    o.normal = v.normal;
    o.height01 = normalize_height01(v.position.z);
    return o;
}

fn height_colormap(t_in: f32) -> vec3<f32> {
    let gamma = uniforms.z_params.w;
    let t = pow(clamp(t_in, 0.0, 1.0), gamma);
    let c0 = vec3<f32>(0.12, 0.19, 0.72);
    let c1 = vec3<f32>(0.11, 0.72, 0.71);
    let c2 = vec3<f32>(0.97, 0.90, 0.23);
    let c3 = vec3<f32>(0.86, 0.12, 0.14);
    if (t < 0.33) {
        return mix(c0, c1, t / 0.33);
    }
    if (t < 0.66) {
        return mix(c1, c2, (t - 0.33) / 0.33);
    }
    return mix(c2, c3, (t - 0.66) / 0.34);
}

@fragment
fn fs_main(v: VsOut) -> @location(0) vec4<f32> {
    var n: vec3<f32>;
    if (uniforms.flags.x == 1u && length(v.normal) > 0.0) {
        n = normalize(v.normal);
    } else {
        n = normalize(cross(dpdx(v.world_pos), dpdy(v.world_pos)));
    }

    let light = normalize(uniforms.light_dir.xyz);
    let lambert = max(dot(n, light), 0.0);
    let view_dir = normalize(-v.world_pos);
    let half_vec = normalize(light + view_dir);
    let spec = pow(max(dot(n, half_vec), 0.0), 32.0) * 0.18;
    var base = vec3<f32>(0.58, 0.61, 0.66);
    if (uniforms.flags.y == 1u) {
        base = height_colormap(v.height01);
    }
    let ambient = base * 0.30;
    let diffuse = base * lambert * 0.95;
    let fresnel = pow(1.0 - max(dot(n, view_dir), 0.0), 3.0) * 0.2;
    return vec4<f32>(ambient + diffuse + vec3<f32>(spec + fresnel), 1.0);
}
"#;

const LINE_WGSL: &str = r#"
struct Uniforms {
    mvp: mat4x4<f32>,
    light_dir: vec4<f32>,
    z_params: vec4<f32>,
    flags: vec4<u32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VsIn {
    @location(0) position: vec3<f32>,
};

@vertex
fn vs_main(v: VsIn) -> @builtin(position) vec4<f32> {
    let z_min = uniforms.z_params.x;
    let z_max = uniforms.z_params.y;
    let z_scale = uniforms.z_params.z;
    let z_mid = 0.5 * (z_min + z_max);
    let z = z_mid + (v.position.z - z_mid) * z_scale;
    return uniforms.mvp * vec4<f32>(v.position.x, v.position.y, z, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.23, 0.42, 0.78);
}
"#;
