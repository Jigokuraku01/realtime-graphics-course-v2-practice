use std::ops::Mul;
use std::path::Path;
use std::time::Instant;
use std::{f32::consts::PI, fs};

use wgpu_app::{AppConfig, KeyCode, ObjMesh, ObjVertex, WgpuApp, WgpuState, load_obj, run};

#[repr(C)]
#[derive(Clone, Copy)]
struct Mat4(pub [[f32; 4]; 4]); 

unsafe impl bytemuck::Pod for Mat4 {}
unsafe impl bytemuck::Zeroable for Mat4 {}

impl Mat4 {
    fn from_cols(cols: [[f32; 4]; 4]) -> Self {
        Mat4(cols)
    }

    fn identity() -> Self {
        Mat4([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    fn from_translation(x: f32, y: f32, z: f32) -> Self {
        let mut m = Mat4::identity();
        m.0[3] = [x, y, z, 1.0];
        m
    }

    fn from_scale(s: f32) -> Self {
        let mut m = Mat4::identity();
        m.0[0][0] = s;
        m.0[1][1] = s;
        m.0[2][2] = s;
        m
    }

    fn from_rotation_x(a: f32) -> Self {
        let (s, c) = a.sin_cos();
        Mat4([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, c, s, 0.0],
            [0.0, -s, c, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    fn from_rotation_y(a: f32) -> Self {
        let (s, c) = a.sin_cos();
        Mat4([
            [c, 0.0, -s, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [s, 0.0, c, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    fn from_rotation_z(a: f32) -> Self {
        let (s, c) = a.sin_cos();
        Mat4([
            [c, s, 0.0, 0.0],
            [-s, c, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }
}

impl Mul for Mat4 {
    type Output = Mat4;

    fn mul(self, rhs: Mat4) -> Mat4 {
        let mut out = [[0.0f32; 4]; 4];
        for col in 0..4 {
            for row in 0..4 {
                out[col][row] = (0..4)
                    .map(|k| self.0[k][row] * rhs.0[col][k])
                    .sum();
            }
        }
        Mat4(out)
    }
}

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

fn create_depth_buffer(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let depth_buffer = device.create_texture(&wgpu::wgt::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth24Plus,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });

    let depth_buffer_view = depth_buffer.create_view(&wgpu::wgt::TextureViewDescriptor {
        label: None,
        format: Some(wgpu::TextureFormat::Depth24Plus),
        dimension: Some(wgpu::TextureViewDimension::D2),
        aspect: wgpu::TextureAspect::DepthOnly,
        mip_level_count: Some(1),
        array_layer_count: Some(1),
        usage: Some(wgpu::TextureUsages::RENDER_ATTACHMENT),
        ..Default::default()
    });

    (depth_buffer, depth_buffer_view)
}

struct Practice04 {
    pipeline: wgpu::RenderPipeline,
    last_frame_start: Instant,
    time: f32,
    bunny: ObjMesh,
    bunny_x: f32,
    bunny_y: f32,
    vertices_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    depth_buffer: wgpu::Texture,
    depth_buffer_view: wgpu::TextureView,
}

impl WgpuApp for Practice04 {
    fn new(app: &WgpuState) -> Self {
        let shader = load_shader_module(&app.device, Path::new(PROJECT_ROOT).join("shader.wgsl"))
            .expect("failed to load shader");

        let pipeline_layout = app
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                immediate_size: 128,
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
                        array_stride: std::mem::size_of::<ObjVertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            wgpu::VertexAttribute {
                                shader_location: 0,
                                format: wgpu::VertexFormat::Float32x3,
                                offset: 0,
                            },
                            wgpu::VertexAttribute {
                                shader_location: 1,
                                format: wgpu::VertexFormat::Float32x3,
                                offset: std::mem::size_of::<[f32; 3]>() as u64,
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
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: Some(wgpu::Face::Back),
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24Plus,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            });

        let bunny = load_obj(Path::new(PROJECT_ROOT).join("bunny.obj")).unwrap();

        let bunny_x = 0.0;
        let bunny_y: f32 = 0.0;

        let vertices_buffer = app.device.create_buffer(&wgpu::wgt::BufferDescriptor {
            label: None,
            size: (std::mem::size_of::<ObjVertex>() * bunny.vertices.len()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = app.device.create_buffer(&wgpu::wgt::BufferDescriptor {
            label: None,
            size: (std::mem::size_of::<u32>() * bunny.indices.len()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let (depth_buffer, depth_buffer_view) =
            create_depth_buffer(&app.device, app.width(), app.height());

        app.queue
            .write_buffer(&vertices_buffer, 0, bytemuck::cast_slice(&bunny.vertices));
        app.queue
            .write_buffer(&index_buffer, 0, bytemuck::cast_slice(&bunny.indices));
        Self {
            pipeline,
            last_frame_start: Instant::now(),
            time: 0.0,
            bunny,
            bunny_x,
            bunny_y,
            vertices_buffer,
            index_buffer,
            depth_buffer,
            depth_buffer_view,
        }
    }

    fn redraw(&mut self, app: &mut WgpuState) {
        let Some(surface_texture) = app.begin_frame() else {
            return;
        };

        let depth_size = self.depth_buffer.size();
        if depth_size.width != app.width() || depth_size.height != app.height() {
            let (buffer, view) = create_depth_buffer(&app.device, app.width(), app.height());
            self.depth_buffer = buffer;
            self.depth_buffer_view = view;
        }

        let now = Instant::now();
        let dt = (now - self.last_frame_start).as_secs_f32();
        self.time += dt;
        self.last_frame_start = now;
        let speed: f32 = 2.0;
        let scale: f32 = 0.5;

        let angle = self.time;

        let add_bias = dt * speed;

        if app.keydown.contains(&KeyCode::ArrowRight) {
            self.bunny_x += add_bias;
        } else if app.keydown.contains(&KeyCode::ArrowLeft) {
            self.bunny_x -= add_bias;
        } else if app.keydown.contains(&KeyCode::ArrowUp) {
            self.bunny_y += add_bias;
        } else if app.keydown.contains(&KeyCode::ArrowDown) {
            self.bunny_y -= add_bias;
        }

        let offsets = [(-1.2, 0.0), (0.0, 0.0), (1.2, 0.0)];
        let rotations = [
            Mat4::from_rotation_z(angle),
            Mat4::from_rotation_y(angle),
            Mat4::from_rotation_x(angle),
        ];

        let model_matrices: Vec<Mat4> = offsets
            .iter()
            .zip(rotations.iter())
            .map(|(&(ox, oy), &rotation)| {
                Mat4::from_translation(self.bunny_x + ox, self.bunny_y + oy, 0.0) * rotation * Mat4::from_scale(scale)
            })
            .collect();

        let view_matrix = Mat4::from_cols([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, -3.0, 1.0],
        ]);

        let near = 0.1;
        let far = 100.0;
        let angle_right_seen = PI / 4.0;
        let angle_left_seen = -PI / 4.0;
        let angle_top_seen = PI / 4.0;
        let angle_bottom_seen = -PI / 4.0;
        let right = angle_right_seen.tan() * near;
        let left = angle_left_seen.tan() * near;
        let top = angle_top_seen.tan() * near;
        let bottom = angle_bottom_seen.tan() * near;

        let projection_matrix = Mat4::from_cols([
            [2.0 * near / (right - left), 0.0, 0.0, 0.0],
            [0.0, 2.0 * near / (top - bottom), 0.0, 0.0],
            [
                (right + left) / (right - left),
                (top + bottom) / (top - bottom),
                far / (near - far),
                -1.0,
            ],
            [0.0, 0.0, near * far / (near - far), 0.0],
        ]);
        let view_projection_matrix = projection_matrix * view_matrix;

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
                            r: 0.01,
                            g: 0.02,
                            b: 0.03,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_buffer_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_immediates(64, bytemuck::bytes_of(&view_projection_matrix));
            render_pass.set_vertex_buffer(0, self.vertices_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            for model_matrix in &model_matrices {
                render_pass.set_immediates(0, bytemuck::bytes_of(model_matrix));
                render_pass.draw_indexed(0..(self.bunny.indices.len() as u32), 0, 0..1);
            }
        }

        app.queue.submit(std::iter::once(encoder.finish()));
        app.queue.present(surface_texture);
    }
}

fn main() {
    run::<Practice04>(AppConfig {
        title: "Practice04",
        width: 1280,
        height: 720,
        srgb: true,
    });
}
