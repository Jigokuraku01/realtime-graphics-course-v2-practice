use wgpu::{
    naga::{SwizzleComponent::W, valid::LiteralError::NaN},
    wgc::command,
};
use wgpu_app::{AppConfig, WgpuApp, WgpuState, run};

const PROJECT_ROOT: &str = env!("CARGO_MANIFEST_DIR");

struct Practice01 {
    pipeline: wgpu::RenderPipeline,
}

impl WgpuApp for Practice01 {
    fn new(app: &WgpuState) -> Self {
        let shader_source = include_str!("shader.wgsl");
        let shader_module = app
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("shader"),
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });

        let render_pipeline = app
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("pipeline"),
                vertex: wgpu::VertexState {
                    module: &shader_module,
                    entry_point: Some("vertexMain"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader_module,
                    entry_point: Some("fragmentMain"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: app.surface_format(),
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                layout: None,
                depth_stencil: None,
                multiview_mask: None,
                cache: None,
                multisample: wgpu::MultisampleState::default(),
            });

        Self {
            pipeline: render_pipeline,
        }
    }

    fn redraw(&mut self, app: &mut WgpuState) {
        let Some(surface_texture) = app.begin_frame() else {
            return;
        };

        let target_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Frame rendering code goes here

        let mut command_encoder = app
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(&"my super duper pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target_view,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                    resolve_target: None,
                })],
                ..Default::default()
            });
            render_pass.set_pipeline(&self.pipeline);
            render_pass.draw(0..3, 0..1);
        }

        let command_buffer = command_encoder.finish();
        app.queue.submit([command_buffer]);

        app.queue.present(surface_texture);
    }
}

fn main() {
    run::<Practice01>(AppConfig {
        title: "Practice01",
        width: 1280,
        height: 720,
        srgb: false,
    });
}
