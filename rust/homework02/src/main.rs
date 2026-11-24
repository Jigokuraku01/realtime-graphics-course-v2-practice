mod camera;
mod gpu_scene;
mod light_marker;
mod renderer;
mod scene;
mod shadow_map;
mod shadow_math;

use std::path::PathBuf;
use std::sync::OnceLock;

use wgpu_app::{AppConfig, run};

static SCENE_PATH: OnceLock<PathBuf> = OnceLock::new();

fn main() {
    let mut args = std::env::args_os().skip(1);
    let path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/sponza/sponza.obj")
    });
    if args.next().is_some() {
        eprintln!("Expected at most one OBJ scene path");
        std::process::exit(2);
    }
    if !path.is_file() {
        eprintln!("OBJ scene not found: {}", path.display());
        std::process::exit(2);
    }
    eprintln!("Loading OBJ scene: {}", path.display());
    SCENE_PATH.set(path).expect("scene path already set");

    run::<renderer::Renderer>(AppConfig {
        title: "Homework 2: OBJ scene",
        width: 1280,
        height: 720,
        srgb: false,
    });
}
