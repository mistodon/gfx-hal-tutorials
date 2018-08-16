extern crate gfx_hal_tutorials;

extern crate gfx_backend_metal as backend;
extern crate gfx_hal;
extern crate winit;

// Saves us from having to import gfx types every time.
use gfx_hal_tutorials::prelude::*;

use winit::{Event, EventsLoop, KeyboardInput, VirtualKeyCode, WindowBuilder, WindowEvent};

fn main() {
    let mut events_loop = EventsLoop::new();

    let window = WindowBuilder::new()
        .with_title("Part 01: Resizing")
        .with_dimensions((256, 256).into())
        .build(&events_loop)
        .unwrap();

    let instance = backend::Instance::create("Part 01: Resizing", 1);

    let mut surface = instance.create_surface(&window);

    let mut adapter = instance.enumerate_adapters().remove(0);

    let (device, mut queue_group) = adapter
        .open_with::<_, Graphics>(1, |family| surface.supports_queue_family(family))
        .unwrap();

    let mut command_pool =
        device.create_command_pool_typed(&queue_group, CommandPoolCreateFlags::empty(), 16);

    let vertex_shader_module = {
        let spirv = include_bytes!("../../assets/gen/shaders/part00.vert.spv");
        device.create_shader_module(spirv).unwrap()
    };

    let fragment_shader_module = {
        let spirv = include_bytes!("../../assets/gen/shaders/part00.frag.spv");
        device.create_shader_module(spirv).unwrap()
    };

    let frame_semaphore = device.create_semaphore();
    let frame_fence = device.create_fence(false);

    // This could theoretically change between swapchain creations, but we're going
    // to ignore that for now so that we only have to build our render pass and
    // pipeline once.
    let surface_color_format = {
        let physical_device = &adapter.physical_device;
        let (_, formats, _) = surface.compatibility(physical_device);

        match formats {
            None => Format::Rgba8Srgb,
            Some(options) => options
                .into_iter()
                .find(|format| format.base_format().1 == ChannelType::Srgb)
                .unwrap(),
        }
    };

    let render_pass = {
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

        let dependency = SubpassDependency {
            passes: SubpassRef::External..SubpassRef::Pass(0),
            stages: PipelineStage::COLOR_ATTACHMENT_OUTPUT..PipelineStage::COLOR_ATTACHMENT_OUTPUT,
            accesses: Access::empty()
                ..(Access::COLOR_ATTACHMENT_READ | Access::COLOR_ATTACHMENT_WRITE),
        };

        device.create_render_pass(&[color_attachment], &[subpass], &[dependency])
    };

    let pipeline_layout = device.create_pipeline_layout(&[], &[]);

    let pipeline = {
        let vs_entry = EntryPoint::<backend::Backend> {
            entry: "main",
            module: &vertex_shader_module,
            specialization: &[],
        };

        let fs_entry = EntryPoint::<backend::Backend> {
            entry: "main",
            module: &fragment_shader_module,
            specialization: &[],
        };

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

        pipeline_desc
            .blender
            .targets
            .push(ColorBlendDesc(ColorMask::ALL, BlendState::ALPHA));

        device
            .create_graphics_pipeline(&pipeline_desc, None)
            .unwrap()
    };

    // We're going to defer the construction of our swapchain, frame images, and
    // framebuffers until the mainloop, because we will need to repeat it whenever
    // the window resizes.
    //
    // For now we leave them empty.
    let mut swapchain_stuff: Option<(_, _, _)> = None;

    loop {
        let mut quitting = false;
        let mut resizing = false;

        events_loop.poll_events(|event| {
            if let Event::WindowEvent { event, .. } = event {
                match event {
                    WindowEvent::CloseRequested => quitting = true,
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => quitting = true,

                    // We need to recreate our swapchain if we resize, so track it.
                    WindowEvent::Resized(_) => {
                        resizing = true;
                    }

                    _ => {}
                }
            }
        });

        // We need to destroy things if we're resizing - because we'll recreate them
        // or if quitting - because we want them destroyed for good.
        if (resizing || quitting) && swapchain_stuff.is_some() {
            // Take ownership over the old stuff so we can destroy it.
            // The value of swapchain_stuff is now `None`.
            let (swapchain, frame_views, framebuffers) = swapchain_stuff.take().unwrap();

            // We want to wait for all queues to be idle and reset the command pool,
            // so that we know that no commands are being executed while we destroy
            // the swapchain.
            device.wait_idle().unwrap();
            command_pool.reset();

            // Destroy all the old stuff.
            for framebuffer in framebuffers {
                device.destroy_framebuffer(framebuffer);
            }

            for image_view in frame_views {
                device.destroy_image_view(image_view);
            }
            device.destroy_swapchain(swapchain);
        }

        if quitting {
            break;
        }

        let window_size: (u32, u32) = window
            .get_inner_size()
            .unwrap()
            .to_physical(window.get_hidpi_factor())
            .into();

        // If we don't have a swapchain here, we destroyed it and we need to
        // recreate it.
        if swapchain_stuff.is_none() {
            // On the currently tested version of gfx-hal, you need to recreate
            // the surface. I'm unsure if this is a bug or not.
            surface = instance.create_surface(&window);

            // Here we just create the swapchain, frame images, and framebuffers
            // like we did in part 00, and store them in swapchain_stuff.
            let (width, height) = window_size;
            let (swapchain, backbuffer) = {
                let extent = { Extent2D { width, height } };

                let swap_config = SwapchainConfig::new()
                    .with_color(surface_color_format)
                    .with_image_usage(image::Usage::COLOR_ATTACHMENT);

                device.create_swapchain(&mut surface, swap_config, None, &extent)
            };

            let (frame_views, framebuffers) = match backbuffer {
                Backbuffer::Images(images) => {
                    let extent = Extent {
                        width,
                        height,
                        depth: 1,
                    };

                    let color_range = SubresourceRange {
                        aspects: Aspects::COLOR,
                        levels: 0..1,
                        layers: 0..1,
                    };

                    let image_views = images
                        .into_iter()
                        .map(|image| {
                            device
                                .create_image_view(
                                    &image,
                                    ViewKind::D2,
                                    surface_color_format,
                                    Swizzle::NO,
                                    color_range.clone(),
                                )
                                .unwrap()
                        })
                        .collect::<Vec<_>>();

                    let fbos = image_views
                        .iter()
                        .map(|image_view| {
                            device
                                .create_framebuffer(&render_pass, vec![image_view], extent)
                                .unwrap()
                        })
                        .collect();

                    (image_views, fbos)
                }
                Backbuffer::Framebuffer(fbo) => (Vec::new(), vec![fbo]),
            };

            // Store the new stuff.
            swapchain_stuff = Some((swapchain, frame_views, framebuffers));
        }

        // To access the swapchain, we need to get a mutable reference to the
        // contents of swapchain_stuff.
        let (swapchain, _frame_views, framebuffers) = swapchain_stuff.as_mut().unwrap();

        device.reset_fence(&frame_fence);
        command_pool.reset();

        let frame_index: SwapImageIndex = swapchain
            .acquire_image(FrameSync::Semaphore(&frame_semaphore))
            .expect("Failed to acquire frame");

        let finished_command_buffer = {
            let mut command_buffer = command_pool.acquire_command_buffer(false);

            let (width, height) = window_size;
            let viewport = Viewport {
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: width as i16,
                    h: height as i16,
                },
                depth: 0.0..1.0,
            };

            command_buffer.set_viewports(0, &[viewport.clone()]);
            command_buffer.set_scissors(0, &[viewport.rect]);

            command_buffer.bind_graphics_pipeline(&pipeline);

            {
                let mut encoder = command_buffer.begin_render_pass_inline(
                    &render_pass,
                    &framebuffers[frame_index as usize],
                    viewport.rect,
                    &[ClearValue::Color(ClearColor::Float([0.0, 0.0, 0.0, 1.0]))],
                );

                encoder.draw(0..3, 0..1);
            }

            command_buffer.finish()
        };

        let submission = Submission::new()
            .wait_on(&[(&frame_semaphore, PipelineStage::BOTTOM_OF_PIPE)])
            .submit(Some(finished_command_buffer));

        queue_group.queues[0].submit(submission, Some(&frame_fence));

        device.wait_for_fence(&frame_fence, !0);

        swapchain
            .present(&mut queue_group.queues[0], frame_index, &[])
            .expect("Present failed");
    }

    // Cleanup
    // Note that we don't have to destroy the swapchain, frame images, or
    // framebuffers, because they will already have been destroyed before breaking
    // out of the mainloop.
    device.destroy_graphics_pipeline(pipeline);
    device.destroy_pipeline_layout(pipeline_layout);
    device.destroy_render_pass(render_pass);
    device.destroy_shader_module(vertex_shader_module);
    device.destroy_shader_module(fragment_shader_module);
    device.destroy_command_pool(command_pool.into_raw());
    device.destroy_fence(frame_fence);
    device.destroy_semaphore(frame_semaphore);
}
