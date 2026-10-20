use std::collections::HashMap;

use glam::Vec3;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SurfaceVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

pub struct SphereMesh {
    pub vertices: Vec<SurfaceVertex>,
    pub indices: Vec<u32>,
}

pub fn make_icosphere(subdivisions: u32) -> SphereMesh {
    let golden_ratio = (1.0 + 5.0_f32.sqrt()) / 2.0;
    let positions = [
        Vec3::new(-1.0, golden_ratio, 0.0),
        Vec3::new(1.0, golden_ratio, 0.0),
        Vec3::new(-1.0, -golden_ratio, 0.0),
        Vec3::new(1.0, -golden_ratio, 0.0),
        Vec3::new(0.0, -1.0, golden_ratio),
        Vec3::new(0.0, 1.0, golden_ratio),
        Vec3::new(0.0, -1.0, -golden_ratio),
        Vec3::new(0.0, 1.0, -golden_ratio),
        Vec3::new(golden_ratio, 0.0, -1.0),
        Vec3::new(golden_ratio, 0.0, 1.0),
        Vec3::new(-golden_ratio, 0.0, -1.0),
        Vec3::new(-golden_ratio, 0.0, 1.0),
    ];
    let mut vertices: Vec<_> = positions
        .into_iter()
        .map(|p| surface_vertex(p.normalize()))
        .collect();

    let mut faces: Vec<[u32; 3]> = vec![
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ];

    for _ in 0..subdivisions {
        let mut midpoints = HashMap::new();
        let mut next_faces = Vec::with_capacity(faces.len() * 4);
        for [a, b, c] in faces {
            let ab = midpoint(a, b, &mut vertices, &mut midpoints);
            let bc = midpoint(b, c, &mut vertices, &mut midpoints);
            let ca = midpoint(c, a, &mut vertices, &mut midpoints);
            next_faces.extend_from_slice(&[[a, ab, ca], [b, bc, ab], [c, ca, bc], [ab, bc, ca]]);
        }
        faces = next_faces;
    }

    SphereMesh {
        vertices,
        indices: faces.into_iter().flatten().collect(),
    }
}

fn surface_vertex(position: Vec3) -> SurfaceVertex {
    let position = position.to_array();
    SurfaceVertex {
        position,
        normal: position,
    }
}

fn midpoint(
    a: u32,
    b: u32,
    vertices: &mut Vec<SurfaceVertex>,
    cache: &mut HashMap<(u32, u32), u32>,
) -> u32 {
    let edge = (a.min(b), a.max(b));
    if let Some(&index) = cache.get(&edge) {
        return index;
    }

    let pa = Vec3::from_array(vertices[a as usize].position);
    let pb = Vec3::from_array(vertices[b as usize].position);
    let index = u32::try_from(vertices.len()).expect("icosphere has too many vertices");
    vertices.push(surface_vertex((pa + pb).normalize()));
    cache.insert(edge, index);
    index
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RingVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub radial: f32,
}

pub struct RingMesh {
    pub vertices: Vec<RingVertex>,
    pub indices: Vec<u32>,
}

pub fn make_ring(inner_radius: f32, outer_radius: f32, segments: u32) -> RingMesh {
    assert!(inner_radius > 0.0 && inner_radius < outer_radius);
    assert!(segments >= 3);

    let mut vertices = Vec::with_capacity((segments * 2) as usize);
    let mut indices = Vec::with_capacity((segments * 6) as usize);
    for i in 0..segments {
        let angle = std::f32::consts::TAU * (i as f32) / (segments as f32);
        let (sin, cos) = angle.sin_cos();
        let normal = [0.0, 1.0, 0.0];
        vertices.push(RingVertex {
            position: [inner_radius * cos, 0.0, inner_radius * sin],
            normal,
            radial: 0.0,
        });
        vertices.push(RingVertex {
            position: [outer_radius * cos, 0.0, outer_radius * sin],
            normal,
            radial: 1.0,
        });

        let inner = i * 2;
        let outer = inner + 1;
        let next_inner = ((i + 1) % segments) * 2;
        let next_outer = next_inner + 1;
        indices.extend_from_slice(&[inner, next_inner, outer, outer, next_inner, next_outer]);
    }

    RingMesh { vertices, indices }
}
