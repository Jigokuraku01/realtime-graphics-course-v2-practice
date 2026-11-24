#include <wgpu_app.hpp>
#include <file_utils.hpp>
#include <image_utils.hpp>
#include <math/math.hpp>

#include <webgpu.h>

#include <chrono>
#include <exception>
#include <filesystem>
#include <iostream>
#include <numbers>
#include <random>
#include <unordered_set>

static std::filesystem::path const projectRoot = PROJECT_ROOT;

struct CameraUniforms
{
    math::matrix4f viewProjectionTranspose;
    math::vector3f position;
    float padding = 0.f;
};

struct Particle
{
    math::vector3f position;
};

WGPUShaderModule createShaderModule(WGPUDevice device, std::filesystem::path const & path) {
    auto const source = loadFile(path);

    WGPUShaderSourceWGSL shaderSourceWGSL = WGPU_SHADER_SOURCE_WGSL_INIT;
    shaderSourceWGSL.code = {source.data(), source.size()};

    WGPUShaderModuleDescriptor shaderModuleDescriptor = WGPU_SHADER_MODULE_DESCRIPTOR_INIT;
    shaderModuleDescriptor.nextInChain = &shaderSourceWGSL.chain;

    return wgpuDeviceCreateShaderModule(device, &shaderModuleDescriptor);
}

WGPUBuffer createBuffer(WGPUDevice device, std::size_t size, WGPUBufferUsage usage) {
    WGPUBufferDescriptor bufferDescriptor = WGPU_BUFFER_DESCRIPTOR_INIT;
    bufferDescriptor.size = size;
    bufferDescriptor.usage = usage | WGPUBufferUsage_CopyDst;
    return wgpuDeviceCreateBuffer(device, &bufferDescriptor);
}

template <typename T>
WGPUBuffer loadBuffer(WGPUDevice device, WGPUQueue queue, std::vector<T> const & data, WGPUBufferUsage usage) {
    auto buffer = createBuffer(device, data.size() * sizeof(T), usage);
    wgpuQueueWriteBuffer(queue, buffer, 0, data.data(), data.size() * sizeof(T));
    return buffer;
}

WGPURenderPipeline createPipeline(WGPUDevice device, WGPUShaderModule shaderModule, WGPUTextureFormat surfaceFormat, WGPUBindGroupLayout bindGroupLayout) {
    WGPUPipelineLayoutDescriptor pipelineLayoutDescriptor = WGPU_PIPELINE_LAYOUT_DESCRIPTOR_INIT;
    pipelineLayoutDescriptor.bindGroupLayoutCount = 1;
    pipelineLayoutDescriptor.bindGroupLayouts = &bindGroupLayout;

    WGPUPipelineLayout pipelineLayout = wgpuDeviceCreatePipelineLayout(device, &pipelineLayoutDescriptor);

    WGPUColorTargetState colorTargetState = WGPU_COLOR_TARGET_STATE_INIT;
    colorTargetState.format = surfaceFormat;
    colorTargetState.writeMask = WGPUColorWriteMask_All;

    WGPUFragmentState fragmentState = WGPU_FRAGMENT_STATE_INIT;
    fragmentState.module = shaderModule;
    fragmentState.entryPoint = {"fragmentMain", WGPU_STRLEN};
    fragmentState.targetCount = 1;
    fragmentState.targets = &colorTargetState;

    WGPUVertexAttribute attribute = WGPU_VERTEX_ATTRIBUTE_INIT;
    attribute.offset = offsetof(Particle, position);
    attribute.shaderLocation = 0;
    attribute.format = WGPUVertexFormat_Float32x3;

    WGPUVertexBufferLayout vertexBufferLayout = WGPU_VERTEX_BUFFER_LAYOUT_INIT;
    vertexBufferLayout.arrayStride = sizeof(Particle);
    vertexBufferLayout.attributeCount = 1;
    vertexBufferLayout.attributes = &attribute;
    vertexBufferLayout.stepMode = WGPUVertexStepMode_Instance;

    WGPUDepthStencilState depthStencilState = WGPU_DEPTH_STENCIL_STATE_INIT;
    depthStencilState.format = WGPUTextureFormat_Depth24Plus;
    depthStencilState.depthWriteEnabled = WGPUOptionalBool_True;
    depthStencilState.depthCompare = WGPUCompareFunction_Less;

    WGPURenderPipelineDescriptor renderPipelineDescriptor = WGPU_RENDER_PIPELINE_DESCRIPTOR_INIT;
    renderPipelineDescriptor.layout = pipelineLayout;
    renderPipelineDescriptor.vertex.module = shaderModule;
    renderPipelineDescriptor.vertex.entryPoint = {"vertexMain", WGPU_STRLEN};
    renderPipelineDescriptor.vertex.buffers = &vertexBufferLayout;
    renderPipelineDescriptor.vertex.bufferCount = 1;
    renderPipelineDescriptor.primitive.topology = WGPUPrimitiveTopology_TriangleList;
    renderPipelineDescriptor.primitive.cullMode = WGPUCullMode_None;
    renderPipelineDescriptor.fragment = &fragmentState;
    renderPipelineDescriptor.depthStencil = &depthStencilState;

    WGPURenderPipeline renderPipeline = wgpuDeviceCreateRenderPipeline(device, &renderPipelineDescriptor);
    wgpuPipelineLayoutRelease(pipelineLayout);

    return renderPipeline;
}

