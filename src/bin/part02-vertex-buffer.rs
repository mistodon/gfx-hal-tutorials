extern crate gfx_hal_tutorials;

#[cfg(windows)]
extern crate gfx_backend_dx12 as backend;
#[cfg(target_os = "macos")]
extern crate gfx_backend_metal as backend;
#[cfg(all(unix, not(target_os = "macos")))]
extern crate gfx_backend_vulkan as backend;

extern crate gfx_hal;
extern crate winit;

use gfx_hal_tutorials::prelude::*;
use gfx_hal_tutorials::teapot;
use winit::{Event, EventsLoop, KeyboardInput, VirtualKeyCode, WindowBuilder, WindowEvent};

// To store a mesh in a vertex buffer, we first need a vertex format.
// We're going to include a 3D vertex position, and a per-vertex color attribute.
// TODO: Add big warning about layout
#[derive(Debug, Clone, Copy)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 4],
}

// In a sensible application, we would load our mesh from somewhere, but for this
// example, we'll just store it in a const.
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
        .with_title("Part 02: Vertex buffers")
        .with_dimensions((256, 256).into())
        .build(&events_loop)
        .unwrap();

    let instance = backend::Instance::create("Part 02: Vertex buffers", 1);
    let mut surface = instance.create_surface(&window);
    let mut adapter = instance.enumerate_adapters().remove(0);
    let (device, mut queue_group) = adapter
        .open_with::<_, Graphics>(1, |family| surface.supports_queue_family(family))
        .unwrap();
    let mut command_pool =
        device.create_command_pool_typed(&queue_group, CommandPoolCreateFlags::empty(), 16);

    let physical_device = &adapter.physical_device;

    let (_, formats, _) = surface.compatibility(physical_device);

    let surface_color_format = {
        match formats {
            Some(choices) => choices
                .into_iter()
                .find(|format| format.base_format().1 == ChannelType::Srgb)
                .unwrap(),
            None => Format::Rgba8Srgb,
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

    // We're using new shaders for this tutorial - check out the source in
    // source_assets/shaders/part02.*
    let vertex_shader_module = {
        let spirv = include_bytes!("../../assets/gen/shaders/part02.vert.spv");
        device.create_shader_module(spirv).unwrap()
    };

    let fragment_shader_module = {
        let spirv = include_bytes!("../../assets/gen/shaders/part02.frag.spv");
        device.create_shader_module(spirv).unwrap()
    };

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

        // TODO: I guess this needs a tiny bit of explanation.
        pipeline_desc.vertex_buffers.push(VertexBufferDesc {
            binding: 0,
            stride: std::mem::size_of::<Vertex>() as u32,
            rate: 0,
        });

        // We have to declare our two vertex attributes: position and color.
        // Note that their locations have to match the locations in the shader, and
        // their format has to be appropriate for the data type in the shader.
        // vec3 = Rgb32Float
        // vec4 = Rgba32Float
        //
        // Additionally, the second attribute must have an offset of 12 bytes in the
        // vertex, because this is the size of the first field.
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

    // Now lets create a vertex buffer to upload our mesh data into. We'll need both
    // the buffer, and the memory it's using so we can destroy and deallocate them at
    // the end.
    //
    // TODO: ???
    let memory_types = physical_device.memory_properties().memory_types;

    // Here we're going to optionally load the teapot mesh, if a command line arg
    // is passed in.
    let teapot = load_teapot_mesh();
    let mesh = if std::env::args().nth(1) == Some("teapot".into()) {
        &teapot
    } else {
        MESH
    };

    let (vertex_buffer, vertex_buffer_memory) = {
        // TODO: Explain all of this pish
        let item_count = mesh.len();
        let stride = std::mem::size_of::<Vertex>() as u64;
        let buffer_len = item_count as u64 * stride;
        let unbound_buffer = device
            .create_buffer(buffer_len, buffer::Usage::VERTEX)
            .unwrap();
        let req = device.get_buffer_requirements(&unbound_buffer);
        let upload_type = memory_types
            .iter()
            .enumerate()
            .position(|(id, ty)| {
                req.type_mask & (1 << id) != 0 && ty.properties.contains(Properties::CPU_VISIBLE)
            })
            .unwrap()
            .into();

        let buffer_memory = device.allocate_memory(upload_type, req.size).unwrap();
        let buffer = device
            .bind_buffer_memory(&buffer_memory, 0, unbound_buffer)
            .unwrap();

        // Fill the buffer with vertex data
        {
            let mut dest = device
                .acquire_mapping_writer::<Vertex>(&buffer_memory, 0..buffer_len)
                .unwrap();
            dest.copy_from_slice(mesh);
            device.release_mapping_writer(dest);
        }

        (buffer, buffer_memory)
    };

    let frame_semaphore = device.create_semaphore();
    let frame_fence = device.create_fence(false);

    let mut swapchain_stuff: Option<(_, _, _, _)> = None;

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
            let (swapchain, _extent, frame_views, framebuffers) = swapchain_stuff.take().unwrap();

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

        if swapchain_stuff.is_none() {
            let (caps, _, _) = surface.compatibility(physical_device);

            let swap_config = SwapchainConfig::from_caps(&caps, surface_color_format);
            let extent = swap_config.extent.to_extent();
            let (swapchain, backbuffer) = device.create_swapchain(&mut surface, swap_config, None);

            let (frame_views, framebuffers) = match backbuffer {
                Backbuffer::Images(images) => {
                    let color_range = SubresourceRange {
                        aspects: Aspects::COLOR,
                        levels: 0..1,
                        layers: 0..1,
                    };

                    let image_views = images
                        .iter()
                        .map(|image| {
                            device
                                .create_image_view(
                                    image,
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

            swapchain_stuff = Some((swapchain, extent, frame_views, framebuffers));
        }

        let (swapchain, extent, _frame_views, framebuffers) = swapchain_stuff.as_mut().unwrap();

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
                    w: extent.width as i16,
                    h: extent.height as i16,
                },
                depth: 0.0..1.0,
            };

            command_buffer.set_viewports(0, &[viewport.clone()]);
            command_buffer.set_scissors(0, &[viewport.rect]);

            command_buffer.bind_graphics_pipeline(&pipeline);

            // TODO: explain
            command_buffer.bind_vertex_buffers(0, vec![(&vertex_buffer, 0)]);

            {
                let mut encoder = command_buffer.begin_render_pass_inline(
                    &render_pass,
                    &framebuffers[frame_index as usize],
                    viewport.rect,
                    &[ClearValue::Color(ClearColor::Float([0.0, 0.0, 0.0, 1.0]))],
                );

                // Instead of drawing the vertex range 0..3, we now want to draw
                // however many vertices our mesh has.
                let num_vertices = mesh.len() as u32;
                encoder.draw(0..num_vertices, 0..1);
            }

            command_buffer.finish()
        };

        let submission = Submission::new()
            .wait_on(&[(&frame_semaphore, PipelineStage::BOTTOM_OF_PIPE)])
            .submit(vec![finished_command_buffer]);

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

    // Note that we now have to destroy our vertex buffer and its memory
    device.destroy_buffer(vertex_buffer);
    device.free_memory(vertex_buffer_memory);

    device.destroy_fence(frame_fence);
    device.destroy_semaphore(frame_semaphore);
}

fn load_teapot_mesh() -> Vec<Vertex> {
    let scale = 0.29;
    teapot::TEAPOT_VERTICES
        .iter()
        .map(|position| {
            let [mut x, mut y, mut z] = *position;
            x *= scale;
            y *= scale;
            z *= scale;
            Vertex {
                position: [x, y, z],
                color: [z, z, 0.5, 1.0],
            }
        })
        .collect()
}
