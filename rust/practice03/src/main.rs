use bytemuck::{Pod, Zeroable};
use glam::Vec2;
use std::fs;
use std::path::Path;
use std::time::Instant;
use std::vec::Vec;

use wgpu_app::{AppConfig, KeyCode, MouseButton, WgpuApp, WgpuState, run};

const PROJECT_ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn load_shader_module(
    device: &wgpu::Device,
    path: impl AsRef<Path>,
) -> std::io::Result<wgpu::ShaderModule> {
    let source = fs::read_to_string(path)?;
    Ok(device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(source.into()),
    }))
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: Vec2,
    color: [u8; 4],
    distance: f32,
}

fn lerp(v0: &Vertex, v1: &Vertex, t: f32) -> Vertex {
    Vertex {
        position: v0.position.lerp(v1.position, t),
        color: v0.color,
        distance: 0.0,
    }
}

fn bezier(vertices: &[Vertex], t: f32) -> Vertex {
    let mut temp = vertices.to_vec();

    for k in (0..vertices.len()).rev() {
        for i in 0..k {
            temp[i] = lerp(&temp[i], &temp[i + 1], t);
        }
    }

    temp[0]
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Immediates {
    view: [f32; 16],
    is_dotted: u32,
    time: f32,
}

struct Practice03 {
    pipeline: wgpu::RenderPipeline,
    last_frame_start: Instant,
    time: f32,
    vertices: Vec<Vertex>,
    buffer: wgpu::Buffer,
    buffer_capacity: u64,
    are_vertices_changed: bool,
    are_bezier_vertices_changed: bool,
    quality: u32,
    bezier_vertices: Vec<Vertex>,
    bezier_buffer: Option<wgpu::Buffer>,
}

impl WgpuApp for Practice03 {
    fn new(app: &WgpuState) -> Self {
        let shader = load_shader_module(&app.device, Path::new(PROJECT_ROOT).join("shader.wgsl"))
            .expect("failed to load shader");

        let pipeline_layout = app
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                immediate_size: std::mem::size_of::<Immediates>() as u32,
                ..Default::default()
            });

        let pipeline = app
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vertexMain"),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x2,
                                offset: 0,
                                shader_location: 0,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Unorm8x4,
                                offset: std::mem::size_of::<Vec2>() as u64,
                                shader_location: 1,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32,
                                offset: std::mem::size_of::<Vec2>() as u64
                                    + std::mem::size_of::<[u8; 4]>() as u64,
                                shader_location: 2,
                            },
                        ],
                    })],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fragmentMain"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: app.surface_format(),
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::LineStrip,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            });

        let vertices = vec![];

        let buffer_capacity = 1;

        let buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: buffer_capacity * std::mem::size_of::<Vertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let quality = 4;

        Self {
            pipeline,
            last_frame_start: Instant::now(),
            time: 0.0,
            vertices: vertices,
            buffer: buffer,
            buffer_capacity: buffer_capacity,
            are_vertices_changed: false,
            are_bezier_vertices_changed: false,
            quality: quality,
            bezier_vertices: Vec::new(),
            bezier_buffer: None,
        }
    }

    fn redraw(&mut self, app: &mut WgpuState) {
        let Some(surface_texture) = app.begin_frame() else {
            return;
        };

        let now = Instant::now();
        let dt = (now - self.last_frame_start).as_secs_f32();
        self.time += dt;
        self.last_frame_start = now;

        let mut immediates = Immediates {
            view: [
                2.0 / (app.width() as f32),
                0.0,
                0.0,
                0.0,
                0.0,
                -2.0 / (app.height() as f32),
                0.0,
                0.0,
                0.0,
                0.0,
                1.0,
                0.0,
                -1.0,
                1.0,
                0.0,
                1.0,
            ],
            is_dotted: 0,
            time: self.time,
        };

        let target_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = app.device.create_command_encoder(&Default::default());

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.07,
                            g: 0.21,
                            b: 0.30,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            render_pass.set_pipeline(&self.pipeline);
            if !self.vertices.is_empty() {
                if self.are_vertices_changed {
                    while self.buffer_capacity < self.vertices.len() as u64 {
                        self.buffer_capacity *= 2;
                    }
                    self.buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
                        label: None,
                        size: self.buffer_capacity * std::mem::size_of::<Vertex>() as u64,
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                    self.are_vertices_changed = false;
                    app.queue
                        .write_buffer(&self.buffer, 0, bytemuck::cast_slice(&self.vertices));
                }
                if self.are_bezier_vertices_changed {
                    let mut distance = 0.0;
                    let mut previous_position: Option<Vec2> = None;
                    self.are_bezier_vertices_changed = false;
                    let vertices_sections_count = (self.vertices.len() - 1) as u32;
                    let bezie_sections_count = vertices_sections_count * self.quality;
                    let bezier_vertices_count = bezie_sections_count + 1;
                    self.bezier_vertices.clear();
                    self.bezier_vertices
                        .extend((0..bezier_vertices_count).map(|i| {
                            let t = i as f32 / bezie_sections_count as f32;
                            let mut v = bezier(&self.vertices, t);
                            v.color = [255, 200, 60, 255];
                            v.distance = distance;
                            if let Some(previous_position) = previous_position {
                                distance += v.position.distance(previous_position)
                            }
                            v.distance = distance;
                            previous_position = Some(v.position);
                            v
                        }));
                    self.bezier_buffer = Some(app.device.create_buffer(&wgpu::BufferDescriptor {
                        label: None,
                        size: self.bezier_vertices.len() as u64
                            * std::mem::size_of::<Vertex>() as u64,
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    }));
                    app.queue.write_buffer(
                        self.bezier_buffer.as_ref().unwrap(),
                        0,
                        bytemuck::cast_slice(&self.bezier_vertices),
                    );
                }
                render_pass.set_immediates(0, bytemuck::bytes_of(&immediates));
                render_pass.set_vertex_buffer(0, self.buffer.slice(..));
                render_pass.draw(0..self.vertices.len() as u32, 0..1);
                if let Some(bezier_buffer) = self.bezier_buffer.as_ref() {
                    immediates.is_dotted = 1;
                    render_pass.set_immediates(0, bytemuck::bytes_of(&immediates));
                    render_pass.set_vertex_buffer(0, bezier_buffer.slice(..));
                    render_pass.draw(0..self.bezier_vertices.len() as u32, 0..1);
                }
            }
        }

        app.queue.submit(std::iter::once(encoder.finish()));
        app.queue.present(surface_texture);
    }

    fn on_keydown(&mut self, gpu: &mut WgpuState, key: KeyCode) {
        if key == KeyCode::ArrowLeft {
            if self.quality > 1 {
                self.quality -= 1;
            }
            self.are_bezier_vertices_changed = true;
        }
        if key == KeyCode::ArrowRight {
            self.quality += 1;
            self.are_bezier_vertices_changed = true;
        }
    }

    fn on_mousedown(&mut self, gpu: &mut WgpuState, button: MouseButton) {
        if button == MouseButton::Left {
            self.vertices.push(Vertex {
                position: Vec2::new(gpu.mouse.x, gpu.mouse.y),
                color: [171, 92, 255, 255],
                distance: 0.0,
            });
            self.are_vertices_changed = true;
            self.are_bezier_vertices_changed = true;
        }
        if button == MouseButton::Right {
            self.vertices.pop();
            self.are_vertices_changed = true;
            self.are_bezier_vertices_changed = true;
        }
    }
}

fn main() {
    run::<Practice03>(AppConfig {
        title: "Practice03",
        width: 1280,
        height: 720,
        srgb: false,
    });
}
