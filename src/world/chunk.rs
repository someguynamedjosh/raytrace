use rand::prelude::*;

use crate::render::constants::*;
use crate::render::{Material, MATERIALS};
use crate::util;

use super::functions;

// The index of the LOD that takes up an entire chunk.
pub const MAX_LOD: usize = 6;
pub const CHUNK_SIZE: usize = 1 << MAX_LOD; // 64
pub const CHUNK_VOLUME: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

pub enum Chunk {
    Empty,
    NonEmpty(ChunkData),
}

impl Chunk {
    pub fn generate(chunk_coord: &util::Coord3D) -> Chunk {
        let origin = (
            chunk_coord.0 * CHUNK_SIZE,
            chunk_coord.1 * CHUNK_SIZE,
            chunk_coord.2 * CHUNK_SIZE,
        );
        let mut data = UnfinishedChunkData::new();

        let mountain_noise = functions::MountainNoise::new();
        let mut random = rand::thread_rng();
        let height =
            |x, y| (mountain_noise.get(x as f64 / 200.0, y as f64 / 200.0) * 80.0 + 10.0) as usize;
        let material = |random: &mut ThreadRng, height: usize| {
            if height < 12 {
                2
            } else if height < 30 {
                let threshold = (height - 12) as u32;
                if random.next_u32() % (30 - 12) < threshold {
                    5
                } else {
                    2
                }
            } else if height < 35 {
                5
            } else if height < 60 {
                let threshold = (height - 35) as u32;
                if random.next_u32() % (60 - 35) < threshold {
                    6
                } else {
                    5
                }
            } else {
                6
            }
        };

        for (cx, cy) in util::coord_iter_2d(CHUNK_SIZE) {
            let height_val = height(cx + origin.0, cy + origin.1);
            if height_val < origin.2 {
                continue;
            }
            for z in origin.2..height_val.min(origin.2 + CHUNK_SIZE) {
                let material_val = material(&mut random, z);
                let cz = z - origin.2;
                data.set_block(&(cx, cy, cz), material_val);
            }
        }
        data.finalize()
    }

    pub fn copy_blocks(
        &self,
        target: &mut [u16],
        target_stride: usize,
        target_offset: &util::Coord3D,
    ) {
        if let Self::NonEmpty(data) = self {
            data.copy_blocks(target, target_stride, target_offset);
        } else {
            // Write air to every value.
            for (x, y, z) in util::coord_iter_3d(CHUNK_SIZE) {
                target[util::coord_to_index_3d(
                    &(
                        x + target_offset.0,
                        y + target_offset.1,
                        z + target_offset.2,
                    ),
                    target_stride,
                )] = 0;
            }
        }
    }

    pub fn copy_minefield(
        &self,
        target: &mut [u8],
        target_stride: usize,
        target_offset: &util::Coord3D,
    ) {
        if let Self::NonEmpty(data) = self {
            data.copy_minefield(target, target_stride, target_offset);
        } else {
            // Write a chunk-sized LOD to every value.
            for (x, y, z) in util::coord_iter_3d(CHUNK_SIZE) {
                target[util::coord_to_index_3d(
                    &(
                        x + target_offset.0,
                        y + target_offset.1,
                        z + target_offset.2,
                    ),
                    target_stride,
                )] = MAX_LOD as u8;
            }
        }
    }
}

pub struct ChunkMip {
    minefield: Vec<u8>,
    blocks: Vec<Material>,
}

impl ChunkMip {
    fn new() -> ChunkMip {
        ChunkMip {
            minefield: vec![0; CHUNK_VOLUME],
            blocks: vec![Material::black(); CHUNK_VOLUME],
        }
    }

    pub fn from_chunks(neighborhood: &[&Chunk]) -> ChunkMip {
        debug_assert!(
            neighborhood.len() == 8,
            "Neighborhood must be a 2x2x2 region!"
        );
        let mut mip = Self::new();
        for index in 0..8 {
            let offset = util::scale_coord_3d(&util::index_to_coord_3d(index, 2), CHUNK_SIZE / 2);
            mip.incorporate_chunk(&neighborhood[index], &offset);
        }
        mip
    }

