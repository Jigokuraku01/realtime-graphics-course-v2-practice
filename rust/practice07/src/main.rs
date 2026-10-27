use glam::{Mat4, Vec3};
use std::f32::consts::PI;
use std::fs;
use std::mem;
use std::path::Path;
use std::time::Instant;
use wgpu::util::DeviceExt;

use wgpu_app::{AppConfig, KeyCode, ObjVertex, WgpuApp, WgpuState, load_obj, run};

const PROJECT_ROOT: &str = env!("CARGO_MANIFEST_DIR");

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniforms {
    view_projection: Mat4,
    position: Vec3,
    padding: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct LightsUniforms {
    ambient_intensity: Vec3,
    _padding0: f32,
    dir_light_direction: Vec3,
    dir_light_intensity: f32,
    light_position: Vec3,
    brightness: f32,
    attenuation: Vec3,
    _padding1: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Immediates {
    model: Mat4,
    albedo: Vec3,
    roughness: f32,
    specular_factor: f32,
    padding: Vec3,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct LightImmediates {
    position: Vec3,
    _padding: f32,
}

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

struct Practice07 {
    depth_buffer: Option<wgpu::Texture>,
    depth_buffer_view: Option<wgpu::TextureView>,
    camera_uniform_buffer: wgpu::Buffer,
    lights_uniform_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    lights_bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
    light_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    last_frame_start: Instant,
    time: f32,
    camera_distance: f32,
    camera_rotation: f32,
    point_light_enabled: bool,
}

fn load_buffer<T: bytemuck::Pod>(
    device: &wgpu::Device,
    data: &[T],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(data),
        usage: wgpu::BufferUsages::COPY_DST | usage,
    })
}

impl WgpuApp for Practice07 {
    fn new(app: &WgpuState) -> Self {
        let shader = load_shader_module(&app.device, Path::new(PROJECT_ROOT).join("shader.wgsl"))
            .expect("failed to load shader");

        let light_shader = load_shader_module(
            &app.device,
            Path::new(PROJECT_ROOT).join("light_shader.wgsl"),
        )
        .expect("failed to load shader");

        let camera_bind_group_layout =
            app.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        let lights_bind_group_layout =
            app.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        let camera_uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: mem::size_of::<CameraUniforms>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });

        let lights_uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: mem::size_of::<LightsUniforms>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });

        let camera_bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_uniform_buffer.as_entire_binding(),
            }],
        });

        let lights_bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &lights_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: lights_uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = app
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[
                    Some(&camera_bind_group_layout),
                    Some(&lights_bind_group_layout),
                ],
                immediate_size: mem::size_of::<Immediates>() as u32,
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
                        array_stride: mem::size_of::<ObjVertex>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            wgpu::VertexAttribute {
                                offset: mem::offset_of!(ObjVertex, position) as wgpu::BufferAddress,
                                shader_location: 0,
                                format: wgpu::VertexFormat::Float32x3,
                            },
                            wgpu::VertexAttribute {
                                offset: mem::offset_of!(ObjVertex, normal) as wgpu::BufferAddress,
                                shader_location: 1,
                                format: wgpu::VertexFormat::Float32x3,
                            },
                            wgpu::VertexAttribute {
                                offset: mem::offset_of!(ObjVertex, texcoord) as wgpu::BufferAddress,
                                shader_location: 2,
                                format: wgpu::VertexFormat::Float32x2,
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

        let light_pipeline_layout =
            app.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    bind_group_layouts: &[Some(&camera_bind_group_layout)],
                    immediate_size: mem::size_of::<LightImmediates>() as u32,
                    ..Default::default()
                });

        let light_pipeline = app
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&light_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &light_shader,
                    entry_point: Some("vertexMain"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &light_shader,
                    entry_point: Some("fragmentMain"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: app.surface_format(),
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::LineList,
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

        let suzanne = load_obj(Path::new(PROJECT_ROOT).join("suzanne.obj")).unwrap();

        let vertex_buffer = load_buffer(&app.device, &suzanne.vertices, wgpu::BufferUsages::VERTEX);
        let index_buffer = load_buffer(&app.device, &suzanne.indices, wgpu::BufferUsages::INDEX);

        Self {
            depth_buffer: None,
            depth_buffer_view: None,
            camera_uniform_buffer,
            lights_uniform_buffer,
            camera_bind_group,
            lights_bind_group,
            pipeline,
            light_pipeline,
            vertex_buffer,
            index_buffer,
            index_count: suzanne.indices.len() as u32,
            last_frame_start: Instant::now(),
            time: 0.0,
            camera_distance: 9.0,
            camera_rotation: 0.0,
            point_light_enabled: true,
        }
    }

    fn on_keydown(&mut self, _app: &mut WgpuState, key: KeyCode) {
        if key == KeyCode::Space {
            self.point_light_enabled = !self.point_light_enabled;
        }
    }

    fn redraw(&mut self, app: &mut WgpuState) {
        let Some(surface_texture) = app.begin_frame() else {
            return;
        };

        if self.depth_buffer.as_ref().is_none_or(|depth_buffer| {
            depth_buffer.width() != app.width() || depth_buffer.height() != app.height()
        }) {
            let new_depth_buffer = app.device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                dimension: wgpu::TextureDimension::D2,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TRANSIENT_ATTACHMENT,
                size: wgpu::Extent3d {
                    width: app.width(),
                    height: app.height(),
                    depth_or_array_layers: 1,
                },
                format: wgpu::TextureFormat::Depth24Plus,
                mip_level_count: 1,
                sample_count: 1,
                view_formats: &[],
            });

            let new_depth_buffer_view =
                new_depth_buffer.create_view(&wgpu::TextureViewDescriptor::default());

            self.depth_buffer = Some(new_depth_buffer);
            self.depth_buffer_view = Some(new_depth_buffer_view);
        }

        let now = Instant::now();
        let dt = (now - self.last_frame_start).as_secs_f32();
        self.time += dt;
        self.last_frame_start = now;

        if app.keydown.contains(&KeyCode::ArrowLeft) {
            self.camera_rotation += 5.0 * dt;
        }

        if app.keydown.contains(&KeyCode::ArrowRight) {
            self.camera_rotation -= 5.0 * dt;
        }

        if app.keydown.contains(&KeyCode::ArrowUp) {
            self.camera_distance += 7.0 * dt;
        }

        if app.keydown.contains(&KeyCode::ArrowDown) {
            self.camera_distance -= 7.0 * dt;
        }
        self.camera_distance = self.camera_distance.clamp(3.0, 50.0);

        let view_matrix = Mat4::from_translation(Vec3::new(0.0, 0.0, -self.camera_distance))
            * Mat4::from_rotation_y(-self.camera_rotation);

        let projection_matrix = Mat4::perspective_rh(
            PI / 3.0,
            app.width() as f32 / app.height() as f32,
            0.01,
            100.0,
        );

        let target_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let camera_uniforms = CameraUniforms {
            view_projection: projection_matrix * view_matrix,
            position: Vec3::new(
                self.camera_distance * self.camera_rotation.sin(),
                0.0,
                self.camera_distance * self.camera_rotation.cos(),
            ),

            padding: 0.0,
        };

        app.queue.write_buffer(
            &self.camera_uniform_buffer,
            0,
            bytemuck::bytes_of(&camera_uniforms),
        );
        let light_position = Vec3::new(
            2.0 * (self.time * 1.7).cos(),
            1.3 * (self.time).sin(),
            2.0 * (self.time).sin(),
        );

        let lights_uniforms = LightsUniforms {
            ambient_intensity: Vec3::new(0.01, 0.01, 0.01),
            _padding0: 0.0,
            dir_light_direction: Vec3::new(self.time.cos() * 0.5, 1.0, self.time.sin() * 0.5)
                .normalize(),
            dir_light_intensity: 0.3,
            light_position,
            brightness: if self.point_light_enabled { 1.0 } else { 0.0 },
            attenuation: Vec3::new(1.0, 0.09, 0.032),
            _padding1: 0.0,
        };

        app.queue.write_buffer(
            &self.lights_uniform_buffer,
            0,
            bytemuck::bytes_of(&lights_uniforms),
        );

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
                            g: 0.03,
                            b: 0.05,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: self.depth_buffer_view.as_ref().unwrap(),
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            render_pass.set_pipeline(&self.pipeline);

            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.lights_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

            const ROUGHNESSES: [f32; 3] = [0.2, 0.45, 0.75];
            const SPECULAR_FACTORS: [f32; 3] = [0.15, 0.55, 1.0];
            let stride = 3.0;

            for (row, &roughness) in ROUGHNESSES.iter().enumerate() {
                for (column, &specular_factor) in SPECULAR_FACTORS.iter().enumerate() {
                    let model_matrix = Mat4::from_translation(Vec3::new(
                        (column as f32 - 1.0) * stride,
                        (1.0 - row as f32) * stride,
                        0.0,
                    ));

                    let immediates = Immediates {
                        model: model_matrix,
                        albedo: Vec3::new(0.16, 0.58, 0.48),
                        padding: Vec3 {
                            x: 0.0,
                            y: 0.0,
                            z: 0.0,
                        },
                        roughness,
                        specular_factor,
                    };

                    render_pass.set_immediates(0, bytemuck::bytes_of(&immediates));
                    render_pass.draw_indexed(0..self.index_count, 0, 0..1);
                }
            }

            render_pass.set_pipeline(&self.light_pipeline);
            let light_immediates = LightImmediates {
                position: light_position,
                _padding: 0.0,
            };
            render_pass.set_immediates(0, bytemuck::bytes_of(&light_immediates));
            render_pass.draw(0..6, 0..1);
        }

        app.queue.submit(std::iter::once(encoder.finish()));
        app.queue.present(surface_texture);
    }
}

fn main() {
    run::<Practice07>(AppConfig {
        title: "Practice07",
        width: 1280,
        height: 720,
        srgb: true,
    });
}
