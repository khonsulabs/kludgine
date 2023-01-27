window.SIDEBAR_ITEMS = {"constant":[["COPY_BUFFER_ALIGNMENT","Buffer to buffer copy as well as buffer clear offsets and sizes must be aligned to this number."],["COPY_BYTES_PER_ROW_ALIGNMENT","Buffer-Texture copies must have `bytes_per_row` aligned to this number."],["MAP_ALIGNMENT","Size to align mappings."],["PUSH_CONSTANT_ALIGNMENT","Alignment all push constants need"],["QUERY_RESOLVE_BUFFER_ALIGNMENT","An offset into the query resolve buffer has to be aligned to this."],["QUERY_SET_MAX_QUERIES","Maximum queries in a query set"],["QUERY_SIZE","Size of a single piece of query data."],["VERTEX_STRIDE_ALIGNMENT","Vertex buffer strides have to be aligned to this number."]],"enum":[["AddressMode","How edges should be handled in texture addressing."],["AstcBlock","ASTC block dimensions"],["AstcChannel","ASTC RGBA channel"],["Backend","Backends supported by wgpu."],["BindingResource","Resource that can be bound to a pipeline."],["BindingType","Specific type of a binding."],["BlendFactor","Alpha blend factor."],["BlendOperation","Alpha blend operation."],["BufferBindingType","Specific type of a buffer binding."],["CompareFunction","Comparison function used for depth and stencil operations."],["CompositeAlphaMode","Specifies how the alpha channel of the textures should be handled during compositing."],["DeviceType","Supported physical device types."],["Dx12Compiler","Selects which DX12 shader compiler to use."],["Error","Error type"],["ErrorFilter","Filter for error scopes."],["Face","Face of a vertex."],["FilterMode","Texel mixing mode when sampling between texels."],["FrontFace","Vertex winding order which classifies the “front” face of a triangle."],["IndexFormat","Format of indices used with pipeline."],["LoadOp","Operation to perform to the output attachment at the start of a render pass."],["MaintainBase","Passed to `Device::poll` to control how and if it should block."],["MapMode","Type of buffer mapping."],["PolygonMode","Type of drawing mode for polygons"],["PowerPreference","Power Preference when choosing a physical adapter."],["PredefinedColorSpace","Color spaces supported on the web."],["PresentMode","Behavior of the presentation engine based on frame rate."],["PrimitiveTopology","Primitive type the input mesh is composed of."],["QueryType","Type of query contained in a QuerySet."],["SamplerBindingType","Specific type of a sampler binding."],["SamplerBorderColor","Color variation to use when sampler addressing mode is [`AddressMode::ClampToBorder`]"],["ShaderModel","Collections of shader features a device supports if they support less than WebGPU normally allows."],["ShaderSource","Source of a shader module."],["StencilOperation","Operation to perform on the stencil value."],["StorageTextureAccess","Specific type of a sample in a texture binding."],["SurfaceError","Result of an unsuccessful call to [`Surface::get_current_texture`]."],["SurfaceStatus","Status of the recieved surface image."],["TextureAspect","Kind of data the texture holds."],["TextureDimension","Dimensionality of a texture."],["TextureFormat","Underlying texture data format."],["TextureSampleType","Specific type of a sample in a texture binding."],["TextureViewDimension","Dimensions of a particular texture view."],["VertexFormat","Vertex Format for a [`VertexAttribute`] (input)."],["VertexStepMode","Whether a vertex buffer is indexed by vertex or by instance."]],"macro":[["include_spirv","Macro to load a SPIR-V module statically."],["include_spirv_raw","Macro to load raw SPIR-V data statically, for use with `Features::SPIRV_SHADER_PASSTHROUGH`."],["include_wgsl","Macro to load a WGSL module statically."],["vertex_attr_array","Macro to produce an array of `VertexAttribute`."]],"mod":[["util","Utility structures and functions that are built on top of the main `wgpu` API."]],"struct":[["Adapter","Handle to a physical graphics and/or compute device."],["AdapterInfo","Information about an adapter."],["Backends","Represents the backends that wgpu will use."],["BindGroup","Handle to a binding group."],["BindGroupDescriptor","Describes a group of bindings and the resources to be bound."],["BindGroupEntry","An element of a [`BindGroupDescriptor`], consisting of a bindable resource and the slot to bind it to."],["BindGroupLayout","Handle to a binding group layout."],["BindGroupLayoutDescriptor","Describes a [`BindGroupLayout`]."],["BindGroupLayoutEntry","Describes a single binding inside a bind group."],["BlendComponent","Describes a blend component of a [`BlendState`]."],["BlendState","Describe the blend state of a render pipeline, within [`ColorTargetState`]."],["Buffer","Handle to a GPU-accessible buffer."],["BufferAsyncError","Error occurred when trying to async map a buffer."],["BufferBinding","Describes the segment of a buffer to bind."],["BufferSlice","Slice into a [`Buffer`]."],["BufferUsages","Different ways that you can use a buffer."],["BufferView","Read only view into a mapped buffer."],["BufferViewMut","Write only view into mapped buffer."],["Color","RGBA double precision color."],["ColorTargetState","Describes the color state of a render pipeline."],["ColorWrites","Color write mask. Disabled color channels will not be written to."],["CommandBuffer","Handle to a command buffer on the GPU."],["CommandBufferDescriptor","Describes a `CommandBuffer`."],["CommandEncoder","Encodes a series of GPU operations."],["ComputePass","In-progress recording of a compute pass."],["ComputePassDescriptor","Describes the attachments of a compute pass."],["ComputePipeline","Handle to a compute pipeline."],["ComputePipelineDescriptor","Describes a compute pipeline."],["CreateSurfaceError","[`Instance::create_surface()`] or a related function failed."],["DepthBiasState","Describes the biasing setting for the depth target."],["DepthStencilState","Describes the depth/stencil state in a render pipeline."],["Device","Open connection to a graphics and/or compute device."],["DownlevelCapabilities","Lists various ways the underlying platform does not conform to the WebGPU standard."],["DownlevelFlags","Binary flags listing features that may or may not be present on downlevel adapters."],["Extent3d","Extent of a texture related operation."],["Features","Features that are not guaranteed to be supported."],["FragmentState","Describes the fragment processing in a render pipeline."],["ImageCopyBufferBase","View of a buffer which can be used to copy to/from a texture."],["ImageCopyTextureBase","View of a texture which can be used to copy to/from a buffer/texture."],["ImageCopyTextureTaggedBase","View of a texture which can be used to copy to a texture, including color space and alpha premultiplication information."],["ImageDataLayout","Layout of a texture in a buffer’s memory."],["ImageSubresourceRange","Subresource range within an image"],["Instance","Context for all other wgpu objects. Instance of wgpu."],["InstanceDescriptor","Options for creating an instance."],["Limits","Represents the sets of limits an adapter/device supports."],["MultisampleState","Describes the multi-sampling state of a render pipeline."],["Operations","Pair of load and store operations for an attachment aspect."],["Origin2d","Origin of a copy from a 2D image."],["Origin3d","Origin of a copy to/from a texture."],["PipelineLayout","Handle to a pipeline layout."],["PipelineLayoutDescriptor","Describes a [`PipelineLayout`]."],["PipelineStatisticsTypes","Flags for which pipeline data should be recorded."],["PresentationTimestamp","Nanosecond timestamp used by the presentation engine."],["PrimitiveState","Describes the state of primitive assembly and rasterization in a render pipeline."],["PushConstantRange","A range of push constant memory to pass to a shader stage."],["QuerySet","Handle to a query set."],["Queue","Handle to a command queue on a device."],["QueueWriteBufferView","A read-only view into a staging buffer."],["RenderBundle","Pre-prepared reusable bundle of GPU operations."],["RenderBundleDepthStencil","Describes the depth/stencil attachment for render bundles."],["RenderBundleEncoder","Encodes a series of GPU operations into a reusable “render bundle”."],["RenderBundleEncoderDescriptor","Describes a [`RenderBundleEncoder`]."],["RenderPass","In-progress recording of a render pass."],["RenderPassColorAttachment","Describes a color attachment to a [`RenderPass`]."],["RenderPassDepthStencilAttachment","Describes a depth/stencil attachment to a [`RenderPass`]."],["RenderPassDescriptor","Describes the attachments of a render pass."],["RenderPipeline","Handle to a rendering (graphics) pipeline."],["RenderPipelineDescriptor","Describes a render (graphics) pipeline."],["RequestAdapterOptionsBase","Options for requesting adapter."],["RequestDeviceError","Requesting a device failed."],["Sampler","Handle to a sampler."],["SamplerDescriptor","Describes a [`Sampler`]."],["ShaderModule","Handle to a compiled shader module."],["ShaderModuleDescriptor","Descriptor for use with [`Device::create_shader_module`]."],["ShaderModuleDescriptorSpirV","Descriptor for a shader module given by SPIR-V binary, for use with [`Device::create_shader_module_spirv`]."],["ShaderStages","Describes the shader stages that a binding will be visible from."],["StencilFaceState","Describes stencil state in a render pipeline."],["StencilState","State of the stencil operation (fixed-pipeline stage)."],["SubmissionIndex","Identifier for a particular call to [`Queue::submit`]. Can be used as part of an argument to [`Device::poll`] to block for a particular submission to finish."],["Surface","Handle to a presentable surface."],["SurfaceCapabilities","Defines the capabilities of a given surface and adapter."],["SurfaceTexture","Surface texture that can be rendered to. Result of a successful call to [`Surface::get_current_texture`]."],["Texture","Handle to a texture on the GPU."],["TextureFormatFeatureFlags","Feature flags for a texture format."],["TextureFormatFeatures","Features supported by a given texture format"],["TextureUsages","Different ways that you can use a texture."],["TextureView","Handle to a texture view."],["TextureViewDescriptor","Describes a [`TextureView`]."],["VertexAttribute","Vertex inputs (attributes) to shaders."],["VertexBufferLayout","Describes how the vertex buffer is interpreted."],["VertexState","Describes the vertex processing in a render pipeline."]],"trait":[["UncapturedErrorHandler","Type for the callback of uncaptured error handler"]],"type":[["BufferAddress","Integral type used for buffer offsets."],["BufferDescriptor","Describes a [`Buffer`]."],["BufferSize","Integral type used for buffer slice sizes."],["CommandEncoderDescriptor","Describes a [`CommandEncoder`]."],["DeviceDescriptor","Describes a [`Device`]."],["DynamicOffset","Integral type used for dynamic bind group offsets."],["ImageCopyBuffer","View of a buffer which can be used to copy to/from a texture."],["ImageCopyTexture","View of a texture which can be used to copy to/from a buffer/texture."],["ImageCopyTextureTagged","View of a texture which can be used to copy to a texture, including color space and alpha premultiplication information."],["Label","Object debugging label."],["Maintain","Passed to [`Device::poll`] to control how and if it should block."],["QuerySetDescriptor","Describes a [`QuerySet`]."],["RenderBundleDescriptor","Describes a [`RenderBundle`]."],["RequestAdapterOptions","Additional information required when requesting an adapter."],["ShaderLocation","Integral type used for binding locations in shaders."],["SurfaceConfiguration","Describes a [`Surface`]."],["TextureDescriptor","Describes a [`Texture`]."]]};