WGPUTexture loadTexture(WGPUDevice device, WGPUQueue queue, std::filesystem::path const & path) {
    std::vector<Image> mipmaps;
    mipmaps.push_back(loadImage(path));

    while (mipmaps.back().width > 1 || mipmaps.back().height > 1) {
        mipmaps.push_back(downsample(mipmaps.back()));
    }

    WGPUTextureDescriptor textureDescriptor = WGPU_TEXTURE_DESCRIPTOR_INIT;
    textureDescriptor.usage = WGPUTextureUsage_CopyDst | WGPUTextureUsage_TextureBinding;
    textureDescriptor.dimension = WGPUTextureDimension_2D;
    textureDescriptor.size = {mipmaps.front().width, mipmaps.front().height, 1};
    textureDescriptor.format = WGPUTextureFormat_RGBA8Unorm;
    textureDescriptor.mipLevelCount = mipmaps.size();
    WGPUTexture texture = wgpuDeviceCreateTexture(device, &textureDescriptor);

    for (std::uint32_t level = 0; level < mipmaps.size(); ++level) {
        Image const & image = mipmaps[level];

        WGPUTexelCopyTextureInfo texelCopy {
            .texture = texture,
            .mipLevel = level,
            .origin = {0, 0, 0},
            .aspect = WGPUTextureAspect_All,
        };
        WGPUTexelCopyBufferLayout bufferLayout {
            .offset = 0,
            .bytesPerRow = image.width * 4,
            .rowsPerImage = image.height,
        };
        WGPUExtent3D writeSize{image.width, image.height, 1};
        wgpuQueueWriteTexture(queue, &texelCopy, image.pixels.get(), image.width * image.height * 4, &bufferLayout, &writeSize);
    }

    return texture;
}

WGPUBindGroupLayout createBindGroupLayout(WGPUDevice device) {
    WGPUBindGroupLayoutEntry entries[3] = {
        WGPU_BIND_GROUP_LAYOUT_ENTRY_INIT,
        WGPU_BIND_GROUP_LAYOUT_ENTRY_INIT,
        WGPU_BIND_GROUP_LAYOUT_ENTRY_INIT,
    };
    entries[0].binding = 0;
    entries[0].visibility = WGPUShaderStage_Vertex;
    entries[0].buffer.type = WGPUBufferBindingType_Uniform;
    entries[1].binding = 1;
    entries[1].visibility = WGPUShaderStage_Fragment;
    entries[1].texture.viewDimension = WGPUTextureViewDimension_2D;
    entries[1].texture.sampleType = WGPUTextureSampleType_Float;
    entries[2].binding = 2;
    entries[2].visibility = WGPUShaderStage_Fragment;
    entries[2].sampler.type = WGPUSamplerBindingType_Filtering;

    WGPUBindGroupLayoutDescriptor descriptor = WGPU_BIND_GROUP_LAYOUT_DESCRIPTOR_INIT;
    descriptor.entries = entries;
    descriptor.entryCount = std::size(entries);

    return wgpuDeviceCreateBindGroupLayout(device, &descriptor);
}

