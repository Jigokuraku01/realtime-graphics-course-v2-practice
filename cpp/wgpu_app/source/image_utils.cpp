#include <image_utils.hpp>
#include <math/aliases.hpp>

#define STB_IMAGE_IMPLEMENTATION
#include <stb_image.h>

namespace
{

    struct StbiDeleter {
        void operator()(stbi_uc * pixels) {
            stbi_image_free(pixels);
        }
    };

    float toLinear(float x) {
        if (x <= 0.04045f) {
            return x / 12.92f;
        } else {
            return std::pow((x + 0.055f) / 1.055f, 2.4f);
        }
    }

    float toSrgb(float x) {
        if (x <= 0.0031308f) {
            return x * 12.92f;
        } else {
            return 1.055f * std::pow(x, 1.0f / 2.4f) - 0.055f;
        }
    }

    math::vector4f toFloat(Image::Pixel const & pixel, bool srgb) {
        math::vector4f result = math::cast<float>(pixel) / 255.f;
        if (srgb) {
            result[0] = toLinear(result[0]);
            result[1] = toLinear(result[1]);
            result[2] = toLinear(result[2]);
        }
        return result;
    }

    Image::Pixel toPixel(math::vector4f const & color, bool srgb) {
        math::vector4f result = color;
        if (srgb) {
            result[0] = toSrgb(result[0]);
            result[1] = toSrgb(result[1]);
            result[2] = toSrgb(result[2]);
        }
        return math::cast<std::uint8_t>(result * 255.f);
    }

}

Image loadImage(std::filesystem::path const & path) {
    int width, height, channels;
    std::unique_ptr<stbi_uc, StbiDeleter> pixels(stbi_load(path.c_str(), &width, &height, &channels, 4));
    if (!pixels)
        return {};

    Image result;
    result.width = width;
    result.height = height;
    result.pixels.reset(new Image::Pixel[width * height]);
    std::copy(pixels.get(), pixels.get() + width * height * 4, reinterpret_cast<stbi_uc *>(result.pixels.get()));

    return result;
}

Image downsample(Image const & image, bool srgb) {
    Image result;
    result.width = image.width / 2;
    result.height = image.height / 2;
    result.pixels.reset(new Image::Pixel[result.width * result.height]);

    for (std::uint32_t y = 0; y < result.height; ++y) {
        for (std::uint32_t x = 0; x < result.width; ++x) {
            math::vector4f sum{};
            float count = 1.f;
            sum += toFloat(image.at(2 * x, 2 * y), srgb);

            bool const hasX = 2 * x + 1 < image.width;
            bool const hasY = 2 * y + 1 < image.height;

            if (hasX) {
                count += 1.f;
                sum += toFloat(image.at(2 * x + 1, 2 * y), srgb);
            }

            if (hasY) {
                count += 1.f;
                sum += toFloat(image.at(2 * x, 2 * y + 1), srgb);
            }

            if (hasX && hasY) {
                count += 1.f;
                sum += toFloat(image.at(2 * x + 1, 2 * y + 1), srgb);
            }

            result.at(x, y) = toPixel(sum / count, srgb);
        }
    }

    return result;
}
