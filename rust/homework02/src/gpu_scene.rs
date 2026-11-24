use std::collections::HashMap;
use std::mem;
use std::path::{Path, PathBuf};

use wgpu::util::DeviceExt;

use crate::scene::{Material, Scene, Vertex};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MaterialUniform {
    ambient: [f32; 4],
    diffuse: [f32; 4],
    specular: [f32; 4],
    params: [f32; 4],
}

pub struct GpuMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    material_index: usize,
}

struct GpuMaterial {
    _uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

struct GpuTexture {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
}

pub struct GpuScene {
    meshes: Vec<GpuMesh>,
    materials: Vec<GpuMaterial>,
    _textures: Vec<GpuTexture>,
    _sampler: wgpu::Sampler,
}

pub fn material_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("material layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

pub fn vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    const ATTRIBUTES: [wgpu::VertexAttribute; 3] = [
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x3,
            offset: 0,
            shader_location: 0,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x3,
            offset: 12,
            shader_location: 1,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x2,
            offset: 24,
            shader_location: 2,
        },
    ];
    wgpu::VertexBufferLayout {
        array_stride: mem::size_of::<Vertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ATTRIBUTES,
    }
}

fn srgb_to_linear(byte: u8) -> f32 {
    let color = f32::from(byte) / 255.0;
    if color <= 0.04045 {
        color / 12.92
    } else {
        ((color + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb_byte(linear: f32) -> u8 {
    let linear = linear.clamp(0.0, 1.0);
    let color = if linear <= 0.0031308 {
        12.92 * linear
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    };
    (color * 255.0).round() as u8
}

fn downsample_srgb(image: &image::RgbaImage) -> image::RgbaImage {
    let width = (image.width() / 2).max(1);
    let height = (image.height() / 2).max(1);
    let decode: [f32; 256] = std::array::from_fn(|byte| srgb_to_linear(byte as u8));
    image::RgbaImage::from_fn(width, height, |x, y| {
        let mut rgb = [0.0_f32; 3];
        let mut alpha_sum = 0.0_f32;
        let x_start = x * image.width() / width;
        let x_end = (x + 1) * image.width() / width;
        let y_start = y * image.height() / height;
        let y_end = (y + 1) * image.height() / height;
        for source_y in y_start..y_end {
            for source_x in x_start..x_end {
                let pixel = image.get_pixel(source_x, source_y).0;
                let alpha = f32::from(pixel[3]) / 255.0;
                alpha_sum += alpha;
                for channel in 0..3 {
                    rgb[channel] += decode[pixel[channel] as usize] * alpha;
                }
            }
        }
        let samples = ((x_end - x_start) * (y_end - y_start)) as f32;
        let alpha = (alpha_sum / samples * 255.0).round() as u8;
        let color = rgb.map(|sum| linear_to_srgb_byte(sum / alpha_sum.max(1e-6)));
        image::Rgba([color[0], color[1], color[2], alpha])
    })
}

fn upload_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    path: &Path,
    srgb: bool,
) -> Result<GpuTexture, String> {
    let mut image = image::ImageReader::open(path)
        .map_err(|error| format!("{}: {error}", path.display()))?
        .decode()
        .map_err(|error| format!("{}: {error}", path.display()))?
        .to_rgba8();
    let (width, height) = image.dimensions();
    let mip_level_count = u32::BITS - width.max(height).leading_zeros();
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("material texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: if srgb {
            wgpu::TextureFormat::Rgba8UnormSrgb
        } else {
            wgpu::TextureFormat::Rgba8Unorm
        },
        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    for level in 0..mip_level_count {
        let (level_width, level_height) = image.dimensions();
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: level,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            image.as_raw(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(level_width * 4),
                rows_per_image: Some(level_height),
            },
            wgpu::Extent3d {
                width: level_width,
                height: level_height,
                depth_or_array_layers: 1,
            },
        );
        if level + 1 < mip_level_count {
            image = if srgb {
                downsample_srgb(&image)
            } else {
                image::imageops::resize(
                    &image,
                    (level_width / 2).max(1),
                    (level_height / 2).max(1),
                    image::imageops::FilterType::Triangle,
                )
            };
        }
    }
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    Ok(GpuTexture {
        _texture: texture,
        view,
    })
}

fn white_texture(device: &wgpu::Device, queue: &wgpu::Queue) -> GpuTexture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("white fallback texture"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &[255, 255, 255, 255],
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    GpuTexture {
        _texture: texture,
        view,
    }
}

fn get_texture(
    path: Option<&PathBuf>,
    srgb: bool,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    textures: &mut Vec<GpuTexture>,
    cache: &mut HashMap<(PathBuf, bool), usize>,
) -> Result<usize, String> {
    let Some(path) = path else {
        return Ok(0);
    };
    let key = (path.clone(), srgb);
    if let Some(index) = cache.get(&key) {
        return Ok(*index);
    }
    let index = textures.len();
    textures.push(upload_texture(device, queue, path, srgb)?);
    cache.insert(key, index);
    Ok(index)
}

impl GpuScene {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: Scene,
        material_layout: &wgpu::BindGroupLayout,
    ) -> Result<Self, String> {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("material sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        let mut textures = vec![white_texture(device, queue)];
        let mut cache = HashMap::<(PathBuf, bool), usize>::new();
        let mut materials = Vec::with_capacity(scene.materials.len());
        for source in &scene.materials {
            let albedo_index = get_texture(
                source.albedo_map.as_ref(),
                true,
                device,
                queue,
                &mut textures,
                &mut cache,
            )?;
            let opacity_index = get_texture(
                source.opacity_map.as_ref(),
                false,
                device,
                queue,
                &mut textures,
                &mut cache,
            )?;
            materials.push(create_material(
                device,
                material_layout,
                &sampler,
                source,
                &textures[albedo_index].view,
                &textures[opacity_index].view,
            ));
        }

        let mut batches: Vec<(Vec<Vertex>, Vec<u32>)> = (0..materials.len())
            .map(|_| (Vec::new(), Vec::new()))
            .collect();
        for mesh in scene.meshes {
            let material_index = mesh.material_index;
            let (vertices, indices) = batches.get_mut(material_index).ok_or_else(|| {
                format!(
                    "mesh {} has invalid material index {material_index}",
                    mesh.name
                )
            })?;
            let end = vertices
                .len()
                .checked_add(mesh.vertices.len())
                .ok_or("scene contains too many vertices")?;
            let index_end = indices
                .len()
                .checked_add(mesh.indices.len())
                .ok_or("scene contains too many indices")?;
            if end > u32::MAX as usize || index_end > u32::MAX as usize {
                return Err("scene contains too much geometry for u32 indices".into());
            }
            let first_vertex = vertices.len() as u32;
            indices.extend(mesh.indices.into_iter().map(|index| index + first_vertex));
            vertices.extend(mesh.vertices);
        }
        let meshes = batches
            .into_iter()
            .enumerate()
            .filter(|(_, (_, indices))| !indices.is_empty())
            .map(|(material_index, (vertices, indices))| GpuMesh {
                vertex_buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("scene material vertices"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                }),
                index_buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("scene material indices"),
                    contents: bytemuck::cast_slice(&indices),
                    usage: wgpu::BufferUsages::INDEX,
                }),
                index_count: indices.len() as u32,
                material_index,
            })
            .collect();

        Ok(Self {
            meshes,
            materials,
            _textures: textures,
            _sampler: sampler,
        })
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        for mesh in &self.meshes {
            pass.set_bind_group(1, &self.materials[mesh.material_index].bind_group, &[]);
            pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.index_count, 0, 0..1);
        }
    }
}

fn create_material(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    source: &Material,
    albedo_view: &wgpu::TextureView,
    opacity_view: &wgpu::TextureView,
) -> GpuMaterial {
    let uniform = MaterialUniform {
        ambient: source.ambient.extend(0.0).to_array(),
        diffuse: source.diffuse.extend(0.0).to_array(),
        specular: source.specular.extend(0.0).to_array(),
        params: [
            source.shininess,
            source.albedo_map.is_some() as i32 as f32,
            source.opacity,
            source.opacity_map.is_some() as i32 as f32,
        ],
    };
    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(&source.name),
        contents: bytemuck::bytes_of(&uniform),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(&source.name),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(albedo_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(opacity_view),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    });
    GpuMaterial {
        _uniform_buffer: uniform_buffer,
        bind_group,
    }
}
