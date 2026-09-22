#include <wgpu_app.hpp>
#include <file_utils.hpp>
#include <math/aliases.hpp>
#include <math/detail/alloca.hpp>

#include <webgpu.h>

#include <chrono>
#include <exception>
#include <filesystem>
#include <iostream>
#include <span>

static std::filesystem::path const projectRoot = PROJECT_ROOT;

struct vertex
{
    math::vector2f position;
    math::vector4ub color;
};

vertex lerp(vertex const & v0, vertex const & v1, float t) {
    return {
        .position = math::lerp(v0.position, v1.position, t),
        .color = math::cast<std::uint8_t>(math::lerp(math::cast<float>(v0.color), math::cast<float>(v1.color), t)),
    };
}

vertex in_place_bezier(std::span<vertex> vertices, float t) {
    std::size_t const n = vertices.size();

    for (std::size_t k = n - 1; k > 0; --k) {
        for (std::size_t i = 0; i < k; ++i) {
            vertices[i] = lerp(vertices[i], vertices[i + 1], t);
        }
    }

    return vertices[0];
}

vertex bezier(std::span<vertex const> vertices, float t) {
    std::size_t const n = vertices.size();

    vertex *scratch = math_alloca(vertex, n);
    std::copy(vertices.begin(), vertices.end(), scratch);
    return in_place_bezier(std::span{scratch, n}, t);
}

WGPUShaderModule createShaderModule(WGPUDevice device, std::filesystem::path const &path) {
    auto const source = loadFile(path);

    WGPUShaderSourceWGSL shaderSourceWGSL = WGPU_SHADER_SOURCE_WGSL_INIT;
    shaderSourceWGSL.code = {source.data(), source.size()};

    WGPUShaderModuleDescriptor shaderModuleDescriptor = WGPU_SHADER_MODULE_DESCRIPTOR_INIT;
    shaderModuleDescriptor.nextInChain = &shaderSourceWGSL.chain;

    return wgpuDeviceCreateShaderModule(device, &shaderModuleDescriptor);
}

WGPURenderPipeline createPipeline(WGPUDevice device, WGPUShaderModule shaderModule,
                                  WGPUTextureFormat surfaceFormat) {
    WGPUPipelineLayoutDescriptor pipelineLayoutDescriptor = WGPU_PIPELINE_LAYOUT_DESCRIPTOR_INIT;
    pipelineLayoutDescriptor.immediateSize = 64;

    WGPUPipelineLayout pipelineLayout = wgpuDeviceCreatePipelineLayout(device, &pipelineLayoutDescriptor);

    WGPUColorTargetState colorTargetState = WGPU_COLOR_TARGET_STATE_INIT;
    colorTargetState.format = surfaceFormat;
    colorTargetState.writeMask = WGPUColorWriteMask_All;

    WGPUFragmentState fragmentState = WGPU_FRAGMENT_STATE_INIT;
    fragmentState.module = shaderModule;
    fragmentState.entryPoint = {"fragmentMain", WGPU_STRLEN};
    fragmentState.targetCount = 1;
    fragmentState.targets = &colorTargetState;

    WGPURenderPipelineDescriptor renderPipelineDescriptor = WGPU_RENDER_PIPELINE_DESCRIPTOR_INIT;
    renderPipelineDescriptor.layout = pipelineLayout;
    renderPipelineDescriptor.vertex.module = shaderModule;
    renderPipelineDescriptor.vertex.entryPoint = {"vertexMain", WGPU_STRLEN};
    renderPipelineDescriptor.primitive.topology = WGPUPrimitiveTopology_TriangleList;
    renderPipelineDescriptor.fragment = &fragmentState;

    WGPURenderPipeline renderPipeline = wgpuDeviceCreateRenderPipeline(device, &renderPipelineDescriptor);
    wgpuPipelineLayoutRelease(pipelineLayout);

    return renderPipeline;
}

