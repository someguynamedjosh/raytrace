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
use crate::util::{self, prelude::*};
use crate::world::{ChunkStorage, CHUNK_SIZE, CHUNK_VOLUME};
use array_macro::array;
use ash::vk;
use std::rc::Rc;

// How thick each new slice of terrain data is.
const SLICE_SIZE: usize = 16;
const SLICES_PER_CHUNK: usize = CHUNK_SIZE / SLICE_SIZE;

struct TerrainUploadRequest {
    origin: SignedCoord3D,
    // Which step to upload, [0, ROOT_BLOCK_WIDTH / SLICE_SIZE).
    slice: usize,
    axis: Axis,
}

pub struct TerrainUploadManager {
    core: Rc<Core>,
    minefield_upload_buffer: Buffer<u8>,
    material_upload_buffer: Buffer<u32>,
}

impl TerrainUploadManager {
    pub fn new(core: Rc<Core>) -> Self {
        // Enough space to upload one slice at a time.
        const SIZE: usize = ROOT_BLOCK_WIDTH * ROOT_BLOCK_WIDTH * SLICE_SIZE;
        let minefield_upload_buffer = Buffer::create(
            Rc::clone(&core),
            "tum_minefield_upload",
            SIZE as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
        );
        let material_upload_buffer = Buffer::create(
            Rc::clone(&core),
            "tum_material_upload",
            SIZE as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
        );
        Self {
            core,
            minefield_upload_buffer,
            material_upload_buffer,
        }
    }

    fn upload_slice(
        &mut self,
        commands: &mut CommandBuffer,
        chunks: &mut ChunkStorage,
        data: &RenderData,
        request: TerrainUploadRequest,
    ) {
        let mut mat_data = self.material_upload_buffer.bind_all();
        let mut min_data = self.minefield_upload_buffer.bind_all();
        // The dimensions of the data that will be copied into the buffer and eventually copied
        // to the images on the GPU.
        let data_shape = match request.axis {
            Axis::X => (SLICE_SIZE, ROOT_BLOCK_WIDTH, ROOT_BLOCK_WIDTH),
            Axis::Y => (ROOT_BLOCK_WIDTH, SLICE_SIZE, ROOT_BLOCK_WIDTH),
            Axis::Z => (ROOT_BLOCK_WIDTH, ROOT_BLOCK_WIDTH, SLICE_SIZE),
        };
        // The maximum boundaries of the data that will be copied from each chunk.
        let chunk_area_shape = match request.axis {
            Axis::X => (SLICE_SIZE, CHUNK_SIZE, CHUNK_SIZE),
            Axis::Y => (CHUNK_SIZE, SLICE_SIZE, CHUNK_SIZE),
            Axis::Z => (CHUNK_SIZE, CHUNK_SIZE, SLICE_SIZE),
        };
        // How many chunks we need to skip along the primary axis.
        let chunk_offset = request.slice / SLICES_PER_CHUNK;
        let chunk_offset = match request.axis {
            Axis::X => (chunk_offset, 0, 0),
            Axis::Y => (0, chunk_offset, 0),
            Axis::Z => (0, 0, chunk_offset),
        };
        for (d1, d2) in [(0, 0), (1, 0), (0, 1), (1, 1)].iter().cloned() {
            // Which piece of the slice we are currently copying.
            let piece_offset = match request.axis {
                Axis::X => (0, d1, d2),
                Axis::Y => (d1, 0, d2),
                Axis::Z => (d1, d2, 0),
            };
            // Which chunk we are loading from.
            let world_coord = piece_offset.add(chunk_offset).sign().add(request.origin);
            let chunk =
                chunks.borrow_packed_chunk_data(&(world_coord.0, world_coord.1, world_coord.2, 0));
            // How far into the chunk we should start copying from.
            let axis_offset = request.slice % SLICES_PER_CHUNK * SLICE_SIZE;
            let source_start = match request.axis {
                Axis::X => (axis_offset, 0, 0),
                Axis::Y => (0, axis_offset, 0),
                Axis::Z => (0, 0, axis_offset),
            };
            let target_start = piece_offset.scale(CHUNK_SIZE).sign();
            util::copy_3d_bounded_auto_clip(
                chunk_area_shape,
                &chunk.materials,
                (CHUNK_SIZE, CHUNK_SIZE, CHUNK_SIZE),
                source_start,
                mat_data.as_slice_mut(),
                data_shape,
                target_start,
            );
            util::copy_3d_bounded_auto_clip(
                chunk_area_shape,
                &chunk.minefield,
                (CHUNK_SIZE, CHUNK_SIZE, CHUNK_SIZE),
                source_start,
                min_data.as_slice_mut(),
                data_shape,
                target_start,
            );
        }
        drop(mat_data);
        drop(min_data);
        let axis_offset = request.slice % (ROOT_BLOCK_VOLUME / SLICE_SIZE) * SLICE_SIZE;
        let target_offset = match request.axis {
            Axis::X => vk::Offset3D {
                x: axis_offset as i32,
                y: 0,
                z: 0,
            },
            Axis::Y => vk::Offset3D {
                x: 0,
                y: axis_offset as i32,
                z: 0,
            },
            Axis::Z => vk::Offset3D {
                x: 0,
                y: 0,
                z: axis_offset as i32,
            },
        };
        let data_shape = vk::Extent3D {
            width: data_shape.0 as u32,
            height: data_shape.1 as u32,
            depth: data_shape.2 as u32,
        };
        commands.transition_layout(
            &data.material_images[0],
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        );
        commands.copy_buffer_to_image_offset(
            &self.material_upload_buffer,
            0,
            data_shape.width,
            data_shape.height,
            &data.material_images[0],
            target_offset,
            &data_shape,
        );
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
        commands.copy_buffer_to_image_offset(
            &self.minefield_upload_buffer,
            0,
            data_shape.width,
            data_shape.height,
            &data.minefield_images[0],
            target_offset,
            &data_shape,
        );
        commands.transition_layout(
            &data.minefield_images[0],
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::GENERAL,
        );
    }

    pub fn test_command(
        &mut self,
        commands: &mut CommandBuffer,
        chunks: &mut ChunkStorage,
        data: &RenderData,
    ) {
        self.upload_slice(
            commands,
            chunks,
            data,
            TerrainUploadRequest {
                origin: (1, -1, -1),
                slice: 0,
                axis: Axis::X,
            },
        );
    }
}
