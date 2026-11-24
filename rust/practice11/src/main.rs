use std::f32::consts::PI;
use std::fs;
use std::mem;
use std::path::Path;
use std::time::Instant;

use glam::{Mat4, Vec3, Vec4};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use wgpu::util::DeviceExt;
use wgpu_app::{AppConfig, KeyCode, WgpuApp, WgpuState, downsample, run};

const PROJECT_ROOT: &str = env!("CARGO_MANIFEST_DIR");
const PARTICLE_COUNT: u32 = 256;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniforms {
    view_projection: Mat4,
    position: Vec3,
    padding: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Particle {
    position: [f32; 3],
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

fn load_particle_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    path: impl AsRef<Path>,
    srgb: bool,
) -> wgpu::Texture {
    let image = image::open(path)
        .expect("failed to load particle texture")
        .into_rgba8();
    let mut mipmaps = vec![image];

    while mipmaps.last().unwrap().width() > 1 || mipmaps.last().unwrap().height() > 1 {
        let next = downsample(mipmaps.last().unwrap(), srgb);
        mipmaps.push(next);
    }

    let image = &mipmaps[0];
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("particle texture"),
        size: wgpu::Extent3d {
            width: image.width(),
            height: image.height(),
            depth_or_array_layers: 1,
        },
        mip_level_count: mipmaps.len() as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    for (level, mipmap) in mipmaps.iter().enumerate() {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: level as u32,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            mipmap.as_raw(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * mipmap.width()),
                rows_per_image: Some(mipmap.height()),
            },
            wgpu::Extent3d {
                width: mipmap.width(),
                height: mipmap.height(),
                depth_or_array_layers: 1,
            },
        );
    }

    texture
}

struct Practice11 {
    depth_buffer: Option<wgpu::Texture>,
    depth_buffer_view: Option<wgpu::TextureView>,
    camera_uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
    particle_buffer: wgpu::Buffer,
    last_frame_start: Instant,
    camera_distance: f32,
    camera_rotation: f32,
    camera_azimuth: f32,
}

impl WgpuApp for Practice11 {
    fn new(app: &WgpuState) -> Self {
        let shader = load_shader_module(&app.device, Path::new(PROJECT_ROOT).join("shader.wgsl"))
            .expect("failed to load shader");

        let bind_group_layout =
            app.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });

        let camera_uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: mem::size_of::<CameraUniforms>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });

        let particle_texture = load_particle_texture(
            &app.device,
            &app.queue,
            Path::new(PROJECT_ROOT).join("particle.png"),
            false,
        );
        let particle_texture_view =
            particle_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let particle_sampler = app.device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });

        let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&particle_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&particle_sampler),
                },
            ],
        });

        let pipeline_layout = app
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[Some(&bind_group_layout)],
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
                        array_stride: mem::size_of::<Particle>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[wgpu::VertexAttribute {
                            offset: mem::offset_of!(Particle, position) as wgpu::BufferAddress,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x3,
                        }],
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
                    cull_mode: None,
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

        let mut rng = StdRng::seed_from_u64(0);
        let particles = (0..PARTICLE_COUNT)
            .map(|_| Particle {
                position: [
                    rng.random_range(-1.0..1.0) * 2.0,
                    rng.random_range(-1.0..1.0) * 0.5,
                    rng.random_range(-1.0..1.0) * 2.0,
                ],
            })
            .collect::<Vec<_>>();
        let particle_buffer = load_buffer(&app.device, &particles, wgpu::BufferUsages::VERTEX);

        Self {
            depth_buffer: None,
            depth_buffer_view: None,
            camera_uniform_buffer,
            bind_group,
            pipeline,
            particle_buffer,
            last_frame_start: Instant::now(),
            camera_distance: 4.0,
            camera_rotation: 0.0,
            camera_azimuth: PI / 12.0,
        }
    }

    fn redraw(&mut self, app: &mut WgpuState) {
        let Some(surface_texture) = app.begin_frame() else {
            return;
        };

        if self.depth_buffer.as_ref().is_none_or(|depth_buffer| {
            depth_buffer.width() != app.width() || depth_buffer.height() != app.height()
        }) {
            let depth_buffer = app.device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: app.width(),
                    height: app.height(),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth24Plus,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TRANSIENT_ATTACHMENT,
                view_formats: &[],
            });
            let depth_buffer_view =
                depth_buffer.create_view(&wgpu::TextureViewDescriptor::default());
            self.depth_buffer = Some(depth_buffer);
            self.depth_buffer_view = Some(depth_buffer_view);
        }

        let now = Instant::now();
        let dt = (now - self.last_frame_start).as_secs_f32();
        self.last_frame_start = now;

        if app.keydown.contains(&KeyCode::ArrowLeft) {
            self.camera_rotation += 3.0 * dt;
        }
        if app.keydown.contains(&KeyCode::ArrowRight) {
            self.camera_rotation -= 3.0 * dt;
        }
        if app.keydown.contains(&KeyCode::ArrowUp) {
            self.camera_distance += 3.0 * dt;
        }
        if app.keydown.contains(&KeyCode::ArrowDown) {
            self.camera_distance -= 3.0 * dt;
        }

        let view = Mat4::from_translation(Vec3::new(0.0, 0.0, -self.camera_distance))
            * Mat4::from_rotation_x(self.camera_azimuth)
            * Mat4::from_rotation_y(-self.camera_rotation);
        let projection = Mat4::perspective_rh(
            PI / 3.0,
            app.width() as f32 / app.height() as f32,
            0.01,
            100.0,
        );
        let camera_uniforms = CameraUniforms {
            view_projection: projection * view,
            position: (view.inverse() * Vec4::new(0.0, 0.0, 0.0, 1.0)).truncate(),
            padding: 0.0,
        };
        app.queue.write_buffer(
            &self.camera_uniform_buffer,
            0,
            bytemuck::bytes_of(&camera_uniforms),
        );

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
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
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
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.particle_buffer.slice(..));
            render_pass.draw(0..6, 0..PARTICLE_COUNT);
        }

        app.queue.submit(std::iter::once(encoder.finish()));
        app.queue.present(surface_texture);
    }
}

fn main() {
    run::<Practice11>(AppConfig {
        title: "Practice11",
        width: 1280,
        height: 720,
        srgb: false,
    });
}
