use glam::{Mat4, Vec3, Vec4};
use wgpu::util::DeviceExt;
use wgpu_app::WgpuState;

use crate::camera::Camera;
use crate::colors::{ColorVertex, make_colors};
use crate::mesh::{RingVertex, SurfaceVertex, make_icosphere, make_ring};
use crate::system_generator::{BodySpec, generate_system};

pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

const SURFACE_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<SurfaceVertex>() as wgpu::BufferAddress,
    step_mode: wgpu::VertexStepMode::Vertex,
    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
};
const COLOR_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<ColorVertex>() as wgpu::BufferAddress,
    step_mode: wgpu::VertexStepMode::Vertex,
    attributes: &wgpu::vertex_attr_array![2 => Float32x3],
};
const RING_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<RingVertex>() as wgpu::BufferAddress,
    step_mode: wgpu::VertexStepMode::Vertex,
    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32],
};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct SceneUniform {
    view_projection: [[f32; 4]; 4],
    camera_position: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ObjectUniform {
    model: [[f32; 4]; 4],
    color: [f32; 4],
}

struct Geometry {
    vertex: wgpu::Buffer,
    index: wgpu::Buffer,
    index_count: u32,
}

impl Geometry {
    fn new<V: bytemuck::Pod>(
        app: &WgpuState,
        label: &str,
        vertices: &[V],
        indices: &[u32],
    ) -> Self {
        let vertex = app
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let index = app
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            });
        Self {
            vertex,
            index,
            index_count: indices.len() as u32,
        }
    }
}

struct ObjectGpu {
    uniform: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    tint: [f32; 4], // сохраненный на стороне rust цвет
}

impl ObjectGpu {
    fn new(app: &WgpuState, layout: &wgpu::BindGroupLayout, label: &str, tint: [f32; 4]) -> Self {
        let uniform = app.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: std::mem::size_of::<ObjectUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        Self {
            uniform,
            bind_group,
            tint,
        }
    }

    fn update(&self, app: &WgpuState, model: Mat4) {
        let data = ObjectUniform {
            model: model.to_cols_array_2d(),
            color: self.tint,
        };
        app.queue
            .write_buffer(&self.uniform, 0, bytemuck::bytes_of(&data));
    }
}

struct Body {
    spec: BodySpec,
    position: Vec3,
    colors: wgpu::Buffer,
    object: ObjectGpu,
}

struct PlanetRing {
    planet: usize,
    tilt: f32, // наклон кольца
    geometry: Geometry,
    object: ObjectGpu,
}

pub struct SolarSystem {
    sphere: Geometry,
    sphere_vertices: Vec<SurfaceVertex>,
    rings: Vec<PlanetRing>,
    glow_ring: Geometry,
    bodies: Vec<Body>,
    sun_glow: ObjectGpu,
    scene_uniform: wgpu::Buffer,
    scene_bind_group: wgpu::BindGroup,
    sun_pipeline: wgpu::RenderPipeline,
    planet_pipeline: wgpu::RenderPipeline,
    ring_pipeline: wgpu::RenderPipeline,
    glow_pipeline: wgpu::RenderPipeline,
    time: f32,
}

