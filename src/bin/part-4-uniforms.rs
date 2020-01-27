#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct PushConstants {
    transform: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct UniformBlock {
    ambient_light: [f32; 4],
    light_direction: [f32; 4],
    light_color: [f32; 4],
}

#[derive(serde::Deserialize)]
#[repr(C)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
}

// TODO: Reorder declarations so that they're as close to their usage sites as they can be
// TODO: Try to create the window with a LogicalSize directly - without screwing
//  up swapchain dimensions.
// TODO: Look at the error types for every `expect` to set a good message.
fn main() {
    use gfx_hal::{device::Device, window::Surface, Instance as _};

    const APP_NAME: &'static str = "Part 4: Uniforms";
    const WINDOW_SIZE: [u32; 2] = [512, 512];

    let event_loop = winit::event_loop::EventLoop::new();

    let (logical_window_size, physical_window_size) = {
        use winit::dpi::{LogicalSize, PhysicalSize};

        let dpi = event_loop.primary_monitor().scale_factor();
        let logical: LogicalSize<u32> = WINDOW_SIZE.into();
        let physical: PhysicalSize<u32> = logical.to_physical(dpi);

        (logical, physical)
    };

    use gfx_hal::window::Extent2D;
    let mut surface_extent = Extent2D {
        width: physical_window_size.width,
        height: physical_window_size.height,
    };

    let window = winit::window::WindowBuilder::new()
        .with_title(APP_NAME)
        .with_inner_size(logical_window_size)
        .build(&event_loop)
        .expect("TODO");

    let (instance, surface, adapter) = {
        let instance = backend::Instance::create(APP_NAME, 1).expect("TODO");
        let surface = unsafe { instance.create_surface(&window).expect("TODO") };
        let adapter = instance.enumerate_adapters().remove(0);

        (instance, surface, adapter)
    };

    let (device, mut queue_group) = {
        use gfx_hal::queue::QueueFamily;

        let queue_family = adapter
            .queue_families
            .iter()
            .find(|family| {
                surface.supports_queue_family(family) && family.queue_type().supports_graphics()
            })
            .expect("TODO");

        let mut gpu = unsafe {
            use gfx_hal::adapter::PhysicalDevice;

            adapter
                .physical_device
                .open(&[(queue_family, &[1.0])], gfx_hal::Features::empty())
                .expect("TODO")
        };

        (gpu.device, gpu.queue_groups.pop().expect("TODO"))
    };

    let surface_color_format = {
        use gfx_hal::format::{ChannelType, Format};

        let supported_formats = surface.supported_formats(&adapter.physical_device);
        supported_formats.map_or(Format::Rgba8Srgb, |formats| {
            formats
                .iter()
                .find(|format| format.base_format().1 == ChannelType::Srgb)
                .map(|format| *format)
                .unwrap_or(formats[0])
        })
    };

    let render_pass = {
        use gfx_hal::image::Layout;
        use gfx_hal::pass::{
            Attachment, AttachmentLoadOp, AttachmentOps, AttachmentStoreOp, SubpassDesc,
        };

        let color_attachment = Attachment {
            format: Some(surface_color_format),
            samples: 1,
            ops: AttachmentOps::new(AttachmentLoadOp::Clear, AttachmentStoreOp::Store),
            stencil_ops: AttachmentOps::DONT_CARE,
            layouts: Layout::Undefined..Layout::Present,
        };

        let subpass = SubpassDesc {
            colors: &[(0, Layout::ColorAttachmentOptimal)],
            depth_stencil: None,
            inputs: &[],
            resolves: &[],
            preserves: &[],
        };

        unsafe {
            device
                .create_render_pass(&[color_attachment], &[subpass], &[])
                .expect("TODO")
        }
    };

    let (pipeline_layout, desc_set_layout) = unsafe {
        use gfx_hal::pso::{DescriptorSetLayoutBinding, DescriptorType, ShaderStageFlags};

        let desc_set_layout = device
            .create_descriptor_set_layout(
                &[DescriptorSetLayoutBinding {
                    binding: 0,
                    ty: DescriptorType::UniformBuffer,
                    count: 1,
                    stage_flags: ShaderStageFlags::FRAGMENT,
                    immutable_samplers: false,
                }],
                &[],
            )
            .expect("TODO");

        let push_constant_bytes = std::mem::size_of::<PushConstants>() as u32;

        let pipeline_layout = device
            .create_pipeline_layout(
                vec![&desc_set_layout],
                &[(ShaderStageFlags::VERTEX, 0..push_constant_bytes)],
            )
            .expect("TODO");

        (pipeline_layout, desc_set_layout)
    };

    let pipeline = {
        use gfx_hal::pass::Subpass;
        use gfx_hal::pso::{
            self, BlendState, ColorBlendDesc, ColorMask, EntryPoint, Face, GraphicsPipelineDesc,
            GraphicsShaderSet, Primitive, Rasterizer, Specialization,
        };
        use glsl_to_spirv::ShaderType;

        let compile_shader = |glsl, shader_type| {
            use std::io::{Cursor, Read};

            let mut spirv_bytes = vec![];
            let mut compiled_file = glsl_to_spirv::compile(glsl, shader_type).expect("TODO");
            compiled_file.read_to_end(&mut spirv_bytes).expect("TODO");
            let spirv = pso::read_spirv(Cursor::new(&spirv_bytes)).expect("TODO");
            unsafe { device.create_shader_module(&spirv).expect("TODO") }
        };

        let vertex_shader_module =
            compile_shader(include_str!("shaders/part-4.vert"), ShaderType::Vertex);

        let fragment_shader_module =
            compile_shader(include_str!("shaders/part-4.frag"), ShaderType::Fragment);

        let (vs_entry, fs_entry) = (
            EntryPoint {
                entry: "main",
                module: &vertex_shader_module,
                specialization: Specialization::default(),
            },
            EntryPoint {
                entry: "main",
                module: &fragment_shader_module,
                specialization: Specialization::default(),
            },
        );

        let shader_entries = GraphicsShaderSet {
            vertex: vs_entry,
            hull: None,
            domain: None,
            geometry: None,
            fragment: Some(fs_entry),
        };

        let mut pipeline_desc = GraphicsPipelineDesc::new(
            shader_entries,
            Primitive::TriangleList,
            Rasterizer {
                cull_face: Face::BACK,
                ..Rasterizer::FILL
            },
            &pipeline_layout,
            Subpass {
                index: 0,
                main_pass: &render_pass,
            },
        );

        pipeline_desc.blender.targets.push(ColorBlendDesc {
            mask: ColorMask::ALL,
            blend: Some(BlendState::ALPHA),
        });

        // Vertex buffer description
        {
            use gfx_hal::format::Format;
            use gfx_hal::pso::{AttributeDesc, Element, VertexBufferDesc, VertexInputRate};

            pipeline_desc.vertex_buffers.push(VertexBufferDesc {
                binding: 0,
                stride: std::mem::size_of::<Vertex>() as u32,
                rate: VertexInputRate::Vertex,
            });

            pipeline_desc.attributes.push(AttributeDesc {
                location: 0,
                binding: 0,
                element: Element {
                    format: Format::Rgb32Sfloat,
                    offset: 0,
                },
            });

            pipeline_desc.attributes.push(AttributeDesc {
                location: 1,
                binding: 0,
                element: Element {
                    format: Format::Rgb32Sfloat,
                    offset: 12,
                },
            });
        }

        unsafe {
            let pipeline = device
                .create_graphics_pipeline(&pipeline_desc, None)
                .expect("TODO");

            device.destroy_shader_module(vertex_shader_module);
            device.destroy_shader_module(fragment_shader_module);

            pipeline
        }
    };

    let (command_pool, mut command_buffer) = unsafe {
        use gfx_hal::command::Level;
        use gfx_hal::pool::{CommandPool, CommandPoolCreateFlags};

        let mut command_pool = device
            .create_command_pool(queue_group.family, CommandPoolCreateFlags::empty())
            .expect("TODO");
        let command_buffer = command_pool.allocate_one(Level::Primary);

        (command_pool, command_buffer)
    };

    let submission_complete_semaphore = device.create_semaphore().expect("TODO");
    let submission_complete_fence = device.create_fence(true).expect("TODO");

    let binary_mesh_data = include_bytes!("../../assets/teapot_mesh.bin");
    let mesh: Vec<Vertex> = bincode::deserialize(binary_mesh_data).expect("TODO");

    let (vertex_buffer_memory, vertex_buffer) = unsafe {
        use gfx_hal::{adapter::PhysicalDevice, buffer::Usage, memory::Properties, MemoryTypeId};

        // TODO: Ensure aligned to `limits.non_coherent_atom_size`
        let buffer_len = (mesh.len() * std::mem::size_of::<Vertex>()) as u64;
        let mut buffer = device
            .create_buffer(buffer_len, Usage::VERTEX)
            .expect("TODO");

        let req = device.get_buffer_requirements(&buffer);

        let memory_types = adapter.physical_device.memory_properties().memory_types;
        let memory_type = memory_types
            .iter()
            .enumerate()
            .find(|(id, mem_type)| {
                let type_supported = req.type_mask & (1_u64 << id) != 0;
                type_supported && mem_type.properties.contains(Properties::CPU_VISIBLE)
            })
            .map(|(id, _ty)| MemoryTypeId(id))
            .expect("TODO");

        let buffer_memory = device.allocate_memory(memory_type, req.size).expect("TODO");
        device
            .bind_buffer_memory(&buffer_memory, 0, &mut buffer)
            .expect("TODO");

        let mapped_memory = device
            .map_memory(&buffer_memory, 0..buffer_len)
            .expect("TODO");

        std::ptr::copy_nonoverlapping(
            mesh.as_ptr() as *const u8,
            mapped_memory,
            buffer_len as usize,
        );

        device
            .flush_mapped_memory_ranges(vec![(&buffer_memory, 0..buffer_len)])
            .expect("TODO");

        device.unmap_memory(&buffer_memory);

        (buffer_memory, buffer)
    };

    let mut desc_set_pool = unsafe {
        use gfx_hal::pso::{DescriptorPoolCreateFlags, DescriptorRangeDesc, DescriptorType};

        device
            .create_descriptor_pool(
                1,
                &[DescriptorRangeDesc {
                    ty: DescriptorType::UniformBuffer,
                    count: 1,
                }],
                DescriptorPoolCreateFlags::empty(),
            )
            .expect("TODO")
    };

    let desc_set = unsafe {
        use gfx_hal::pso::DescriptorPool;

        desc_set_pool.allocate_set(&desc_set_layout).expect("TODO")
    };

    let (uniform_buffer_memory, uniform_buffer) = unsafe {
        use gfx_hal::{adapter::PhysicalDevice, buffer::Usage, memory::Properties, MemoryTypeId};

        // TODO: Ensure aligned to `limits.non_coherent_atom_size`
        let buffer_len = std::mem::size_of::<UniformBlock>() as u64;
        let mut buffer = device
            .create_buffer(buffer_len, Usage::UNIFORM)
            .expect("TODO");

        let req = device.get_buffer_requirements(&buffer);

        let memory_types = adapter.physical_device.memory_properties().memory_types;
        let memory_type = memory_types
            .iter()
            .enumerate()
            .find(|(id, mem_type)| {
                let type_supported = req.type_mask & (1_u64 << id) != 0;
                type_supported && mem_type.properties.contains(Properties::CPU_VISIBLE)
            })
            .map(|(id, _ty)| MemoryTypeId(id))
            .expect("TODO");

        let buffer_memory = device.allocate_memory(memory_type, req.size).expect("TODO");
        device
            .bind_buffer_memory(&buffer_memory, 0, &mut buffer)
            .expect("TODO");

        (buffer_memory, buffer)
    };

    unsafe {
        use gfx_hal::pso::{Descriptor, DescriptorSetWrite};

        device.write_descriptor_sets(vec![DescriptorSetWrite {
            set: &desc_set,
            binding: 0,
            array_offset: 0,
            descriptors: Some(Descriptor::Buffer(&uniform_buffer, None..None)),
        }]);
    }

    // TODO: Order sensibly
    struct Resources<B: gfx_hal::Backend> {
        instance: B::Instance,
        surface: B::Surface,
        device: B::Device,
        render_pass: B::RenderPass,
        pipeline_layout: B::PipelineLayout,
        pipeline: B::GraphicsPipeline,
        command_pool: B::CommandPool,
        submission_complete_semaphore: B::Semaphore,
        submission_complete_fence: B::Fence,
        vertex_buffer_memory: B::Memory,
        vertex_buffer: B::Buffer,
        uniform_buffer_memory: B::Memory,
        uniform_buffer: B::Buffer,
        desc_set_pool: B::DescriptorPool,
        desc_set_layout: B::DescriptorSetLayout,
    }

    struct ResourceHolder<B: gfx_hal::Backend>(Option<Resources<B>>);

    impl<B: gfx_hal::Backend> Drop for ResourceHolder<B> {
        fn drop(&mut self) {
            let Resources {
                instance,
                surface,
                device,
                command_pool,
                render_pass,
                pipeline_layout,
                pipeline,
                submission_complete_semaphore,
                submission_complete_fence,
                vertex_buffer_memory,
                vertex_buffer,
                desc_set_layout,
                desc_set_pool,
                uniform_buffer_memory,
                uniform_buffer,
            } = self.0.take().unwrap();

            // Clean up resources
            unsafe {
                device.destroy_buffer(uniform_buffer);
                device.free_memory(uniform_buffer_memory);
                device.destroy_descriptor_pool(desc_set_pool);
                device.destroy_descriptor_set_layout(desc_set_layout);
                device.free_memory(vertex_buffer_memory);
                device.destroy_buffer(vertex_buffer);
                device.destroy_fence(submission_complete_fence);
                device.destroy_semaphore(submission_complete_semaphore);
                device.destroy_graphics_pipeline(pipeline);
                device.destroy_pipeline_layout(pipeline_layout);
                device.destroy_render_pass(render_pass);
                device.destroy_command_pool(command_pool);
                instance.destroy_surface(surface);
            }
        }
    }

    let mut resource_holder: ResourceHolder<backend::Backend> = ResourceHolder(Some(Resources {
        instance,
        surface,
        device,
        command_pool,
        render_pass,
        pipeline_layout,
        pipeline,
        submission_complete_semaphore,
        submission_complete_fence,
        vertex_buffer_memory,
        vertex_buffer,
        desc_set_layout,
        desc_set_pool,
        uniform_buffer_memory,
        uniform_buffer,
    }));

    let start_time = std::time::Instant::now();

    let mut should_rebuild_swapchain = true;

    event_loop.run(move |event, _, control_flow| {
        use winit::event::{Event, WindowEvent};
        use winit::event_loop::ControlFlow;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(dims) => {
                    surface_extent = Extent2D {
                        width: dims.width,
                        height: dims.height,
                    };
                    should_rebuild_swapchain = true;
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    surface_extent = Extent2D {
                        width: new_inner_size.width,
                        height: new_inner_size.height,
                    };
                    should_rebuild_swapchain = true;
                }
                _ => (),
            },
            Event::MainEventsCleared => window.request_redraw(),
            Event::RedrawRequested(_) => {
                use gfx_hal::window::PresentationSurface;

                let res = resource_holder.0.as_mut().unwrap();

                let mut viewport = {
                    use gfx_hal::pso::{Rect, Viewport};

                    Viewport {
                        rect: Rect {
                            x: 0,
                            y: 0,
                            w: surface_extent.width as i16,
                            h: surface_extent.height as i16,
                        },
                        depth: 0.0..1.0,
                    }
                };

                if should_rebuild_swapchain {
                    use gfx_hal::window::SwapchainConfig;

                    let caps = res.surface.capabilities(&adapter.physical_device);
                    let mut swap_config =
                        SwapchainConfig::from_caps(&caps, surface_color_format, surface_extent);
                    if caps.image_count.contains(&3) {
                        swap_config.image_count = 3;
                    }

                    surface_extent = swap_config.extent;

                    unsafe {
                        res.surface
                            .configure_swapchain(&res.device, swap_config)
                            .expect("TODO");
                    };

                    viewport.rect.w = surface_extent.width as _;
                    viewport.rect.h = surface_extent.height as _;
                    should_rebuild_swapchain = false;
                }

                // Write uniform data to descriptor set
                unsafe {
                    let t = start_time.elapsed().as_secs_f32() / 3.0;
                    let pi = std::f32::consts::PI;
                    let r = (t + 0.0 * pi / 3.0).sin().max(0.0);
                    let g = (t + 2.0 * pi / 3.0).sin().max(0.0);
                    let b = (t + 4.0 * pi / 3.0).sin().max(0.0);

                    let uniform_block = UniformBlock {
                        ambient_light: [0.1 * r, 0.1 * g, 0.1 * b, 1.0],
                        light_direction: [1.0, -1.0, 1.0, 1.0],
                        light_color: [r, g, b, 1.0],
                    };

                    let buffer_len = std::mem::size_of::<UniformBlock>() as u64;

                    let mapped_memory = res
                        .device
                        .map_memory(&res.uniform_buffer_memory, 0..buffer_len)
                        .expect("TODO");

                    std::ptr::copy_nonoverlapping(
                        &uniform_block as *const UniformBlock as *const u8,
                        mapped_memory,
                        buffer_len as usize,
                    );

                    res.device
                        .flush_mapped_memory_ranges(vec![(
                            &res.uniform_buffer_memory,
                            0..buffer_len,
                        )])
                        .expect("TODO");

                    res.device.unmap_memory(&res.uniform_buffer_memory);
                }

                let surface_image = unsafe {
                    match res.surface.acquire_image(!0) {
                        Ok((image, _)) => image,
                        Err(_) => {
                            should_rebuild_swapchain = true;
                            return;
                        }
                    }
                };

                let framebuffer = unsafe {
                    use std::borrow::Borrow;

                    use gfx_hal::image::Extent;

                    res.device
                        .create_framebuffer(
                            &res.render_pass,
                            vec![surface_image.borrow()],
                            Extent {
                                width: surface_extent.width,
                                height: surface_extent.height,
                                depth: 1,
                            },
                        )
                        .unwrap()
                };

                unsafe {
                    use gfx_hal::pool::CommandPool;

                    let render_timeout_ns = 1_000_000_000;
                    res.device
                        .wait_for_fence(&res.submission_complete_fence, render_timeout_ns)
                        .expect("TODO");
                    res.device
                        .reset_fence(&res.submission_complete_fence)
                        .expect("TODO");
                    res.command_pool.reset(false);
                }

                unsafe {
                    use gfx_hal::command::{
                        ClearColor, ClearValue, CommandBuffer, CommandBufferFlags, SubpassContents,
                    };

                    command_buffer.begin_primary(CommandBufferFlags::ONE_TIME_SUBMIT);

                    command_buffer.set_viewports(0, &[viewport.clone()]);
                    command_buffer.set_scissors(0, &[viewport.rect]);
                    command_buffer.bind_graphics_pipeline(&res.pipeline);

                    command_buffer.bind_vertex_buffers(0, vec![(&res.vertex_buffer, 0)]);

                    command_buffer.bind_graphics_descriptor_sets(
                        &res.pipeline_layout,
                        0,
                        vec![&desc_set],
                        &[],
                    );

                    command_buffer.begin_render_pass(
                        &res.render_pass,
                        &framebuffer,
                        viewport.rect,
                        &[ClearValue {
                            color: ClearColor {
                                float32: [0.0, 0.0, 0.0, 1.0],
                            },
                        }],
                        SubpassContents::Inline,
                    );

                    fn make_transform(
                        translate: [f32; 3],
                        angle: f32,
                        scale: f32,
                    ) -> [[f32; 4]; 4] {
                        let c = angle.cos() * scale;
                        let s = angle.sin() * scale;
                        let [dx, dy, dz] = translate;

                        [
                            [c, 0., s, 0.],
                            [0., scale, 0., 0.],
                            [-s, 0., c, 0.],
                            [dx, dy, dz, 1.],
                        ]
                    }

                    let angle = start_time.elapsed().as_secs_f32();

                    let things_to_draw = &[
                        PushConstants {
                            transform: make_transform([0., 0.5, 0.5], -angle, 0.5),
                        },
                        PushConstants {
                            transform: make_transform([0., 0., 0.5], angle, 1.0),
                        },
                        PushConstants {
                            transform: make_transform([0., -0.5, 0.5], -angle, 0.5),
                        },
                    ];

                    for thing in things_to_draw {
                        use gfx_hal::pso::ShaderStageFlags;

                        let size_in_bytes = std::mem::size_of::<PushConstants>();
                        let size_in_u32s = size_in_bytes / std::mem::size_of::<u32>();
                        let start_ptr = thing as *const PushConstants as *const u32;
                        let bytes = std::slice::from_raw_parts(start_ptr, size_in_u32s);
                        command_buffer.push_graphics_constants(
                            &res.pipeline_layout,
                            ShaderStageFlags::VERTEX,
                            0,
                            bytes,
                        );

                        let vertex_count = mesh.len() as u32;
                        command_buffer.draw(0..vertex_count, 0..1);
                    }

                    command_buffer.end_render_pass();
                    command_buffer.finish();
                }

                unsafe {
                    use gfx_hal::queue::{CommandQueue, Submission};

                    let submission = Submission {
                        command_buffers: vec![&command_buffer],
                        wait_semaphores: None,
                        signal_semaphores: vec![&res.submission_complete_semaphore],
                    };
                    queue_group.queues[0].submit(submission, Some(&res.submission_complete_fence));

                    let result = queue_group.queues[0].present_surface(
                        &mut res.surface,
                        surface_image,
                        Some(&res.submission_complete_semaphore),
                    );

                    res.device.destroy_framebuffer(framebuffer);

                    should_rebuild_swapchain |= result.is_err();
                }
            }
            _ => (),
        }
    });
}
