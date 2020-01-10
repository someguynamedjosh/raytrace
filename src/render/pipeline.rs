use ash::version::DeviceV1_0;
use ash::vk;

use std::ffi::CString;
use std::ops::Deref;

use image::GenericImageView;

use super::constants::*;
use super::core::Core;
use super::descriptors::{self, DescriptorData, DescriptorPrototype, PrototypeGenerator};

struct Stage {
    vk_pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
}

impl Stage {
    unsafe fn destroy(&mut self, core: &Core) {
        core.device
            .destroy_pipeline_layout(self.pipeline_layout, None);
        core.device.destroy_pipeline(self.vk_pipeline, None);
    }
}

pub struct Pipeline {
    command_buffers: Vec<vk::CommandBuffer>,
    frame_available_semaphore: vk::Semaphore,
    frame_complete_semaphore: vk::Semaphore,
    frame_complete_fence: vk::Fence,
    render_data: RenderData,
    descriptor_collection: DescriptorCollection,
    test_stage: Stage,
}

impl Pipeline {
    pub fn new(core: &Core) -> Pipeline {
        let (frame_available_semaphore, frame_complete_semaphore) = create_semaphores(core);
        let (frame_complete_fence,) = create_fences(core);
        let command_buffers = create_command_buffers(core);

        let render_data = RenderData::create(core);
        let descriptor_collection = DescriptorCollection::create(core, &render_data);

        let test_stage = create_test_stage(core, &descriptor_collection);

        let mut pipeline = Pipeline {
            command_buffers,
            frame_available_semaphore,
            frame_complete_semaphore,
            frame_complete_fence,
            render_data,
            descriptor_collection,
            test_stage,
        };
        pipeline.record_command_buffers(core);
        pipeline
    }

    fn record_command_buffers(&mut self, core: &Core) {
        for (index, buffer) in self.command_buffers.iter().enumerate() {
            let swapchain_image = core.swapchain_info.swapchain_images[index];
            let buffer = *buffer;
            cmd_begin(core, buffer);
            cmd_transition_layout(
                core,
                buffer,
                swapchain_image,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::GENERAL,
            );
            let layout = self.test_stage.pipeline_layout;
            let set = self.descriptor_collection.test_data.variants[0];
            cmd_bind_descriptor_set(core, buffer, set, layout, 0);
            let set = self.descriptor_collection.swapchain.variants[index];
            cmd_bind_descriptor_set(core, buffer, set, layout, 1);
            cmd_bind_pipeline(core, buffer, self.test_stage.vk_pipeline);
            unsafe {
                core.device.cmd_dispatch(buffer, 30, 30, 1);
            }
            cmd_transition_layout(
                core,
                buffer,
                swapchain_image,
                vk::ImageLayout::GENERAL,
                vk::ImageLayout::PRESENT_SRC_KHR,
            );
            cmd_end(core, buffer);
        }
    }

    pub fn draw_frame(&mut self, core: &Core) {
        let (image_index, _is_suboptimal) = unsafe {
            core.swapchain_info
                .swapchain_loader
                .acquire_next_image(
                    core.swapchain_info.swapchain,
                    std::u64::MAX,
                    self.frame_available_semaphore,
                    vk::Fence::null(),
                )
                .expect("Failed to acquire next swapchain image.")
        };

        let wait_semaphores = [self.frame_available_semaphore];
        let signal_semaphores = [self.frame_complete_semaphore];
        let wait_stage_mask = [vk::PipelineStageFlags::ALL_COMMANDS];
        let submit_info = vk::SubmitInfo {
            wait_semaphore_count: 1,
            p_wait_semaphores: wait_semaphores.as_ptr(),
            p_wait_dst_stage_mask: wait_stage_mask.as_ptr(),
            command_buffer_count: 1,
            p_command_buffers: &self.command_buffers[image_index as usize],
            signal_semaphore_count: 1,
            p_signal_semaphores: signal_semaphores.as_ptr(),
            ..Default::default()
        };

        unsafe {
            let wait_fence = self.frame_complete_fence;
            core.device
                .wait_for_fences(&[wait_fence], true, std::u64::MAX)
                .expect("Failed to wait for previous frame to finish rendering.");
            core.device
                .reset_fences(&[wait_fence])
                .expect("Failed to reset fence.");
            core.device
                .queue_submit(core.compute_queue, &[submit_info], wait_fence)
                .expect("Failed to submit command queue.");
        }

        let wait_semaphores = [self.frame_complete_semaphore];
        let swapchains = [core.swapchain_info.swapchain];
        let present_info = vk::PresentInfoKHR {
            wait_semaphore_count: 1,
            p_wait_semaphores: wait_semaphores.as_ptr(),
            swapchain_count: 1,
            p_swapchains: swapchains.as_ptr(),
            p_image_indices: &image_index,
            ..Default::default()
        };

        unsafe {
            core.swapchain_info
                .swapchain_loader
                .queue_present(core.present_queue, &present_info)
                .expect("Failed to present swapchain image.");
        }
    }

