extern crate gfx_backend_metal as backend;
extern crate gfx_hal;
extern crate winit;

use gfx_hal::{
    command::{ClearColor, ClearValue}, format::{Aspects, ChannelType, Format, Swizzle},
    image::{self, Access, Extent, Layout, SubresourceRange, ViewKind},
    pass::{
        Attachment, AttachmentLoadOp, AttachmentOps, AttachmentStoreOp, Subpass, SubpassDependency,
        SubpassDesc, SubpassRef,
    },
    pool::CommandPoolCreateFlags,
    pso::{
        BlendState, ColorBlendDesc, ColorMask, EntryPoint, GraphicsPipelineDesc, GraphicsShaderSet,
        PipelineStage, Rasterizer, Rect, Viewport,
    },
    queue::Submission, window::Extent2D, Backbuffer, Device, FrameSync, Graphics, Instance,
    Primitive, Surface, SwapImageIndex, Swapchain, SwapchainConfig,
};

use winit::{Event, EventsLoop, KeyboardInput, VirtualKeyCode, WindowBuilder, WindowEvent};

fn main() {
    // Create a window with winit
    let mut events_loop = EventsLoop::new();

    let window = WindowBuilder::new()
        .with_title("Part 00: Triangle")
        .with_dimensions(512, 512)
        .build(&events_loop)
        .unwrap();

    // Initialize our long-lived graphics state.
    // We expect these to live for the whole duration of our program.
    let instance = backend::Instance::create("Part 00: Triangle", 1);

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

    let mut frame_semaphore = device.create_semaphore();
    let mut frame_fence = device.create_fence(false);

    // Initialize our swapchain, pipeline, framebuffers, etc.
    // We expect to have to rebuild these when the window is resized -
    // however we're going to ignore that for this example.
    let surface_color_format = {
        let physical_device = &adapter.physical_device;
        let (_, formats, _) = surface.compatibility(physical_device);

        let format = match formats {
            None => Format::Rgba8Srgb,
            Some(options) => options
                .into_iter()
                .find(|format| format.base_format().1 == ChannelType::Srgb)
                .unwrap(),
        };

        format
    };

    let window_size = window.get_inner_size().unwrap();
    let viewport = Viewport {
        rect: Rect {
            x: 0,
            y: 0,
            w: window_size.0 as u16,
            h: window_size.1 as u16,
        },
        depth: 0.0..1.0,
    };

    let (mut swapchain, backbuffer) = {
        let extent = {
            let (width, height) = window_size;
            Extent2D { width, height }
        };

        let swap_config = SwapchainConfig::new()
            .with_color(surface_color_format)
            .with_image_usage(image::Usage::COLOR_ATTACHMENT);

        device.create_swapchain(&mut surface, swap_config, None, &extent)
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

    let (frame_images, framebuffers) = match backbuffer {
        Backbuffer::Images(images) => {
            let (width, height) = window_size;
            let extent = Extent {
                width: width,
                height: height,
                depth: 1,
            };

            let color_range = SubresourceRange {
                aspects: Aspects::COLOR,
                levels: 0..1,
                layers: 0..1,
            };

            let image_view_pairs = images
                .into_iter()
                .map(|image| {
                    let image_view = device
                        .create_image_view(
                            &image,
                            ViewKind::D2,
                            surface_color_format,
                            Swizzle::NO,
                            color_range.clone(),
                        )
                        .unwrap();
                    (image, image_view)
                })
                .collect::<Vec<_>>();

            let fbos = image_view_pairs
                .iter()
                .map(|&(_, ref image_view)| {
                    device
                        .create_framebuffer(&render_pass, Some(image_view), extent)
                        .unwrap()
                })
                .collect();

            (image_view_pairs, fbos)
        }
        Backbuffer::Framebuffer(fbo) => (Vec::new(), vec![fbo]),
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

        device.create_graphics_pipeline(&pipeline_desc).unwrap()
    };

    // Mainloop starts here
    loop {
        let mut quitting = false;

        // If the window is closed, or Escape is pressed, quit
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
                    _ => {}
                }
            }
        });

        if quitting {
            break;
        }

        // Start rendering
        device.reset_fence(&frame_fence);
        command_pool.reset();

        let frame_index: SwapImageIndex = swapchain
            .acquire_image(FrameSync::Semaphore(&mut frame_semaphore))
            .expect("Failed to acquire frame");

        let finished_command_buffer = {
            let mut command_buffer = command_pool.acquire_command_buffer(false);
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

        queue_group.queues[0].submit(submission, Some(&mut frame_fence));

        device.wait_for_fence(&frame_fence, !0);

        swapchain
            .present(&mut queue_group.queues[0], frame_index, &[])
            .expect("Present failed");
    }

    // Cleanup
    device.destroy_graphics_pipeline(pipeline);
    device.destroy_pipeline_layout(pipeline_layout);

    for framebuffer in framebuffers {
        device.destroy_framebuffer(framebuffer);
    }

    for (_, image_view) in frame_images {
        device.destroy_image_view(image_view);
    }

    device.destroy_render_pass(render_pass);
    device.destroy_swapchain(swapchain);

    device.destroy_shader_module(vertex_shader_module);
    device.destroy_shader_module(fragment_shader_module);
    device.destroy_command_pool(command_pool.into_raw());
    device.destroy_fence(frame_fence);
    device.destroy_semaphore(frame_semaphore);
}
