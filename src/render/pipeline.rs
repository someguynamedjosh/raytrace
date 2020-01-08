use ash::version::DeviceV1_0;
use ash::vk;

use std::ffi::CString;

use super::constants::*;
use super::core::Core;

struct Stage {
    vk_pipeline: vk::Pipeline,
    descriptor_set_layout: vk::DescriptorSetLayout,
    pipeline_layout: vk::PipelineLayout,
}

pub struct Pipeline {
    test_stage: Stage,
    _command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    descriptor_pool: vk::DescriptorPool,
    frame_available_semaphores: Vec<vk::Semaphore>,
    frame_complete_semaphores: Vec<vk::Semaphore>,
    frame_complete_fences: Vec<vk::Fence>,
    swapchain_image_available_fences: Vec<Option<vk::Fence>>,
    current_frame: usize,
    descriptor_sets: Vec<vk::DescriptorSet>,
}

impl Pipeline {
    pub fn new(core: &Core) -> Pipeline {
        let (frame_available_semaphores, frame_complete_semaphores) = create_semaphores(core);
        let frame_complete_fences = create_fences(core);
        let swapchain_size = core.swapchain_info.swapchain_images.len();
        let swapchain_image_available_fences = (0..swapchain_size).map(|_| None).collect();
        let test_stage = create_test_stage(core);
        let descriptor_pool = create_descriptor_pool(core, swapchain_size as u32);
        let descriptor_sets = create_descriptor_sets(core, descriptor_pool, swapchain_size as u32);
        let command_pool = create_command_pool(core);
        let command_buffers =
            create_command_buffers(core, command_pool, &test_stage, &descriptor_sets);
        Pipeline {
            test_stage,
            _command_pool: command_pool,
            command_buffers,
            descriptor_pool,
            frame_available_semaphores,
            frame_complete_semaphores,
            frame_complete_fences,
            swapchain_image_available_fences,
            current_frame: 0,
            descriptor_sets,
        }
    }

    pub fn draw_frame(&mut self, core: &Core) {
        unsafe {
            core.device
                .wait_for_fences(
                    &[self.frame_complete_fences[self.current_frame]],
                    true,
                    std::u64::MAX,
                )
                .expect("Failed to wait for previous frame to finish rendering.");
        }

        let (image_index, _is_suboptimal) = unsafe {
            core.swapchain_info
                .swapchain_loader
                .acquire_next_image(
                    core.swapchain_info.swapchain,
                    std::u64::MAX,
                    self.frame_available_semaphores[self.current_frame],
                    vk::Fence::null(),
                )
                .expect("Failed to acquire next swapchain image.")
        };

        if let Some(fence) = self.swapchain_image_available_fences[image_index as usize] {
            unsafe {
                core.device
                    .wait_for_fences(&[fence], true, std::u64::MAX)
                    .expect("Failed to wait for swapchain image to finish being used.");
            }
        }
        self.swapchain_image_available_fences[image_index as usize] =
            Some(self.frame_complete_fences[self.current_frame]);
        let wait_semaphores = [self.frame_available_semaphores[self.current_frame]];
        let signal_semaphores = [self.frame_complete_semaphores[self.current_frame]];
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
            let wait_fence = self.frame_complete_fences[self.current_frame];
            core.device
                .reset_fences(&[wait_fence])
                .expect("Failed to reset fence.");
            core.device
                .queue_submit(core.compute_queue, &[submit_info], wait_fence)
                .expect("Failed to submit command queue.");
        }

        let wait_semaphores = [self.frame_complete_semaphores[self.current_frame]];
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

        self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
    }
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