    pub unsafe fn destroy(&mut self, core: &Core) {
        core.device
            .device_wait_idle()
            .expect("Failed to wait for device to finish rendering.");

        self.test_stage.destroy(core);

        self.descriptor_collection.destroy(core);
        self.render_data.destroy(core);

        core.device.destroy_fence(self.frame_complete_fence, None);
        core.device
            .destroy_semaphore(self.frame_available_semaphore, None);
        core.device
            .destroy_semaphore(self.frame_complete_semaphore, None);
    }
}

struct DescriptorCollection {
    pool: vk::DescriptorPool,
    swapchain: DescriptorData,
    test_data: DescriptorData,
}

impl DescriptorCollection {
    fn create(core: &Core, render_data: &RenderData) -> DescriptorCollection {
        let generators = [
            Box::new(generate_test_data_descriptor_prototypes) as PrototypeGenerator<RenderData>,
            Box::new(generate_swapchain_descriptor_prototypes) as PrototypeGenerator<RenderData>,
        ];
        let names = ["test_data", "swapchain"];
        let (pool, datas) =
            descriptors::generate_descriptor_pool(&generators, &names, core, render_data);
        let mut datas_consumer = datas.into_iter();
        DescriptorCollection {
            pool,
            test_data: datas_consumer.next().unwrap(),
            swapchain: datas_consumer.next().unwrap(),
        }
    }

    fn destroy(&mut self, core: &Core) {
        unsafe {
            core.device
                .destroy_descriptor_set_layout(self.swapchain.layout, None);
            core.device
                .destroy_descriptor_set_layout(self.test_data.layout, None);
            core.device.destroy_descriptor_pool(self.pool, None);
        }
    }
}

fn generate_swapchain_descriptor_prototypes(
    core: &Core,
    _render_data: &RenderData,
) -> Vec<Vec<DescriptorPrototype>> {
    let views = &core.swapchain_info.swapchain_image_views;
    views
        .iter()
        .map(|image_view| {
            vec![DescriptorPrototype::StorageImage(
                *image_view,
                vk::ImageLayout::GENERAL,
            )]
        })
        .collect()
}

fn generate_test_data_descriptor_prototypes(
    _core: &Core,
    render_data: &RenderData,
) -> Vec<Vec<DescriptorPrototype>> {
    let blue_noise = &render_data.blue_noise;
    vec![vec![DescriptorPrototype::CombinedImageSampler(
        blue_noise.image_view,
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        blue_noise.sampler,
    )]]
}

struct Buffer {
    native: vk::Buffer,
    memory: vk::DeviceMemory,
    size: u64,
}

impl Buffer {
    fn create(core: &Core, name: &str, size: u64, usage: vk::BufferUsageFlags) -> Buffer {
        let create_info = vk::BufferCreateInfo {
            size,
            usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };
        let buffer = unsafe {
            core.device
                .create_buffer(&create_info, None)
                .expect("Failed to create buffer.")
        };
        core.set_debug_name(buffer, name);

        let memory_requirements = unsafe { core.device.get_buffer_memory_requirements(buffer) };
        let memory_allocation_info = vk::MemoryAllocateInfo {
            allocation_size: memory_requirements.size,
            memory_type_index: core.find_compatible_memory_type(
                memory_requirements.memory_type_bits,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            ),
            ..Default::default()
        };
        let memory = unsafe {
            core.device
                .allocate_memory(&memory_allocation_info, None)
                .expect("Failed to allocate memory for buffer.")
        };
        unsafe {
            core.device
                .bind_buffer_memory(buffer, memory, 0)
                .expect("Failed to bind buffer to device memory.");
        }
        core.set_debug_name(memory, &format!("{}_memory", name));

        Buffer {
            native: buffer,
            memory,
            size,
        }
    }

