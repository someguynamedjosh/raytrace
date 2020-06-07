use super::descriptor_sets::DescriptorCollection;
use super::render_data::RenderData;
use super::shaders::{self, Stage};
use super::structs::DenoisePushData;
use super::TerrainUploadManager;
use crate::game::Game;
use crate::render::constants::*;
use crate::render::general::command_buffer::CommandBuffer;
use crate::render::general::core::Core;
use crate::util::{self, prelude::*};
use ash::version::DeviceV1_0;
use ash::vk;
use cgmath::{Matrix3, SquareMatrix};
use std::rc::Rc;

pub struct Pipeline {
    core: Rc<Core>,

    x_shader_groups: u32,
    y_shader_groups: u32,

    command_buffers: Vec<CommandBuffer>,
    frame_available_semaphore: vk::Semaphore,
    frame_complete_semaphore: vk::Semaphore,
    frame_complete_fence: vk::Fence,
    render_data: RenderData,
    descriptor_collection: DescriptorCollection,
    tum: TerrainUploadManager,

    denoise_stage: Stage,
    finalize_stage: Stage,
    raytrace_stage: Stage,
}

impl Pipeline {
    pub fn new(core: Rc<Core>, game: &mut Game) -> Pipeline {
        let frame_available_semaphore = core.create_semaphore("frame_available");
        let frame_complete_semaphore = core.create_semaphore("frame_complete");
        let frame_complete_fence = core.create_fence(true, "frame_complete");
        let swapchain_length = core.swapchain.swapchain_images.len() as u32;
        let command_buffers = CommandBuffer::create_multiple(core.clone(), swapchain_length);

        let swapchain_extent = core.swapchain.swapchain_extent;
        let x_shader_groups = swapchain_extent.width / SHADER_GROUP_SIZE as u32;
        let y_shader_groups = swapchain_extent.height / SHADER_GROUP_SIZE as u32 + 1;

        let mut render_data = RenderData::create(core.clone());
        render_data.initialize(game);
        let descriptor_collection = DescriptorCollection::create(core.clone(), &render_data);
        let mut tum = TerrainUploadManager::new(Rc::clone(&core));

        let denoise_stage = shaders::create_denoise_stage(core.clone(), &descriptor_collection);
        let finalize_stage = shaders::create_finalize_stage(core.clone(), &descriptor_collection);
        let raytrace_stage = shaders::create_raytrace_stage(core.clone(), &descriptor_collection);

        let mut pipeline = Pipeline {
            core,

            x_shader_groups,
            y_shader_groups,

            command_buffers,
            frame_available_semaphore,
            frame_complete_semaphore,
            frame_complete_fence,
            render_data,
            descriptor_collection,
            tum,

            denoise_stage,
            finalize_stage,
            raytrace_stage,
        };
        pipeline.record_command_buffers();
        pipeline
    }

    fn record_command_buffers(&mut self) {
        for (index, buffer) in self.command_buffers.iter().enumerate() {
            let swapchain_image = self.core.swapchain.swapchain_images[index];

            buffer.set_debug_name(&format!("primary_command_buffer_{}", index));

            buffer.begin();

            let layout = self.raytrace_stage.pipeline_layout;
            let set = self.descriptor_collection.raytrace.variants[0];
            buffer.bind_descriptor_set(layout, 0, set);
            buffer.bind_pipeline(self.raytrace_stage.vk_pipeline);
            buffer.dispatch(self.x_shader_groups, self.y_shader_groups, 1);

            buffer.transition_layout(
                &swapchain_image,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::GENERAL,
            );

            let layout = self.denoise_stage.pipeline_layout;
            let ping_set = self.descriptor_collection.denoise.variants[0];
            let pong_set = self.descriptor_collection.denoise.variants[1];
            buffer.bind_pipeline(self.denoise_stage.vk_pipeline);

            for (index, size) in [1, 2, 4, 8, 8, 16].iter().enumerate() {
                buffer.bind_descriptor_set(
                    layout,
                    0,
                    if index % 2 == 0 { ping_set } else { pong_set },
                );
                buffer.push_constants(
                    layout,
                    vk::ShaderStageFlags::COMPUTE,
                    &DenoisePushData { size: *size },
                );
                buffer.dispatch(self.x_shader_groups, self.y_shader_groups, 1);
            }

            let layout = self.finalize_stage.pipeline_layout;
            let set = self.descriptor_collection.finalize.variants[0];
            buffer.bind_descriptor_set(layout, 0, set);
            let set = self.descriptor_collection.swapchain.variants[index];
            buffer.bind_descriptor_set(layout, 1, set);
            buffer.bind_pipeline(self.finalize_stage.vk_pipeline);
            buffer.dispatch(self.x_shader_groups, self.y_shader_groups, 1);

            buffer.transition_layout(
                &swapchain_image,
                vk::ImageLayout::GENERAL,
                vk::ImageLayout::PRESENT_SRC_KHR,
            );
            buffer.end();
        }
    }

