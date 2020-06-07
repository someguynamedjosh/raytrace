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

/// Upon consuming this request, the next slice along the specified axis will be uploaded.
struct TerrainUploadRequest {
    origin: SignedCoord3D,
    // [0, ROOT_BLOCK_WIDTH / SLICE_SIZE), how many slices to offset in each axis.
    num_slices: Coord3D,
    axis: Axis,
    // What position this LOD will be at after the request is completed.
    new_position: Position,
}

// This stores the origin of the current region and how many slices of the next region have been
// loaded.
#[derive(Clone)]
struct Position {
    origin: SignedCoord3D,
    num_loaded_slices: Coord3D,
}

impl Position {
    fn render_offset(&self, lod: isize) -> SignedCoord3D {
        self.origin
            .add((1, 1, 1))
            .scale(CHUNK_SIZE as _)
            .add(self.num_loaded_slices.scale(SLICE_SIZE).signed())
            .scale(1 << lod)
    }
}

impl Default for Position {
    fn default() -> Self {
        Self {
            origin: (-1, -1, -1),
            num_loaded_slices: (0, 0, 0),
        }
    }
}

pub struct TerrainUploadManager {
    core: Rc<Core>,
    minefield_upload_buffer: Buffer<u8>,
    material_upload_buffer: Buffer<u32>,
    request_queue: Vec<TerrainUploadRequest>,
    lod_positions: [Position; NUM_LODS],
    gpu_lod_positions: [Position; NUM_LODS],
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
            request_queue: Vec::new(),
            lod_positions: array![Position::default(); NUM_LODS],
            gpu_lod_positions: array![Position::default(); NUM_LODS],
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
        // We only need to start copying chunks at this offset (+ the request origin).
        let chunk_offset = request.num_slices.shrink(SLICES_PER_CHUNK);
        // How far into the first chunk we should start copying from. (Also how much we need to copy
        // from the last chunk.)
        let area_start = request
            .num_slices
            .wrap(SLICES_PER_CHUNK.repeat())
            .scale(SLICE_SIZE);
        for (d1, d2) in util::coord_iter_2d(3) {
            // Which piece of the slice we are currently copying.
            let piece_offset = match request.axis {
                Axis::X => (0, d1, d2),
                Axis::Y => (d1, 0, d2),
                Axis::Z => (d1, d2, 0),
            };
            // Which chunk we are loading from.
            let world_coord = piece_offset.add(chunk_offset).signed().add(request.origin);
            let chunk =
                chunks.borrow_packed_chunk_data(&(world_coord.0, world_coord.1, world_coord.2, 0));
            // The coordinate inside the chunk to start copying from.
            let mut copy_start = (0, 0, 0);
            // Basically if we are copying from a chunk at the start of a particular axis, the
            // coordinate we start copying from inside that chunk should have the start coordinate
            // specified by area_start on that axis.
            if piece_offset.0 == 0 {
                copy_start.0 = area_start.0;
            }
            if piece_offset.1 == 0 {
                copy_start.1 = area_start.1;
            }
            if piece_offset.2 == 0 {
                copy_start.2 = area_start.2;
            }
            // Copying should end before this coordinate on all axes. size = copy_end - copy_start.
            let mut copy_end = (CHUNK_SIZE, CHUNK_SIZE, CHUNK_SIZE);
            // Basically if we are copying from a chunk at the end of a particular axis, the
            // coordinate we start copying before should be specified by area_start. In combination
            // with the previous effect, we will always end up copying a ROOT_BLOCK_WIDTH sized
            // chunk of data.
            if piece_offset.0 == 2 {
                copy_end.0 = area_start.0;
            }
            if piece_offset.1 == 2 {
                copy_end.1 = area_start.1;
            }
            if piece_offset.2 == 2 {
                copy_end.2 = area_start.2;
            }
            // Also we should end copying at start + SLICE_SIZE along the main axis of the slice.
            match request.axis {
                Axis::X => copy_end.0 = copy_start.0 + SLICE_SIZE,
                Axis::Y => copy_end.1 = copy_start.1 + SLICE_SIZE,
                Axis::Z => copy_end.2 = copy_start.2 + SLICE_SIZE,
            }
            // The size of the data that will be copied.
            let copy_size = copy_end.sub(copy_start);
            if copy_size.0 == 0 || copy_size.1 == 0 || copy_size.2 == 0 {
                continue;
            }
            assert!(copy_size.inside(chunk_area_shape));
            // Compute generally where we should copy the data to (which chunk)
            let target_start = piece_offset
                .add(match request.axis {
                    Axis::X => (0, chunk_offset.1, chunk_offset.2),
                    Axis::Y => (chunk_offset.0, 0, chunk_offset.2),
                    Axis::Z => (chunk_offset.0, chunk_offset.1, 0),
                })
                .wrap(2.repeat())
                .scale(CHUNK_SIZE)
                .signed();
            // If we copied with an offset on an off axis, the destination should have that same
            // offset on that same off axis. Don't copy the main axis offset because that one picks
            // out data for this particular slice, and the buffer is only one slice long along the
            // main axis.
            let target_offset = match request.axis {
                Axis::X => (0, copy_start.1, copy_start.2),
                Axis::Y => (copy_start.0, 0, copy_start.2),
                Axis::Z => (copy_start.0, copy_start.1, 0),
            };
            let target_start = target_start.add(target_offset.signed());
            util::copy_3d_bounded_auto_clip(
                copy_size,
                &chunk.materials,
                (CHUNK_SIZE, CHUNK_SIZE, CHUNK_SIZE),
                copy_start,
                mat_data.as_slice_mut(),
                data_shape,
                target_start,
            );
            util::copy_3d_bounded_auto_clip(
                copy_size,
                &chunk.minefield,
                (CHUNK_SIZE, CHUNK_SIZE, CHUNK_SIZE),
                copy_start,
                min_data.as_slice_mut(),
                data_shape,
                target_start,
            );
        }
        drop(mat_data);
        drop(min_data);
        let axis_num_slices = match request.axis {
            Axis::X => request.num_slices.0,
            Axis::Y => request.num_slices.1,
            Axis::Z => request.num_slices.2,
        };
        let axis_offset = axis_num_slices % (ROOT_BLOCK_VOLUME / SLICE_SIZE) * SLICE_SIZE;
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

