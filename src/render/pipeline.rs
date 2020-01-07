use ash::version::DeviceV1_0;
use ash::vk;

use super::constants::*;
use super::core::Core;

pub struct Pipeline {
    _command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    frame_available_semaphores: Vec<vk::Semaphore>,
    frame_complete_semaphores: Vec<vk::Semaphore>,
    frame_complete_fences: Vec<vk::Fence>,
    swapchain_image_available_fences: Vec<Option<vk::Fence>>,
    current_frame: usize,
}

impl Pipeline {
    pub fn new(core: &Core) -> Pipeline {
        let command_pool = create_command_pool(core);
        let command_buffers = create_command_buffers(core, command_pool);
        let (frame_available_semaphores, frame_complete_semaphores) = create_semaphores(core);
        let frame_complete_fences = create_fences(core);
        let swapchain_size = core.swapchain_info.swapchain_images.len();
        let swapchain_image_available_fences = (0..swapchain_size).map(|_| None).collect();
        Pipeline {
            _command_pool: command_pool,
            command_buffers,
            frame_available_semaphores,
            frame_complete_semaphores,
            frame_complete_fences,
            swapchain_image_available_fences,
            current_frame: 0,
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
