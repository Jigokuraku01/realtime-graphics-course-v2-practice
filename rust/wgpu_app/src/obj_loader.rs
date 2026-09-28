use std::collections::HashMap;
use std::fs;
use std::path::Path;
use glam::Vec3;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ObjVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub texcoord: [f32; 2],
}

pub struct ObjMesh {
    pub vertices: Vec<ObjVertex>,
    pub indices: Vec<u32>,
}

#[derive(Eq, PartialEq, Hash, Copy, Clone)]
struct VertexKey {
    position: u32,
    texcoord: u32,
    normal: u32,
}

#[derive(Copy, Clone)]
struct ObjIndex {
    position: i32,
    texcoord: i32,
    normal: i32,
}

pub fn load_obj(path: impl AsRef<Path>) -> std::io::Result<ObjMesh> {
    let contents = fs::read_to_string(path)?;
    let mut positions = Vec::new();
    let mut texcoords = Vec::new();
    let mut normals = Vec::new();
    let mut unique = HashMap::new();
    let mut mesh = ObjMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
    };

    for (line_number, line) in contents.lines().enumerate() {
        let line_number = line_number + 1;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(rest) = line.strip_prefix("v ") {
            positions.push(parse_vec3(rest, "invalid OBJ vertex", line_number)?);
        } else if let Some(rest) = line.strip_prefix("vt ") {
            let parts = parse_floats(rest, 2, "invalid OBJ texcoord", line_number)?;
            texcoords.push([parts[0], parts[1]]);
        } else if let Some(rest) = line.strip_prefix("vn ") {
            normals.push(parse_vec3(rest, "invalid OBJ normal", line_number)?);
        } else if let Some(rest) = line.strip_prefix("f ") {
            let corners = parse_face(rest, line_number)?;
            if corners.len() < 3 {
                return Err(invalid_obj("OBJ face has fewer than three vertices", line_number));
            }

            for i in 1..corners.len() - 1 {
                for index in [corners[0], corners[i], corners[i + 1]] {
                    let key = VertexKey {
                        position: resolve_index(index.position, positions.len(), line_number)?,
                        texcoord: if index.texcoord == 0 {
                            0
                        } else {
                            resolve_index(index.texcoord, texcoords.len(), line_number)? + 1
                        },
                        normal: if index.normal == 0 {
                            0
                        } else {
                            resolve_index(index.normal, normals.len(), line_number)? + 1
                        },
                    };

                    let index = *unique.entry(key).or_insert_with(|| {
                        let vertex = ObjVertex {
                            position: positions[key.position as usize].to_array(),
                            normal: if key.normal == 0 {
                                [0.0; 3]
                            } else {
                                normals[(key.normal - 1) as usize].to_array()
                            },
                            texcoord: if key.texcoord == 0 {
                                [0.0; 2]
                            } else {
                                texcoords[(key.texcoord - 1) as usize]
                            },
                        };
                        mesh.vertices.push(vertex);
                        (mesh.vertices.len() - 1) as u32
                    });
                    mesh.indices.push(index);
                }
            }
        }
    }

    Ok(mesh)
}

fn parse_vec3(input: &str, what: &str, line_number: usize) -> std::io::Result<Vec3> {
    let parts = parse_floats(input, 3, what, line_number)?;
    Ok(Vec3::new(parts[0], parts[1], parts[2]))
}

fn parse_floats(
    input: &str,
    count: usize,
    what: &str,
    line_number: usize,
) -> std::io::Result<Vec<f32>> {
    let values = input
        .split_whitespace()
        .take(count)
        .map(|part| {
            part.parse::<f32>().map_err(|_| {
                invalid_obj(what, line_number)
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if values.len() != count {
        return Err(invalid_obj(what, line_number));
    }
    Ok(values)
}

fn parse_face(input: &str, line_number: usize) -> std::io::Result<Vec<ObjIndex>> {
    let mut corners = Vec::new();
    for part in input.split_whitespace() {
        if corners.len() == 16 {
            return Err(invalid_obj("OBJ face has too many vertices", line_number));
        }
        corners.push(parse_face_corner(part, line_number)?);
    }
    Ok(corners)
}

fn parse_face_corner(part: &str, line_number: usize) -> std::io::Result<ObjIndex> {
    let mut pieces = part.split('/');
    let position = pieces
        .next()
        .ok_or_else(|| invalid_obj("invalid OBJ face", line_number))?
        .parse::<i32>()
        .map_err(|_| invalid_obj("invalid OBJ face", line_number))?;
    let texcoord = pieces
        .next()
        .filter(|value| !value.is_empty())
        .map(|value| value.parse::<i32>())
        .transpose()
        .map_err(|_| invalid_obj("invalid OBJ face", line_number))?
        .unwrap_or(0);
    let normal = pieces
        .next()
        .filter(|value| !value.is_empty())
        .map(|value| value.parse::<i32>())
        .transpose()
        .map_err(|_| invalid_obj("invalid OBJ face", line_number))?
        .unwrap_or(0);

    Ok(ObjIndex {
        position,
        texcoord,
        normal,
    })
}

fn resolve_index(index: i32, size: usize, line_number: usize) -> std::io::Result<u32> {
    let resolved = if index > 0 {
        index - 1
    } else {
        size as i32 + index
    };
    if index == 0 || resolved < 0 || resolved >= size as i32 {
        return Err(invalid_obj("invalid OBJ index", line_number));
    }
    Ok(resolved as u32)
}

fn invalid_obj(message: &str, line_number: usize) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!("{message} at line {line_number}"),
    )
}
