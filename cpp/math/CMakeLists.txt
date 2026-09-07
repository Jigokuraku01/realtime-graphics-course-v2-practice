add_library(math INTERFACE)
target_include_directories(math INTERFACE ${CMAKE_CURRENT_SOURCE_DIR}/include)
target_sources(math PRIVATE
    include/math/detail/alloca.hpp
    include/math/affine.hpp
    include/math/math.hpp
    include/math/aliases.hpp
    include/math/box.hpp
    include/math/quaternion.hpp
    include/math/utils.hpp
    include/math/matrix.hpp
    include/math/vector.hpp
    include/math/gauss.hpp
    include/math/bezier.hpp
)
