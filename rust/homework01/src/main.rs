mod camera;
mod colors;
mod mesh;
mod renderer;
mod solar_system;
mod starfield;
mod system_generator;

use wgpu_app::{AppConfig, run};

fn main() {
    run::<renderer::Renderer>(AppConfig {
        title: "Planetary System",
        width: 1280,
        height: 720,
        srgb: true,
    });
}
