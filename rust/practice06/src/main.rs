use glam::{Mat4, Vec2, Vec3};
use std::f32::consts::PI;
use std::fs;
use std::mem;
use std::path::Path;
use std::time::Instant;
use wgpu::naga::SwizzleComponent::W;
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

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
struct TextureImmediates {
    center: glam::Vec2,
    extent: glam::Vec2,
    mode: i32,
    time: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
struct DrakoImmediates {
    model: glam::Mat4,
    view_projection: glam::Mat4,
}

struct Practice05 {
    depth_buffer: Option<wgpu::Texture>,
    depth_buffer_view: Option<wgpu::TextureView>,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    main_pipeline: wgpu::RenderPipeline,
    texture_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    offscreen_sampler: wgpu::Sampler,
    offscreen_texture: Option<wgpu::Texture>,
    offscreen_view_vec: Option<Vec<wgpu::TextureView>>,
    offscreen_bind_group_vec: Option<Vec<wgpu::BindGroup>>,
    nearest_sampler: wgpu::Sampler,
    index_count: u32,
    last_frame_start: Instant,
    time: f32,
    camera_distance: f32,
    model_rotation: f32,
    prev_screen_width: u32,
    prev_screen_height: u32,
    textire_count: u32,
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
        let main_shader = load_shader_module(
            &app.device,
            Path::new(PROJECT_ROOT).join("shader_main.wgsl"),
        )
        .expect("failed to load shader");

        let texture_shader = load_shader_module(
            &app.device,
            Path::new(PROJECT_ROOT).join("shader_texture.wgsl"),
        )
        .expect("failed to load shader");

