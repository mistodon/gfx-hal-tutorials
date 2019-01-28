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
//
// One thing to note is that the Rust compiler reserves the right to lay out your
// structs any way it likes in memory. This is bad for us, since we need to tell
// our pipeline the layout of our vertices. By adding #[repr(C)] we can guarantee
// that they'll be laid out the same way as they would in C, making it at least
// deterministic.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
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
            specialization: Default::default(),
        };

        let fs_entry = EntryPoint::<backend::Backend> {
            entry: "main",
            module: &fragment_shader_module,
            specialization: Default::default(),
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

        // We need to let our pipeline know about all the different formats of
        // vertex buffer we're going to use. The `binding` number is an ID for
        // this entry. The `stride` how the size of one element (vertex) in bytes.
        // The `rate` is used for instanced rendering, so we'll ignore it for now.
        pipeline_desc.vertex_buffers.push(VertexBufferDesc {
            binding: 0,
            stride: std::mem::size_of::<Vertex>() as u32,
            rate: 0,
        });

        // We have to declare our two vertex attributes: position and color.
        // Note that their locations have to match the locations in the shader, and
        // their format has to be appropriate for the data type in the shader:
        //
        // vec3 = Rgb32Float (three 32-bit floats)
        // vec4 = Rgba32Float (four 32-bit floats)
        //
        // Additionally, the second attribute must have an offset of 12 bytes in the
        // vertex, because this is the size of the first field. The `binding`
        // parameter refers back to the ID we gave in VertexBufferDesc.
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

    // Your graphics card provides different types of memory, with some being more
    // efficient for certain tasks. For example, CPU_VISIBLE memory will allow you
    // to write to it directly but be slow to use for rendering, while DEVICE_LOCAL
    // memory is fast for rendering, but you'll require a staging buffer to copy data
    // into it.
    //
    // We get a list of the available memory types here so we can choose one later.
    let memory_types = physical_device.memory_properties().memory_types;

    // Here we're going to optionally load the teapot mesh, if a command line arg
    // is passed in. You can ignore this and just use MESH if you want, but teapots
    // are traditional after all.
    let teapot = load_teapot_mesh();
    let mesh = if std::env::args().nth(1) == Some("teapot".into()) {
        &teapot
    } else {
        MESH
    };

    // Here's where we create the buffer itself, and the memory to hold it. There's
    // a lot in here, and in future parts we'll extract it to a utility function.
    let (vertex_buffer, vertex_buffer_memory) = {
        // First we create an unbound buffer (e.g, a buffer not currently bound to
        // any memory). We need to work out the size of it in bytes, and declare
        // that we want to use it for vertex data.
        let item_count = mesh.len();
        let stride = std::mem::size_of::<Vertex>() as u64;
        let buffer_len = item_count as u64 * stride;
        let unbound_buffer = device
            .create_buffer(buffer_len, buffer::Usage::VERTEX)
            .unwrap();

        // Next, we need the graphics card to tell us what the memory requirements
        // for this buffer are. This includes the size, alignment, and available
        // memory types. We know how big our data is, but we have to store it in
        // a valid way for the device.
        let req = device.get_buffer_requirements(&unbound_buffer);

        // This complicated looking statement filters through memory types to pick
        // one that's appropriate. We call enumerate to give us the ID (the index)
        // of each type, which might look something like this:
        //
        // id   ty
        // ==   ==
        // 0    DEVICE_LOCAL
        // 1    COHERENT | CPU_VISIBLE
        // 2    DEVICE_LOCAL | CPU_VISIBLE
        // 3    DEVICE_LOCAL | CPU_VISIBLE | CPU_CACHED
        //
        // We then want to find the first type that is supported by our memory
        // requirements (e.g, `id` is in the `type_mask` bitfield), and also has
        // the CPU_VISIBLE property (so we can copy vertex data directly into it.)
        let upload_type = memory_types
            .iter()
            .enumerate()
            .find(|(id, ty)| {
                let type_supported = req.type_mask & (1_u64 << id) != 0;
                type_supported && ty.properties.contains(Properties::CPU_VISIBLE)
            })
            .map(|(id, _ty)| MemoryTypeId(id))
            .expect("Could not find approprate vertex buffer memory type.");

        // Now that we know the type and size of memory we need, we can allocate it
        // and bind out buffer to it. The `0` there is an offset, which you could
        // use to bind multiple buffers to the same block of memory.
        let buffer_memory = device.allocate_memory(upload_type, req.size).unwrap();
        let buffer = device
            .bind_buffer_memory(&buffer_memory, 0, unbound_buffer)
            .unwrap();

        // Finally, we can copy our vertex data into the buffer. To do this we get
        // a writer corresponding to the range of memory we want to write to. This
        // writer essentially memory maps the data and acts as a slice that we can
        // write into. Once we do that, we unmap the memory, and our buffer should
        // now be full.
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
    let present_semaphore = device.create_semaphore();

    let mut swapchain_stuff: Option<(_, _, _, _)> = None;

    let mut rebuild_swapchain = false;

    loop {
        let mut quitting = false;

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
                        rebuild_swapchain = true;
                    }
                    _ => {}
                }
            }
        });

        if (rebuild_swapchain || quitting) && swapchain_stuff.is_some() {
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
            rebuild_swapchain = false;
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

        command_pool.reset();

        let frame_index: SwapImageIndex = {
            match swapchain.acquire_image(!0, FrameSync::Semaphore(&frame_semaphore)) {
                Ok(i) => i,
                Err(_) => {
                    rebuild_swapchain = true;
                    continue;
                }
            }
        };

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

            // This is where we tell our pipeline to use a specific vertex buffer.
            // The first argument again referse to the vertex buffer `binding` as
            // defined above. Next is a vec of buffers to bind. Each is a pair of
            // (buffer, offset) where offset is relative to that `binding` number
            // again. Basically, we only have one vertex buffer, and one type of
            // vertex buffer, so you can ignore the numbers completely for now.
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
            .signal(&[&present_semaphore])
            .submit(vec![finished_command_buffer]);

        queue_group.queues[0].submit(submission, None);

        let result = swapchain.present(
            &mut queue_group.queues[0],
            frame_index,
            vec![&present_semaphore],
        );

        if result.is_err() {
            rebuild_swapchain = true;
        }
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

    device.destroy_semaphore(frame_semaphore);
}

// You can probably ignore this, but it's always nice to have a teapot mesh.
fn load_teapot_mesh() -> Vec<Vertex> {
    let scale = 0.27;
    teapot::TEAPOT_VERTICES
        .chunks(3)
        .map(|position| {
            let x = position[0] * scale;
            let y = position[1] * scale;
            let z = position[2] * scale;
            Vertex {
                position: [x, y + 0.4, z],
                color: [x.abs(), y.abs(), z.abs(), 1.0],
            }
        })
        .collect()
}
