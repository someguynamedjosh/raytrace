use super::structs::RaytraceUniformData;
use crate::game::Game;
use crate::render::constants::*;
use crate::render::general::command_buffer::CommandBuffer;
use crate::render::general::core::Core;
use crate::render::general::structures::{
    Buffer, BufferWrapper, DataDestination, ExtentWrapper, ImageOptions, ImageWrapper,
    SampledImage, SamplerOptions, StorageImage,
};
use crate::render::pipeline::render_data::RenderData;
use crate::util::{self, traits::*};
use crate::world::{ChunkStorage, CHUNK_SIZE, CHUNK_VOLUME};
use array_macro::array;
use ash::vk;
use std::rc::Rc;

// How thick each new slice of terrain data is.
const UPLOAD_STEP: usize = 16;

pub struct TerrainUploadManager {
    core: Rc<Core>,
    minefield_upload_buffer: Buffer<u8>,
    material_upload_buffer: Buffer<u32>,
}

impl TerrainUploadManager {
    pub fn new(core: Rc<Core>) -> Self {
        let minefield_upload_buffer = Buffer::create(
            Rc::clone(&core),
            "tum_minefield_upload",
            CHUNK_VOLUME as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
        );
        let material_upload_buffer = Buffer::create(
            Rc::clone(&core),
            "tum_material_upload",
            CHUNK_VOLUME as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
        );
        Self {
            core,
            minefield_upload_buffer,
            material_upload_buffer,
        }
    }

    pub fn test_command(
        &mut self,
        commands: &mut CommandBuffer,
        chunks: &mut ChunkStorage,
        data: &RenderData,
    ) {
        let mut mat_data = self.material_upload_buffer.bind_all();
        let mut min_data = self.minefield_upload_buffer.bind_all();
        // Where to put data in the buffer.
        let mut buffer_offset = 0;
        for (x, z) in [(0, 0), (1, 0), (0, 1), (1, 1)].iter().cloned() {
            let chunk_coord = (x, 2, z);
            let world_coord = chunk_coord.sign().sub((1, 1, 1));
            let chunk =
                chunks.borrow_packed_chunk_data(&(world_coord.0, world_coord.1, world_coord.2, 0));
            util::copy_3d_bounded_auto_clip(
                &chunk.materials,
                CHUNK_SIZE,
                (0, buffer_offset as isize, 0),
                (CHUNK_SIZE, UPLOAD_STEP, CHUNK_SIZE),
                mat_data.as_slice_mut(),
                CHUNK_SIZE,
            );
            util::copy_3d_bounded_auto_clip(
                &chunk.minefield,
                CHUNK_SIZE,
                (0, buffer_offset as isize, 0),
                (CHUNK_SIZE, UPLOAD_STEP, CHUNK_SIZE),
                min_data.as_slice_mut(),
                CHUNK_SIZE,
            );
            buffer_offset += UPLOAD_STEP;
        }
        drop(mat_data);
        drop(min_data);
        commands.transition_layout(
            &data.material_images[0],
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        );
        let mut buffer_offset = 0;
        for (x, z) in [(0, 0), (1, 0), (0, 1), (1, 1)].iter().cloned() {
            commands.copy_buffer_to_image_offset(
                &self.material_upload_buffer,
                buffer_offset as u64 * 4,
                CHUNK_SIZE as u32,
                CHUNK_SIZE as u32,
                &data.material_images[0],
                vk::Offset3D {
                    x: x * CHUNK_SIZE as i32,
                    y: 0,
                    z: z * CHUNK_SIZE as i32,
                },
                &vk::Extent3D {
                    width: CHUNK_SIZE as u32,
                    height: UPLOAD_STEP as u32,
                    depth: CHUNK_SIZE as u32,
                },
            );
            buffer_offset += UPLOAD_STEP * CHUNK_SIZE;
        }
        commands.transition_layout(
            &data.material_images[0],
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::GENERAL,
        );
        commands.transition_layout(
            &data.minefield_images[0],
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        );
        let mut buffer_offset = 0;
        for (x, z) in [(0, 0), (1, 0), (0, 1), (1, 1)].iter().cloned() {
            commands.copy_buffer_to_image_offset(
                &self.minefield_upload_buffer,
                buffer_offset as u64,
                CHUNK_SIZE as u32,
                CHUNK_SIZE as u32,
                &data.minefield_images[0],
                vk::Offset3D {
                    x: x * CHUNK_SIZE as i32,
                    y: 0,
                    z: z * CHUNK_SIZE as i32,
                },
                &vk::Extent3D {
                    width: CHUNK_SIZE as u32,
                    height: UPLOAD_STEP as u32,
                    depth: CHUNK_SIZE as u32,
                },
            );
            buffer_offset += UPLOAD_STEP * CHUNK_SIZE;
        }
        commands.transition_layout(
            &data.minefield_images[0],
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::GENERAL,
        );
    }
}