    unsafe fn bind_all<PtrType>(&mut self, core: &Core) -> *mut PtrType {
        core.device
            .map_memory(self.memory, 0, self.size, Default::default())
            .expect("Failed to bind memory.") as *mut PtrType
    }

    unsafe fn unbind(&mut self, core: &Core) {
        core.device.unmap_memory(self.memory)
    }

    fn destroy(&mut self, core: &Core) {
        unsafe {
            core.device.destroy_buffer(self.native, None);
            core.device.free_memory(self.memory, None);
        }
    }
}

impl Deref for Buffer {
    type Target = vk::Buffer;

    fn deref(&self) -> &vk::Buffer {
        &self.native
    }
}

struct Image {
    native: vk::Image,
    memory: vk::DeviceMemory,
}

impl Image {
    fn create(
        core: &Core,
        name: &str,
        typ: vk::ImageType,
        extent: vk::Extent3D,
        format: vk::Format,
    ) -> Image {
        let create_info = vk::ImageCreateInfo {
            image_type: typ,
            extent,
            format,
            samples: vk::SampleCountFlags::TYPE_1,
            mip_levels: 1,
            array_layers: 1,
            // TODO: Better usage.
            usage: vk::ImageUsageFlags::TRANSFER_DST,
            tiling: vk::ImageTiling::OPTIMAL,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };
        let image = unsafe {
            core.device
                .create_image(&create_info, None)
                .expect("Failed to create buffer.")
        };
        core.set_debug_name(image, name);

        let memory_requirements = unsafe { core.device.get_image_memory_requirements(image) };
        let memory_allocation_info = vk::MemoryAllocateInfo {
            allocation_size: memory_requirements.size,
            memory_type_index: core.find_compatible_memory_type(
                memory_requirements.memory_type_bits,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            ),
            ..Default::default()
        };
        let memory = unsafe {
            core.device
                .allocate_memory(&memory_allocation_info, None)
                .expect("Failed to allocate memory for image.")
        };
        core.set_debug_name(memory, &format!("{}_memory", name));
        unsafe {
            core.device
                .bind_image_memory(image, memory, 0)
                .expect("Failed to bind image to device memory.");
        }

        Image {
            native: image,
            memory,
        }
    }

    fn destroy(&mut self, core: &Core) {
        unsafe {
            core.device.destroy_image(self.native, None);
            core.device.free_memory(self.memory, None);
        }
    }
}

impl Deref for Image {
    type Target = vk::Image;

    fn deref(&self) -> &vk::Image {
        &self.native
    }
}

struct SampledImage {
    image: vk::Image,
    image_view: vk::ImageView,
    sampler: vk::Sampler,
    memory: vk::DeviceMemory,
    extent: vk::Extent3D,
}

