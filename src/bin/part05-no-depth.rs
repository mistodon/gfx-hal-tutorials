extern crate gfx_hal_tutorials;

extern crate gfx_backend_metal as backend;
extern crate gfx_hal;
extern crate winit;

use gfx_hal_tutorials::prelude::*;
use gfx_hal_tutorials::utils;

use backend::Backend;

use winit::{Event, EventsLoop, KeyboardInput, VirtualKeyCode, WindowBuilder, WindowEvent};

#[derive(Debug, Clone, Copy)]
struct UniformBlock {
    projection: [[f32; 4]; 4],
}

#[derive(Debug, Clone, Copy)]
struct PushConstants {
    tint: [f32; 4],
    position: [f32; 3],
}

#[derive(Debug, Clone, Copy)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 4],
}

const MESH: &[Vertex] = &[
    Vertex {
        position: [0.0, -1.0, 0.0],
        color: [1.0, 0.0, 0.0, 1.0],
    },
    Vertex {
        position: [-1.0, 0.0, 0.0],
        color: [0.0, 0.0, 1.0, 1.0],
    },
    Vertex {
        position: [0.0, 1.0, 0.0],
        color: [0.0, 1.0, 0.0, 1.0],
    },
    Vertex {
        position: [0.0, -1.0, 0.0],
        color: [1.0, 0.0, 0.0, 1.0],
    },
    Vertex {
        position: [0.0, 1.0, 0.0],
        color: [0.0, 1.0, 0.0, 1.0],
    },
    Vertex {
        position: [1.0, 0.0, 0.0],
        color: [1.0, 1.0, 0.0, 1.0],
    },
];

