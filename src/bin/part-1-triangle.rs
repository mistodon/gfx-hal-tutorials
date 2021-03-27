fn main() {
    use std::iter;
    use std::mem::ManuallyDrop;

    use gfx_hal::{
        device::Device,
        window::{Extent2D, PresentationSurface, Surface},
        Instance,
    };
    use shaderc::ShaderKind;

    const APP_NAME: &'static str = "Part 1: Drawing a triangle";
    const WINDOW_SIZE: [u32; 2] = [512, 512];
    const DIMS: gfx_hal::window::Extent2D = gfx_hal::window::Extent2D {
        width: 512,
        height: 512,
    };

    // Any `winit` application starts with an event loop. You need one of these
    // to create a window.
    let event_loop = winit::event_loop::EventLoop::new();

    // Before we create a window, we also need to know what size to make it.
    //
    // Note that logical and physical window size are different though!
    //
    // Physical size is the real-life size of the display, in physical pixels.
    // Logical size is the scaled display, according to the OS. High-DPI
    // displays will present a smaller logical size, which you can scale up by
    // the DPI to determine the physical size.
    let (logical_window_size, physical_window_size) = {
        use winit::dpi::{LogicalSize, PhysicalSize};

        let dpi = event_loop.primary_monitor().scale_factor();
        let logical: LogicalSize<u32> = WINDOW_SIZE.into();
        let physical: PhysicalSize<u32> = logical.to_physical(dpi);

        (logical, physical)
    };

    // This will be the size of the final image we render, and therefore the
    // size of the surface we render to.
    //
    // We use the *physical* size because we want a rendered image that covers
    // every real pixel.
    let mut surface_extent = Extent2D {
        width: physical_window_size.width,
        height: physical_window_size.height,
    };

    // We use the *logical* size to build the window because this will give a
    // consistent size on displays of different pixel densities.
    let window = winit::window::WindowBuilder::new()
        .with_title(APP_NAME)
        .with_inner_size(logical_window_size)
        .build(&event_loop)
        .expect("Failed to create window");

    // The `instance` is an entry point to the graphics API. The `1` in the
    // call is a version number - we don't care about that for now.
    //
    // The `surface` is an abstraction of the OS window we're drawing into.
    // In `gfx`, it also manages the swap chain, which is a chain of
    // multiple images for us to render to. While one is being displayed, we
    // can write to another one - and then swap them, hence the name.
    //
    // The `adapter` represents a physical device. A graphics card for example.
    // The host may have more than one, but below, we just take the first.
    let (instance, surface, adapter) = {
        let instance = backend::Instance::create(APP_NAME, 1).expect("Backend not supported");

        let surface = unsafe {
            instance
                .create_surface(&window)
                .expect("Failed to create surface for window")
        };

        let adapter = instance.enumerate_adapters().remove(0);

        (instance, surface, adapter)
    };

    let (device, mut queue_group) = {
        use gfx_hal::queue::QueueFamily;

        // We need a command queue to submit commands to the GPU.
        // Here we select the family (type) of queue we want. For rendering
        // (as opposed to compute, etc.) we need one that supports graphics.
        // We also of course need one that our surface supports.
        let queue_family = adapter
            .queue_families
            .iter()
            .find(|family| {
                surface.supports_queue_family(family) && family.queue_type().supports_graphics()
            })
            .expect("No compatible queue family found");

        // The `open` method returns us a logical `device`, and the set of
        // queue groups we asked for.
        //
        // A logical device is a view of the physical device, with or without
        // certain features. Features are similar to Rust features (optional
        // functionality) and in our example here, we don't request any.
        //
        // A `queue_group` is exactly what it sounds like. In the call below,
        // we're requesting one queue group of the above `queue_family`. We're
        // also asking for only one queue (because the list `&[1.0]` has only
        // one item) with priority `1.0`. The priorities are relative and so
        // are not important if you only have one queue.
        let mut gpu = unsafe {
            use gfx_hal::adapter::PhysicalDevice;

            adapter
                .physical_device
                .open(&[(queue_family, &[1.0])], gfx_hal::Features::empty())
                .expect("Failed to open device")
        };

        (gpu.device, gpu.queue_groups.pop().unwrap())
    };

    // Earlier we obtained a command queue to submit drawing commands to. The
    // data structure that carries those commands is called a `command_buffer`,
    // which are allocated from a `command_pool`.
    let (command_pool, mut command_buffer) = unsafe {
        use gfx_hal::command::Level;
        use gfx_hal::pool::{CommandPool, CommandPoolCreateFlags};

        // To create our command pool, we have to specify the type of queue we
        // will be submitting it to. Luckily, we already have a queue and can
        // get the family from that.
        //
        // Ignore `CommandPoolCreateFlags` for now.
        let mut command_pool = device
            .create_command_pool(queue_group.family, CommandPoolCreateFlags::empty())
            .expect("Out of memory");

        // If we were planning to draw things in parallel or otherwise optimize
        // our command submissions, we might use more than one buffer. But for
        // now we'll just allocate a single one and re-use it for each frame.
        //
        // Level indicates whether it's a primary or secondary command buffer.
        // Secondary buffers are those nested within primary ones, but we don't
        // need to worry about that just now.
        let command_buffer = command_pool.allocate_one(Level::Primary);

        (command_pool, command_buffer)
    };

    // We need to determine a format for the pixels in our surface image -
    // that is: what bytes, in what order, represent which color components.
    //
    // First we get a list of supported formats (where `None` means that any is
    // supported). Next, we try to pick one that supports SRGB, so that gamma
    // correction is handled for us. If we can't, we just pick the first one,
    // or default to `Rgba8Srgb`.
    let surface_color_format = {
        use gfx_hal::format::{ChannelType, Format};

        let supported_formats = surface
            .supported_formats(&adapter.physical_device)
            .unwrap_or(vec![]);

        let default_format = *supported_formats.get(0).unwrap_or(&Format::Rgba8Srgb);

        supported_formats
            .into_iter()
            .find(|format| format.base_format().1 == ChannelType::Srgb)
            .unwrap_or(default_format)
    };

    // A render pass defines which attachments (images) are to be used for
    // what purposes. Right now, we only have a color attachment for the final
    // output, but eventually we might have depth/stencil attachments, or even
    // other color attachments for other purposes.
    let render_pass = {
        use gfx_hal::image::Layout;
        use gfx_hal::pass::{
            Attachment, AttachmentLoadOp, AttachmentOps, AttachmentStoreOp, SubpassDesc,
        };

        // This is an attachment for the final output. Note that it must have
        // the same pixel format as our surface. It has `1` sample-per-pixel
        // (which isn't worth thinking about too much).
        //
        // The `ops` parameter describes what to do to the image at the start
        // and end of the render pass (for color and depth). We want to `Clear`
        // it first, and then `Store` our rendered pixels to it at the end.
        //
        // The `stencil_ops` are the same, but for the stencil buffer, which we
        // aren't using yet.
        //
        // The `layouts` parameter defines the before and after layouts for the
        // image - essentially how it is laid out in memory. This is only a
        // hint and mostly for optimisation. Here, we know we're going to
        // `Present` the image to the window, so we want a layout optimised for
        // that by the end.
        let color_attachment = Attachment {
            format: Some(surface_color_format),
            samples: 1,
            ops: AttachmentOps::new(AttachmentLoadOp::Clear, AttachmentStoreOp::Store),
            stencil_ops: AttachmentOps::DONT_CARE,
            layouts: Layout::Undefined..Layout::Present,
        };

        // A render pass could have multiple subpasses to it, but here we only
        // want one. The `0` is an id - an index into the final list of
        // attachments. It means we're using attachment `0` as a color
        // attachment.
        //
        // The `Layout` is the layout to be used *during* the render pass.
        let subpass = SubpassDesc {
            colors: &[(0, Layout::ColorAttachmentOptimal)],
            depth_stencil: None,
            inputs: &[],
            resolves: &[],
            preserves: &[],
        };

        unsafe {
            // Note that we're passing a list of attachments here.
            //
            // The attachment in index `0` - `color_attachment` - will be
            // bound as a color attachment, because the subpass above
            // specifies the id `0`.
            //
            // The third parameter is for expressing `dependencies` between
            // subpasses, which we don't need.
            device
                .create_render_pass(
                    iter::once(color_attachment),
                    iter::once(subpass),
                    iter::empty(),
                )
                .expect("Out of memory")
        }
    };

    let pipeline_layout = unsafe {
        device
            .create_pipeline_layout(iter::empty(), iter::empty())
            .expect("Out of memory")
    };

    let vertex_shader = include_str!("shaders/part-1.vert");
    let fragment_shader = include_str!("shaders/part-1.frag");

    /// Compile some GLSL shader source to SPIR-V.
    ///
    /// We tend to write shaders in high-level languages, but the GPU doesn't
    /// work with that directly. Instead, we can convert it to an intermediate
    /// representation: SPIR-V. This is more easily interpreted and optimized
    /// by your graphics card. As an added bonus, this allows us to use the
    /// same shader code across different backends.
    fn compile_shader(glsl: &str, shader_kind: ShaderKind) -> Vec<u32> {
        let mut compiler = shaderc::Compiler::new().unwrap();

        // The `compile_into_spirv` function is pretty straightforward.
        // It optionally takes a filename (which we haven't, hence "unnamed").
        // It also takes the entry point of the shader ("main"), and some
        // other compiler options which we're also ignoring (`None`).
        let compiled_shader = compiler
            .compile_into_spirv(glsl, shader_kind, "unnamed", "main", None)
            .expect("Failed to compile shader");

        // The result is an opaque object. We can use the `as_binary` method to
        // get a `&[u32]` view of it, and then convert it to an owned `Vec`.
        compiled_shader.as_binary().to_vec()
    }

    /// Create a pipeline with the given layout and shaders.
    ///
    /// A pipeline contains nearly all the required information for rendering,
    /// and is only usable within the render pass it's defined for.
    unsafe fn make_pipeline<B: gfx_hal::Backend>(
        device: &B::Device,
        render_pass: &B::RenderPass,
        pipeline_layout: &B::PipelineLayout,
        vertex_shader: &str,
        fragment_shader: &str,
    ) -> B::GraphicsPipeline {
        use gfx_hal::pass::Subpass;
        use gfx_hal::pso::{
            BlendState, ColorBlendDesc, ColorMask, EntryPoint, Face, GraphicsPipelineDesc,
            InputAssemblerDesc, Primitive, PrimitiveAssemblerDesc, Rasterizer, Specialization,
        };
        let vertex_shader_module = device
            .create_shader_module(&compile_shader(vertex_shader, ShaderKind::Vertex))
            .expect("Failed to create vertex shader module");

        let fragment_shader_module = device
            .create_shader_module(&compile_shader(fragment_shader, ShaderKind::Fragment))
            .expect("Failed to create fragment shader module");

        // Shader modules are re-usable, and we could choose to define multiple
        // entry functions or multiple different specialized versions for
        // different pipelines. We specify which to use with the `EntryPoint`
        // struct here.
        //
        // The `entry` parameter here refers to the name of the function in the
        // shader that serves as the entry point.
        //
        // The `specialization` parameter allows you to tweak specific
        // constants in the shaders. That's not in scope for this part, so we
        // just use the empty default.
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
        // We're not using vertex buffers or attributes, and we're definitely
        // not using tesselation/geometry shaders. So for now, all we have to
        // specify is that we want to render a `TriangleList` using the vertex
        // shader (`vs_entry`) that we loaded before.
        let primitive_assembler = PrimitiveAssemblerDesc::Vertex {
            buffers: &[],
            attributes: &[],
            input_assembler: InputAssemblerDesc::new(Primitive::TriangleList),
            vertex: vs_entry,
            tessellation: None,
            geometry: None,
        };
        // Here is where we configure our pipeline. The `new` function sets the
        // required properties, after which we can add additional sections to
        // define what kind of render targets/attachments and vertex buffers it
        // accepts.
        let mut pipeline_desc = GraphicsPipelineDesc::new(
            primitive_assembler,
            Rasterizer {
                cull_face: Face::BACK,
                ..Rasterizer::FILL
            },
            Some(fs_entry),
            pipeline_layout,
            Subpass {
                index: 0,
                main_pass: render_pass,
            },
        );

        // Here we specify that our pipeline will render to a color attachment.
        // The `mask` defines which color channels (red, green, blue, alpha) it
        // will write, and the `blend` parameter specifies how to blend the
        // rendered pixel with the existing pixel in the attachment.
        //
        // In this case, the `BlendState::ALPHA` preset says to blend them
        // based on their alpha values, which is usually what you want.
        pipeline_desc.blender.targets.push(ColorBlendDesc {
            mask: ColorMask::ALL,
            blend: Some(BlendState::ALPHA),
        });
        let pipeline = device
            .create_graphics_pipeline(&pipeline_desc, None)
            .expect("Failed to create graphics pipeline");

        // Once the pipeline is created, we no longer need to keep
        // the shader modules in memory. In theory, we could keep
        // them around for creating other pipelines with the same
        // shaders, but we don't need to.
        device.destroy_shader_module(vertex_shader_module);
        device.destroy_shader_module(fragment_shader_module);

        pipeline
    }

    let pipeline = unsafe {
        make_pipeline::<backend::Backend>(
            &device,
            &render_pass,
            &pipeline_layout,
            vertex_shader,
            fragment_shader,
        )
    };

    // Since the GPU may operate asynchronously, there are a few important
    // things we have to synchronize. We use _fences_ to synchronize the CPU
    // with the GPU, and we use _semaphores_ to synchronize separate processes
    // within the GPU.
    //
    // Firstly, we have to ensure that our GPU commands have been submitted to
    // the queue before we re-use the command buffer. This is what the
    // `submission_complete_fence` is for.
    //
    // Secondly, we have to ensure that our image has been rendered before we
    // display it on the screen.
    // This is what the `rendering_complete_semaphore` is for.
    let submission_complete_fence = device.create_fence(true).expect("Out of memory");
    let rendering_complete_semaphore = device.create_semaphore().expect("Out of memory");

    struct Resources<B: gfx_hal::Backend> {
        instance: B::Instance,
        surface: B::Surface,
        device: B::Device,
        render_passes: Vec<B::RenderPass>,
        pipeline_layouts: Vec<B::PipelineLayout>,
        pipelines: Vec<B::GraphicsPipeline>,
        command_pool: B::CommandPool,
        submission_complete_fence: B::Fence,
        rendering_complete_semaphore: B::Semaphore,
    }

    // We put the resources in an `ManuallyDrop` so that we can `take` the
    // contents later and destroy them.
    struct ResourceHolder<B: gfx_hal::Backend>(ManuallyDrop<Resources<B>>);

    impl<B: gfx_hal::Backend> Drop for ResourceHolder<B> {
        fn drop(&mut self) {
            unsafe {
                // We are moving the `Resources` out of the struct...
                let Resources {
                    instance,
                    mut surface,
                    device,
                    command_pool,
                    render_passes,
                    pipeline_layouts,
                    pipelines,
                    submission_complete_fence,
                    rendering_complete_semaphore,
                } = ManuallyDrop::take(&mut self.0);

                // ... and destroying them individually:
                device.destroy_semaphore(rendering_complete_semaphore);
                device.destroy_fence(submission_complete_fence);
                for pipeline in pipelines {
                    device.destroy_graphics_pipeline(pipeline);
                }
                for pipeline_layout in pipeline_layouts {
                    device.destroy_pipeline_layout(pipeline_layout);
                }
                for render_pass in render_passes {
                    device.destroy_render_pass(render_pass);
                }
                device.destroy_command_pool(command_pool);
                surface.unconfigure_swapchain(&device);
                instance.destroy_surface(surface);
            }
        }
    }

    let mut resource_holder: ResourceHolder<backend::Backend> =
        ResourceHolder(ManuallyDrop::new(Resources {
            instance,
            surface,
            device,
            command_pool,
            render_passes: vec![render_pass],
            pipeline_layouts: vec![pipeline_layout],
            pipelines: vec![pipeline],
            submission_complete_fence,
            rendering_complete_semaphore,
        }));

    // This will be very important later! It must be initialized to `true` so
    // that we rebuild the swapchain on the first frame.
    let mut should_configure_swapchain = true;

    // Note that this takes a `move` closure. This means it will take ownership
    // over any resources referenced within. It also means they will be dropped
    // only when the application is quit.
    event_loop.run(move |event, _, control_flow| {
        use winit::event::{Event, WindowEvent};
        use winit::event_loop::ControlFlow;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                // If the window changes size, or the display changes
                // DPI / scale-factor, then the *physical* size will change,
                // which means our surface needs updated too.
                //
                // When the surface changes size, we need to rebuild the
                // swapchain so that its images are the right size.
                WindowEvent::Resized(dims) => {
                    surface_extent = Extent2D {
                        width: dims.width,
                        height: dims.height,
                    };
                    should_configure_swapchain = true;
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    surface_extent = Extent2D {
                        width: new_inner_size.width,
                        height: new_inner_size.height,
                    };
                    should_configure_swapchain = true;
                }
                _ => (),
            },
            // In an interactive application, you would handle your logic
            // updates here.
            //
            // Right now, we just want to redraw the window each frame
            // and that's all.
            Event::MainEventsCleared => window.request_redraw(),
            Event::RedrawRequested(_) => {
                // We will need to reference our resources in our rendering
                // commands.
                //
                // Because I'm lazy and we're storing resources in `Vec`s,
                // we also take references to the contents here to avoid
                // confusing ourselves with different indices later.
                let res: &mut Resources<_> = &mut resource_holder.0;
                let render_pass = &res.render_passes[0];
                let pipeline = &res.pipelines[0];

                unsafe {
                    use gfx_hal::pool::CommandPool;

                    // We refuse to wait more than a second, to avoid hanging.
                    let render_timeout_ns = 1_000_000_000;

                    // Graphics commands may execute asynchronously, so to
                    // ensure we're finished rendering the previous frame
                    // before starting this new one, we wait here for the
                    // rendering to signal the `submission_complete_fence` from
                    // the previous frame.
                    //
                    // This may not be the most efficient option - say if you
                    // wanted to render more than one frame simulatneously
                    // - but for our example, it simplifies things.
                    res.device
                        .wait_for_fence(&res.submission_complete_fence, render_timeout_ns)
                        .expect("Out of memory or device lost");

                    // Once the fence has been signalled, we must reset it
                    res.device
                        .reset_fence(&mut res.submission_complete_fence)
                        .expect("Out of memory");

                    // This clears out the previous frame's command buffer and
                    // returns it to the pool for use this frame.
                    res.command_pool.reset(false);
                }

                // If the window is resized, or the rendering context is
                // otherwise invalidated, we may need to recreate our whole
                // swapchain.
                //
                // For now, all that entails is calling the
                // `configure_swapchain` method with the correct config, but
                // in future parts, we may have to recreate other resources
                // here.
                if should_configure_swapchain {
                    use gfx_hal::window::SwapchainConfig;

                    let caps = res.surface.capabilities(&adapter.physical_device);

                    // We pass our `surface_extent` as a desired default, but
                    // it may return us a different value, depending on what it
                    // supports.
                    let mut swapchain_config =
                        SwapchainConfig::from_caps(&caps, surface_color_format, surface_extent);

                    // If our device supports having 3 images in our swapchain,
                    // then we want to use that.
                    //
                    // This seems to fix some fullscreen slowdown on macOS.
                    if caps.image_count.contains(&3) {
                        swapchain_config.image_count = 3;
                    }

                    // In case the surface returned an extent different from
                    // the size we requested, we update our value.
                    surface_extent = swapchain_config.extent;

                    unsafe {
                        res.surface
                            .configure_swapchain(&res.device, swapchain_config)
                            .expect("Failed to configure swapchain");
                    };

                    should_configure_swapchain = false;
                }

                // Our swapchain consists of two or more images. We want to
                // display one of them on screen, and then render to a
                // different one so we can swap them out smoothly. The
                // `acquire_image` method gives us a free one to render on.
                //
                // If it fails, there could be an issue with our swapchain, so
                // we early-out and rebuild it for next frame.
                let surface_image = unsafe {
                    // We refuse to wait more than a second, to avoid hanging.
                    let acquire_timeout_ns = 1_000_000_000;

                    match res.surface.acquire_image(acquire_timeout_ns) {
                        Ok((image, _)) => image,
                        Err(_) => {
                            should_configure_swapchain = true;
                            return;
                        }
                    }
                };

                // The Vulkan API, which `gfx` is based on, doesn't allow you
                // to render directly to images. Instead, you render to an
                // abstract framebuffer which represents your render target.
                // In practice, there may be no difference in our case, but
                // it's somthing to be aware of.
                let framebuffer = unsafe {
                    use gfx_hal::image::Extent;
                    use gfx_hal::window;

                    let caps = res.surface.capabilities(&adapter.physical_device);
                    let swap_config =
                        window::SwapchainConfig::from_caps(&caps, surface_color_format, DIMS);
                    let fat = swap_config.framebuffer_attachment();

                    res.device
                        .create_framebuffer(
                            render_pass,
                            iter::once(fat),
                            Extent {
                                width: surface_extent.width,
                                height: surface_extent.height,
                                depth: 1,
                            },
                        )
                        .unwrap()
                };

                // A viewport defines the rectangular section of the screen
                // to draw into. Here we're specifying the whole screen.
                // This will be used once we start rendering.
                let viewport = {
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

                unsafe {
                    use gfx_hal::command::{
                        ClearColor, ClearValue, CommandBuffer, CommandBufferFlags,
                        RenderAttachmentInfo, SubpassContents,
                    };
                    use std::borrow::Borrow;

                    // This is how we start our command buffer. We set a
                    // flag telling it we're only going to submit it once,
                    // rather than submit the same commands over and over.
                    command_buffer.begin_primary(CommandBufferFlags::ONE_TIME_SUBMIT);

                    // This is how we specify which part of the surface
                    // we are drawing into. Changing the viewport will stretch
                    // the resulting image into that rect. Changing the scissor
                    // will crop it.
                    command_buffer.set_viewports(0, iter::once(viewport.clone()));
                    command_buffer.set_scissors(0, iter::once(viewport.rect));

                    // Here we say which render pass we're in. This
                    // defines which framebuffer (images) we'll draw to, and
                    // also specifies what color to clear them to first, if
                    // they have been configured to be cleared.
                    command_buffer.begin_render_pass(
                        render_pass,
                        &framebuffer,
                        viewport.rect,
                        iter::once(RenderAttachmentInfo {
                            image_view: surface_image.borrow(),
                            clear_value: ClearValue {
                                color: ClearColor {
                                    float32: [0.0, 0.0, 0.0, 1.0],
                                },
                            },
                        }),
                        SubpassContents::Inline,
                    );

                    // This sets the pipeline that will be used to draw.
                    // We can change this whenever we like, but it can be
                    // inefficient to do so. Regardless, we only have one right
                    // now.
                    command_buffer.bind_graphics_pipeline(pipeline);

                    // This is the command that actually tells the GPU to draw
                    // some triangles. The `0..3` in the first parameter means
                    // "draw vertices 0, 1, and 2". (For now, all those numbers
                    // refer to is the `gl_VertexIndex` parameter in our vertex
                    // shader.
                    // The second parameter means "draw instance 0". Ignore
                    // that for now as we're not using instanced rendering.
                    command_buffer.draw(0..3, 0..1);

                    // Here we finish our only render pass. We could begin
                    // another, but since we're done, we also close off the
                    // command buffer, which is now ready to submit to the GPU.
                    command_buffer.end_render_pass();
                    command_buffer.finish();
                }

                unsafe {
                    use gfx_hal::queue::CommandQueue;

                    // A `Submission` contains references to the command
                    // buffers to submit, and also any semaphores used for
                    // scheduling.
                    //
                    // If you wanted to ensure a previous submission was
                    // complete before starting this one, you could add
                    // `wait_semaphores`.
                    //
                    // In our case though, all we want to do is tell
                    // `rendering_complete_semaphore` when we're done.

                    // Commands must be submitted to an appropriate queue. We
                    // requested a graphics queue, and so we are submitting
                    // graphics commands.
                    //
                    // We tell the submission to notify
                    // `submission_complete_fence` when the submission is
                    // complete, at which point we can reclaim the command
                    // buffer we used for next frame.
                    queue_group.queues[0].submit(
                        iter::once(&command_buffer),
                        iter::empty(),
                        iter::once(&res.rendering_complete_semaphore),
                        Some(&mut res.submission_complete_fence),
                    );

                    // Finally, the `present` takes the output of our
                    // rendering and displays it onscreen. We pass the
                    // `rendering_complete_semaphore` so that we can be sure
                    // the image we want to display has been rendered.
                    let result = queue_group.queues[0].present(
                        &mut res.surface,
                        surface_image,
                        Some(&mut res.rendering_complete_semaphore),
                    );

                    // If presenting failed, it could be a problem with the
                    // swapchain. For example, if the window was resized, our
                    // image is no longer the correct dimensions.
                    //
                    // In the hopes that we can avoid the same error next
                    // frame, we'll rebuild the swapchain.
                    should_configure_swapchain |= result.is_err();

                    // We created this at the start of the frame
                    // so we should destroy it too to avoid leaking it.
                    res.device.destroy_framebuffer(framebuffer);
                }
            }
            _ => (),
        }
    });
}
