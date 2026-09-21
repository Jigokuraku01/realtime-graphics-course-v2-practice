use std::fs;
use std::path::Path;
use std::time::Instant;
use std::vec::Vec;
use glam::Vec2;

use wgpu_app::{run, AppConfig, WgpuApp, WgpuState, KeyCode};

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
#[derive(Clone, Copy)]
struct Vertex {
    position: Vec2,
    color: [u8; 4],
}

fn lerp(v0: &Vertex, v1: &Vertex, t: f32) -> Vertex {
    Vertex {
        position: v0.position.lerp(v1.position, t),
        color: v0.color,
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

struct Practice03 {
    pipeline: wgpu::RenderPipeline,
    last_frame_start: Instant,
    time: f32,
    vertices: Vec<Vertex>,
}

impl WgpuApp for Practice03 {
    fn new(app: &WgpuState) -> Self {
        let shader = load_shader_module(&app.device, Path::new(PROJECT_ROOT).join("shader.wgsl"))
            .expect("failed to load shader");

        let pipeline_layout = app.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            immediate_size: 64,
            ..Default::default()
        });

        let pipeline = app.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertexMain"),
                buffers: &[],
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
                ..Default::default()
            },
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

        let vertices = vec![
            Vertex{ position: Vec2::new(0.0, 0.0), color: [125, 207, 182, 255] },
            Vertex{ position: Vec2::new(0.5, 0.0), color: [251, 209, 162, 255] },
            Vertex{ position: Vec2::new(0.0, 0.5), color: [247, 146,  86, 255] },
        ];

        Self { pipeline, last_frame_start: Instant::now(), time: 0.0, vertices }
    }

    fn redraw(&mut self, app: &mut WgpuState) {
        let Some(surface_texture) = app.begin_frame() else {
            return;
        };

        let now = Instant::now();
        let dt = (now - self.last_frame_start).as_secs_f32();
        self.time += dt;
        self.last_frame_start = now;

        let view_matrix: [f32; 16] = [
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ];

        let target_view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = app.device.create_command_encoder(&Default::default());

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color{r: 0.07, g: 0.21, b: 0.30, a: 1.0}),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_immediates(0, bytemuck::bytes_of(&view_matrix));
            render_pass.draw(0..3, 0..1);
        }

        app.queue.submit(std::iter::once(encoder.finish()));
        app.queue.present(surface_texture);
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
