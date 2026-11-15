//! OBJ geometry and the MTL properties used by Homework 2.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};

use glam::{Vec2, Vec3};

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    pub min: Vec3,
    pub max: Vec3,
}

impl Bounds {
    pub fn empty() -> Self {
        Self {
            min: Vec3::splat(f32::INFINITY),
            max: Vec3::splat(f32::NEG_INFINITY),
        }
    }

    pub fn is_valid(self) -> bool {
        self.min.cmple(self.max).all()
    }

    pub fn include(&mut self, point: Vec3) {
        self.min = self.min.min(point);
        self.max = self.max.max(point);
    }

    pub fn include_bounds(&mut self, other: Self) {
        if other.is_valid() {
            self.include(other.min);
            self.include(other.max);
        }
    }

    pub fn center(self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn size(self) -> Vec3 {
        self.max - self.min
    }

    pub fn corners(self) -> [Vec3; 8] {
        std::array::from_fn(|i| {
            Vec3::new(
                if i & 1 == 0 { self.min.x } else { self.max.x },
                if i & 2 == 0 { self.min.y } else { self.max.y },
                if i & 4 == 0 { self.min.z } else { self.max.z },
            )
        })
    }
}

#[derive(Clone, Debug)]
pub struct Material {
    pub name: String,
    /// `map_Ka` in the assignment; `map_Kd` is accepted as a fallback.
    pub albedo_map: Option<PathBuf>,
    pub opacity_map: Option<PathBuf>,
    pub ambient: Vec3,
    pub diffuse: Vec3,
    pub specular: Vec3,
    pub shininess: f32,
    pub opacity: f32,
}

impl Material {
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            albedo_map: None,
            opacity_map: None,
            ambient: Vec3::splat(0.1),
            diffuse: Vec3::ONE,
            specular: Vec3::splat(0.2),
            shininess: 32.0,
            opacity: 1.0,
        }
    }
}

#[derive(Debug)]
pub struct Mesh {
    pub name: String,
    pub material_index: usize,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub bounds: Bounds,
}

#[derive(Debug)]
pub struct Scene {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
    pub bounds: Bounds,
}

#[derive(Debug)]
pub enum SceneError {
    Io {
        path: PathBuf,
        source: io::Error,
    },
    Parse {
        path: PathBuf,
        line: usize,
        message: String,
    },
    Empty {
        path: PathBuf,
    },
}

impl fmt::Display for SceneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
            Self::Parse {
                path,
                line,
                message,
            } => write!(f, "{}:{line}: {message}", path.display()),
            Self::Empty { path } => write!(f, "{} contains no faces", path.display()),
        }
    }
}

impl Error for SceneError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl Scene {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, SceneError> {
        let path = path.as_ref();
        let mut materials = vec![Material::named("default")];
        let mut material_indices = HashMap::from([(String::from("default"), 0)]);
        load_material_libraries(path, &mut materials, &mut material_indices)?;

        let file = File::open(path).map_err(|source| SceneError::Io {
            path: path.to_owned(),
            source,
        })?;
        let mut scene = Self {
            meshes: Vec::new(),
            materials,
            bounds: Bounds::empty(),
        };
        let mut positions = Vec::<Vec3>::new();
        let mut texcoords = Vec::<Vec2>::new();
        let mut normals = Vec::<Vec3>::new();
        let mut object_name = String::from("object");
        let mut material_index = 0;
        let mut mesh = MeshBuilder::new(object_name.clone(), material_index);