impl SampledImage {
    fn create(
        core: &Core,
        name: &str,
        typ: vk::ImageType,
        extent: vk::Extent3D,
        format: vk::Format,
    ) -> SampledImage {
        let create_info = vk::ImageCreateInfo {
            image_type: typ,
            extent,
            format,
            samples: vk::SampleCountFlags::TYPE_1,
            mip_levels: 1,
            array_layers: 1,
            // TODO: Better usage.
            usage: vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            tiling: vk::ImageTiling::OPTIMAL,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };
        let image = unsafe {
            core.device
                .create_image(&create_info, None)
                .expect("Failed to create buffer.")
        };
        core.set_debug_name(image, &format!("{}", name));

        let memory_requirements = unsafe { core.device.get_image_memory_requirements(image) };
        let memory_allocation_info = vk::MemoryAllocateInfo {
            allocation_size: memory_requirements.size,
            memory_type_index: core.find_compatible_memory_type(
                memory_requirements.memory_type_bits,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            ),
            ..Default::default()
        };
        let memory = unsafe {
            core.device
                .allocate_memory(&memory_allocation_info, None)
                .expect("Failed to allocate memory for image.")
        };
        core.set_debug_name(memory, &format!("{}_memory", name));
        unsafe {
            core.device
                .bind_image_memory(image, memory, 0)
                .expect("Failed to bind image to device memory.");
        }

        let image_view_create_info = vk::ImageViewCreateInfo {
            image,
            view_type: match typ {
                vk::ImageType::TYPE_1D => vk::ImageViewType::TYPE_1D,
                vk::ImageType::TYPE_2D => vk::ImageViewType::TYPE_2D,
                vk::ImageType::TYPE_3D => vk::ImageViewType::TYPE_3D,
                _ => unreachable!("Encountered unspecified ImageType."),
            },
            format,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            ..Default::default()
        };
        let image_view = unsafe {
            core.device
                .create_image_view(&image_view_create_info, None)
                .expect("Failed to create image view for sampled image.")
        };
        core.set_debug_name(image_view, &format!("{}_view", name));

        let sampler_create_info = vk::SamplerCreateInfo {
            mag_filter: vk::Filter::NEAREST,
            min_filter: vk::Filter::NEAREST,
            address_mode_u: vk::SamplerAddressMode::CLAMP_TO_BORDER,
            address_mode_v: vk::SamplerAddressMode::CLAMP_TO_BORDER,
            address_mode_w: vk::SamplerAddressMode::CLAMP_TO_BORDER,
            border_color: vk::BorderColor::FLOAT_OPAQUE_BLACK,
            unnormalized_coordinates: vk::TRUE, // Make coords in the range 0-(width) instead of 0-1
            compare_enable: vk::FALSE,
            ..Default::default()
        };
        let sampler = unsafe {
            core.device
                .create_sampler(&sampler_create_info, None)
                .expect("Failed to create sampler for sampled image.")
        };
        core.set_debug_name(sampler, &format!("{}_sampler", name));

        SampledImage {
            image,
            image_view,
            sampler,
            memory,
            extent,
        }
    }

    fn load_from_png(&mut self, core: &Core, bytes: &[u8]) {
        let size = self.extent.width * self.extent.height * self.extent.depth * 4;
        let data = image::load_from_memory_with_format(bytes, image::ImageFormat::PNG)
            .expect("Failed to decode PNG data.");
        let mut buffer = Buffer::create(
            core,
            "texture_upload_buffer",
            size as u64,
            vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST,
        );
        unsafe {
            let buffer_ptr = buffer.bind_all::<u8>(core);
            for (index, pixel) in data.pixels().enumerate() {
                // RGBA
                *buffer_ptr.offset(index as isize * 4 + 0) = (pixel.2).0[0];
                *buffer_ptr.offset(index as isize * 4 + 1) = (pixel.2).0[1];
                *buffer_ptr.offset(index as isize * 4 + 2) = (pixel.2).0[2];
                *buffer_ptr.offset(index as isize * 4 + 3) = (pixel.2).0[3];
            }
            buffer.unbind(core);
        }
        let upload_commands = create_command_buffer(core, "texture_upload_queue");
        cmd_begin_one_time_submit(core, upload_commands);
        cmd_transition_layout(
            core,
            upload_commands,
            self.image,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        );
        cmd_copy_buffer_to_image(
            core,
            upload_commands,
            buffer.native,
            self.image,
            self.extent,
        );
        cmd_transition_layout(
            core,
            upload_commands,
            self.image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );
        cmd_end(core, upload_commands);
        execute_and_destroy_buffer(core, upload_commands);
        buffer.destroy(core);
    }

    fn destroy(&mut self, core: &Core) {
        unsafe {
            core.device.destroy_sampler(self.sampler, None);
            core.device.destroy_image_view(self.image_view, None);
            core.device.destroy_image(self.image, None);
            core.device.free_memory(self.memory, None);
        }
    }
}

impl Deref for SampledImage {
    type Target = vk::Sampler;

    fn deref(&self) -> &vk::Sampler {
        &self.sampler
    }
}

struct RenderData {
    upload_buffers: Vec<Buffer>,
    upload_destinations: Vec<u16>,
    block_data_atlas: Image,

    chunk_map: Image,
    region_map: Image,

