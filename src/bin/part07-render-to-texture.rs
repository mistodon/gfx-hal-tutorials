extern crate gfx_hal_tutorials;

#[cfg(windows)]
extern crate gfx_backend_dx12 as backend;
#[cfg(target_os = "macos")]
extern crate gfx_backend_metal as backend;
#[cfg(all(unix, not(target_os = "macos")))]
extern crate gfx_backend_vulkan as backend;

extern crate gfx_hal;

extern crate image;

extern crate winit;

use gfx_hal_tutorials::prelude::*;
use gfx_hal_tutorials::utils;

use backend::Backend;

use winit::{Event, EventsLoop, KeyboardInput, VirtualKeyCode, WindowBuilder, WindowEvent};

#[derive(Debug, Clone, Copy)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 4],
    uv: [f32; 2],
}

const MESH: &[Vertex] = &[
    Vertex {
        position: [0.0, -1.0, 0.0],
        color: [1.0, 0.0, 0.0, 1.0],
        uv: [1.0, 0.0],
    },
    Vertex {
        position: [-1.0, 0.0, 0.0],
        color: [0.0, 0.0, 1.0, 1.0],
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [0.0, 1.0, 0.0],
        color: [0.0, 1.0, 0.0, 1.0],
        uv: [0.0, 1.0],
    },
    Vertex {
        position: [0.0, -1.0, 0.0],
        color: [1.0, 0.0, 0.0, 1.0],
        uv: [1.0, 0.0],
    },
    Vertex {
        position: [0.0, 1.0, 0.0],
        color: [0.0, 1.0, 0.0, 1.0],
        uv: [0.0, 1.0],
    },
    Vertex {
        position: [1.0, 0.0, 0.0],
        color: [1.0, 1.0, 0.0, 1.0],
        uv: [1.0, 1.0],
    },
];

#[derive(Debug, Clone, Copy)]
struct UniformBlock {
    projection: [[f32; 4]; 4],
}

#[derive(Debug, Clone, Copy)]
struct PushConstants {
    tint: [f32; 4],
    position: [f32; 3],
}

