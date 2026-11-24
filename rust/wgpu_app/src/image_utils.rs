use image::RgbaImage;

fn srgb_to_linear(value: f32) -> f32 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(value: f32) -> f32 {
    if value <= 0.0031308 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
}

pub fn downsample(image: &RgbaImage, srgb: bool) -> RgbaImage {
    let width = image.width() / 2;
    let height = image.height() / 2;
    let mut result = RgbaImage::new(width, height);

    for y in 0..height {
        for x in 0..width {
            let source_x = 2 * x;
            let source_y = 2 * y;
            let mut sum = [0.0f32; 4];

            for dy in 0..2 {
                for dx in 0..2 {
                    let pixel = image.get_pixel(source_x + dx, source_y + dy).0;
                    for channel in 0..4 {
                        let value = pixel[channel] as f32 / 255.0;
                        sum[channel] += if srgb && channel < 3 {
                            srgb_to_linear(value)
                        } else {
                            value
                        };
                    }
                }
            }

            let pixel = result.get_pixel_mut(x, y);
            for channel in 0..4 {
                let mut value = sum[channel] * 0.25;
                if srgb && channel < 3 {
                    value = linear_to_srgb(value);
                }
                pixel.0[channel] = (value * 255.0).clamp(0.0, 255.0) as u8;
            }
        }
    }

    result
}
