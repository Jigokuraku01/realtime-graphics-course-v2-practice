use std::fs;
use std::path::Path;
use std::time::Instant;

use bytemuck::bytes_of;
use std::mem::size_of;
use wgpu::naga::front::wgsl::ImplementedEnableExtension::WgpuBindingArray;
use wgpu_app::{AppConfig, KeyCode, WgpuApp, WgpuState, run};

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

struct Practice02 {
    pipeline: wgpu::RenderPipeline,
    last_frame_start: Instant,
    time: f32,
    x_move: f32,
    y_move: f32,
}

#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct Immediates {
    transform: [[f32; 4]; 4],
    view: [[f32; 4]; 4],
}

impl WgpuApp for Practice02 {
    fn new(app: &WgpuState) -> Self {
        let shader = load_shader_module(&app.device, Path::new(PROJECT_ROOT).join("shader.wgsl"))
            .expect("failed to load shader");

        let pipeline_layout = app
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[],
                immediate_size: size_of::<Immediates>() as u32,
            });
        let pipeline = app
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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

        Self {
            pipeline,
            last_frame_start: Instant::now(),
            time: 0.0,
            x_move: 0.0,
            y_move: 0.0,
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

            let angle = self.time;
            let scale: f32 = 0.5;
            let c = angle.cos() * scale;
            let s = angle.sin() * scale;

            let height = app.height() as f32;
            let width = app.width() as f32;

            let mut move_add = dt;

            if app.keydown.contains(&KeyCode::Space) {
                move_add *= 2.0;
            }

            if app.keydown.contains(&KeyCode::ArrowLeft) {
                self.x_move += -move_add;
            }
            if app.keydown.contains(&KeyCode::ArrowRight) {
                self.x_move += move_add;
            }
            if app.keydown.contains(&KeyCode::ArrowUp) {
                self.y_move += move_add;
            }
            if app.keydown.contains(&KeyCode::ArrowDown) {
                self.y_move += -move_add;
            }

            let immediates = Immediates {
                transform: [
                    [c, -s, 0.0, 0.0],
                    [s, c, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [
                        angle.sin() * 0.5 + self.x_move,
                        angle.cos() * 0.5 + self.y_move,
                        0.0,
                        1.0,
                    ],
                ],
                view: [
                    [height / width, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0],
                ],
            };
            render_pass.set_pipeline(&self.pipeline);

            render_pass.set_immediates(0, bytes_of(&immediates));
            render_pass.draw(0..18, 0..1);
        }

        app.queue.submit(std::iter::once(encoder.finish()));
        app.queue.present(surface_texture);
    }
}

fn main() {
    run::<Practice02>(AppConfig {
        title: "Practice02",
        width: 1280,
        height: 720,
        srgb: false,
    });
}