fn main() {
    let mut events_loop = EventsLoop::new();

    let window = WindowBuilder::new()
        .with_title("Part 05: Without depth testing")
        .with_dimensions((256, 256).into())
        .build(&events_loop)
        .unwrap();

    let instance = backend::Instance::create("Part 05: Without depth testing", 1);

    let mut surface = instance.create_surface(&window);

    let mut adapter = instance.enumerate_adapters().remove(0);

    let (device, mut queue_group) = adapter
        .open_with::<_, Graphics>(1, |family| surface.supports_queue_family(family))
        .unwrap();

    let mut command_pool =
        device.create_command_pool_typed(&queue_group, CommandPoolCreateFlags::empty(), 16);

    let vertex_shader_module = {
        let spirv = include_bytes!("../../assets/gen/shaders/part04.vert.spv");
        device.create_shader_module(spirv).unwrap()
    };

    let fragment_shader_module = {
        let spirv = include_bytes!("../../assets/gen/shaders/part04.frag.spv");
        device.create_shader_module(spirv).unwrap()
    };

    let memory_types = adapter.physical_device.memory_properties().memory_types;

    let set_layout = device.create_descriptor_set_layout(
        &[DescriptorSetLayoutBinding {
            binding: 0,
            ty: DescriptorType::UniformBuffer,
            count: 1,
            stage_flags: ShaderStageFlags::VERTEX,
            immutable_samplers: false,
        }],
        &[],
    );

    let mut desc_pool = device.create_descriptor_pool(
        1,
        &[DescriptorRangeDesc {
            ty: DescriptorType::UniformBuffer,
            count: 1,
        }],
    );

    let desc_set = desc_pool.allocate_set(&set_layout).unwrap();

    let (uniform_buffer, mut uniform_memory) = utils::create_buffer::<Backend, UniformBlock>(
        &device,
        &memory_types,
        Properties::CPU_VISIBLE,
        buffer::Usage::UNIFORM,
        &[UniformBlock {
            projection: Default::default(),
        }],
    );

    device.write_descriptor_sets(vec![DescriptorSetWrite {
        set: &desc_set,
        binding: 0,
        array_offset: 0,
        descriptors: Some(Descriptor::Buffer(&uniform_buffer, None..None)),
    }]);

    let (vertex_buffer, vertex_buffer_memory) = utils::create_buffer::<Backend, Vertex>(
        &device,
        &memory_types,
        Properties::CPU_VISIBLE,
        buffer::Usage::VERTEX,
        MESH,
    );

    let frame_semaphore = device.create_semaphore();
    let frame_fence = device.create_fence(false);

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

    let num_push_constants = utils::push_constant_size::<PushConstants>() as u32;

    let pipeline_layout = device.create_pipeline_layout(
        vec![&set_layout],
        &[(ShaderStageFlags::VERTEX, 0..num_push_constants)],
    );

    // These shapes are not in order of depth, so without depth testing they
    // will draw in an irregular order.
    let diamonds = vec![
        PushConstants {
            tint: [0.6, 0.6, 0.6, 1.0],
            position: [-0.1, 0.0, 0.3],
        },
        PushConstants {
            tint: [0.2, 0.2, 0.2, 1.0],
            position: [0.3, 0.8, 0.5],
        },
        PushConstants {
            tint: [0.4, 0.4, 0.4, 1.0],
            position: [0.1, 0.4, 0.4],
        },
        PushConstants {
            tint: [1.0, 1.0, 1.0, 1.0],
            position: [-0.5, -0.8, 0.1],
        },
        PushConstants {
            tint: [0.8, 0.8, 0.8, 1.0],
            position: [-0.3, -0.4, 0.2],
        },
    ];

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

        pipeline_desc.vertex_buffers.push(VertexBufferDesc {
            binding: 0,
            stride: std::mem::size_of::<Vertex>() as u32,
            rate: 0,
        });

        pipeline_desc.attributes.push(AttributeDesc {
            location: 0,
            binding: 0,
            element: Element {
                format: Format::Rgb32Float,
                offset: 0,
            },
        });

        pipeline_desc.attributes.push(AttributeDesc {
            location: 1,
            binding: 0,
            element: Element {
                format: Format::Rgba32Float,
                offset: 12,
            },
        });

        device
            .create_graphics_pipeline(&pipeline_desc, None)
            .unwrap()
    };

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
                    WindowEvent::Resized(_) => {
                        resizing = true;
                    }
                    _ => {}
                }
            }
        });

        if (resizing || quitting) && swapchain_stuff.is_some() {
            let (swapchain, frame_views, framebuffers) = swapchain_stuff.take().unwrap();

            device.wait_idle().unwrap();
            command_pool.reset();

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

        if swapchain_stuff.is_none() {
            surface = instance.create_surface(&window);

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

            swapchain_stuff = Some((swapchain, frame_views, framebuffers));
        }

        let (width, height) = window_size;
        let aspect_corrected_x = height as f32 / width as f32;
        let zoom = 0.5;
        let x_scale = aspect_corrected_x * zoom;
        let y_scale = zoom;

        utils::fill_buffer::<Backend, UniformBlock>(
            &device,
            &mut uniform_memory,
            &[UniformBlock {
                projection: [
                    [x_scale, 0.0, 0.0, 0.0],
                    [0.0, y_scale, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0],
                ],
            }],
        );

        let (swapchain, _frame_views, framebuffers) = swapchain_stuff.as_mut().unwrap();

        device.reset_fence(&frame_fence);
        command_pool.reset();

        let frame_index: SwapImageIndex = swapchain
            .acquire_image(FrameSync::Semaphore(&frame_semaphore))
            .expect("Failed to acquire frame");

        let finished_command_buffer = {
            let mut command_buffer = command_pool.acquire_command_buffer(false);

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
            command_buffer.bind_vertex_buffers(0, vec![(&vertex_buffer, 0)]);

            command_buffer.bind_graphics_descriptor_sets(&pipeline_layout, 0, vec![&desc_set], &[]);

            {
                let mut encoder = command_buffer.begin_render_pass_inline(
                    &render_pass,
                    &framebuffers[frame_index as usize],
                    viewport.rect,
                    &[ClearValue::Color(ClearColor::Float([0.0, 0.0, 0.0, 1.0]))],
                );

                let num_vertices = MESH.len() as u32;

                for diamond in &diamonds {
                    encoder.push_graphics_constants(
                        &pipeline_layout,
                        ShaderStageFlags::VERTEX,
                        0,
                        utils::push_constant_data(diamond),
                    );

                    encoder.draw(0..num_vertices, 0..1);
                }
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
    device.destroy_graphics_pipeline(pipeline);
    device.destroy_pipeline_layout(pipeline_layout);
    device.destroy_render_pass(render_pass);
    device.destroy_shader_module(vertex_shader_module);
    device.destroy_shader_module(fragment_shader_module);
    device.destroy_command_pool(command_pool.into_raw());
    device.destroy_descriptor_pool(desc_pool);
    device.destroy_descriptor_set_layout(set_layout);
    device.destroy_buffer(uniform_buffer);
    device.free_memory(uniform_memory);
    device.destroy_buffer(vertex_buffer);
    device.free_memory(vertex_buffer_memory);
    device.destroy_fence(frame_fence);
    device.destroy_semaphore(frame_semaphore);
}
