use std::collections::HashSet;
use std::f32::consts::FRAC_PI_2;

use glam::{Mat4, Vec3};
use wgpu_app::KeyCode;

use crate::scene::Bounds;

pub struct Camera {
    pub position: Vec3,
    yaw: f32,
    pitch: f32,
    speed: f32,
    far: f32,
}

impl Camera {
    pub fn new(bounds: Bounds) -> Self {
        let diameter = bounds.size().length().max(1.0);
        Self {
            position: bounds.center() + Vec3::new(0.0, diameter * 0.15, diameter * 0.8),
            yaw: 0.0,
            pitch: -0.15,
            speed: diameter * 0.35,
            far: diameter * 4.0,
        }
    }

    pub fn rotate(&mut self, dx: f32, dy: f32) {
        self.yaw += dx * 0.003;
        self.pitch = (self.pitch - dy * 0.003).clamp(-FRAC_PI_2 + 0.01, FRAC_PI_2 - 0.01);
    }

    pub fn move_with_keys(&mut self, keys: &HashSet<KeyCode>, dt: f32) {
        let forward = keys.contains(&KeyCode::KeyW) as i32 - keys.contains(&KeyCode::KeyS) as i32;
        let right = keys.contains(&KeyCode::KeyD) as i32 - keys.contains(&KeyCode::KeyA) as i32;
        let vertical = keys.contains(&KeyCode::KeyE) as i32 - keys.contains(&KeyCode::KeyQ) as i32;
        let direction = self.forward() * forward as f32
            + self.right() * right as f32
            + Vec3::Y * vertical as f32;
        self.position += direction.normalize_or_zero() * self.speed * dt;
    }

    pub fn forward(&self) -> Vec3 {
        Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            -self.yaw.cos() * self.pitch.cos(),
        )
    }

    fn right(&self) -> Vec3 {
        Vec3::new(self.yaw.cos(), 0.0, self.yaw.sin())
    }

    pub fn view_projection(&self, width: u32, height: u32) -> Mat4 {
        let aspect = width.max(1) as f32 / height.max(1) as f32;
        let projection = Mat4::perspective_rh(60.0_f32.to_radians(), aspect, 0.05, self.far);
        let view = Mat4::look_to_rh(self.position, self.forward(), Vec3::Y);
        projection * view
    }
}
