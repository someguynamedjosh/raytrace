use ash::version::DeviceV1_0;
use ash::vk;

use super::core::Core;

pub fn create_buffer(core: &Core, name: &str) -> vk::CommandBuffer {
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

pub fn execute_and_destroy(core: &Core, buffer: vk::CommandBuffer) {
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

pub fn begin(core: &Core, buffer: vk::CommandBuffer) {
    let begin_info = vk::CommandBufferBeginInfo {
        ..Default::default()
    };
    unsafe {
        core.device
            .begin_command_buffer(buffer, &begin_info)
            .expect("Failed to begin command buffer.");
    }
}

pub fn begin_one_time_submit(core: &Core, buffer: vk::CommandBuffer) {
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

pub fn end(core: &Core, buffer: vk::CommandBuffer) {
    unsafe {
        core.device
            .end_command_buffer(buffer)
            .expect("Failed to end command buffer.");
    }
}

pub fn transition_layout(
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

pub fn bind_descriptor_set(
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

pub fn bind_pipeline(core: &Core, buffer: vk::CommandBuffer, pipeline: vk::Pipeline) {
    unsafe {
        core.device
            .cmd_bind_pipeline(buffer, vk::PipelineBindPoint::COMPUTE, pipeline);
    }
}

pub fn copy_buffer_to_image(
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

pub fn transition_and_copy_buffer_to_image(
    core: &Core,
    buffer: vk::CommandBuffer,
    data_buffer: vk::Buffer,
    image: vk::Image,
    extent: vk::Extent3D,
) {
    transition_layout(
        core,
        buffer,
        image,
        vk::ImageLayout::UNDEFINED,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
    );
    copy_buffer_to_image(core, buffer, data_buffer, image, extent);
    transition_layout(
        core,
        buffer,
        image,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        vk::ImageLayout::GENERAL,
    );
}

pub fn copy_image_to_buffer(
    core: &Core,
    buffer: vk::CommandBuffer,
    image: vk::Image,
    data_buffer: vk::Buffer,
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
        core.device.cmd_copy_image_to_buffer(
            buffer,
            image,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            data_buffer,
            &[copy_info],
        );
    }
}

pub fn transition_and_copy_image_to_buffer(
    core: &Core,
    buffer: vk::CommandBuffer,
    image: vk::Image,
    data_buffer: vk::Buffer,
    extent: vk::Extent3D,
) {
    transition_layout(
        core,
        buffer,
        image,
        vk::ImageLayout::GENERAL,
        vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
    );
    copy_image_to_buffer(core, buffer, image, data_buffer, extent);
}

pub fn copy_image_to_image(
    core: &Core,
    buffer: vk::CommandBuffer,
    source: vk::Image,
    dest: vk::Image,
    extent: vk::Extent3D,
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
        extent,
        ..Default::default()
    };

    unsafe {
        core.device.cmd_copy_image(
            buffer,
            source,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            dest,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &[copy_info],
        );
    }
}
