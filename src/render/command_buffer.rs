use ash::version::DeviceV1_0;
use ash::vk;

use std::rc::Rc;

use super::core::Core;
use super::structures::{BufferWrapper, ExtentWrapper, ImageWrapper, SamplerWrapper};

pub struct CommandBuffer {
    core: Rc<Core>,
    command_buffer: vk::CommandBuffer,
}

impl CommandBuffer {
    pub fn create_multiple(core: Rc<Core>, quantity: u32) -> Vec<Self> {
        let create_info = vk::CommandBufferAllocateInfo {
            command_pool: core.command_pool,
            level: vk::CommandBufferLevel::PRIMARY,
            command_buffer_count: quantity,
            ..Default::default()
        };
        let raw = unsafe {
            core.device
                .allocate_command_buffers(&create_info)
                .expect("Failed to allocate command buffers.")
        };
        raw.into_iter()
            .map(|command_buffer| CommandBuffer {
                core: core.clone(),
                command_buffer,
            })
            .collect()
    }

    pub fn create_single(core: Rc<Core>) -> Self {
        let create_info = vk::CommandBufferAllocateInfo {
            command_pool: core.command_pool,
            level: vk::CommandBufferLevel::PRIMARY,
            command_buffer_count: 1,
            ..Default::default()
        };
        let command_buffer = unsafe {
            core.device
                .allocate_command_buffers(&create_info)
                .expect("Failed to allocate command buffers.")[0]
        };
        Self {
            core,
            command_buffer,
        }
    }

    pub fn get_vk_command_buffer(&self) -> vk::CommandBuffer {
        self.command_buffer
    }

    pub fn set_debug_name(&self, debug_name: &str) {
        self.core.set_debug_name(self.command_buffer, debug_name);
    }

    pub fn blocking_execute_and_destroy(self) {
        let submit_info = vk::SubmitInfo {
            command_buffer_count: 1,
            p_command_buffers: &self.command_buffer,
            ..Default::default()
        };

        unsafe {
            self.core
                .device
                .queue_submit(self.core.compute_queue, &[submit_info], vk::Fence::null())
                .expect("Failed to submit one time submit command buffer.");
            self.core
                .device
                .queue_wait_idle(self.core.compute_queue)
                .expect("Failed to wait for completion of command buffer.");
            self.core
                .device
                .free_command_buffers(self.core.command_pool, &[self.command_buffer]);
        }
    }

    pub fn begin(&self) {
        let begin_info = vk::CommandBufferBeginInfo {
            ..Default::default()
        };
        unsafe {
            self.core
                .device
                .begin_command_buffer(self.command_buffer, &begin_info)
                .expect("Failed to begin command buffer.");
        }
    }

    pub fn begin_one_time_submit(&self) {
        let begin_info = vk::CommandBufferBeginInfo {
            flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
            ..Default::default()
        };
        unsafe {
            self.core
                .device
                .begin_command_buffer(self.command_buffer, &begin_info)
                .expect("Failed to begin command buffer.");
        }
    }

    pub fn end(&self) {
        unsafe {
            self.core
                .device
                .end_command_buffer(self.command_buffer)
                .expect("Failed to end command buffer.");
        }
    }

    pub fn bind_descriptor_set(
        &self,
        pipeline_layout: vk::PipelineLayout,
        index: u32,
        descriptor_set: vk::DescriptorSet,
    ) {
        unsafe {
            self.core.device.cmd_bind_descriptor_sets(
                self.command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                pipeline_layout,
                index,
                &[descriptor_set],
                &[],
            );
        }
    }

    pub fn bind_pipeline(&self, pipeline: vk::Pipeline) {
        unsafe {
            self.core.device.cmd_bind_pipeline(
                self.command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                pipeline,
            );
        }
    }

    pub fn dispatch(&self, group_count_x: u32, group_count_y: u32, group_count_z: u32) {
        unsafe {
            self.core.device.cmd_dispatch(
                self.command_buffer,
                group_count_x,
                group_count_y,
                group_count_z,
            );
        }
    }