        self.gpu_lod_positions[0] = request.new_position;
    }

    pub fn setup_next_request(
        &mut self,
        commands: &mut CommandBuffer,
        chunks: &mut ChunkStorage,
        data: &RenderData,
    ) {
        if self.request_queue.len() == 0 {
            return;
        }
        let request = self.request_queue.remove(0);
        self.upload_slice(commands, chunks, data, request);
    }

    pub fn get_render_offset(&self, lod: usize) -> SignedCoord3D {
        self.gpu_lod_positions[lod].render_offset(lod as _)
    }

    pub fn request_increase(&mut self, axis: Axis) {
        // Load the next slice then increment the number of loaded slices.
        let old_position = self.lod_positions[0].clone();
        let num_slices = match axis {
            Axis::X => &mut self.lod_positions[0].num_loaded_slices.0,
            Axis::Y => &mut self.lod_positions[0].num_loaded_slices.1,
            Axis::Z => &mut self.lod_positions[0].num_loaded_slices.2,
        };
        let coord = match axis {
            Axis::X => &mut self.lod_positions[0].origin.0,
            Axis::Y => &mut self.lod_positions[0].origin.1,
            Axis::Z => &mut self.lod_positions[0].origin.2,
        };
        *num_slices += 1;
        if *num_slices == ROOT_BLOCK_WIDTH / SLICE_SIZE {
            *num_slices = 0;
            *coord += (ROOT_BLOCK_WIDTH / CHUNK_SIZE) as isize;
        }
        // This makes it load the data from the next region instead of the current region.
        let origin_offset = match axis {
            Axis::X => (2, 0, 0),
            Axis::Y => (0, 2, 0),
            Axis::Z => (0, 0, 2),
        };
        self.request_queue.push(TerrainUploadRequest {
            origin: old_position.origin.add(origin_offset),
            num_slices: old_position.num_loaded_slices,
            axis,
            new_position: self.lod_positions[0].clone(),
        });
    }

    pub fn request_decrease(&mut self, axis: Axis) {
        // Rewind the coordinate to the previous slice and then load it from the current region.
        let num_slices = match axis {
            Axis::X => &mut self.lod_positions[0].num_loaded_slices.0,
            Axis::Y => &mut self.lod_positions[0].num_loaded_slices.1,
            Axis::Z => &mut self.lod_positions[0].num_loaded_slices.2,
        };
        let coord = match axis {
            Axis::X => &mut self.lod_positions[0].origin.0,
            Axis::Y => &mut self.lod_positions[0].origin.1,
            Axis::Z => &mut self.lod_positions[0].origin.2,
        };
        if *num_slices == 0 {
            *num_slices = ROOT_BLOCK_WIDTH / SLICE_SIZE;
            *coord -= (ROOT_BLOCK_WIDTH / CHUNK_SIZE) as isize;
        }
        *num_slices -= 1;
        self.request_queue.push(TerrainUploadRequest {
            origin: self.lod_positions[0].origin,
            num_slices: self.lod_positions[0].num_loaded_slices,
            axis,
            new_position: self.lod_positions[0].clone(),
        });
    }

    fn request_move_lod_towards(&mut self, lod: usize, desired_center: SignedCoord3D) {
        let current_pos = &self.lod_positions[lod];
        let delta = desired_center.sub(current_pos.render_offset(lod as _));
        if delta.0 > SLICE_SIZE as _ {
            self.request_increase(Axis::X);
        } else if -delta.0 > SLICE_SIZE as _ {
            self.request_decrease(Axis::X);
        } else if delta.1 > SLICE_SIZE as _ {
            self.request_increase(Axis::Y);
        } else if -delta.1 > SLICE_SIZE as _ {
            self.request_decrease(Axis::Y);
        } else if delta.2 > SLICE_SIZE as _ {
            self.request_increase(Axis::Z);
        } else if -delta.2 > SLICE_SIZE as _ {
            self.request_decrease(Axis::Z);
        }
    }

    pub fn request_move_towards(&mut self, desired_center: SignedCoord3D) {
        self.request_move_lod_towards(0, desired_center);
    }
}