        for (line_number, line) in BufReader::new(file).lines().enumerate() {
            let line_number = line_number + 1;
            let line = line.map_err(|source| SceneError::Io {
                path: path.to_owned(),
                source,
            })?;
            let line = line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let (tag, args) = directive(line);
            match tag {
                "v" => {
                    let v = parse_floats::<3>(args, path, line_number, "vertex position")?;
                    positions.push(Vec3::from_array(v));
                }
                "vt" => {
                    let uv = parse_floats::<2>(args, path, line_number, "texture coordinate")?;
                    // Image files are uploaded top row first, whereas OBJ v=0 is the bottom row.
                    texcoords.push(Vec2::new(uv[0], 1.0 - uv[1]));
                }
                "vn" => {
                    let n = parse_floats::<3>(args, path, line_number, "vertex normal")?;
                    normals.push(Vec3::from_array(n).normalize_or_zero());
                }
                "o" | "g" => {
                    let next_name = if args.is_empty() { "object" } else { args };
                    scene.push_mesh(mesh);
                    object_name = next_name.to_owned();
                    mesh = MeshBuilder::new(object_name.clone(), material_index);
                }
                "usemtl" => {
                    scene.push_mesh(mesh);
                    material_index = if args.is_empty() {
                        0
                    } else if let Some(&index) = material_indices.get(args) {
                        index
                    } else {
                        let index = scene.materials.len();
                        scene.materials.push(Material::named(args));
                        material_indices.insert(args.to_owned(), index);
                        index
                    };
                    mesh = MeshBuilder::new(object_name.clone(), material_index);
                }
                "f" => {
                    let face: Vec<FaceRef> = args
                        .split_whitespace()
                        .map(|token| {
                            parse_face_ref(
                                token,
                                positions.len(),
                                texcoords.len(),
                                normals.len(),
                                path,
                                line_number,
                            )
                        })
                        .collect::<Result<_, _>>()?;
                    if face.len() < 3 {
                        return Err(parse_error(
                            path,
                            line_number,
                            "face has fewer than 3 vertices",
                        ));
                    }
                    for i in 1..face.len() - 1 {
                        mesh.add_triangle(
                            [face[0], face[i], face[i + 1]],
                            &positions,
                            &texcoords,
                            &normals,
                            path,
                            line_number,
                        )?;
                    }
                }
                _ => {} // OBJ has many optional records unrelated to rendering.
            }
        }
        scene.push_mesh(mesh);
        if scene.meshes.is_empty() {
            return Err(SceneError::Empty {
                path: path.to_owned(),
            });
        }
        Ok(scene)
    }

