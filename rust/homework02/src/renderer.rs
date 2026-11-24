use std::mem;
use std::time::Instant;

use glam::{Mat4, Vec3};
use wgpu_app::{KeyCode, MouseButton, WgpuApp, WgpuState};

use crate::SCENE_PATH;
use crate::camera::Camera;
use crate::gpu_scene::{GpuScene, material_layout, vertex_layout};
use crate::light_marker::LightMarker;
use crate::scene::{Bounds, Scene};
use crate::shadow_map::{BlurRenderer, DEPTH_FORMAT, FORMAT, POINT_SIZE, SUN_SIZE, ShadowMap};
use crate::shadow_math::{directional_projection, point_projections};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct FrameUniform {
    view_projection: Mat4,
    sun_projection: Mat4,
    camera_position: [f32; 4],
    ambient: [f32; 4],
    sun_direction: [f32; 4],
    sun_color: [f32; 4],
    point_position_range: [f32; 4],
    point_color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ShadowUniform {
    view_projection: Mat4,
    point_position: [f32; 4],
    params: [f32; 4],
}

struct DepthTarget {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    size: (u32, u32),
}

impl DepthTarget {
    fn new(app: &WgpuState) -> Self {
        let size = (app.width().max(1), app.height().max(1));
        let texture = app.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("main scene depth"),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            _texture: texture,
            view,
            size,
        }
    }
}

fn frame_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("frame and shadow maps layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
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
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::Cube,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

fn shadow_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("shadow pass uniform layout"),
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
    })
}

fn main_pipeline(
    app: &WgpuState,
    frame_layout: &wgpu::BindGroupLayout,
    material_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = app
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scene Phong shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("main.wgsl").into()),
        });
    let layout = app
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("main pipeline layout"),
            bind_group_layouts: &[Some(frame_layout), Some(material_layout)],
            immediate_size: 0,
        });
    app.device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("main scene pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertexMain"),
                buffers: &[Some(vertex_layout())],
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
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        })
}

fn shadow_pipeline(
    app: &WgpuState,
    shadow_layout: &wgpu::BindGroupLayout,
    material_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = app
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shadow moment shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shadow.wgsl").into()),
        });
    let layout = app
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow pipeline layout"),
            bind_group_layouts: &[Some(shadow_layout), Some(material_layout)],
            immediate_size: 0,
        });
    app.device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow moment pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertexMain"),
                buffers: &[Some(vertex_layout())],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragmentMain"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: FORMAT,
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
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: wgpu::DepthBiasState {
                    constant: 1,
                    slope_scale: 1.0,
                    clamp: 0.0,
                },
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        })
}

pub struct Renderer {
    gpu_scene: GpuScene,
    bounds: Bounds,
    camera: Camera,
    main_depth: DepthTarget,
    sun_map: ShadowMap,
    point_map: ShadowMap,
    blur: BlurRenderer,
    frame_buffer: wgpu::Buffer,
    frame_group: wgpu::BindGroup,
    shadow_buffers: Vec<wgpu::Buffer>,
    shadow_groups: Vec<wgpu::BindGroup>,
    main_pipeline: wgpu::RenderPipeline,
    light_marker: LightMarker,
    shadow_pipeline: wgpu::RenderPipeline,
    last_frame: Instant,
    elapsed: f32,
    mouse_captured: bool,
    paused: bool,
    space_down: bool,
}

