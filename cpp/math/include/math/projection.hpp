#pragma once

#include <math/matrix.hpp>

#include <cmath>

namespace math {

template <typename T>
constexpr matrix<T, 4, 4> orthographic(T left, T right, T bottom, T top, T near, T far) {
    auto m = matrix<T, 4, 4>::identity();
    m[0][0] = T{2} / (right - left);
    m[0][3] = - (right + left) / T{2} * m[0][0];
    m[1][1] = T{2} / (top - bottom);
    m[1][3] = - (top + bottom) / T{2} * m[1][1];
    m[2][2] = T{1} / (far - near);
    m[2][3] = - near * m[2][2];
    return m;
}

template <typename T>
constexpr matrix<T, 4, 4> perspective(T fov_y, T aspect_ratio, T near, T far) {
    auto m = matrix<T, 4, 4>::zero();
    m[1][1] = T{1} / std::tan(fov_y / T{2});
    m[0][0] = m[1][1] / aspect_ratio;
    m[2][2] = far / (near - far);
    m[2][3] = m[2][2] * near;
    m[3][2] = - T{1};
    return m;
}

}  // namespace math