int main() try {
    WgpuApp app("Practice03", 1280, 720, false);

    WGPUShaderModule shaderModule = createShaderModule(app.device(), projectRoot / "shader.wgsl");
    WGPURenderPipeline renderPipeline = createPipeline(app.device(), shaderModule, app.surfaceFormat());

    auto lastFrameStart = std::chrono::high_resolution_clock::now();
    float time = 0.f;

    std::vector<vertex> vertices = {
        {{0.0f, 0.0f}, {125, 207, 182, 255}},
        {{0.5f, 0.0f}, {251, 209, 162, 255}},
        {{0.0f, 0.5f}, {247, 146,  86, 255}},
    };

    math::vector2f mouse{0.f, 0.f};

    bool running = true;
    while (running) {
        SDL_Event event;
        while (SDL_PollEvent(&event)) {
            switch (event.type) {
            case SDL_EVENT_QUIT:
                running = false;
                break;
            case SDL_EVENT_WINDOW_PIXEL_SIZE_CHANGED:
                app.resize(event.window.data1, event.window.data2);
                break;
            case SDL_EVENT_KEY_DOWN:
                if (event.key.key == SDLK_LEFT) {
                    // Нажата клавиша влево
                }
                if (event.key.key == SDLK_RIGHT) {
                    // Нажата клавиша вправо
                }
                break;
            case SDL_EVENT_MOUSE_MOTION:
                mouse = {event.motion.x, event.motion.y};
                break;
            case SDL_EVENT_MOUSE_BUTTON_DOWN:
                if (event.button.button == SDL_BUTTON_LEFT) {
                    // Нажата левая кнопка
                }
                if (event.button.button == SDL_BUTTON_RIGHT) {
                    // Нажата правая кнопка
                }
                break;
            }
        }

        std::optional<WGPUSurfaceTexture> surfaceTexture = app.beginFrame();
        if (!surfaceTexture) {
            continue;
        }

        auto const now = std::chrono::high_resolution_clock::now();
        float const dt = std::chrono::duration<float>(now - lastFrameStart).count();
        time += dt;
        lastFrameStart = now;

        float const viewMatrix[16] = {
            1.f, 0.f, 0.f, 0.f,
            0.f, 1.f, 0.f, 0.f,
            0.f, 0.f, 1.f, 0.f,
            0.f, 0.f, 0.f, 1.f,
        };

        WGPUTextureView targetView = wgpuTextureCreateView(surfaceTexture->texture, nullptr);

        WGPUCommandEncoder encoder = wgpuDeviceCreateCommandEncoder(app.device(), nullptr);

        WGPURenderPassColorAttachment colorAttachment = WGPU_RENDER_PASS_COLOR_ATTACHMENT_INIT;
        colorAttachment.view = targetView;
        colorAttachment.loadOp = WGPULoadOp_Clear;
        colorAttachment.storeOp = WGPUStoreOp_Store;
        colorAttachment.clearValue = {0.07, 0.21, 0.30, 1.0};

        WGPURenderPassDescriptor renderPassDescriptor = WGPU_RENDER_PASS_DESCRIPTOR_INIT;
        renderPassDescriptor.colorAttachmentCount = 1;
        renderPassDescriptor.colorAttachments = &colorAttachment;
        WGPURenderPassEncoder renderPass = wgpuCommandEncoderBeginRenderPass(encoder, &renderPassDescriptor);

        wgpuRenderPassEncoderSetPipeline(renderPass, renderPipeline);
        wgpuRenderPassEncoderSetImmediates(renderPass, 0, viewMatrix, sizeof(viewMatrix));
        wgpuRenderPassEncoderDraw(renderPass, vertices.size(), 1, 0, 0);
        wgpuRenderPassEncoderEnd(renderPass);
        wgpuRenderPassEncoderRelease(renderPass);

        WGPUCommandBuffer commandBuffer = wgpuCommandEncoderFinish(encoder, nullptr);
        wgpuCommandEncoderRelease(encoder);

        wgpuQueueSubmit(app.queue(), 1, &commandBuffer);
        wgpuCommandBufferRelease(commandBuffer);

        wgpuSurfacePresent(app.surface());

        wgpuTextureViewRelease(targetView);
        wgpuTextureRelease(surfaceTexture->texture);
    }

    wgpuRenderPipelineRelease(renderPipeline);
    wgpuShaderModuleRelease(shaderModule);
} catch (const std::exception &e) {
    std::cerr << "error: " << e.what() << std::endl;
    return EXIT_FAILURE;
}
