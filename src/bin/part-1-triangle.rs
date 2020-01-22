// TODO: Reorder declarations so that they're as close to their usage sites as they can be
// TODO: Try to create the window with a LogicalSize directly - without screwing
//  up swapchain dimensions.
// TODO: Look at the error types for every `expect` to set a good message.
fn main() {
    use gfx_hal::{device::Device, window::Surface, Instance as _};

    const APP_NAME: &'static str = "Part 1: Drawing a triangle";
    const WINDOW_SIZE: [u32; 2] = [256, 256];

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

    let mut command_pool = unsafe {
        use gfx_hal::pool::CommandPoolCreateFlags;

        device
            .create_command_pool(queue_group.family, CommandPoolCreateFlags::empty())
            .expect("TODO")
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

    let pipeline_layout = unsafe {
        use gfx_hal::pso::ShaderStageFlags;

        // TODO: Can we simplify the layout? No vertex buffer so...
        device
            .create_pipeline_layout(&[], &[(ShaderStageFlags::VERTEX, 0..8)])
            .expect("Can't create pipeline layout")
    };

    let pipeline = {
        use gfx_hal::pass::Subpass;
        use gfx_hal::pso::{
            self, BlendState, ColorBlendDesc, ColorMask, EntryPoint, GraphicsPipelineDesc,
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
            compile_shader(include_str!("shaders/part-1.vert"), ShaderType::Vertex);

        let fragment_shader_module =
            compile_shader(include_str!("shaders/part-1.frag"), ShaderType::Fragment);

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

        let subpass = Subpass {
            index: 0,
            main_pass: &render_pass,
        };

        let mut pipeline_desc = GraphicsPipelineDesc::new(
            shader_entries,
            Primitive::TriangleList,
            Rasterizer::FILL,
            &pipeline_layout,
            subpass,
        );

        pipeline_desc.blender.targets.push(ColorBlendDesc {
            mask: ColorMask::ALL,
            blend: Some(BlendState::ALPHA),
        });

        unsafe {
            let pipeline = device
                .create_graphics_pipeline(&pipeline_desc, None)
                .expect("TODO");

            device.destroy_shader_module(vertex_shader_module);
            device.destroy_shader_module(fragment_shader_module);

            pipeline
        }
    };

    let submission_complete_semaphore = device.create_semaphore().expect("TODO");
    let submission_complete_fence = device.create_fence(true).expect("TODO");
    let mut command_buffer = unsafe {
        use gfx_hal::command::Level;
        use gfx_hal::pool::CommandPool;

        command_pool.allocate_one(Level::Primary)
    };

    // TODO: Order sensibly
    struct Resources<B: gfx_hal::Backend> {
        instance: B::Instance,
        surface: B::Surface,
        device: B::Device,
        command_pool: B::CommandPool,
        render_pass: B::RenderPass,
        pipeline_layout: B::PipelineLayout,
        pipeline: B::GraphicsPipeline,
        submission_complete_semaphore: B::Semaphore,
        submission_complete_fence: B::Fence,
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
            } = self.0.take().unwrap();

            // Clean up resources
            unsafe {
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
    }));

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
                use gfx_hal::image::Extent;
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
                    let swap_config =
                        SwapchainConfig::from_caps(&caps, surface_color_format, surface_extent);

                    surface_extent = swap_config.extent;

                    unsafe {
                        res.surface
                            .configure_swapchain(&res.device, swap_config)
                            .expect("TODO");
                    };

                    viewport.rect.w = surface_extent.width as _;
                    viewport.rect.h = surface_extent.height as _;
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

                    res.device
                        .wait_for_fence(&res.submission_complete_fence, !0)
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
                    command_buffer.draw(0..3, 0..1);
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