fn create_command_buffers(
    core: &Core,
    command_pool: vk::CommandPool,
    stage: &Stage,
    descriptor_sets: &[vk::DescriptorSet],
) -> Vec<vk::CommandBuffer> {
    let allocate_info = vk::CommandBufferAllocateInfo {
        command_buffer_count: core.swapchain_info.swapchain_images.len() as u32,
        command_pool,
        level: vk::CommandBufferLevel::PRIMARY,
        ..Default::default()
    };
    let buffers = unsafe {
        core.device
            .allocate_command_buffers(&allocate_info)
            .expect("Failed to allocate command buffers.")
    };

    let mut transition_frame_to_general = vk::ImageMemoryBarrier {
        old_layout: vk::ImageLayout::UNDEFINED,
        new_layout: vk::ImageLayout::GENERAL,
        src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
        dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
        image: vk::Image::null(),
        subresource_range: vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        },
        ..Default::default()
    };
    let mut transition_frame_to_present = vk::ImageMemoryBarrier {
        old_layout: vk::ImageLayout::GENERAL,
        new_layout: vk::ImageLayout::PRESENT_SRC_KHR,
        ..transition_frame_to_general
    };

    unsafe {
        for (index, buffer) in buffers.iter().enumerate() {
            let swapchain_image = core.swapchain_info.swapchain_images[index];
            transition_frame_to_general.image = swapchain_image;
            transition_frame_to_present.image = swapchain_image;
            let begin_info = vk::CommandBufferBeginInfo {
                ..Default::default()
            };
            core.device
                .begin_command_buffer(*buffer, &begin_info)
                .expect("Failed to start command buffer.");
            core.device.cmd_pipeline_barrier(
                *buffer,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                Default::default(),
                &[],
                &[],
                &[transition_frame_to_general],
            );
            core.device.cmd_bind_descriptor_sets(
                *buffer,
                vk::PipelineBindPoint::COMPUTE,
                stage.pipeline_layout,
                0,
                &[descriptor_sets[index]],
                &[],
            );
            core.device.cmd_bind_pipeline(
                *buffer,
                vk::PipelineBindPoint::COMPUTE,
                stage.vk_pipeline,
            );
            core.device.cmd_dispatch(*buffer, 30, 30, 1);
            core.device.cmd_pipeline_barrier(
                *buffer,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                Default::default(),
                &[],
                &[],
                &[transition_frame_to_present],
            );
            core.device
                .end_command_buffer(*buffer)
                .expect("Failed to end command buffer.");
        }
    }

    buffers
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

fn create_descriptor_sets(
    core: &Core,
    pool: vk::DescriptorPool,
    quantity: u32,
) -> Vec<vk::DescriptorSet> {
    let bindings = [vk::DescriptorSetLayoutBinding {
        binding: 0,
        descriptor_type: vk::DescriptorType::STORAGE_IMAGE,
        descriptor_count: 1,
        stage_flags: vk::ShaderStageFlags::COMPUTE,
        ..Default::default()
    }];
    let layout_create_info = vk::DescriptorSetLayoutCreateInfo {
        binding_count: 1,
        p_bindings: bindings.as_ptr(),
        ..Default::default()
    };
    let layout = unsafe {
        core.device
            .create_descriptor_set_layout(&layout_create_info, None)
            .expect("Failed to create descriptor layout.")
    };

    let layouts = [layout; 3];
    let allocate_info = vk::DescriptorSetAllocateInfo {
        descriptor_pool: pool,
        descriptor_set_count: quantity,
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

    descriptor_sets
}

fn create_semaphores(core: &Core) -> (Vec<vk::Semaphore>, Vec<vk::Semaphore>) {
    let mut a_semaphores = vec![];
    let mut b_semaphores = vec![];

    let create_info = Default::default();

    for _ in 0..MAX_FRAMES_IN_FLIGHT {
        a_semaphores.push(unsafe {
            core.device
                .create_semaphore(&create_info, None)
                .expect("Failed to create semaphore.")
        });
        b_semaphores.push(unsafe {
            core.device
                .create_semaphore(&create_info, None)
                .expect("Failed to create semaphore.")
        });
    }

    (a_semaphores, b_semaphores)
}

fn create_fences(core: &Core) -> Vec<vk::Fence> {
    let mut fences = vec![];

    let create_info = vk::FenceCreateInfo {
        // Start the fences signalled so we don't wait on the first couple of frames.
        flags: vk::FenceCreateFlags::SIGNALED,
        ..Default::default()
    };

    for _ in 0..MAX_FRAMES_IN_FLIGHT {
        fences.push(unsafe {
            core.device
                .create_fence(&create_info, None)
                .expect("Failed to create semaphore.")
        });
    }

    fences
}

fn create_test_stage(core: &Core) -> Stage {
    let shader_source = include_bytes!("../../shaders/spirv/test.comp.spirv");
    let shader_module = create_shader_module(core, shader_source.as_ptr(), shader_source.len());

    let entry_point = CString::new("main").unwrap();
    let shader_stage = create_compute_shader_stage(core, shader_module, &entry_point);

    let output_layout_binding = vk::DescriptorSetLayoutBinding {
        binding: 0,
        descriptor_type: vk::DescriptorType::STORAGE_IMAGE,
        descriptor_count: 1,
        stage_flags: vk::ShaderStageFlags::COMPUTE,
        ..Default::default()
    };

    let bindings = [output_layout_binding];
    let descriptor_set_layout =
        create_descriptor_set_layout(core, &bindings, bindings.len() as u32);

    let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo {
        set_layout_count: 1,
        p_set_layouts: &descriptor_set_layout,
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
        descriptor_set_layout,
        pipeline_layout,
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
