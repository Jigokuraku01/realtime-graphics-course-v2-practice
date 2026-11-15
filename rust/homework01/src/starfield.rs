use std::f32::consts::TAU;

use rand::RngExt;
use wgpu::util::DeviceExt;
use wgpu_app::WgpuState;

use crate::camera::{Camera, CameraUniform};

const STAR_COUNT: usize = 10_000;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Star {
    direction: [f32; 3],
    brightness: f32,
    size_pixels: f32,
}

impl Star {
    const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32, 2 => Float32],
    };
}

pub struct Starfield {
    pipeline: wgpu::RenderPipeline,
    instances: wgpu::Buffer,
    camera_uniform: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
}

impl Starfield {
    pub fn new(app: &WgpuState) -> Self {
        let mut rng = rand::rng();
        let stars: Vec<Star> = (0..STAR_COUNT)
            .map(|_| {
                let z: f32 = rng.random_range(-1.0..1.0);
                let angle: f32 = rng.random_range(0.0..TAU);
                let radius = (1.0 - z * z).sqrt();
                let size_roll: f32 = rng.random();
                let size_pixels = if size_roll < 0.70 {
                    2.0
                } else if size_roll < 0.94 {
                    3.0
                } else {
                    4.0
                };
                Star {
                    direction: [radius * angle.cos(), radius * angle.sin(), z],
                    brightness: rng.random_range(0.65..1.0),
                    size_pixels,
                }
            })
            .collect();
        let instances = app
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("star instances on unit sphere"),
                contents: bytemuck::cast_slice(&stars),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let camera_uniform = app.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("star camera"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group_layout =
            app.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("star camera layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });
        let camera_bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("star camera bind group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_uniform.as_entire_binding(),
            }],
        });
        let pipeline_layout = app
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("star pipeline layout"),
                bind_group_layouts: &[Some(&bind_group_layout)],
                immediate_size: 0,
            });
        let shader = app
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("star shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("stars.wgsl").into()),
            });
        let pipeline = app
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("star pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Some(Star::LAYOUT)],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: app.surface_format(),
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            });

        Self {
            pipeline,
            instances,
            camera_uniform,
            camera_bind_group,
        }
    }

    pub fn update_camera(&self, app: &WgpuState, camera: &Camera) {
        let uniform = camera.uniform(app.width(), app.height());
        app.queue
            .write_buffer(&self.camera_uniform, 0, bytemuck::bytes_of(&uniform));
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.camera_bind_group, &[]);
        pass.set_vertex_buffer(0, self.instances.slice(..));
        pass.draw(0..6, 0..STAR_COUNT as u32);
    }
}