impl Renderer {
    fn render_shadow_layer(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        map: &ShadowMap,
        layer: usize,
        uniform_index: usize,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("shadow map pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &map.raw_layers[layer],
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 1.0,
                        g: 1.0,
                        b: 0.0,
                        a: 0.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &map.depth_layers[layer],
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        pass.set_pipeline(&self.shadow_pipeline);
        pass.set_bind_group(0, &self.shadow_groups[uniform_index], &[]);
        self.gpu_scene.draw(&mut pass);
    }

    fn point_position(&self) -> Vec3 {
        let center = self.bounds.center();
        let size = self.bounds.size();
        let phase = self.elapsed * 0.25;
        Vec3::new(
            center.x + size.x * 0.22 * phase.sin(),
            self.bounds.min.y + size.y * (0.26 + 0.04 * (phase * 0.7).sin()),
            center.z + size.z * 0.03 * phase.cos(),
        )
    }

    fn point_range(&self) -> f32 {
        self.bounds.size().length().max(1.0) * 0.8
    }
}

impl WgpuApp for Renderer {
    fn new(app: &WgpuState) -> Self {
        let scene_path = SCENE_PATH.get().expect("scene path must be set before run");
        let scene = Scene::load(scene_path)
            .unwrap_or_else(|error| panic!("failed to load OBJ scene: {error}"));
        let bounds = scene.bounds;
        let camera = Camera::new(bounds, scene.meshes.len() >= 100);
        let material_layout = material_layout(&app.device);
        let gpu_scene = GpuScene::new(&app.device, &app.queue, scene, &material_layout)
            .unwrap_or_else(|error| panic!("failed to upload OBJ scene: {error}"));
        let blur = BlurRenderer::new(&app.device);
        let sun_map = ShadowMap::new(&app.device, &blur, SUN_SIZE, false);
        let point_map = ShadowMap::new(&app.device, &blur, POINT_SIZE, true);
        let frame_layout = frame_layout(&app.device);
        let shadow_layout = shadow_layout(&app.device);
        let frame_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frame uniform"),
            size: mem::size_of::<FrameUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let moment_sampler = app.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("filtered shadow sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let frame_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("frame and shadow maps"),
            layout: &frame_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: frame_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&sun_map.sample_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&point_map.sample_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&moment_sampler),
                },
            ],
        });
        let mut shadow_buffers = Vec::with_capacity(7);
        let mut shadow_groups = Vec::with_capacity(7);
        for _ in 0..7 {
            let buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("shadow pass uniform"),
                size: mem::size_of::<ShadowUniform>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("shadow pass group"),
                layout: &shadow_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            });
            shadow_buffers.push(buffer);
            shadow_groups.push(group);
        }
        let main_pipeline = main_pipeline(app, &frame_layout, &material_layout);
        let light_marker = LightMarker::new(
            &app.device,
            &frame_layout,
            app.surface_format(),
            DEPTH_FORMAT,
        );
        let shadow_pipeline = shadow_pipeline(app, &shadow_layout, &material_layout);
        Self {
            gpu_scene,
            bounds,
            camera,
            main_depth: DepthTarget::new(app),
            sun_map,
            point_map,
            blur,
            frame_buffer,
            frame_group,
            shadow_buffers,
            shadow_groups,
            main_pipeline,
            light_marker,
            shadow_pipeline,
            last_frame: Instant::now(),
            elapsed: 0.0,
            mouse_captured: false,
            paused: false,
            space_down: false,
        }
    }

    fn redraw(&mut self, app: &mut WgpuState) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;
        if !self.paused {
            self.elapsed += dt;
        }
        self.camera.move_with_keys(&app.keydown, dt);
        if self.main_depth.size != (app.width(), app.height()) {
            self.main_depth = DepthTarget::new(app);
        }

        let Some(frame) = app.begin_frame() else {
            return;
        };
        let target = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let camera_vp = self.camera.view_projection(app.width(), app.height());
        let sun_direction = Vec3::new(
            (self.elapsed * 0.22).sin(),
            0.65 + 0.15 * (self.elapsed * 0.14).sin(),
            (self.elapsed * 0.22).cos(),
        )
        .normalize();
        let sun_projection = directional_projection(self.bounds, camera_vp, -sun_direction);
        let point_position = self.point_position();
        let point_range = self.point_range();
        let point_views = point_projections(point_position, point_range);

        let frame_uniform = FrameUniform {
            view_projection: camera_vp,
            sun_projection,
            camera_position: self.camera.position.extend(0.0).to_array(),
            ambient: [0.25, 0.26, 0.28, 0.0],
            sun_direction: sun_direction.extend(0.0).to_array(),
            sun_color: [1.2, 1.14, 1.04, 0.0],
            point_position_range: point_position.extend(point_range).to_array(),
            point_color: [1.25, 1.12, 0.95, 0.0],
        };
        app.queue
            .write_buffer(&self.frame_buffer, 0, bytemuck::bytes_of(&frame_uniform));

        let sun_uniform = ShadowUniform {
            view_projection: sun_projection,
            point_position: [0.0; 4],
            params: [0.0; 4],
        };
        app.queue
            .write_buffer(&self.shadow_buffers[0], 0, bytemuck::bytes_of(&sun_uniform));
        for (index, projection) in point_views.into_iter().enumerate() {
            let uniform = ShadowUniform {
                view_projection: projection,
                point_position: point_position.extend(0.0).to_array(),
                params: [1.0, point_range, 0.0, 0.0],
            };
            app.queue.write_buffer(
                &self.shadow_buffers[index + 1],
                0,
                bytemuck::bytes_of(&uniform),
            );
        }

        let mut encoder = app
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("homework 2 frame"),
            });
        self.render_shadow_layer(&mut encoder, &self.sun_map, 0, 0);
        for face in 0..6 {
            self.render_shadow_layer(&mut encoder, &self.point_map, face, face + 1);
        }
        self.blur.blur(&mut encoder, &self.sun_map);
        self.blur.blur(&mut encoder, &self.point_map);
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main scene pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.50,
                            g: 0.63,
                            b: 0.78,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.main_depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_pipeline(&self.main_pipeline);
            pass.set_bind_group(0, &self.frame_group, &[]);
            self.gpu_scene.draw(&mut pass);
            self.light_marker.draw(&mut pass, &self.frame_group);
        }
        app.queue.submit([encoder.finish()]);
        app.queue.present(frame);
    }

    fn on_mousemove(&mut self, _app: &mut WgpuState, dx: f32, dy: f32) {
        if self.mouse_captured {
            self.camera.rotate(dx, dy);
        }
    }

    fn on_keydown(&mut self, app: &mut WgpuState, key: KeyCode) {
        if key == KeyCode::Escape {
            app.grab_mouse(false);
            self.mouse_captured = false;
        } else if key == KeyCode::Space && !self.space_down {
            self.paused = !self.paused;
            self.space_down = true;
            app.window.set_title(if self.paused {
                "Homework 2: OBJ scene — paused"
            } else {
                "Homework 2: OBJ scene"
            });
        }
    }

    fn on_keyup(&mut self, _app: &mut WgpuState, key: KeyCode) {
        if key == KeyCode::Space {
            self.space_down = false;
        }
    }

    fn on_mousedown(&mut self, app: &mut WgpuState, button: MouseButton) {
        match button {
            MouseButton::Left if !self.mouse_captured => {
                self.mouse_captured = app.grab_mouse(true);
            }
            MouseButton::Right if self.mouse_captured => {
                app.grab_mouse(false);
                self.mouse_captured = false;
            }
            _ => {}
        }
    }

    fn on_focus_lost(&mut self, app: &mut WgpuState) {
        app.grab_mouse(false);
        self.mouse_captured = false;
        self.space_down = false;
    }
}
