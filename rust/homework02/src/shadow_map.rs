use wgpu::{BindGroupLayout, Device, TextureView};

pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rg32Float;
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;
pub const SUN_SIZE: u32 = 2048;
pub const POINT_SIZE: u32 = 1024;

fn color_texture(device: &Device, label: &'static str, size: u32, layers: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: layers,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    })
}

fn array_view(texture: &wgpu::Texture, layers: u32) -> TextureView {
    texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        base_array_layer: 0,
        array_layer_count: Some(layers),
        ..Default::default()
    })
}

fn layer_views(texture: &wgpu::Texture, layers: u32) -> Vec<TextureView> {
    (0..layers)
        .map(|layer| {
            texture.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: layer,
                array_layer_count: Some(1),
                ..Default::default()
            })
        })
        .collect()
}

pub struct ShadowMap {
    _raw: wgpu::Texture,
    _horizontal: wgpu::Texture,
    _filtered: wgpu::Texture,
    _depth: wgpu::Texture,
    pub raw_layers: Vec<TextureView>,
    horizontal_layers: Vec<TextureView>,
    filtered_layers: Vec<TextureView>,
    pub depth_layers: Vec<TextureView>,
    pub sample_view: TextureView,
    raw_source: wgpu::BindGroup,
    horizontal_source: wgpu::BindGroup,
    pub size: u32,
    layers: u32,
}

impl ShadowMap {
    pub fn new(device: &Device, blur: &BlurRenderer, size: u32, cube: bool) -> Self {
        let layers = if cube { 6 } else { 1 };
        let raw = color_texture(device, "raw shadow moments", size, layers);
        let horizontal = color_texture(device, "horizontal shadow moments", size, layers);
        let filtered = color_texture(device, "filtered shadow moments", size, layers);
        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow depth"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: layers,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let raw_layers = layer_views(&raw, layers);
        let horizontal_layers = layer_views(&horizontal, layers);
        let filtered_layers = layer_views(&filtered, layers);
        let depth_layers = layer_views(&depth, layers);
        let sample_view = filtered.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(if cube {
                wgpu::TextureViewDimension::Cube
            } else {
                wgpu::TextureViewDimension::D2
            }),
            base_array_layer: 0,
            array_layer_count: Some(layers),
            ..Default::default()
        });
        let raw_array = array_view(&raw, layers);
        let horizontal_array = array_view(&horizontal, layers);
        let raw_source = blur.source_bind_group(device, &raw_array);
        let horizontal_source = blur.source_bind_group(device, &horizontal_array);
        Self {
            _raw: raw,
            _horizontal: horizontal,
            _filtered: filtered,
            _depth: depth,
            raw_layers,
            horizontal_layers,
            filtered_layers,
            depth_layers,
            sample_view,
            raw_source,
            horizontal_source,
            size,
            layers,
        }
    }
}

pub struct BlurRenderer {
    layout: BindGroupLayout,
    sampler: wgpu::Sampler,
    pipeline: wgpu::RenderPipeline,
}

impl BlurRenderer {
    pub fn new(device: &Device) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shadow blur source"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow blur sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("separable Gaussian blur"),
            source: wgpu::ShaderSource::Wgsl(include_str!("blur.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow blur pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 16,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow blur pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertexMain"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragmentMain"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        Self {
            layout,
            sampler,
            pipeline,
        }
    }

    fn source_bind_group(&self, device: &Device, source: &TextureView) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blur source bind group"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(source),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    fn pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &TextureView,
        source: &wgpu::BindGroup,
        layer: u32,
        step: [f32; 2],
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("shadow blur pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 1.0,
                        g: 1.0,
                        b: 0.0,
                        a: 0.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, source, &[]);
        let params = [layer as f32, step[0], step[1], 0.0];
        pass.set_immediates(0, bytemuck::bytes_of(&params));
        pass.draw(0..3, 0..1);
    }

    pub fn blur(&self, encoder: &mut wgpu::CommandEncoder, map: &ShadowMap) {
        let texel = 1.0 / map.size as f32;
        for layer in 0..map.layers {
            self.pass(
                encoder,
                &map.horizontal_layers[layer as usize],
                &map.raw_source,
                layer,
                [texel, 0.0],
            );
            self.pass(
                encoder,
                &map.filtered_layers[layer as usize],
                &map.horizontal_source,
                layer,
                [0.0, texel],
            );
        }
    }
}
