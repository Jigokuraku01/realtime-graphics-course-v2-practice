use std::time::Instant;

use wgpu_app::{KeyCode, MouseButton, WgpuApp, WgpuState};

use crate::camera::Camera;
use crate::solar_system::{DEPTH_FORMAT, SolarSystem};
use crate::starfield::Starfield;

pub struct Renderer {
    starfield: Starfield,
    solar_system: SolarSystem,
    depth: DepthTarget,
    camera: Camera,
    last_frame: Instant,
    capture_initialized: bool,
    mouse_captured: bool,
    paused: bool,
    space_down: bool,
}

struct DepthTarget {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    size: (u32, u32),
}

impl DepthTarget {
    fn new(app: &WgpuState) -> Self {
        let size = (app.width(), app.height());
        let texture = app.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("scene depth"),
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

impl WgpuApp for Renderer {
    fn new(app: &WgpuState) -> Self {
        Self {
            starfield: Starfield::new(app),
            solar_system: SolarSystem::new(app),
            depth: DepthTarget::new(app),
            camera: Camera::new(),
            last_frame: Instant::now(),
            capture_initialized: false,
            mouse_captured: false,
            paused: false,
            space_down: false,
        }
    }

    fn redraw(&mut self, app: &mut WgpuState) {
        if !self.capture_initialized {
            self.mouse_captured = app.grab_mouse(true);
            self.capture_initialized = true;
        }

        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32().min(0.05);
        self.last_frame = now;
        self.camera.move_with_keys(&app.keydown, dt);
        self.starfield.update_camera(app, &self.camera);
        self.solar_system
            .update(app, &self.camera, if self.paused { 0.0 } else { dt });

        if self.depth.size != (app.width(), app.height()) {
            self.depth = DepthTarget::new(app);
        }

        let Some(frame) = app.begin_frame() else {
            return;
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = app
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("starfield pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            self.starfield.draw(&mut pass);
        }

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("solar system pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            self.solar_system.draw(&mut pass);
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
                "Planetary System — paused"
            } else {
                "Planetary System"
            });
        }
    }

    fn on_keyup(&mut self, _app: &mut WgpuState, key: KeyCode) {
        if key == KeyCode::Space {
            self.space_down = false;
        }
    }

    fn on_mousedown(&mut self, app: &mut WgpuState, button: MouseButton) {
        if button == MouseButton::Left && !self.mouse_captured {
            self.mouse_captured = app.grab_mouse(true);
        }
    }

    fn on_focus_lost(&mut self, app: &mut WgpuState) {
        app.grab_mouse(false);
        self.mouse_captured = false;
        self.space_down = false;
    }
}