fn main() {
    let mut events_loop = EventsLoop::new();
    let window = WindowBuilder::new()
        .with_title("Part 07: Render-to-texture")
        .with_dimensions((256, 256).into())
        .build(&events_loop)
        .unwrap();

    let instance = backend::Instance::create("Part 07: Render-to-texture", 1);
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

    let depth_format = Format::D32FloatS8Uint;

    let render_pass = {
        let color_attachment = Attachment {
            format: Some(surface_color_format),
            samples: 1,
            ops: AttachmentOps::new(AttachmentLoadOp::Clear, AttachmentStoreOp::Store),
            stencil_ops: AttachmentOps::DONT_CARE,
            layouts: Layout::Undefined..Layout::Present,
        };

        let depth_attachment = Attachment {
            format: Some(depth_format),
            samples: 1,
            ops: AttachmentOps::new(AttachmentLoadOp::Clear, AttachmentStoreOp::DontCare),
            stencil_ops: AttachmentOps::DONT_CARE,
            layouts: Layout::Undefined..Layout::DepthStencilAttachmentOptimal,
        };

        let subpass = SubpassDesc {
            colors: &[(0, Layout::ColorAttachmentOptimal)],
            depth_stencil: Some(&(1, Layout::DepthStencilAttachmentOptimal)),
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

        device.create_render_pass(
            &[color_attachment, depth_attachment],
            &[subpass],
            &[dependency],
        )
    };

    let set_layout = device.create_descriptor_set_layout(
        &[
            DescriptorSetLayoutBinding {
                binding: 0,
                ty: DescriptorType::UniformBuffer,
                count: 1,
                stage_flags: ShaderStageFlags::VERTEX,
                immutable_samplers: false,
            },
            DescriptorSetLayoutBinding {
                binding: 1,
                ty: DescriptorType::SampledImage,
                count: 1,
                stage_flags: ShaderStageFlags::FRAGMENT,
                immutable_samplers: false,
            },
            DescriptorSetLayoutBinding {
                binding: 2,
                ty: DescriptorType::Sampler,
                count: 1,
                stage_flags: ShaderStageFlags::FRAGMENT,
                immutable_samplers: false,
            },
        ],
        &[],
    );

    let num_push_constants = utils::push_constant_size::<PushConstants>() as u32;

    let pipeline_layout = device.create_pipeline_layout(
        vec![&set_layout],
        &[(ShaderStageFlags::VERTEX, 0..num_push_constants)],
    );

    let vertex_shader_module = {
        let spirv = include_bytes!("../../assets/gen/shaders/part06.vert.spv");
        device.create_shader_module(spirv).unwrap()
    };

    let fragment_shader_module = {
        let spirv = include_bytes!("../../assets/gen/shaders/part06.frag.spv");
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

        pipeline_desc.attributes.push(AttributeDesc {
            location: 2,
            binding: 0,
            element: Element {
                format: Format::Rgba32Float,
                offset: 28,
            },
        });

        pipeline_desc.depth_stencil = DepthStencilDesc {
            depth: DepthTest::On {
                fun: Comparison::Less,
                write: true,
            },
            depth_bounds: false,
            stencil: StencilTest::default(),
        };

        device
            .create_graphics_pipeline(&pipeline_desc, None)
            .unwrap()
    };

    let mut desc_pool = device.create_descriptor_pool(
        2,
        &[
            DescriptorRangeDesc {
                ty: DescriptorType::UniformBuffer,
                count: 2,
            },
            DescriptorRangeDesc {
                ty: DescriptorType::SampledImage,
                count: 2,
            },
            DescriptorRangeDesc {
                ty: DescriptorType::Sampler,
                count: 2,
            },
        ],
    );

    let desc_set = desc_pool.allocate_set(&set_layout).unwrap();
    let rtt_desc_set = desc_pool.allocate_set(&set_layout).unwrap();

    let memory_types = physical_device.memory_properties().memory_types;

    let (vertex_buffer, vertex_buffer_memory) = utils::create_buffer::<Backend, Vertex>(
        &device,
        &memory_types,
        Properties::CPU_VISIBLE,
        buffer::Usage::VERTEX,
        MESH,
    );

    let (uniform_buffer, mut uniform_memory) = utils::create_buffer::<Backend, UniformBlock>(
        &device,
        &memory_types,
        Properties::CPU_VISIBLE,
        buffer::Usage::UNIFORM,
        &[UniformBlock {
            projection: Default::default(),
        }],
    );

    // TODO: Explain
    let rtt_semaphore = device.create_semaphore();
    let texture_fence = device.create_fence(false);
    let frame_semaphore = device.create_semaphore();
    let present_semaphore = device.create_semaphore();

    // TODO: Make an empty one of these for our destination texture
    // TODO: Also make a framebuffer binding it and the existing depth texture
    let (rtt_image, rtt_memory, rtt_view, rtt_sampler, rtt_framebuffer, rtt_depth) = {
        let extent = Extent {
            width: 64,
            height: 64,
            depth: 1,
        };

        let (rtt_image, rtt_memory, rtt_view) = utils::create_image::<Backend>(
            &device,
            &memory_types,
            extent.width,
            extent.height,
            Format::Rgba8Srgb,
            img::Usage::SAMPLED,
            Aspects::COLOR,
        );

        let (depth_image, depth_image_memory, depth_image_view) = utils::create_image::<Backend>(
            &device,
            &memory_types,
            extent.width,
            extent.height,
            depth_format,
            img::Usage::DEPTH_STENCIL_ATTACHMENT,
            Aspects::DEPTH | Aspects::STENCIL,
        );

        let rtt_sampler =
            device.create_sampler(img::SamplerInfo::new(Filter::Linear, WrapMode::Clamp));

        let rtt_framebuffer = device
            .create_framebuffer(&render_pass, vec![&rtt_view, &depth_image_view], extent)
            .unwrap();

        (
            rtt_image,
            rtt_memory,
            rtt_view,
            rtt_sampler,
            rtt_framebuffer,
            (depth_image, depth_image_memory, depth_image_view),
        )
    };

    let (texture_image, texture_memory, texture_view, texture_sampler) = {
        let image_bytes = include_bytes!("../../assets/texture.png");
        let img = image::load_from_memory(image_bytes.as_ref())
            .expect("Failed to load image.")
            .to_rgba();
        let (width, height) = img.dimensions();

        let (texture_image, texture_memory, texture_view) = utils::create_image::<Backend>(
            &device,
            &memory_types,
            width,
            height,
            Format::Rgba8Srgb,
            img::Usage::TRANSFER_DST | img::Usage::SAMPLED,
            Aspects::COLOR,
        );

        let texture_sampler =
            device.create_sampler(img::SamplerInfo::new(Filter::Linear, WrapMode::Clamp));

        // TODO: Factor this mess into a utility function or something
        {
            let row_alignment_mask =
                physical_device.limits().min_buffer_copy_pitch_alignment as u32 - 1;
            let image_stride = 4usize;
            let row_pitch =
                (width * image_stride as u32 + row_alignment_mask) & !row_alignment_mask;
            let upload_size = u64::from(height * row_pitch);

            let (image_upload_buffer, mut image_upload_memory) = utils::empty_buffer::<Backend, u8>(
                &device,
                &memory_types,
                Properties::CPU_VISIBLE,
                buffer::Usage::TRANSFER_SRC,
                upload_size as usize,
            );

            {
                let mut data = device
                    .acquire_mapping_writer::<u8>(&image_upload_memory, 0..upload_size)
                    .unwrap();

                for y in 0..height as usize {
                    let row = &(*img)[y * (width as usize) * image_stride
                        ..(y + 1) * (width as usize) * image_stride];
                    let dest_base = y * row_pitch as usize;
                    data[dest_base..dest_base + row.len()].copy_from_slice(row);
                }

                device.release_mapping_writer(data);
            }

            let submit = {
                let mut cmd_buffer = command_pool.acquire_command_buffer(false);

                let image_barrier = Barrier::Image {
                    states: (Access::empty(), Layout::Undefined)
                        ..(Access::TRANSFER_WRITE, Layout::TransferDstOptimal),
                    target: &texture_image,
                    range: SubresourceRange {
                        aspects: Aspects::COLOR,
                        levels: 0..1,
                        layers: 0..1,
                    },
                };

                cmd_buffer.pipeline_barrier(
                    PipelineStage::TOP_OF_PIPE..PipelineStage::TRANSFER,
                    Dependencies::empty(),
                    &[image_barrier],
                );

                cmd_buffer.copy_buffer_to_image(
                    &image_upload_buffer,
                    &texture_image,
                    Layout::TransferDstOptimal,
                    &[BufferImageCopy {
                        buffer_offset: 0,
                        buffer_width: row_pitch / (image_stride as u32),
                        buffer_height: height as u32,
                        image_layers: SubresourceLayers {
                            aspects: Aspects::COLOR,
                            level: 0,
                            layers: 0..1,
                        },
                        image_offset: Offset { x: 0, y: 0, z: 0 },
                        image_extent: Extent {
                            width,
                            height,
                            depth: 1,
                        },
                    }],
                );

                let image_barrier = Barrier::Image {
                    states: (Access::TRANSFER_WRITE, Layout::TransferDstOptimal)
                        ..(Access::SHADER_READ, Layout::ShaderReadOnlyOptimal),
                    target: &texture_image,
                    range: SubresourceRange {
                        aspects: Aspects::COLOR,
                        levels: 0..1,
                        layers: 0..1,
                    },
                };

                cmd_buffer.pipeline_barrier(
                    PipelineStage::TRANSFER..PipelineStage::FRAGMENT_SHADER,
                    Dependencies::empty(),
                    &[image_barrier],
                );

                cmd_buffer.finish()
            };

            let submission = Submission::new().submit(Some(submit));
            queue_group.queues[0].submit(submission, Some(&texture_fence));

            device.destroy_buffer(image_upload_buffer);
            device.free_memory(image_upload_memory);
        }

        (texture_image, texture_memory, texture_view, texture_sampler)
    };

    device.write_descriptor_sets(vec![
        DescriptorSetWrite {
            set: &rtt_desc_set,
            binding: 0,
            array_offset: 0,
            descriptors: Some(Descriptor::Buffer(&uniform_buffer, None..None)),
        },
        DescriptorSetWrite {
            set: &rtt_desc_set,
            binding: 1,
            array_offset: 0,
            descriptors: Some(Descriptor::Image(&texture_view, Layout::Undefined)),
        },
        DescriptorSetWrite {
            set: &rtt_desc_set,
            binding: 2,
            array_offset: 0,
            descriptors: Some(Descriptor::Sampler(&texture_sampler)),
        },
    ]);

    device.write_descriptor_sets(vec![
        DescriptorSetWrite {
            set: &desc_set,
            binding: 0,
            array_offset: 0,
            descriptors: Some(Descriptor::Buffer(&uniform_buffer, None..None)),
        },
        DescriptorSetWrite {
            set: &desc_set,
            binding: 1,
            array_offset: 0,
            descriptors: Some(Descriptor::Image(&rtt_view, Layout::Undefined)),
        },
        DescriptorSetWrite {
            set: &desc_set,
            binding: 2,
            array_offset: 0,
            descriptors: Some(Descriptor::Sampler(&rtt_sampler)),
        },
    ]);

    let diamonds = vec![
        PushConstants {
            tint: [1.0, 1.0, 1.0, 1.0],
            position: [-0.5, 0.0, 0.0],
        },
        PushConstants {
            tint: [0.5, 0.5, 0.5, 1.0],
            position: [0.5, 0.0, 0.1],
        },
    ];

    let mut swapchain_stuff: Option<(_, _, _, _, _, _, _)> = None;

    let mut rebuild_swapchain = false;

    device.wait_for_fence(&texture_fence, !0);

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
            let (
                swapchain,
                _extent,
                frame_views,
                framebuffers,
                depth_image,
                depth_image_view,
                depth_image_memory,
            ) = swapchain_stuff.take().unwrap();

            device.wait_idle().unwrap();
            command_pool.reset();

            for framebuffer in framebuffers {
                device.destroy_framebuffer(framebuffer);
            }

            for image_view in frame_views {
                device.destroy_image_view(image_view);
            }

            device.destroy_image_view(depth_image_view);
            device.destroy_image(depth_image);
            device.free_memory(depth_image_memory);

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

            let (depth_image, depth_image_memory, depth_image_view) = utils::create_image::<Backend>(
                &device,
                &memory_types,
                extent.width as u32,
                extent.height as u32,
                depth_format,
                img::Usage::DEPTH_STENCIL_ATTACHMENT,
                Aspects::DEPTH | Aspects::STENCIL,
            );

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
                                .create_framebuffer(
                                    &render_pass,
                                    vec![image_view, &depth_image_view],
                                    extent,
                                )
                                .unwrap()
                        })
                        .collect();

                    (image_views, fbos)
                }
                Backbuffer::Framebuffer(fbo) => (Vec::new(), vec![fbo]),
            };

            swapchain_stuff = Some((
                swapchain,
                extent,
                frame_views,
                framebuffers,
                depth_image,
                depth_image_view,
                depth_image_memory,
            ));
        }

        let (
            swapchain,
            extent,
            _frame_views,
            framebuffers,
            _depth_image,
            _depth_image_view,
            _depth_image_memory,
        ) = swapchain_stuff.as_mut().unwrap();

        let (width, height) = (extent.width, extent.height);
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

        let offscreen_command_buffer = {
            let mut command_buffer = command_pool.acquire_command_buffer(false);

            let viewport = Viewport {
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: 64,
                    h: 64,
                },
                depth: 0.0..1.0,
            };

            command_buffer.set_viewports(0, &[viewport.clone()]);
            command_buffer.set_scissors(0, &[viewport.rect]);

            command_buffer.bind_graphics_pipeline(&pipeline);
            command_buffer.bind_vertex_buffers(0, vec![(&vertex_buffer, 0)]);

            command_buffer.bind_graphics_descriptor_sets(
                &pipeline_layout,
                0,
                vec![&rtt_desc_set],
                &[],
            );

            {
                let mut encoder = command_buffer.begin_render_pass_inline(
                    &render_pass,
                    &rtt_framebuffer,
                    viewport.rect,
                    &[
                        ClearValue::Color(ClearColor::Float([0.0, 0.0, 0.5, 1.0])),
                        ClearValue::DepthStencil(ClearDepthStencil(1.0, 0)),
                    ],
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
            .signal(&[&rtt_semaphore])
            .submit(vec![offscreen_command_buffer]);

        queue_group.queues[0].submit(submission, None);

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
            command_buffer.bind_vertex_buffers(0, vec![(&vertex_buffer, 0)]);

            command_buffer.bind_graphics_descriptor_sets(&pipeline_layout, 0, vec![&desc_set], &[]);

            {
                let mut encoder = command_buffer.begin_render_pass_inline(
                    &render_pass,
                    &framebuffers[frame_index as usize],
                    viewport.rect,
                    &[
                        ClearValue::Color(ClearColor::Float([0.0, 0.0, 0.0, 1.0])),
                        ClearValue::DepthStencil(ClearDepthStencil(1.0, 0)),
                    ],
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
            .wait_on(&[(&rtt_semaphore, PipelineStage::BOTTOM_OF_PIPE)])
            .signal(&[&present_semaphore])
            .submit(vec![finished_command_buffer]);

        queue_group.queues[0].submit(submission, None);

        // TODO: In first submission, signal the offscreen semaphore and submit
        // the final image render.

        // TODO: Find out why we're using a fence and if we could use a semaphore.

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
    device.destroy_descriptor_pool(desc_pool);
    device.destroy_image(texture_image);
    device.destroy_image_view(texture_view);
    device.destroy_sampler(texture_sampler);
    device.free_memory(texture_memory);
    device.destroy_descriptor_set_layout(set_layout);
    device.destroy_buffer(uniform_buffer);
    device.free_memory(uniform_memory);
    device.destroy_buffer(vertex_buffer);
    device.free_memory(vertex_buffer_memory);

    device.destroy_semaphore(frame_semaphore);
}
