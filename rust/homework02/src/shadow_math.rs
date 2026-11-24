use glam::{Mat4, Vec3, Vec4};

use crate::scene::Bounds;

pub fn directional_projection(
    scene_bounds: Bounds,
    camera_vp: Mat4,
    light_direction: Vec3,
) -> Mat4 {
    let visible_bounds = camera_frustum_bounds(camera_vp)
        .map(|frustum| Bounds {
            min: scene_bounds.min.max(frustum.min),
            max: scene_bounds.max.min(frustum.max),
        })
        .filter(|bounds| bounds.is_valid())
        .unwrap_or(scene_bounds);

    let direction = if light_direction.length_squared() > 1e-12 {
        light_direction.normalize()
    } else {
        -Vec3::Y
    };
    let up = if direction.dot(Vec3::Y).abs() > 0.95 {
        Vec3::Z
    } else {
        Vec3::Y
    };
    let distance = scene_bounds.size().length().max(1.0) * 2.0;
    let eye = scene_bounds.center() - direction * distance;
    let view = Mat4::look_to_rh(eye, direction, up);

    let mut x_min = f32::INFINITY;
    let mut x_max = f32::NEG_INFINITY;
    let mut y_min = f32::INFINITY;
    let mut y_max = f32::NEG_INFINITY;
    for corner in visible_bounds.corners() {
        let projected = view.transform_point3(corner);
        x_min = x_min.min(projected.x);
        x_max = x_max.max(projected.x);
        y_min = y_min.min(projected.y);
        y_max = y_max.max(projected.y);
    }
    let (left, right) = padded_range(x_min, x_max);
    let (bottom, top) = padded_range(y_min, y_max);

    let mut z_min = f32::INFINITY;
    let mut z_max = f32::NEG_INFINITY;
    for corner in scene_bounds.corners() {
        let z = view.transform_point3(corner).z;
        z_min = z_min.min(z);
        z_max = z_max.max(z);
    }
    let mut near = (-z_max).max(0.001);
    let mut far = (-z_min).max(near + 0.01);
    let depth_padding = (far - near) * 0.01 + 0.01;
    near = (near - depth_padding).max(0.001);
    far += depth_padding;

    Mat4::orthographic_rh(left, right, bottom, top, near, far) * view
}

pub fn point_projections(position: Vec3, far: f32) -> [Mat4; 6] {
    let far = far.max(0.02);
    let near = (far * 0.0005).max(0.01);
    let projection = Mat4::perspective_lh(std::f32::consts::FRAC_PI_2, 1.0, near, far);
    let directions = [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z];
    let ups = [Vec3::Y, Vec3::Y, -Vec3::Z, Vec3::Z, Vec3::Y, Vec3::Y];
    std::array::from_fn(|face| projection * Mat4::look_to_lh(position, directions[face], ups[face]))
}

fn camera_frustum_bounds(camera_vp: Mat4) -> Option<Bounds> {
    let determinant = camera_vp.determinant();
    if !determinant.is_finite() || determinant.abs() < 1e-12 {
        return None;
    }
    let inverse = camera_vp.inverse();
    let mut bounds = Bounds::empty();
    for x in [-1.0, 1.0] {
        for y in [-1.0, 1.0] {
            for z in [0.0, 1.0] {
                let homogenous = inverse * Vec4::new(x, y, z, 1.0);
                if !homogenous.is_finite() || homogenous.w.abs() < 1e-12 {
                    return None;
                }
                bounds.include(homogenous.truncate() / homogenous.w);
            }
        }
    }
    bounds.is_valid().then_some(bounds)
}

fn padded_range(min: f32, max: f32) -> (f32, f32) {
    let middle = (min + max) * 0.5;
    let half_extent = ((max - min) * 0.505).max(0.01);
    (middle - half_extent, middle + half_extent)
}