    // TODO: Allow for custom pipeline stage flag specification.
    pub fn transition_layout(
        &self,
        image: &impl ImageWrapper,
        from: vk::ImageLayout,
        to: vk::ImageLayout,
    ) {
        self.transition_layout_mipped(image, from, to, 1)
    }
    pub fn transition_layout_mipped(
        &self,
        image: &impl ImageWrapper,
        from: vk::ImageLayout,
        to: vk::ImageLayout,
        mip_level_count: u32,
    ) {
        let image_barrier = vk::ImageMemoryBarrier {
            old_layout: from,
            new_layout: to,
            src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            image: image.get_vk_image(),
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: mip_level_count,
                base_array_layer: 0,
                layer_count: 1,
            },
            ..Default::default()
        };
        unsafe {
            self.core.device.cmd_pipeline_barrier(
                self.command_buffer,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                Default::default(),
                &[],
                &[],
                &[image_barrier],
            );
        }
    }
    pub fn copy_buffer_to_image(
        &self,
        data_buffer: &impl BufferWrapper,
        image: &impl ImageWrapper,
        extent: &impl ExtentWrapper,
    ) {
        self.copy_buffer_to_image_mip(data_buffer, image, extent, 0);
    }

    pub fn copy_buffer_to_image_mip(
        &self,
        data_buffer: &impl BufferWrapper,
        image: &impl ImageWrapper,
        extent: &impl ExtentWrapper,
        mip_level: u32,
    ) {
        let copy_info = vk::BufferImageCopy {
            buffer_offset: 0,
            buffer_row_length: 0,
            buffer_image_height: 0,

            image_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level,
                base_array_layer: 0,
                layer_count: 1,
            },

            image_extent: extent.get_vk_extent(),
            ..Default::default()
        };

        unsafe {
            self.core.device.cmd_copy_buffer_to_image(
                self.command_buffer,
                data_buffer.get_vk_buffer(),
                image.get_vk_image(),
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[copy_info],
            );
        }
    }

    pub fn copy_buffer_to_image_offset(
        &self,
        data_buffer: &impl BufferWrapper,
        image: &impl ImageWrapper,
        extent: &impl ExtentWrapper,
        offset: vk::Offset3D,
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

            image_extent: extent.get_vk_extent(),
            image_offset: offset,
            ..Default::default()
        };

        unsafe {
            self.core.device.cmd_copy_buffer_to_image(
                self.command_buffer,
                data_buffer.get_vk_buffer(),
                image.get_vk_image(),
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[copy_info],
            );
        }
    }

    pub fn transition_and_copy_buffer_to_image(
        &self,
        data_buffer: &impl BufferWrapper,
        image: &impl ImageWrapper,
        extent: &impl ExtentWrapper,
    ) {
        self.transition_layout(
            image,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        );
        self.copy_buffer_to_image(data_buffer, image, extent);
        self.transition_layout(
            image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::GENERAL,
        );
    }

    pub fn copy_image_to_buffer(
        &self,
        image: &impl ImageWrapper,
        extent: &impl ExtentWrapper,
        data_buffer: &impl BufferWrapper,
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

            image_extent: extent.get_vk_extent(),
            ..Default::default()
        };

        unsafe {
            self.core.device.cmd_copy_image_to_buffer(
                self.command_buffer,
                image.get_vk_image(),
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                data_buffer.get_vk_buffer(),
                &[copy_info],
            );
        }
    }

    pub fn transition_and_copy_image_to_buffer(
        &self,
        image: &impl ImageWrapper,
        extent: &impl ExtentWrapper,
        data_buffer: &impl BufferWrapper,
    ) {
        self.transition_layout(
            image,
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
        );
        self.copy_image_to_buffer(image, extent, data_buffer);
    }

    pub fn copy_image_to_image(
        &self,
        source: &impl ImageWrapper,
        extent: &impl ExtentWrapper,
        dest: &impl ImageWrapper,
    ) {
        let copy_info = vk::ImageCopy {
            src_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            dst_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            extent: extent.get_vk_extent(),
            ..Default::default()
        };

        unsafe {
            self.core.device.cmd_copy_image(
                self.command_buffer,
                source.get_vk_image(),
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                dest.get_vk_image(),
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[copy_info],
            );
        }
    }
}