    fn incorporate_chunk(
        &mut self,
        chunk: &Chunk,
        offset: &util::Coord3D,
    ) {
        if let Chunk::NonEmpty(data) = chunk {
            // Min LOD 1 because chunks store LOD 0 data.
            self.incorporate_minefield(&data.minefield, offset, 1);
            self.incorporate_blocks(&data.blocks, offset);
        } else {
            for index in 0..CHUNK_VOLUME {
                // The entire chunk is empty. Write the LOD corresponding to chunk size everywhere.
                self.minefield[index] = MAX_LOD as u8;
            }
            // The material can stay black.
        }
    }

    // Takes a minefield, shrinks it, and writes the shrunk data to a part of this mip's minefield.
    // The offset specifies where to place it, since the data will only take up 1/8th of the total
    // space.
    fn incorporate_minefield(
        &mut self,
        minefield: &[u8],
        offset: &util::Coord3D,
        min_lod: u8,
    ) {
        for coord in util::coord_iter_3d(CHUNK_SIZE / 2) {
            let source_coord = util::scale_coord_3d(&coord, 2);
            let target_coord = util::offset_coord_3d(&coord, &offset);
            let mut lowest_lod = u8::max_value();
            for offset in &[
                (0, 0, 0),
                (0, 0, 1),
                (0, 1, 0),
                (0, 1, 1),
                (1, 0, 0),
                (1, 0, 1),
                (1, 1, 0),
                (1, 1, 1),
            ] {
                lowest_lod = lowest_lod.min(
                    minefield[util::coord_to_index_3d(
                        &util::offset_coord_3d(&source_coord, offset),
                        CHUNK_SIZE,
                    )],
                );
            }
            lowest_lod = lowest_lod.max(min_lod);
            self.minefield[util::coord_to_index_3d(&target_coord, CHUNK_SIZE)] = lowest_lod;
        }
    }

    // Shrinks block id data to material data with dimensions twice as small.
    fn incorporate_blocks(
        &mut self,
        blocks: &[u16],
        offset: &util::Coord3D,
    ) {
        for coord in util::coord_iter_3d(CHUNK_SIZE / 2) {
            let source_coord = util::scale_coord_3d(&coord, 2);
            let target_coord = util::offset_coord_3d(&coord, &offset);
            let mut material = Material::black();
            for offset in &[
                (0, 0, 0),
                (0, 0, 1),
                (0, 1, 0),
                (0, 1, 1),
                (1, 0, 0),
                (1, 0, 1),
                (1, 1, 0),
                (1, 1, 1),
            ] {
                let material_id = blocks[util::coord_to_index_3d(
                        &util::offset_coord_3d(&source_coord, offset),
                        CHUNK_SIZE,
                    )];
                if material_id == 0 {
                    continue;
                }
                material.add(&MATERIALS[material_id as usize]);
            }
            material.multiply(1.0 / 8.0);
            self.blocks[util::coord_to_index_3d(&target_coord, CHUNK_SIZE)] = material;
        }
    }

    pub fn copy_minefield(
        &self,
        target: &mut [u8],
        target_stride: usize,
        target_offset: &util::Coord3D,
    ) {
        for coord in util::coord_iter_3d(CHUNK_SIZE) {
            let source_index = util::coord_to_index_3d(&coord, CHUNK_SIZE);
            let target_coord = (
                coord.0 + target_offset.0,
                coord.1 + target_offset.1,
                coord.2 + target_offset.2,
            );
            let target_index = util::coord_to_index_3d(&target_coord, target_stride);
            target[target_index] = self.minefield[source_index];
        }
    }
}

pub struct ChunkData {
    /// The minefield contains the index of the most detailed LOD that still has a value at each
    /// position in the chunk. For example, if a particular coordinate only has air, but there is
    /// still other blocks in a 2x2x2 vicinity, then the minefield will contain "1" at that
    /// position. If there is a block at that position, it would contain "0". If we have to go all
    /// the way up to the LOD using 16^3 neighborhoods before we find a non-empty value, it would
    /// contain "4" at that position.
    pub minefield: Vec<u8>,
    pub blocks: Vec<u16>,
}