    fn push_mesh(&mut self, mesh: MeshBuilder) {
        if let Some(mesh) = mesh.finish() {
            self.bounds.include_bounds(mesh.bounds);
            self.meshes.push(mesh);
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct FaceRef {
    position: usize,
    texcoord: Option<usize>,
    normal: Option<usize>,
}

struct MeshBuilder {
    name: String,
    material_index: usize,
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    vertex_indices: HashMap<FaceRef, u32>,
    generated_normals: Vec<Vec3>,
    missing_normals: Vec<bool>,
    bounds: Bounds,
}

impl MeshBuilder {
    fn new(name: String, material_index: usize) -> Self {
        Self {
            name,
            material_index,
            vertices: Vec::new(),
            indices: Vec::new(),
            vertex_indices: HashMap::new(),
            generated_normals: Vec::new(),
            missing_normals: Vec::new(),
            bounds: Bounds::empty(),
        }
    }

    fn add_triangle(
        &mut self,
        face: [FaceRef; 3],
        positions: &[Vec3],
        texcoords: &[Vec2],
        normals: &[Vec3],
        path: &Path,
        line: usize,
    ) -> Result<(), SceneError> {
        let mut indices = [0_u32; 3];
        for (i, vertex) in face.into_iter().enumerate() {
            indices[i] = self.add_vertex(vertex, positions, texcoords, normals, path, line)?;
        }
        let a = positions[face[0].position];
        let b = positions[face[1].position];
        let c = positions[face[2].position];
        let normal = (b - a).cross(c - a);
        for (i, vertex) in face.into_iter().enumerate() {
            if vertex.normal.is_none() {
                self.generated_normals[indices[i] as usize] += normal;
            }
        }
        self.indices.extend_from_slice(&indices);
        Ok(())
    }

    fn add_vertex(
        &mut self,
        face: FaceRef,
        positions: &[Vec3],
        texcoords: &[Vec2],
        normals: &[Vec3],
        path: &Path,
        line: usize,
    ) -> Result<u32, SceneError> {
        if let Some(&index) = self.vertex_indices.get(&face) {
            return Ok(index);
        }
        let index = u32::try_from(self.vertices.len())
            .map_err(|_| parse_error(path, line, "mesh has more than u32::MAX vertices"))?;
        let position = positions[face.position];
        let normal = face.normal.map_or(Vec3::ZERO, |index| normals[index]);
        let uv = face.texcoord.map_or(Vec2::ZERO, |index| texcoords[index]);
        self.vertices.push(Vertex {
            position: position.to_array(),
            normal: normal.to_array(),
            uv: uv.to_array(),
        });
        self.generated_normals.push(Vec3::ZERO);
        self.missing_normals.push(face.normal.is_none());
        self.vertex_indices.insert(face, index);
        self.bounds.include(position);
        Ok(index)
    }

    fn finish(mut self) -> Option<Mesh> {
        if self.indices.is_empty() {
            return None;
        }
        for (i, vertex) in self.vertices.iter_mut().enumerate() {
            if self.missing_normals[i] {
                let normal = self.generated_normals[i].normalize_or_zero();
                vertex.normal = if normal == Vec3::ZERO {
                    Vec3::Y.to_array()
                } else {
                    normal.to_array()
                };
            }
        }
        Some(Mesh {
            name: self.name,
            material_index: self.material_index,
            vertices: self.vertices,
            indices: self.indices,
            bounds: self.bounds,
        })
    }
}

fn load_material_libraries(
    obj_path: &Path,
    materials: &mut Vec<Material>,
    material_indices: &mut HashMap<String, usize>,
) -> Result<(), SceneError> {
    let file = File::open(obj_path).map_err(|source| SceneError::Io {
        path: obj_path.to_owned(),
        source,
    })?;
    let obj_dir = obj_path.parent().unwrap_or_else(|| Path::new("."));
    for line in BufReader::new(file).lines() {
        let line = line.map_err(|source| SceneError::Io {
            path: obj_path.to_owned(),
            source,
        })?;
        let line = line.split('#').next().unwrap_or("").trim();
        let (tag, args) = directive(line);
        if tag == "mtllib" {
            for filename in args.split_whitespace() {
                let path = obj_dir.join(filename);
                load_mtl(&path, materials, material_indices)?;
            }
        }
    }
    Ok(())
}

fn load_mtl(
    path: &Path,
    materials: &mut Vec<Material>,
    material_indices: &mut HashMap<String, usize>,
) -> Result<(), SceneError> {
    let file = File::open(path).map_err(|source| SceneError::Io {
        path: path.to_owned(),
        source,
    })?;
    let mtl_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut current = None::<usize>;
    for (line_number, line) in BufReader::new(file).lines().enumerate() {
        let line_number = line_number + 1;
        let line = line.map_err(|source| SceneError::Io {
            path: path.to_owned(),
            source,
        })?;
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let (tag, args) = directive(line);
        if tag == "newmtl" {
            if args.is_empty() {
                return Err(parse_error(path, line_number, "newmtl has no name"));
            }
            let index = if let Some(&index) = material_indices.get(args) {
                materials[index] = Material::named(args);
                index
            } else {
                let index = materials.len();
                materials.push(Material::named(args));
                material_indices.insert(args.to_owned(), index);
                index
            };
            current = Some(index);
            continue;
        }
        let Some(index) = current else { continue };
        let material = &mut materials[index];
        match tag {
            "Ka" => {
                material.ambient =
                    Vec3::from_array(parse_floats::<3>(args, path, line_number, "Ka")?);
            }
            "Kd" => {
                material.diffuse =
                    Vec3::from_array(parse_floats::<3>(args, path, line_number, "Kd")?);
            }
            "Ks" => {
                material.specular =
                    Vec3::from_array(parse_floats::<3>(args, path, line_number, "Ks")?);
            }
            "Ns" => {
                material.shininess = parse_floats::<1>(args, path, line_number, "Ns")?[0].max(1.0);
            }
            "d" => {
                material.opacity =
                    parse_floats::<1>(args, path, line_number, "d")?[0].clamp(0.0, 1.0);
            }
            "Tr" => {
                material.opacity =
                    1.0 - parse_floats::<1>(args, path, line_number, "Tr")?[0].clamp(0.0, 1.0);
            }
            "map_Ka" => {
                material.albedo_map =
                    Some(mtl_dir.join(texture_filename(args, path, line_number)?));
            }
            "map_Kd" if material.albedo_map.is_none() => {
                material.albedo_map =
                    Some(mtl_dir.join(texture_filename(args, path, line_number)?));
            }
            "map_d" => {
                material.opacity_map =
                    Some(mtl_dir.join(texture_filename(args, path, line_number)?));
            }
            _ => {}
        }
    }
    Ok(())
}

fn directive(line: &str) -> (&str, &str) {
    let mut parts = line.splitn(2, char::is_whitespace);
    let tag = parts.next().unwrap_or("");
    let args = parts.next().unwrap_or("").trim();
    (tag, args)
}

fn texture_filename(args: &str, path: &Path, line: usize) -> Result<String, SceneError> {
    // The common MTL map options have space-separated arguments; the filename is last.
    args.split_whitespace()
        .last()
        .map(|name| name.trim_matches('"'))
        .filter(|name| !name.is_empty())
        // Many OBJ assets use Windows separators even on other platforms.
        .map(|name| name.replace('\\', "/"))
        .ok_or_else(|| parse_error(path, line, "texture map has no filename"))
}

fn parse_floats<const N: usize>(
    args: &str,
    path: &Path,
    line: usize,
    what: &str,
) -> Result<[f32; N], SceneError> {
    let mut parts = args.split_whitespace();
    let mut values = [0.0; N];
    for value in &mut values {
        let token = parts
            .next()
            .ok_or_else(|| parse_error(path, line, format!("{what} has too few values")))?;
        *value = token
            .parse::<f32>()
            .map_err(|_| parse_error(path, line, format!("invalid {what} value: {token}")))?;
        if !value.is_finite() {
            return Err(parse_error(path, line, format!("non-finite {what} value")));
        }
    }
    Ok(values)
}

fn parse_face_ref(
    token: &str,
    position_count: usize,
    texcoord_count: usize,
    normal_count: usize,
    path: &Path,
    line: usize,
) -> Result<FaceRef, SceneError> {
    let mut fields = token.split('/');
    let position = resolve_index(
        fields.next().unwrap_or(""),
        position_count,
        "position",
        path,
        line,
    )?;
    let texcoord = fields
        .next()
        .filter(|field| !field.is_empty())
        .map(|field| resolve_index(field, texcoord_count, "texcoord", path, line))
        .transpose()?;
    let normal = fields
        .next()
        .filter(|field| !field.is_empty())
        .map(|field| resolve_index(field, normal_count, "normal", path, line))
        .transpose()?;
    if fields.next().is_some() {
        return Err(parse_error(path, line, "face vertex has too many fields"));
    }
    Ok(FaceRef {
        position,
        texcoord,
        normal,
    })
}

fn resolve_index(
    raw: &str,
    count: usize,
    kind: &str,
    path: &Path,
    line: usize,
) -> Result<usize, SceneError> {
    let index = raw
        .parse::<isize>()
        .map_err(|_| parse_error(path, line, format!("invalid {kind} index: {raw}")))?;
    let resolved = if index > 0 {
        index - 1
    } else if index < 0 {
        count as isize + index
    } else {
        -1
    };
    if resolved < 0 || resolved >= count as isize {
        return Err(parse_error(
            path,
            line,
            format!("{kind} index {index} is out of range (count {count})"),
        ));
    }
    Ok(resolved as usize)
}

fn parse_error(path: &Path, line: usize, message: impl Into<String>) -> SceneError {
    SceneError::Parse {
        path: path.to_owned(),
        line,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::ErrorKind;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            loop {
                let id = NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
                let path = std::env::temp_dir()
                    .join(format!("homework02-scene-{}-{id}", std::process::id()));
                match fs::create_dir(&path) {
                    Ok(()) => return Self(path),
                    Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
                    Err(error) => panic!("cannot create {}: {error}", path.display()),
                }
            }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn loads_materials_objects_and_negative_quad_indices() {
        let fixture = Fixture::new();
        fs::write(
            fixture.0.join("sample.mtl"),
            "newmtl leaves\nKa 0.2 0.3 0.4\nKd 0.8 0.7 0.6\nKs 0.3 0.4 0.5\nNs 64\nmap_Ka tex/albedo.png\nmap_d tex/mask.png\nnewmtl stone\nmap_Kd stone.png\n",
        )
        .unwrap();
        let obj = fixture.0.join("sample.obj");
        fs::write(
            &obj,
            "mtllib sample.mtl\no floor\nv -1 0 -1\nv 1 0 -1\nv 1 0 1\nv -1 0 1\nvt 0 0\nvt 1 0\nvt 1 1\nvt 0 1\nusemtl leaves\nf -4/-4 -3/-3 -2/-2 -1/-1\no wall\nusemtl stone\nf 1 2 3\n",
        )
        .unwrap();

        let scene = Scene::load(&obj).unwrap();
        assert_eq!(scene.meshes.len(), 2);
        assert_eq!(scene.meshes[0].name, "floor");
        assert_eq!(scene.meshes[0].indices.len(), 6);
        assert_eq!(scene.meshes[0].vertices.len(), 4);
        assert_eq!(scene.meshes[0].vertices[0].uv, [0.0, 1.0]);
        assert_eq!(scene.meshes[0].vertices[0].normal, [0.0, -1.0, 0.0]);
        assert_eq!(scene.meshes[1].name, "wall");
        assert_eq!(scene.bounds.min, Vec3::new(-1.0, 0.0, -1.0));
        assert_eq!(scene.bounds.max, Vec3::new(1.0, 0.0, 1.0));
        let leaves = &scene.materials[scene.meshes[0].material_index];
        assert_eq!(
            leaves.albedo_map.as_ref().unwrap(),
            &fixture.0.join("tex/albedo.png")
        );
        assert_eq!(
            leaves.opacity_map.as_ref().unwrap(),
            &fixture.0.join("tex/mask.png")
        );
        assert_eq!(leaves.specular, Vec3::new(0.3, 0.4, 0.5));
        assert_eq!(leaves.shininess, 64.0);
        let stone = &scene.materials[scene.meshes[1].material_index];
        assert_eq!(
            stone.albedo_map.as_ref().unwrap(),
            &fixture.0.join("stone.png")
        );
    }

    #[test]
    fn rejects_out_of_range_face_indices() {
        let fixture = Fixture::new();
        let obj = fixture.0.join("broken.obj");
        fs::write(&obj, "v 0 0 0\nv 1 0 0\nf 1 2 3\n").unwrap();
        let error = Scene::load(&obj).unwrap_err();
        assert!(matches!(error, SceneError::Parse { line: 3, .. }));
    }

    #[test]
    fn resolves_windows_texture_paths_relative_to_mtl() {
        let fixture = Fixture::new();
        fs::write(
            fixture.0.join("sample.mtl"),
            "newmtl leaves\nmap_Ka textures\\albedo.png\nmap_d textures\\opacity.png\n",
        )
        .unwrap();
        let obj = fixture.0.join("sample.obj");
        fs::write(
            &obj,
            "mtllib sample.mtl\nv 0 0 0\nv 1 0 0\nv 0 1 0\nusemtl leaves\nf 1 2 3\n",
        )
        .unwrap();

        let scene = Scene::load(&obj).unwrap();
        let material = &scene.materials[scene.meshes[0].material_index];
        assert_eq!(
            material.albedo_map.as_ref().unwrap(),
            &fixture.0.join("textures/albedo.png")
        );
        assert_eq!(
            material.opacity_map.as_ref().unwrap(),
            &fixture.0.join("textures/opacity.png")
        );
    }

    #[test]
    fn honors_explicit_normals() {
        let fixture = Fixture::new();
        let obj = fixture.0.join("normals.obj");
        fs::write(
            &obj,
            "v 0 0 0\nv 1 0 0\nv 0 1 0\nvt 0 0\nvt 1 0\nvt 0 1\nvn 0 0 1\nf 1/1/1 2/2/1 3/3/1\n",
        )
        .unwrap();
        let scene = Scene::load(obj).unwrap();
        assert_eq!(scene.meshes[0].vertices[2].normal, [0.0, 0.0, 1.0]);
    }
}
