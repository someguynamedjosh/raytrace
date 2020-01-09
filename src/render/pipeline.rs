use ash::version::DeviceV1_0;
use ash::vk;

use std::ffi::CString;

use super::core::Core;

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
    test_stage: Stage,
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    descriptor_pool: vk::DescriptorPool,
    frame_available_semaphore: vk::Semaphore,
    frame_complete_semaphore: vk::Semaphore,
    frame_complete_fence: vk::Fence,
    descriptor_set_layouts: DescriptorSetLayouts,
    descriptor_sets: DescriptorSets,
}

impl Pipeline {
    pub fn new(core: &Core) -> Pipeline {
        let (frame_available_semaphore, frame_complete_semaphore) = create_semaphores(core);
        let (frame_complete_fence,) = create_fences(core);
        let swapchain_size = core.swapchain_info.swapchain_images.len();
        let command_pool = create_command_pool(core);
        let command_buffers = create_command_buffers(core, command_pool);

        let descriptor_pool = create_descriptor_pool(core, swapchain_size as u32);
        let (descriptor_set_layouts, descriptor_sets) =
            create_descriptor_sets(core, descriptor_pool);

        let test_stage = create_test_stage(core, &descriptor_set_layouts);

        let mut pipeline = Pipeline {
            test_stage,
            command_pool,
            command_buffers,
            descriptor_pool,
            frame_available_semaphore,
            frame_complete_semaphore,
            frame_complete_fence,
            descriptor_set_layouts,
            descriptor_sets,
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
            let set = self.descriptor_sets.swapchain_outputs[index];
            let layout = self.test_stage.pipeline_layout;
            cmd_bind_descriptor_set(core, buffer, set, layout);
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

        self.descriptor_set_layouts.destroy(core);
        self.test_stage.destroy(core);
        core.device
            .destroy_descriptor_pool(self.descriptor_pool, None);
        core.device.destroy_command_pool(self.command_pool, None);

        core.device.destroy_fence(self.frame_complete_fence, None);
        core.device
            .destroy_semaphore(self.frame_available_semaphore, None);
        core.device
            .destroy_semaphore(self.frame_complete_semaphore, None);
    }
}

struct DescriptorSetLayouts {
    swapchain_output: vk::DescriptorSetLayout,
}

impl DescriptorSetLayouts {
    fn destroy(&mut self, core: &Core) {
        unsafe {
            core.device
                .destroy_descriptor_set_layout(self.swapchain_output, None);
        }
    }
}

struct DescriptorSets {
    swapchain_outputs: Vec<vk::DescriptorSet>,
}

fn create_descriptor_sets(
    core: &Core,
    pool: vk::DescriptorPool,
) -> (DescriptorSetLayouts, DescriptorSets) {
    let swapchain_outputs = create_swapchain_output_descriptor_sets(core, pool);

    (
        DescriptorSetLayouts {
            swapchain_output: swapchain_outputs.0,
        },
        DescriptorSets {
            swapchain_outputs: swapchain_outputs.1,
        },
    )
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

    (unsafe {
        core.device
            .create_fence(&create_info, None)
            .expect("Failed to create fence.")
    },)
}

fn create_command_pool(core: &Core) -> vk::CommandPool {
    let create_info = vk::CommandPoolCreateInfo {
        queue_family_index: core.queue_family_indices.compute.unwrap(),
        ..Default::default()
    };
    unsafe {
        core.device
            .create_command_pool(&create_info, None)
            .expect("Failed to create command pool.")
    }
}

fn create_command_buffers(core: &Core, command_pool: vk::CommandPool) -> Vec<vk::CommandBuffer> {
    let allocate_info = vk::CommandBufferAllocateInfo {
        command_buffer_count: core.swapchain_info.swapchain_images.len() as u32,
        command_pool,
        level: vk::CommandBufferLevel::PRIMARY,
        ..Default::default()
    };
    unsafe {
        core.device
            .allocate_command_buffers(&allocate_info)
            .expect("Failed to allocate command buffers.")
    }
}

fn create_descriptor_pool(core: &Core, num_swapchain_images: u32) -> vk::DescriptorPool {
    let pool_size = vk::DescriptorPoolSize {
        ty: vk::DescriptorType::STORAGE_IMAGE,
        descriptor_count: num_swapchain_images,
        ..Default::default()
    };
    let create_info = vk::DescriptorPoolCreateInfo {
        pool_size_count: 1,
        p_pool_sizes: &pool_size,
        max_sets: num_swapchain_images,
        ..Default::default()
    };
    unsafe {
        core.device
            .create_descriptor_pool(&create_info, None)
            .expect("Failed to create descriptor pool.")
    }
}

fn create_descriptor_set_layout(
    core: &Core,
    bindings: &[vk::DescriptorSetLayoutBinding],
    num_bindings: u32,
) -> vk::DescriptorSetLayout {
    let descriptor_set_layout_create_info = vk::DescriptorSetLayoutCreateInfo {
        binding_count: num_bindings,
        p_bindings: bindings.as_ptr(),
        ..Default::default()
    };
    unsafe {
        core.device
            .create_descriptor_set_layout(&descriptor_set_layout_create_info, None)
            .expect("Failed to create descriptor set layout.")
    }
}

fn create_swapchain_output_descriptor_sets(
    core: &Core,
    pool: vk::DescriptorPool,
) -> (vk::DescriptorSetLayout, Vec<vk::DescriptorSet>) {
    let output_layout_binding = vk::DescriptorSetLayoutBinding {
        binding: 0,
        descriptor_type: vk::DescriptorType::STORAGE_IMAGE,
        descriptor_count: 1,
        stage_flags: vk::ShaderStageFlags::COMPUTE,
        ..Default::default()
    };

    let bindings = [output_layout_binding];
    let layout = create_descriptor_set_layout(core, &bindings, bindings.len() as u32);

    let quantity = core.swapchain_info.swapchain_images.len();
    let mut layouts = vec![];
    for _ in 0..quantity {
        layouts.push(layout);
    }
    let layouts = layouts;
    let allocate_info = vk::DescriptorSetAllocateInfo {
        descriptor_pool: pool,
        descriptor_set_count: quantity as u32,
        p_set_layouts: layouts.as_ptr(),
        ..Default::default()
    };
    let descriptor_sets = unsafe {
        core.device
            .allocate_descriptor_sets(&allocate_info)
            .expect("Failed to create descriptor sets.")
    };

    let mut image_infos = vec![];
    for index in 0..quantity {
        image_infos.push(vk::DescriptorImageInfo {
            image_view: core.swapchain_info.swapchain_image_views[index as usize],
            // TODO: figure this out.
            image_layout: vk::ImageLayout::GENERAL,
            ..Default::default()
        });
    }
    let mut writes = vec![];
    for (index, set) in descriptor_sets.iter().enumerate() {
        writes.push(vk::WriteDescriptorSet {
            dst_set: *set,
            dst_binding: 0,
            descriptor_count: 1,
            descriptor_type: vk::DescriptorType::STORAGE_IMAGE,
            p_image_info: &image_infos[index],
            ..Default::default()
        });
    }

    unsafe {
        core.device.update_descriptor_sets(&writes, &[]);
    }

    (layout, descriptor_sets)
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
    _core: &Core,
    module: vk::ShaderModule,
    entry_point: &CString,
) -> vk::PipelineShaderStageCreateInfo {
    vk::PipelineShaderStageCreateInfo {
        module,
        p_name: entry_point.as_ptr(),
        stage: vk::ShaderStageFlags::COMPUTE,
        ..Default::default()
    }
}

fn create_test_stage(core: &Core, layouts: &DescriptorSetLayouts) -> Stage {
    let shader_source = include_bytes!("../../shaders/spirv/test.comp.spirv");
    let shader_module = create_shader_module(core, shader_source.as_ptr(), shader_source.len());

    let entry_point = CString::new("main").unwrap();
    let shader_stage = create_compute_shader_stage(core, shader_module, &entry_point);

    let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo {
        set_layout_count: 1,
        p_set_layouts: &layouts.swapchain_output,
        ..Default::default()
    };
    let pipeline_layout = unsafe {
        core.device
            .create_pipeline_layout(&pipeline_layout_create_info, None)
            .expect("Failed to create pipeline layout.")
    };

    let compute_pipeline_create_info = vk::ComputePipelineCreateInfo {
        stage: shader_stage,
        layout: pipeline_layout,
        ..Default::default()
    };

    let compute_pipeline = unsafe {
        core.device
            .create_compute_pipelines(
                vk::PipelineCache::null(),
                &[compute_pipeline_create_info],
                None,
            )
            .expect("Failed to create compute pipeline.")[0]
    };

    unsafe {
        core.device.destroy_shader_module(shader_module, None);
    }

    Stage {
        vk_pipeline: compute_pipeline,
        pipeline_layout,
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
) {
    unsafe {
        core.device.cmd_bind_descriptor_sets(
            buffer,
            vk::PipelineBindPoint::COMPUTE,
            pipeline_layout,
            0,
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