int main() try {
    WgpuApp app("Practice11", 1280, 720, false);

    WGPUBindGroupLayout bindGroupLayout = createBindGroupLayout(app.device());
    WGPUShaderModule shaderModule = createShaderModule(app.device(), projectRoot / "shader.wgsl");
    WGPURenderPipeline renderPipeline = createPipeline(app.device(), shaderModule, app.surfaceFormat(), bindGroupLayout);

    WGPUBuffer cameraUniformBuffer = createBuffer(app.device(), sizeof(CameraUniforms), WGPUBufferUsage_Uniform);

    WGPUTexture particleTexture = loadTexture(app.device(), app.queue(), projectRoot / "particle.png");
    WGPUTextureView particleTextureView = wgpuTextureCreateView(particleTexture, nullptr);

    WGPUSamplerDescriptor samplerDescriptor = WGPU_SAMPLER_DESCRIPTOR_INIT;
    samplerDescriptor.addressModeU = WGPUAddressMode_ClampToEdge;
    samplerDescriptor.addressModeV = WGPUAddressMode_ClampToEdge;
    samplerDescriptor.addressModeW = WGPUAddressMode_ClampToEdge;
    samplerDescriptor.magFilter = WGPUFilterMode_Linear;
    samplerDescriptor.minFilter = WGPUFilterMode_Linear;
    samplerDescriptor.mipmapFilter = WGPUMipmapFilterMode_Linear;
    WGPUSampler particleSampler = wgpuDeviceCreateSampler(app.device(), &samplerDescriptor);

    WGPUBindGroupEntry entries[3] = {
        WGPU_BIND_GROUP_ENTRY_INIT,
        WGPU_BIND_GROUP_ENTRY_INIT,
        WGPU_BIND_GROUP_ENTRY_INIT,
    };
    entries[0].binding = 0;
    entries[0].buffer = cameraUniformBuffer;
    entries[0].size = WGPU_WHOLE_SIZE;
    entries[1].binding = 1;
    entries[1].textureView = particleTextureView;
    entries[2].binding = 2;
    entries[2].sampler = particleSampler;

    WGPUBindGroupDescriptor bindGroupDescriptor = WGPU_BIND_GROUP_DESCRIPTOR_INIT;
    bindGroupDescriptor.layout = bindGroupLayout;
    bindGroupDescriptor.entries = entries;
    bindGroupDescriptor.entryCount = std::size(entries);
    WGPUBindGroup bindGroup = wgpuDeviceCreateBindGroup(app.device(), &bindGroupDescriptor);

    std::default_random_engine rng;
    std::uniform_real_distribution<float> distribution(-1.f, 1.f);

    std::vector<Particle> particles(256);
    for (Particle & particle : particles) {
        particle.position = {distribution(rng) * 2.f, distribution(rng) * 0.5f, distribution(rng) * 2.f};
    }
    WGPUBuffer particleBuffer = loadBuffer(app.device(), app.queue(), particles, WGPUBufferUsage_Vertex);

    WGPUTexture depthBuffer = nullptr;
    WGPUTextureView depthBufferView = nullptr;

    auto lastFrameStart = std::chrono::high_resolution_clock::now();
    float camera_distance = 4.f;
    float camera_rotation = 0.f;
    float camera_azimuth = std::numbers::pi_v<float> / 12.f;
    std::unordered_set<SDL_Keycode> keydown;

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
                keydown.insert(event.key.key);
                break;
            case SDL_EVENT_KEY_UP:
                keydown.erase(event.key.key);
                break;
            }
        }

        std::optional<WGPUSurfaceTexture> surfaceTexture = app.beginFrame();
        if (!surfaceTexture) {
            continue;
        }

        if (!depthBuffer || wgpuTextureGetWidth(depthBuffer) != app.width() || wgpuTextureGetHeight(depthBuffer) != app.height()) {
            if (depthBufferView) wgpuTextureViewRelease(depthBufferView);
            if (depthBuffer) wgpuTextureRelease(depthBuffer);

            WGPUTextureDescriptor depthBufferDescriptor = WGPU_TEXTURE_DESCRIPTOR_INIT;
            depthBufferDescriptor.usage = WGPUTextureUsage_RenderAttachment;
            depthBufferDescriptor.dimension = WGPUTextureDimension_2D;
            depthBufferDescriptor.size = {(std::uint32_t)app.width(), (std::uint32_t)app.height(), 1};
            depthBufferDescriptor.format = WGPUTextureFormat_Depth24Plus;
            depthBuffer = wgpuDeviceCreateTexture(app.device(), &depthBufferDescriptor);

            WGPUTextureViewDescriptor depthBufferViewDescriptor = WGPU_TEXTURE_VIEW_DESCRIPTOR_INIT;
            depthBufferViewDescriptor.format = WGPUTextureFormat_Depth24Plus;
            depthBufferViewDescriptor.dimension = WGPUTextureViewDimension_2D;
            depthBufferViewDescriptor.mipLevelCount = 1;
            depthBufferViewDescriptor.arrayLayerCount = 1;
            depthBufferViewDescriptor.aspect = WGPUTextureAspect_DepthOnly;
            depthBufferView = wgpuTextureCreateView(depthBuffer, &depthBufferViewDescriptor);
        }

        auto const now = std::chrono::high_resolution_clock::now();
        float const dt = std::chrono::duration<float>(now - lastFrameStart).count();
        lastFrameStart = now;

        if (keydown.contains(SDLK_LEFT)) camera_rotation += 3.f * dt;
        if (keydown.contains(SDLK_RIGHT)) camera_rotation -= 3.f * dt;
        if (keydown.contains(SDLK_UP)) camera_distance += 3.f * dt;
        if (keydown.contains(SDLK_DOWN)) camera_distance -= 3.f * dt;

        math::matrix4f const view = math::translation(math::vector{0.f, 0.f, - camera_distance})
            * math::rotation_yz(camera_azimuth)
            * math::rotation_xz(camera_rotation);
        math::matrix4f const projection = math::perspective(std::numbers::pi_v<float> / 3.f, app.width() * 1.f / app.height(), 0.01f, 100.f);
        math::matrix4f const inverseView = math::inverse(view).value();
        CameraUniforms const cameraUniforms {
            .viewProjectionTranspose = math::transpose(projection * view),
            .position = math::head<3>(inverseView * math::vector{0.f, 0.f, 0.f, 1.f}),
        };
        wgpuQueueWriteBuffer(app.queue(), cameraUniformBuffer, 0, &cameraUniforms, sizeof(cameraUniforms));

        WGPUTextureView targetView = wgpuTextureCreateView(surfaceTexture->texture, nullptr);
        WGPUCommandEncoder encoder = wgpuDeviceCreateCommandEncoder(app.device(), nullptr);

        WGPURenderPassColorAttachment colorAttachment = WGPU_RENDER_PASS_COLOR_ATTACHMENT_INIT;
        colorAttachment.view = targetView;
        colorAttachment.loadOp = WGPULoadOp_Clear;
        colorAttachment.storeOp = WGPUStoreOp_Store;
        colorAttachment.clearValue = {0.0, 0.0, 0.0, 1.0};

        WGPURenderPassDepthStencilAttachment depthStencilAttachment = WGPU_RENDER_PASS_DEPTH_STENCIL_ATTACHMENT_INIT;
        depthStencilAttachment.view = depthBufferView;
        depthStencilAttachment.depthLoadOp = WGPULoadOp_Clear;
        depthStencilAttachment.depthStoreOp = WGPUStoreOp_Discard;
        depthStencilAttachment.depthClearValue = 1.f;
        depthStencilAttachment.depthReadOnly = WGPU_FALSE;

        WGPURenderPassDescriptor renderPassDescriptor = WGPU_RENDER_PASS_DESCRIPTOR_INIT;
        renderPassDescriptor.colorAttachmentCount = 1;
        renderPassDescriptor.colorAttachments = &colorAttachment;
        renderPassDescriptor.depthStencilAttachment = &depthStencilAttachment;
        WGPURenderPassEncoder renderPass = wgpuCommandEncoderBeginRenderPass(encoder, &renderPassDescriptor);

        wgpuRenderPassEncoderSetPipeline(renderPass, renderPipeline);
        wgpuRenderPassEncoderSetBindGroup(renderPass, 0, bindGroup, 0, nullptr);
        wgpuRenderPassEncoderSetVertexBuffer(renderPass, 0, particleBuffer, 0, WGPU_WHOLE_SIZE);
        wgpuRenderPassEncoderDraw(renderPass, 6, particles.size(), 0, 0);

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

    if (depthBufferView) wgpuTextureViewRelease(depthBufferView);
    if (depthBuffer) wgpuTextureRelease(depthBuffer);
    wgpuBufferRelease(particleBuffer);
    wgpuBufferRelease(cameraUniformBuffer);
    wgpuBindGroupRelease(bindGroup);
    wgpuSamplerRelease(particleSampler);
    wgpuTextureViewRelease(particleTextureView);
    wgpuTextureRelease(particleTexture);
    wgpuRenderPipelineRelease(renderPipeline);
    wgpuShaderModuleRelease(shaderModule);
    wgpuBindGroupLayoutRelease(bindGroupLayout);
} catch (const std::exception &e) {
    std::cerr << "error: " << e.what() << std::endl;
    return EXIT_FAILURE;
}
