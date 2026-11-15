use crate::mesh::SurfaceVertex;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ColorVertex {
    pub color: [f32; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceStyle {
    pub base: [f32; 3],       // основной цвет rgb
    pub accent: [f32; 3],     // второй цвет rgb
    pub band_frequency: f32,  // частота полос
    pub band_strength: f32,   // преобладание полос над неровным узором
    pub band_warp: f32,       // искривление полос
    pub grain_frequency: f32, // частота мелких неровностей
    pub grain_strength: f32,  //насколько неровности затемняют свет
    pub phase: f32,           // сдвиг узора
    pub is_sun: bool,
}

impl SurfaceStyle {
    pub fn sun() -> Self {
        Self {
            base: [1.0, 0.40, 0.04],
            accent: [1.0, 0.90, 0.23],
            band_frequency: 0.0,
            band_strength: 0.0,
            band_warp: 0.0,
            grain_frequency: 0.0,
            grain_strength: 0.0,
            phase: 0.0,
            is_sun: true,
        }
    }
}

pub fn hsv_to_rgb(hue: f32, saturation: f32, value: f32) -> [f32; 3] {
    let sector = hue.rem_euclid(1.0) * 6.0;
    let chroma = value * saturation;
    let secondary = chroma * (1.0 - ((sector % 2.0) - 1.0).abs());
    let offset = value - chroma;
    let rgb = match sector as u32 {
        0 => [chroma, secondary, 0.0],
        1 => [secondary, chroma, 0.0],
        2 => [0.0, chroma, secondary],
        3 => [0.0, secondary, chroma],
        4 => [secondary, 0.0, chroma],
        _ => [chroma, 0.0, secondary],
    };
    rgb.map(|channel| channel + offset)
}

pub fn make_colors(vertices: &[SurfaceVertex], style: SurfaceStyle, time: f32) -> Vec<ColorVertex> {
    vertices
        .iter()
        .map(|vertex| {
            let [x, y, z] = vertex.position;
            let color = if style.is_sun {
                let fire = 0.5
                    + 0.5
                        * (x * 19.0 + time * 3.0).sin()
                        * (y * 23.0 - time * 2.0).sin()
                        * (z * 17.0 + time * 1.7).sin();
                mix(style.base, style.accent, fire)
            } else {
                let grain = (x * style.grain_frequency + style.phase).sin()
                    * (z * style.grain_frequency * 0.73 - y * 3.7).sin();
                let bands = 0.5
                    + 0.5
                        * (y * style.band_frequency + grain * style.band_warp + style.phase).sin();
                let irregular = 0.5 + 0.5 * grain;
                let pattern = irregular * (1.0 - style.band_strength) + bands * style.band_strength;
                let brightness = 1.0 - style.grain_strength * (1.0 - irregular);
                mix(style.base, style.accent, pattern)
                    .map(|channel: f32| (channel * brightness).clamp(0.0, 1.0))
            };
            ColorVertex { color }
        })
        .collect()
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}