        let offscree_sampler = app.device.create_sampler(&wgpu::wgt::SamplerDescriptor {
            label: Some("offscree_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });

        let nearest_sampler = app.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Nearest sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let texture_bind_group_layout =
            app.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
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

        let main_pipeline = app
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(
                    &app.device
                        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                            immediate_size: 128,
                            ..Default::default()
                        }),
                ),
                vertex: wgpu::VertexState {
                    module: &main_shader,
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
                        ],
                    })],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &main_shader,
                    entry_point: Some("fragmentMain"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
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

        let texture_pipeline =
            app.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: None,
                    layout: Some(&app.device.create_pipeline_layout(
                        &wgpu::PipelineLayoutDescriptor {
                            label: None,
                            immediate_size: mem::size_of::<TextureImmediates>() as u32,
                            bind_group_layouts: &[Some(&texture_bind_group_layout)],
                        },
                    )),
                    vertex: wgpu::VertexState {
                        module: &texture_shader,
                        entry_point: Some("vertexMain"),
                        buffers: &[],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &texture_shader,
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
                        ..Default::default()
                    },
                    depth_stencil: None,
                    multisample: Default::default(),
                    multiview_mask: None,
                    cache: None,
                });

        let dragon = load_obj(Path::new(PROJECT_ROOT).join("dragon.obj")).unwrap();

        let vertex_buffer = load_buffer(&app.device, &dragon.vertices, wgpu::BufferUsages::VERTEX);
        let index_buffer = load_buffer(&app.device, &dragon.indices, wgpu::BufferUsages::INDEX);

        Self {
            depth_buffer: None,
            depth_buffer_view: None,
            texture_bind_group_layout,
            main_pipeline,
            texture_pipeline,
            offscreen_sampler: offscree_sampler,
            offscreen_bind_group_vec: None,
            offscreen_texture: None,
            offscreen_view_vec: None,
            nearest_sampler: nearest_sampler,
            vertex_buffer,
            index_buffer,
            index_count: dragon.indices.len() as u32,
            last_frame_start: Instant::now(),
            time: 0.0,
            camera_distance: 2.0,
            model_rotation: -PI * 0.5,
            prev_screen_height: app.height(),
            prev_screen_width: app.width(),
            textire_count: 4,
        }
    }

    fn redraw(&mut self, app: &mut WgpuState) {
        let Some(surface_texture) = app.begin_frame() else {
            return;
        };
        let offscreen_width = (app.width() / 2).max(1);
        let offscreen_height = (app.height() / 2).max(1);

        if self.depth_buffer.as_ref().is_none_or(|depth_buffer| {
            depth_buffer.width() != offscreen_width || depth_buffer.height() != offscreen_height
        }) {
            let new_depth_buffer = app.device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                dimension: wgpu::TextureDimension::D2,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TRANSIENT_ATTACHMENT,
                size: wgpu::Extent3d {
                    width: offscreen_width,
                    height: offscreen_height,
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
        self.camera_distance = self.camera_distance.max(0.1);

        let view_matrix = Mat4::from_translation(Vec3::new(0.0, 0.0, -self.camera_distance));

        let projection_matrix = Mat4::perspective_rh(
            PI / 3.0,
            offscreen_width as f32 / offscreen_height as f32,
            0.01,
            100.0,
        );

        let target_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = app.device.create_command_encoder(&Default::default());

        if (app.height(), app.width()) != (self.prev_screen_height, self.prev_screen_width)
            || self.offscreen_bind_group_vec.is_none()
        {
            let new_offscreen_texture = app.device.create_texture(&wgpu::wgt::TextureDescriptor {
                label: Some("offscreen_texture"),
                size: wgpu::Extent3d {
                    width: offscreen_width,
                    height: offscreen_height,
                    depth_or_array_layers: 4,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });

            let new_offscreen_info_view: Vec<(wgpu::TextureView, wgpu::BindGroup)> = (0..self
                .textire_count)
                .map(|k| {
                    let tmp = new_offscreen_texture.create_view(&wgpu::TextureViewDescriptor {
                        dimension: Some(wgpu::TextureViewDimension::D2),
                        base_array_layer: k,
                        array_layer_count: Some(1),
                        ..Default::default()
                    });
                    let tmp_bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("offscreen_bind_group_vec"),
                        layout: &self.texture_bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&tmp),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(&self.offscreen_sampler),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: wgpu::BindingResource::Sampler(&self.nearest_sampler),
                            },
                        ],
                    });
                    (tmp, tmp_bind_group)
                })
                .collect();

            let (views, bind_groups): (Vec<wgpu::TextureView>, Vec<wgpu::BindGroup>) =
                new_offscreen_info_view.into_iter().unzip();

            self.offscreen_view_vec = Some(views);
            self.offscreen_bind_group_vec = Some(bind_groups);

            self.prev_screen_height = app.height();
            self.prev_screen_width = app.width();
            self.offscreen_texture = Some(new_offscreen_texture);
        }

        {
            let color_vec: Vec<wgpu::Color> = [
                wgpu::Color {
                    r: 0.02,
                    g: 0.02,
                    b: 0.05,
                    a: 1.0,
                },
                wgpu::Color {
                    r: 0.05,
                    g: 0.15,
                    b: 0.40,
                    a: 1.0,
                },
                wgpu::Color {
                    r: 0.55,
                    g: 0.12,
                    b: 0.45,
                    a: 1.0,
                },
                wgpu::Color {
                    r: 0.95,
                    g: 0.35,
                    b: 0.15,
                    a: 1.0,
                },
            ]
            .into_iter()
            .take(self.textire_count as usize)
            .collect();

            let aspect = offscreen_width as f32 / offscreen_height as f32;
            let half_height = 0.75 * self.camera_distance;
            let half_width = half_height * aspect;
            let ortho = Mat4::orthographic_rh(
                -half_width,
                half_width,
                -half_height,
                half_height,
                0.01,
                100.0,
            );

            let projection_matrix_vec = [projection_matrix, ortho, ortho, ortho];
            let dist = self.camera_distance;
            let view_matrix_vec = [
                view_matrix,
                Mat4::look_at_rh(Vec3::new(dist, 0.0, 0.0), Vec3::ZERO, Vec3::Y),
                Mat4::look_at_rh(Vec3::new(0.0, 0.0, -dist), Vec3::ZERO, Vec3::Y),
                Mat4::look_at_rh(Vec3::new(0.0, dist, 0.0), Vec3::ZERO, Vec3::NEG_Z),
            ];
            let model_matrix =
                Mat4::from_rotation_y(-self.model_rotation) * Mat4::from_scale(Vec3::splat(2.0));

            for iter in 0..self.textire_count {
                let k = iter as usize;

                let offscreen_view = &self.offscreen_view_vec.as_ref().unwrap()[k];
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &offscreen_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(color_vec[k]),
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

                let view_projection_matrix = projection_matrix_vec[k] * view_matrix_vec[k];

                render_pass.set_pipeline(&self.main_pipeline);

                let drako_immediates = DrakoImmediates {
                    model: model_matrix,
                    view_projection: view_projection_matrix,
                };

                render_pass.set_immediates(0, bytemuck::bytes_of(&drako_immediates));
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.index_count, 0, 0..1);
            }
        }
        {
            let mut new_render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("new_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target_view,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                    resolve_target: None,
                })],
                ..Default::default()
            });

            let centers = [
                Vec2::new(-0.5, -0.5),
                Vec2::new(0.5, -0.5),
                Vec2::new(-0.5, 0.5),
                Vec2::new(0.5, 0.5),
            ];
            new_render_pass.set_pipeline(&self.texture_pipeline);

            for (k, center) in centers.into_iter().enumerate() {
                let immediates = TextureImmediates {
                    center,
                    extent: Vec2::new(0.5, 0.5),
                    mode: k as i32,
                    time: self.time,
                };

                new_render_pass.set_immediates(0, bytemuck::bytes_of(&immediates));
                new_render_pass.set_bind_group(
                    0,
                    &self.offscreen_bind_group_vec.as_ref().unwrap()[k],
                    &[],
                );

                new_render_pass.draw(0..6, 0..1);
            }
        }
        app.queue.submit(std::iter::once(encoder.finish()));
        app.queue.present(surface_texture);
    }
}

fn main() {
    run::<Practice05>(AppConfig {
        title: "Practice05",
        width: 1280,
        height: 720,
        srgb: false,
    });
}
