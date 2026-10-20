use glam::{Mat4, Vec3};
use std::f32::consts::PI;
use std::fs;
use std::mem;
use std::path::Path;
use std::time::Instant;
use wgpu::SamplerBindingType::Filtering;
use wgpu::util::DeviceExt;

use wgpu_app::{AppConfig, KeyCode, ObjVertex, WgpuApp, WgpuState, load_obj, run};

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

struct Practice05 {
    depth_buffer: Option<wgpu::Texture>,
    depth_buffer_view: Option<wgpu::TextureView>,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    texture: wgpu::Texture,
    texture_view: wgpu::TextureView,
    last_frame_start: Instant,
    time: f32,
    camera_distance: f32,
    model_rotation: f32,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct Uniforms {
    model: Mat4,
    view_projection: Mat4,
}

#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct Immediates {
    time: f32,
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

impl WgpuApp for Practice05 {
    fn new(app: &WgpuState) -> Self {
        let shader = load_shader_module(&app.device, Path::new(PROJECT_ROOT).join("shader.wgsl"))
            .expect("failed to load shader");

        let uniform_bffer = load_buffer(
            &app.device,
            &[Uniforms {
                model: Mat4::IDENTITY,
                view_projection: Mat4::IDENTITY,
            }],
            wgpu::BufferUsages::UNIFORM,
        );

        let bind_group_layout =
            app.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("uniforms_layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: wgpu::BufferSize::new(
                                    mem::size_of::<Uniforms>() as u64
                                ),
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                view_dimension: wgpu::TextureViewDimension::D2,
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(Filtering),
                            count: None,
                        },
                    ],
                });

        let pipeline_layout = app
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                immediate_size: std::mem::size_of::<Immediates>() as u32,
                label: Some("pipeline_layout"),
                bind_group_layouts: &[Some(&bind_group_layout)],
            });

        let sampler = app.device.create_sampler(&wgpu::wgt::SamplerDescriptor {
            label: Some("sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
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

        let cow = load_obj(Path::new(PROJECT_ROOT).join("cow.obj")).unwrap();

        let vertex_buffer = load_buffer(&app.device, &cow.vertices, wgpu::BufferUsages::VERTEX);
        let index_buffer = load_buffer(&app.device, &cow.indices, wgpu::BufferUsages::INDEX);

        let cow_image = image::open(Path::new(PROJECT_ROOT).join("cow.png"))
            .unwrap()
            .into_rgba8();

        let texture = app.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("cow_texture"),
            size: wgpu::Extent3d {
                width: cow_image.width(),
                height: cow_image.height(),
                depth_or_array_layers: 1,
            },
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            mip_level_count: 4,
            sample_count: 1,
            view_formats: &[],
        });

        let mips: [(u32, [u8; 4]); 3] = [
            (1, [255, 0, 0, 255]),
            (2, [0, 255, 0, 255]),
            (3, [0, 0, 255, 255]),
        ];

        for (level, color) in mips {
            let w = (cow_image.width() >> level).max(1);
            let h = (cow_image.height() >> level).max(1);
            let mut data = Vec::with_capacity((w * h * 4) as usize);
            for _ in 0..w * h {
                data.extend_from_slice(&color);
            }

            app.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: level,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(w * 4),
                    rows_per_image: Some(h),
                },
                wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
            );
        }

        app.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            cow_image.as_raw(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(cow_image.width() * 4),
                rows_per_image: Some(cow_image.height()),
            },
            wgpu::Extent3d {
                width: cow_image.width(),
                height: cow_image.height(),
                depth_or_array_layers: 1,
            },
        );

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("uniforms_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_bffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self {
            depth_buffer: None,
            depth_buffer_view: None,
            pipeline,
            vertex_buffer,
            index_buffer,
            index_count: cow.indices.len() as u32,
            texture,
            texture_view,
            last_frame_start: Instant::now(),
            time: 0.0,
            camera_distance: 3.0,
            model_rotation: PI * 0.75,
            uniform_buffer: uniform_bffer,
            bind_group: bind_group,
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
            self.model_rotation += 5.0 * dt;
        }

        if app.keydown.contains(&KeyCode::ArrowRight) {
            self.model_rotation -= 5.0 * dt;
        }

        if app.keydown.contains(&KeyCode::ArrowUp) {
            self.camera_distance += 3.0 * dt;
        }

        if app.keydown.contains(&KeyCode::ArrowDown) {
            self.camera_distance -= 3.0 * dt;
        }

        let beat = self.time * 2.0 * PI * 2.0;
        let sway = 0.12 * beat.sin();
        let bounce = 0.045 * (1.0 + (2.0 * beat).cos());
        let lean = 0.14 * beat.sin();
        let turn = 0.08 * (beat + 0.5).sin();
        let nod = 0.045 * (2.0 * beat).sin();
        let model_matrix = Mat4::from_translation(Vec3::new(sway, bounce, 0.0))
            * Mat4::from_rotation_y(-self.model_rotation + turn)
            * Mat4::from_rotation_z(lean)
            * Mat4::from_rotation_x(nod);

        let view_matrix = Mat4::from_translation(Vec3::new(0.0, 0.0, -self.camera_distance));

        let projection_matrix = Mat4::perspective_rh(
            PI / 3.0,
            app.width() as f32 / app.height() as f32,
            0.01,
            100.0,
        );

        let view_projection_matrix = projection_matrix * view_matrix;

        let target_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = app.device.create_command_encoder(&Default::default());

        {
            let uniforms = Uniforms {
                model: model_matrix,
                view_projection: view_projection_matrix,
            };
            app.queue
                .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.6,
                            g: 0.8,
                            b: 1.0,
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
            let immediates = Immediates { time: self.time };
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_immediates(0, bytemuck::bytes_of(&immediates));
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.index_count, 0, 0..1);
        }

        app.queue.submit(std::iter::once(encoder.finish()));
        app.queue.present(surface_texture);
    }
}

fn main() {
    run::<Practice05>(AppConfig {
        title: "Dancing Cow",
        width: 1280,
        height: 720,
        srgb: false,
    });
}