    pub fn draw_frame(&mut self, game: &mut Game) {
        let (image_index, _is_suboptimal) = unsafe {
            self.core
                .swapchain
                .swapchain_loader
                .acquire_next_image(
                    self.core.swapchain.swapchain,
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
            p_command_buffers: &self.command_buffers[image_index as usize].get_vk_command_buffer(),
            signal_semaphore_count: 1,
            p_signal_semaphores: signal_semaphores.as_ptr(),
            ..Default::default()
        };

        unsafe {
            let wait_fence = self.frame_complete_fence;
            self.core
                .device
                .wait_for_fences(&[wait_fence], true, std::u64::MAX)
                .expect("Failed to wait for previous frame to finish rendering.");
            self.core
                .device
                .reset_fences(&[wait_fence])
                .expect("Failed to reset fence.");
        }

        let camera = game.borrow_camera();
        self.tum.request_move_towards((
            camera.origin.x as isize,
            camera.origin.y as isize,
            camera.origin.z as isize,
        ));

        let mut upload_commands = CommandBuffer::create_single(Rc::clone(&self.core));
        upload_commands.begin_one_time_submit();
        self.tum.setup_next_request(
            &mut upload_commands,
            game.borrow_world_mut(),
            &self.render_data,
        );
        upload_commands.end();
        upload_commands.blocking_execute_and_destroy();

        let camera = game.borrow_camera();
        let util::TripleEulerVector { forward, up, right } =
            util::compute_triple_euler_vector(camera.heading, camera.pitch);

        let uniform_data = &mut self.render_data.raytrace_uniform_data;
        uniform_data.origin = camera.origin;
        uniform_data.forward = forward;
        uniform_data.up = up * 0.4;
        uniform_data.right = right * 0.4;
        // Modulus to prevent overflowing the seed.
        uniform_data.seed = (uniform_data.seed + 1) % BLUE_NOISE_SIZE as u32;
        uniform_data.sun_angle = game.get_sun_angle();

        let off = self.tum.get_render_offset(0);
        let off = (off.0 as i32, off.1 as i32, off.2 as i32).into();
        uniform_data.lod0_rotation = off;
        uniform_data.lod0_space_offset = off;
        let off = self.tum.get_render_offset(1);
        let off = (off.0 as i32, off.1 as i32, off.2 as i32).into();
        uniform_data.lod1_rotation = off;
        uniform_data.lod1_space_offset = off;
        let off = self.tum.get_render_offset(2);
        let off = (off.0 as i32, off.1 as i32, off.2 as i32).into();
        uniform_data.lod2_rotation = off;
        uniform_data.lod2_space_offset = off;
        let off = self.tum.get_render_offset(3);
        let off = (off.0 as i32, off.1 as i32, off.2 as i32).into();
        uniform_data.lod3_rotation = off;
        uniform_data.lod3_space_offset = off;

        let mut buffer_content = self.render_data.raytrace_uniform_data_buffer.bind_all();
        buffer_content[0] = uniform_data.clone();
        drop(buffer_content);

        // Do this after we set the buffer so that it will only affect the next frame.
        let uniform_data = &mut self.render_data.raytrace_uniform_data;
        uniform_data.old_origin = uniform_data.origin;
        let current_transform_matrix = {
            // Multiplying {screenx * depth, screeny * depth, depth} by this gets pixel position in world space.
            let screen_to_world_space =
                Matrix3::from_cols(right.clone() * 0.4, up.clone() * 0.4, forward.clone());
            // Inverting it gives us world space to screen space.
            screen_to_world_space
                .invert()
                .expect("Screen space vectors should cover entire coordinate space.")
        };
        uniform_data.old_transform_c0 = current_transform_matrix[0].clone();
        uniform_data.old_transform_c1 = current_transform_matrix[1].clone();
        uniform_data.old_transform_c2 = current_transform_matrix[2].clone();

        unsafe {
            let wait_fence = self.frame_complete_fence;
            self.core
                .device
                .queue_submit(self.core.compute_queue, &[submit_info], wait_fence)
                .expect("Failed to submit command queue.");
        }

        let wait_semaphores = [self.frame_complete_semaphore];
        let swapchains = [self.core.swapchain.swapchain];
        let present_info = vk::PresentInfoKHR {
            wait_semaphore_count: 1,
            p_wait_semaphores: wait_semaphores.as_ptr(),
            swapchain_count: 1,
            p_swapchains: swapchains.as_ptr(),
            p_image_indices: &image_index,
            ..Default::default()
        };

        unsafe {
            self.core
                .swapchain
                .swapchain_loader
                .queue_present(self.core.present_queue, &present_info)
                .expect("Failed to present swapchain image.");
        }
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        unsafe {
            self.core
                .device
                .device_wait_idle()
                .expect("Failed to wait for device to finish rendering.");

            self.core
                .device
                .destroy_fence(self.frame_complete_fence, None);
            self.core
                .device
                .destroy_semaphore(self.frame_available_semaphore, None);
            self.core
                .device
                .destroy_semaphore(self.frame_complete_semaphore, None);
        }
    }
}