impl ChunkData {
    pub fn copy_blocks(
        &self,
        target: &mut [u16],
        target_stride: usize,
        target_offset: &util::Coord3D,
    ) {
        for coord in util::coord_iter_3d(CHUNK_SIZE) {
            let source_index = util::coord_to_index_3d(&coord, CHUNK_SIZE);
            let target_coord = (
                coord.0 + target_offset.0,
                coord.1 + target_offset.1,
                coord.2 + target_offset.2,
            );
            let target_index = util::coord_to_index_3d(&target_coord, target_stride);
            target[target_index] = self.blocks[source_index];
        }
    }

    pub fn copy_minefield(
        &self,
        target: &mut [u8],
        target_stride: usize,
        target_offset: &util::Coord3D,
    ) {
        for coord in util::coord_iter_3d(CHUNK_SIZE) {
            let source_index = util::coord_to_index_3d(&coord, CHUNK_SIZE);
            let target_coord = (
                coord.0 + target_offset.0,
                coord.1 + target_offset.1,
                coord.2 + target_offset.2,
            );
            let target_index = util::coord_to_index_3d(&target_coord, target_stride);
            target[target_index] = self.minefield[source_index];
        }
    }
}

/// This individually stores each LOD instead of storing it all as a minefield. This makes it easier
/// to generate the world. The data can be compacted into a ChunkData struct when done.
struct UnfinishedChunkData {
    lod0: Vec<u16>,
    /// These LODs store true if there is any block somewhere in their neighborhood.
    other_lods: Vec<Vec<bool>>,
    /// True if the chunk contains no blocks.
    empty: bool,
}

impl UnfinishedChunkData {
    fn new() -> UnfinishedChunkData {
        let mut other_lods = Vec::new();
        let mut volume = CHUNK_VOLUME / 8;
        while volume > 0 {
            other_lods.push(vec![false; volume]);
            // Each LOD represents the same data but at half the resolution (1/8th the number
            // of voxels.)
            volume /= 8;
        }

        UnfinishedChunkData {
            lod0: vec![0; CHUNK_VOLUME],
            other_lods,
            empty: true,
        }
    }

    /// Sets the block at the given position and updates all LODs accordingly.
    fn set_block(&mut self, coord: &util::Coord3D, value: u16) {
        if value == 0 {
            unimplemented!("No implementation for erasing blocks yet.");
        }
        self.empty = false;
        self.lod0[util::coord_to_index_3d(&coord, CHUNK_SIZE)] = value;

        let mut lod_coord = (coord.0 / 2, coord.1 / 2, coord.2 / 2);
        // Array actually starts at LOD1 so this number is 1 lower than it theoretically should be.
        let mut lod_level = 0;
        let mut lod_stride = CHUNK_SIZE / 2;
        while lod_level < self.other_lods.len() {
            self.other_lods[lod_level][util::coord_to_index_3d(&lod_coord, lod_stride)] = true;
            lod_coord = (lod_coord.0 / 2, lod_coord.1 / 2, lod_coord.2 / 2);
            lod_level += 1;
            lod_stride /= 2;
        }
    }

    /// Converts this into a ChunkData object. This converts the individual LODs into a single
    /// minefield in the process.
    fn finalize(self) -> Chunk {
        if self.empty {
            return Chunk::Empty;
        }
        let UnfinishedChunkData {
            lod0, other_lods, ..
        } = self;
        let mut minefield = vec![0; CHUNK_VOLUME];
        for index in 0..CHUNK_VOLUME {
            if lod0[index] > 0 {
                // Leave the minefield value as zero.
                continue;
            }
            // Otherwise, we need to look at coarser and coarser LODs until erventually one of them
            // has a non-empty value.
            let mut coord = util::index_to_coord_3d(index, CHUNK_SIZE);
            let mut lod_stride = CHUNK_SIZE;
            for lod_index in 0..other_lods.len() {
                coord = (coord.0 / 2, coord.1 / 2, coord.2 / 2);
                lod_stride /= 2;
                if other_lods[lod_index][util::coord_to_index_3d(&coord, lod_stride)] {
                    // LOD1 is at index 0
                    minefield[index] = lod_index as u8 + 1;
                    break;
                }
            }
        }

        Chunk::NonEmpty(ChunkData {
            blocks: lod0,
            minefield,
        })
    }
}