    lighting_buffer: Image,
    depth_buffer: Image,
    normal_buffer: Image,
    old_lighting_buffer: Image,
    old_depth_buffer: Image,
    old_normal_buffer: Image,

    lighting_pong_buffer: Image,
    albedo_buffer: Image,
    emission_buffer: Image,
    fog_color_buffer: Image,

    blue_noise: SampledImage,
}

impl RenderData {
    fn make_framebuffer(core: &Core, name: &str, format: vk::Format) -> Image {
        let dimensions = core.swapchain_info.swapchain_extent;
        Image::create(
            core,
            name,
            vk::ImageType::TYPE_2D,
            vk::Extent3D {
                width: dimensions.width,
                height: dimensions.height,
                depth: 1,
            },
            format,
        )
    }

    fn create(core: &Core) -> RenderData {
        let rgba16_unorm = vk::Format::R16G16B16A16_UNORM;
        let rgba8_unorm = vk::Format::R8G8B8A8_UINT;
        let r16_uint = vk::Format::R16_UINT;
        let r8_uint = vk::Format::R8_UINT;
        RenderData {
            upload_buffers: (0..NUM_UPLOAD_BUFFERS)
                .map(|index| {
                    Buffer::create(
                        core,
                        &format!("upload_buffer_{}", index),
                        CHUNK_BLOCK_VOLUME as u64,
                        vk::BufferUsageFlags::TRANSFER_SRC,
                    )
                })
                .collect(),
            upload_destinations: vec![],
            block_data_atlas: Image::create(
                core,
                "block_data_atlas",
                vk::ImageType::TYPE_3D,
                vk::Extent3D {
                    width: ATLAS_BLOCK_WIDTH,
                    height: ATLAS_BLOCK_WIDTH,
                    depth: ATLAS_BLOCK_WIDTH,
                },
                vk::Format::R16_UINT,
            ),

            chunk_map: Image::create(
                core,
                "chunk_map",
                vk::ImageType::TYPE_3D,
                vk::Extent3D {
                    width: ROOT_CHUNK_WIDTH,
                    height: ROOT_CHUNK_WIDTH,
                    depth: ROOT_CHUNK_WIDTH,
                },
                vk::Format::R16_UINT,
            ),
            region_map: Image::create(
                core,
                "region_map",
                vk::ImageType::TYPE_3D,
                vk::Extent3D {
                    width: ROOT_REGION_WIDTH,
                    height: ROOT_REGION_WIDTH,
                    depth: ROOT_REGION_WIDTH,
                },
                vk::Format::R16_UINT,
            ),

            lighting_buffer: Self::make_framebuffer(core, "lighting_buf", rgba16_unorm),
            depth_buffer: Self::make_framebuffer(core, "depth_buf", r16_uint),
            normal_buffer: Self::make_framebuffer(core, "normal_buf", r8_uint),
            old_lighting_buffer: Self::make_framebuffer(core, "old_lighting_buf", rgba16_unorm),
            old_depth_buffer: Self::make_framebuffer(core, "old_depth_buf", r16_uint),
            old_normal_buffer: Self::make_framebuffer(core, "old_normal_buf", r8_uint),

            lighting_pong_buffer: Self::make_framebuffer(core, "lighting_pong_buf", rgba16_unorm),
            albedo_buffer: Self::make_framebuffer(core, "albedo_buf", rgba8_unorm),
            emission_buffer: Self::make_framebuffer(core, "emission_buf", rgba8_unorm),
            fog_color_buffer: Self::make_framebuffer(core, "fog_color_buf", rgba8_unorm),

            blue_noise: {
                let mut tex = SampledImage::create(
                    core,
                    "blue_noise",
                    vk::ImageType::TYPE_2D,
                    vk::Extent3D {
                        width: BLUE_NOISE_WIDTH,
                        height: BLUE_NOISE_HEIGHT,
                        depth: 1,
                    },
                    vk::Format::R8G8B8A8_UNORM,
                );
                tex.load_from_png(core, include_bytes!("blue_noise_512.png"));
                tex
            },
        }
    }

