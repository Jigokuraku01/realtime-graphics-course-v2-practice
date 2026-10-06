#pragma once

#include <math/vector.hpp>

#include <memory>
#include <filesystem>

struct Image {
    using Pixel = math::vector<std::uint8_t, 4>;

    std::uint32_t width;
    std::uint32_t height;
    std::unique_ptr<Pixel[]> pixels;

    Pixel & at(std::uint32_t x, std::uint32_t y) {
        return pixels[y * width + x];
    }

    Pixel const & at(std::uint32_t x, std::uint32_t y) const {
        return pixels[y * width + x];
    }
};

Image loadImage(std::filesystem::path const & path);
Image downsample(Image const & image, bool srgb = false);
