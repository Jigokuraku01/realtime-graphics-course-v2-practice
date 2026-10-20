use std::f32::consts::FRAC_PI_2;

use glam::{Mat4, Vec3};
use wgpu_app::KeyCode;

const MOUSE_SENSITIVITY: f32 = 0.003;
const MOVE_SPEED: f32 = 4.0;

pub struct Camera {
    position: Vec3,
    yaw: f32,   // вокруг мировой оси OY
    pitch: f32, // вверх или вниз
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    right: [f32; 4],      // стрелка вправо от нас
    up: [f32; 4],         // стрелка вверз от нас
    forward: [f32; 4],    // стрелка куда мы смотрим
    projection: [f32; 4], // настройки изображения
}

impl Camera {
    pub fn new() -> Self {
        Self {
            position: Vec3::new(0.0, 2.0, 17.0),
            yaw: 0.0,
            pitch: -0.12,
        }
    }

    pub fn rotate(&mut self, dx: f32, dy: f32) {
        self.yaw += dx * MOUSE_SENSITIVITY;
        self.pitch =
            (self.pitch - dy * MOUSE_SENSITIVITY).clamp(-FRAC_PI_2 + 0.01, FRAC_PI_2 - 0.01);
    }

    pub fn move_with_keys(&mut self, keys: &std::collections::HashSet<KeyCode>, dt: f32) {
        let forward = keys.contains(&KeyCode::KeyW) as i32 - keys.contains(&KeyCode::KeyS) as i32;
        let right = keys.contains(&KeyCode::KeyD) as i32 - keys.contains(&KeyCode::KeyA) as i32;

        let direction = self.forward() * forward as f32 + self.right() * right as f32;
        self.position += direction.normalize_or_zero() * MOVE_SPEED * dt;
    }

    pub fn position(&self) -> Vec3 {
        self.position
    }

    pub fn forward(&self) -> Vec3 {
        Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            -self.yaw.cos() * self.pitch.cos(),
        )
    }

    pub fn right(&self) -> Vec3 {
        Vec3::new(self.yaw.cos(), 0.0, self.yaw.sin())
    }

    pub fn up(&self) -> Vec3 {
        self.right().cross(self.forward())
    }

    pub fn view_projection(&self, width: u32, height: u32) -> Mat4 {
        let aspect = width.max(1) as f32 / height.max(1) as f32;
        let projection = Mat4::perspective_rh(60.0_f32.to_radians(), aspect, 0.01, 100.0);
        let view = Mat4::look_to_rh(self.position, self.forward(), Vec3::Y);
        projection * view
    }

    pub fn uniform(&self, width: u32, height: u32) -> CameraUniform {
        let forward = self.forward();
        let right = self.right();
        let up = self.up();
        let width = width.max(1);
        let height = height.max(1);
        let aspect = width as f32 / height as f32;
        let focal_length = 1.0 / (60.0_f32.to_radians() / 2.0).tan();

        CameraUniform {
            right: right.extend(0.0).to_array(),
            up: up.extend(0.0).to_array(),
            forward: forward.extend(0.0).to_array(),
            projection: [
                focal_length / aspect,
                focal_length,
                1.0 / width as f32,
                1.0 / height as f32,
            ],
        }
    }
}