    fn destroy(&mut self, core: &Core) {
        for upload_buffer in &mut self.upload_buffers {
            upload_buffer.destroy(core);
        }
        self.block_data_atlas.destroy(core);

        self.chunk_map.destroy(core);
        self.region_map.destroy(core);

        self.lighting_buffer.destroy(core);
        self.depth_buffer.destroy(core);
        self.normal_buffer.destroy(core);
        self.old_lighting_buffer.destroy(core);
        self.old_depth_buffer.destroy(core);
        self.old_normal_buffer.destroy(core);

        self.lighting_pong_buffer.destroy(core);
        self.albedo_buffer.destroy(core);
        self.emission_buffer.destroy(core);
        self.fog_color_buffer.destroy(core);

        self.blue_noise.destroy(core);
    }
}

fn create_semaphores(core: &Core) -> (vk::Semaphore, vk::Semaphore) {
    let create_info = Default::default();

    (
        unsafe {
            core.device
                .create_semaphore(&create_info, None)
                .expect("Failed to create semaphore.")
        },
        unsafe {
            core.device
                .create_semaphore(&create_info, None)
                .expect("Failed to create semaphore.")
        },
    )
}

fn create_fences(core: &Core) -> (vk::Fence,) {
    let create_info = vk::FenceCreateInfo {
        // Start the fences signalled so we don't wait on the first couple of frames.
        flags: vk::FenceCreateFlags::SIGNALED,
        ..Default::default()
    };

    let fence = unsafe {
        core.device
            .create_fence(&create_info, None)
            .expect("Failed to create fence.")
    };
    core.set_debug_name(fence, "wait_for_frame_end");
    (fence,)
}

fn create_command_buffer(core: &Core, name: &str) -> vk::CommandBuffer {
    let allocate_info = vk::CommandBufferAllocateInfo {
        command_buffer_count: 1,
        command_pool: core.command_pool,
        level: vk::CommandBufferLevel::PRIMARY,
        ..Default::default()
    };
    let command_buffer = unsafe {
        core.device
            .allocate_command_buffers(&allocate_info)
            .expect("Failed to allocate single-use command buffer.")[0]
    };
    core.set_debug_name(command_buffer, name);
    command_buffer
}

fn create_command_buffers(core: &Core) -> Vec<vk::CommandBuffer> {
    // TODO: debug names.
    let allocate_info = vk::CommandBufferAllocateInfo {
        command_buffer_count: core.swapchain_info.swapchain_images.len() as u32,
        command_pool: core.command_pool,
        level: vk::CommandBufferLevel::PRIMARY,
        ..Default::default()
    };
    unsafe {
        core.device
            .allocate_command_buffers(&allocate_info)
            .expect("Failed to allocate command buffers.")
    }
}

fn create_shader_module(core: &Core, shader_source: *const u8, length: usize) -> vk::ShaderModule {
    let shader_module_create_info = vk::ShaderModuleCreateInfo {
        code_size: length,
        p_code: shader_source as *const u32,
        ..Default::default()
    };
    unsafe {
        core.device
            .create_shader_module(&shader_module_create_info, None)
            .expect("Failed to create shader module.")
    }
}

fn create_compute_shader_stage(
    core: &Core,
    name: &str,
    shader_source: &[u8],
    entry_point: &str,
    descriptor_set_layouts: &[vk::DescriptorSetLayout],
) -> Stage {
    let shader_module = create_shader_module(core, shader_source.as_ptr(), shader_source.len());
    let entry_point_cstring = CString::new(entry_point).unwrap();
    let vk_stage = vk::PipelineShaderStageCreateInfo {
        module: shader_module,
        p_name: entry_point_cstring.as_ptr(),
        stage: vk::ShaderStageFlags::COMPUTE,
        ..Default::default()
    };

    let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo {
        set_layout_count: descriptor_set_layouts.len() as u32,
        p_set_layouts: descriptor_set_layouts.as_ptr(),
        ..Default::default()
    };
    let pipeline_layout = unsafe {
        core.device
            .create_pipeline_layout(&pipeline_layout_create_info, None)
            .expect("Failed to create pipeline layout.")
    };
    core.set_debug_name(pipeline_layout, &format!("{}_layout", name));

    let pipeline_create_info = vk::ComputePipelineCreateInfo {
        stage: vk_stage,
        layout: pipeline_layout,
        ..Default::default()
    };
    let pipeline = unsafe {
        core.device
            .create_compute_pipelines(vk::PipelineCache::null(), &[pipeline_create_info], None)
            .expect("Failed to create compute pipeline.")[0]
    };
    core.set_debug_name(pipeline, name);

    unsafe {
        core.device.destroy_shader_module(shader_module, None);
    }
    Stage {
        vk_pipeline: pipeline,
        pipeline_layout,
    }
}

