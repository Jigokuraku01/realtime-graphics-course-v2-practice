mod camera;
mod gpu_scene;
mod renderer;
mod scene;
mod shadow_map;
mod shadow_math;

use std::path::PathBuf;
use std::sync::OnceLock;

use wgpu_app::{AppConfig, run};

static SCENE_PATH: OnceLock<PathBuf> = OnceLock::new();

fn main() {
    let mut args = std::env::args_os();
    let executable = args.next().unwrap_or_default();
    let Some(path) = args.next() else {
        eprintln!("Usage: {} <scene.obj>", PathBuf::from(executable).display());
        std::process::exit(2);
    };
    if args.next().is_some() {
        eprintln!("Expected exactly one OBJ scene path");
        std::process::exit(2);
    }
    SCENE_PATH
        .set(PathBuf::from(path))
        .expect("scene path already set");

    run::<renderer::Renderer>(AppConfig {
        title: "Homework 2: OBJ scene",
        width: 1280,
        height: 720,
        srgb: false,
    });
}