impl SolarSystem {
    pub fn new(app: &WgpuState) -> Self {
        let generated = generate_system(rand::random::<u64>());
        let sphere_mesh = make_icosphere(3);
        let sphere = Geometry::new(
            app,
            "shared indexed sphere",
            &sphere_mesh.vertices,
            &sphere_mesh.indices,
        );
        let glow_mesh = make_ring(0.98, 1.85, 96);
        let glow_ring = Geometry::new(
            app,
            "Sun glow ring",
            &glow_mesh.vertices,
            &glow_mesh.indices,
        );

        let scene_layout = uniform_layout(app, "scene uniform layout", wgpu::ShaderStages::VERTEX);
        let object_layout = uniform_layout(
            app,
            "object uniform layout",
            wgpu::ShaderStages::VERTEX_FRAGMENT,
        );
        let pipeline_layout = app
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("solar system pipeline layout"),
                bind_group_layouts: &[Some(&scene_layout), Some(&object_layout)],
                immediate_size: 0,
            });
        let scene_uniform = app.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera for solid objects"),
            size: std::mem::size_of::<SceneUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let scene_bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scene bind group"),
            layout: &scene_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: scene_uniform.as_entire_binding(),
            }],
        });

        let body_shader = app
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("body shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("body.wgsl").into()),
            });
        let ring_shader = app
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("ring and glow shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("ring.wgsl").into()),
            });
        let sun_pipeline = make_pipeline(
            app,
            &pipeline_layout,
            &body_shader,
            "Sun pipeline",
            "fs_sun",
            &[Some(SURFACE_LAYOUT), Some(COLOR_LAYOUT)],
            true,
            None,
            Some(wgpu::Face::Back),
        );
        let planet_pipeline = make_pipeline(
            app,
            &pipeline_layout,
            &body_shader,
            "planet pipeline",
            "fs_planet",
            &[Some(SURFACE_LAYOUT), Some(COLOR_LAYOUT)],
            true,
            None,
            Some(wgpu::Face::Back),
        );
        let ring_pipeline = make_pipeline(
            app,
            &pipeline_layout,
            &ring_shader,
            "ring pipeline",
            "fs_ring",
            &[Some(RING_LAYOUT)],
            true,
            None,
            None,
        );
        let glow_pipeline = make_pipeline(
            app,
            &pipeline_layout,
            &ring_shader,
            "glow pipeline",
            "fs_glow",
            &[Some(RING_LAYOUT)],
            false,
            Some(wgpu::BlendState::ALPHA_BLENDING),
            None,
        );

        let bodies = generated
            .bodies
            .into_iter()
            .map(|description| {
                let spec = description.spec;
                let name = &description.name;
                let colors = make_colors(&sphere_mesh.vertices, spec.appearance, 0.0);
                let color_buffer =
                    app.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some(name),
                            contents: bytemuck::cast_slice(&colors),
                            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        });
                Body {
                    spec,
                    position: Vec3::ZERO,
                    colors: color_buffer,
                    object: ObjectGpu::new(app, &object_layout, name, [1.0; 4]),
                }
            })
            .collect();
        let rings = generated
            .rings
            .into_iter()
            .map(|spec| {
                let mesh = make_ring(spec.inner_radius, spec.outer_radius, 96);
                let label = format!("ring of planet {}", spec.planet);
                PlanetRing {
                    planet: spec.planet,
                    tilt: spec.tilt,
                    geometry: Geometry::new(app, &label, &mesh.vertices, &mesh.indices),
                    object: ObjectGpu::new(app, &object_layout, &label, spec.color),
                }
            })
            .collect();
        let sun_glow = ObjectGpu::new(
            app,
            &object_layout,
            "Sun glow transform",
            [1.0, 0.42, 0.08, 0.60],
        );

        Self {
            sphere,
            sphere_vertices: sphere_mesh.vertices,
            rings,
            glow_ring,
            bodies,
            sun_glow,
            scene_uniform,
            scene_bind_group,
            sun_pipeline,
            planet_pipeline,
            ring_pipeline,
            glow_pipeline,
            time: 0.0,
        }
    }

    pub fn update(&mut self, app: &WgpuState, camera: &Camera, dt: f32) {
        self.time += dt;
        let scene = SceneUniform {
            view_projection: camera
                .view_projection(app.width(), app.height())
                .to_cols_array_2d(),
            camera_position: camera.position().extend(1.0).to_array(),
        };
        app.queue
            .write_buffer(&self.scene_uniform, 0, bytemuck::bytes_of(&scene));

        for index in 0..self.bodies.len() {
            let spec = self.bodies[index].spec;
            let parent_position = spec
                .parent
                .map_or(Vec3::ZERO, |parent| self.bodies[parent].position);
            let orbit_angle = self.time * spec.orbit_speed + spec.orbit_phase;
            let (inclination_sin, inclination_cos) = spec.orbit_inclination.sin_cos();
            let offset = Vec3::new(
                orbit_angle.cos(),
                orbit_angle.sin() * inclination_sin,
                orbit_angle.sin() * inclination_cos,
            ) * spec.orbit_radius;
            let position = parent_position + offset;
            self.bodies[index].position = position;
            let model = Mat4::from_translation(position)
                * Mat4::from_rotation_z(spec.axial_tilt)
                * Mat4::from_rotation_y(self.time * spec.spin_speed)
                * Mat4::from_scale(Vec3::splat(spec.size));
            self.bodies[index].object.update(app, model);
        }

        let sun_colors = make_colors(
            &self.sphere_vertices,
            self.bodies[0].spec.appearance,
            self.time,
        );
        app.queue
            .write_buffer(&self.bodies[0].colors, 0, bytemuck::cast_slice(&sun_colors));

        for ring in &self.rings {
            let planet = &self.bodies[ring.planet];
            let model = Mat4::from_translation(planet.position)
                * Mat4::from_rotation_z(planet.spec.axial_tilt)
                * Mat4::from_rotation_x(ring.tilt)
                * Mat4::from_scale(Vec3::splat(planet.spec.size));
            ring.object.update(app, model);
        }

        let from_sun = camera.position() - self.bodies[0].position;
        let to_camera = if from_sun.length_squared() > 1e-6 {
            from_sun.normalize()
        } else {
            -camera.forward()
        };
        let reference = if to_camera.y.abs() < 0.95 {
            Vec3::Y
        } else {
            Vec3::X
        };
        let right = reference.cross(to_camera).normalize();
        let local_z = right.cross(to_camera);
        let billboard = Mat4::from_cols(
            right.extend(0.0),
            to_camera.extend(0.0),
            local_z.extend(0.0),
            Vec4::W,
        );
        let glow_model = Mat4::from_translation(self.bodies[0].position)
            * billboard
            * Mat4::from_scale(Vec3::splat(self.bodies[0].spec.size));
        self.sun_glow.update(app, glow_model);
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        pass.set_bind_group(0, &self.scene_bind_group, &[]);
        pass.set_vertex_buffer(0, self.sphere.vertex.slice(..));
        pass.set_index_buffer(self.sphere.index.slice(..), wgpu::IndexFormat::Uint32);
        for (index, body) in self.bodies.iter().enumerate() {
            pass.set_pipeline(if index == 0 {
                &self.sun_pipeline
            } else {
                &self.planet_pipeline
            });
            pass.set_bind_group(1, &body.object.bind_group, &[]);
            pass.set_vertex_buffer(1, body.colors.slice(..));
            pass.draw_indexed(0..self.sphere.index_count, 0, 0..1);
        }

        pass.set_pipeline(&self.ring_pipeline);
        for ring in &self.rings {
            pass.set_vertex_buffer(0, ring.geometry.vertex.slice(..));
            pass.set_index_buffer(ring.geometry.index.slice(..), wgpu::IndexFormat::Uint32);
            pass.set_bind_group(1, &ring.object.bind_group, &[]);
            pass.draw_indexed(0..ring.geometry.index_count, 0, 0..1);
        }

        pass.set_vertex_buffer(0, self.glow_ring.vertex.slice(..));
        pass.set_index_buffer(self.glow_ring.index.slice(..), wgpu::IndexFormat::Uint32);
        pass.set_pipeline(&self.glow_pipeline);
        pass.set_bind_group(1, &self.sun_glow.bind_group, &[]);
        pass.draw_indexed(0..self.glow_ring.index_count, 0, 0..1);
    }
}

fn uniform_layout(
    app: &WgpuState,
    label: &str,
    visibility: wgpu::ShaderStages,
) -> wgpu::BindGroupLayout {
    app.device
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(label),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        })
}

#[allow(clippy::too_many_arguments)]
fn make_pipeline(
    app: &WgpuState,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    label: &str,
    fragment: &str,
    buffers: &[Option<wgpu::VertexBufferLayout<'_>>],
    depth_write: bool,
    blend: Option<wgpu::BlendState>,
    cull_mode: Option<wgpu::Face>,
) -> wgpu::RenderPipeline {
    app.device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("vs_main"),
                buffers,
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some(fragment),
                targets: &[Some(wgpu::ColorTargetState {
                    format: app.surface_format(),
                    blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(depth_write),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        })
}