fn create_test_stage(core: &Core, dc: &DescriptorCollection) -> Stage {
    let shader_source = include_bytes!("../../shaders/spirv/test.comp.spirv");
    create_compute_shader_stage(
        core,
        "test_stage",
        shader_source,
        "main",
        &[dc.test_data.layout, dc.swapchain.layout],
    )
}

fn execute_and_destroy_buffer(core: &Core, buffer: vk::CommandBuffer) {
    let submit_info = vk::SubmitInfo {
        command_buffer_count: 1,
        p_command_buffers: &buffer,
        ..Default::default()
    };

    unsafe {
        core.device
            .queue_submit(core.compute_queue, &[submit_info], vk::Fence::null())
            .expect("Failed to submit command queue.");
        core.device
            .queue_wait_idle(core.compute_queue)
            .expect("Failed to wait for queue completion.");
        core.device
            .free_command_buffers(core.command_pool, &[buffer]);
    }
}

fn cmd_begin(core: &Core, buffer: vk::CommandBuffer) {
    let begin_info = vk::CommandBufferBeginInfo {
        ..Default::default()
    };
    unsafe {
        core.device
            .begin_command_buffer(buffer, &begin_info)
            .expect("Failed to begin command buffer.");
    }
}

fn cmd_begin_one_time_submit(core: &Core, buffer: vk::CommandBuffer) {
    let begin_info = vk::CommandBufferBeginInfo {
        flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
        ..Default::default()
    };
    unsafe {
        core.device
            .begin_command_buffer(buffer, &begin_info)
            .expect("Failed to begin command buffer.");
    }
}

fn cmd_end(core: &Core, buffer: vk::CommandBuffer) {
    unsafe {
        core.device
            .end_command_buffer(buffer)
            .expect("Failed to end command buffer.");
    }
}

fn cmd_transition_layout(
    core: &Core,
    buffer: vk::CommandBuffer,
    image: vk::Image,
    from: vk::ImageLayout,
    to: vk::ImageLayout,
) {
    let image_barrier = vk::ImageMemoryBarrier {
        old_layout: from,
        new_layout: to,
        src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
        dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
        image,
        subresource_range: vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        },
        ..Default::default()
    };
    unsafe {
        core.device.cmd_pipeline_barrier(
            buffer,
            vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            Default::default(),
            &[],
            &[],
            &[image_barrier],
        );
    }
}

fn cmd_bind_descriptor_set(
    core: &Core,
    buffer: vk::CommandBuffer,
    descriptor_set: vk::DescriptorSet,
    pipeline_layout: vk::PipelineLayout,
    index: u32,
) {
    unsafe {
        core.device.cmd_bind_descriptor_sets(
            buffer,
            vk::PipelineBindPoint::COMPUTE,
            pipeline_layout,
            index,
            &[descriptor_set],
            &[],
        );
    }
}

fn cmd_bind_pipeline(core: &Core, buffer: vk::CommandBuffer, pipeline: vk::Pipeline) {
    unsafe {
        core.device
            .cmd_bind_pipeline(buffer, vk::PipelineBindPoint::COMPUTE, pipeline);
    }
}

fn cmd_copy_buffer_to_image(
    core: &Core,
    buffer: vk::CommandBuffer,
    data_buffer: vk::Buffer,
    image: vk::Image,
    extent: vk::Extent3D,
) {
    let copy_info = vk::BufferImageCopy {
        buffer_offset: 0,
        buffer_row_length: 0,
        buffer_image_height: 0,

        image_subresource: vk::ImageSubresourceLayers {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            mip_level: 0,
            base_array_layer: 0,
            layer_count: 1,
        },

        image_extent: extent,
        ..Default::default()
    };

    unsafe {
        core.device.cmd_copy_buffer_to_image(
            buffer,
            data_buffer,
            image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &[copy_info],
        );
    }
}
