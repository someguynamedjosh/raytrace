use rand::prelude::*;

use crate::render::{Material, MATERIALS};
use crate::util;

use super::functions;

// The index of the LOD that takes up an entire chunk.
pub const MAX_LOD: usize = 6;
pub const CHUNK_SIZE: usize = 1 << MAX_LOD; // 64
pub const CHUNK_VOLUME: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

pub enum PackedChunk {
    Empty { scale: u8 },
    NonEmpty(PackedChunkData),
}

impl PackedChunk {
    pub fn copy_materials(
        &self,
        target: &mut [u32],
        target_stride: usize,
        target_offset: &util::Coord3D,
    ) {
        if let Self::NonEmpty(data) = self {
            data.copy_materials(target, target_stride, target_offset);
        } else {
            // Write black to every value.
            for coord in util::coord_iter_3d(CHUNK_SIZE) {
                target[util::coord_to_index_3d(
                    &util::offset_coord_3d(&coord, target_offset),
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
        match self {
            Self::NonEmpty(data) => {
                data.copy_minefield(target, target_stride, target_offset);
            }
            Self::Empty { scale } => {
                // Write a chunk-sized LOD to every value.
                for coord in util::coord_iter_3d(CHUNK_SIZE) {
                    target[util::coord_to_index_3d(
                        &util::offset_coord_3d(&coord, target_offset),
                        target_stride,
                    )] = MAX_LOD as u8 + scale;
                }
            }
        }
    }
}

pub struct PackedChunkData {
    minefield: Vec<u8>,
    materials: Vec<u32>,
}

impl PackedChunkData {
    fn new() -> PackedChunkData {
        PackedChunkData {
            minefield: vec![0; CHUNK_VOLUME],
            materials: vec![0; CHUNK_VOLUME],
        }
    }

    pub fn copy_materials(
        &self,
        target: &mut [u32],
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
            target[target_index] = self.materials[source_index];
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

pub struct UnpackedChunkData {
    materials: Vec<Material>,
    scale: u8,
}

impl UnpackedChunkData {
    /// Scale is what sized block one voxel in this chunk represents. 0 = smallest unit, 1 = double,
    /// 2 = quadruple, 3 = 8x and so on. It basically offsets every LOD value computed for the
    /// minefield in the pack function.
    fn new(scale: u8) -> UnpackedChunkData {
        UnpackedChunkData {
            materials: vec![Material::black(); CHUNK_VOLUME],
            scale,
        }
    }

    /// Neighborhood must be ordered so that X changes the fastest and Z changes the slowest, like
    /// in util::coord_iter_3d().
    pub fn from_smaller_chunks(neighborhood: &[&UnpackedChunkData]) -> UnpackedChunkData {
        debug_assert!(
            neighborhood.len() == 8,
            "Neighborhood must contain 8 chunks."
        );
        let smaller_scale = neighborhood[0].scale;
        for chunk in neighborhood {
            debug_assert!(
                chunk.scale == smaller_scale,
                "Scales of all component chunks must be equal."
            );
        }

        let mut new_data = Self::new(neighborhood[0].scale + 1);
        for (chunk, offset) in neighborhood.iter().zip(util::coord_iter_3d(2)) {
            let offset = util::scale_coord_3d(&offset, CHUNK_SIZE / 2);
            new_data.incorporate_materials_from_smaller_chunk(&chunk.materials, &offset);
        }
        new_data
    }

    fn incorporate_materials_from_smaller_chunk(
        &mut self,
        materials: &[Material],
        offset: &util::Coord3D,
    ) {
        for coord in util::coord_iter_3d(CHUNK_SIZE / 2) {
            let source_coord = util::scale_coord_3d(&coord, 2);
            let target_coord = util::offset_coord_3d(&coord, offset);
            let mut material = Material::black();
            for offset in util::coord_iter_3d(2) {
                let source_coord = util::offset_coord_3d(&source_coord, &offset);
                material.add(&materials[util::coord_to_index_3d(&source_coord, CHUNK_SIZE)]);
            }
            material.multiply(1.0 / 8.0);
            self.materials[util::coord_to_index_3d(&target_coord, CHUNK_SIZE)] = material;
        }
    }

    fn set_block(&mut self, coord: &util::Coord3D, value: Material) {
        self.materials[util::coord_to_index_3d(coord, CHUNK_SIZE)] = value;
    }

    pub fn pack(&self) -> PackedChunk {
        let mut packed_data = PackedChunkData::new();
        let mut lods = Vec::new();
        let mut lod_volume = CHUNK_VOLUME / 8;
        while lod_volume > 0 {
            lods.push(vec![false; lod_volume]);
            lod_volume /= 8;
        }

        for index in 0..CHUNK_VOLUME {
            // If there is a non-empty material at the index, mark the whole chunk as non-empty and
            // modify the LODs accordingly.
            if self.materials[index].power > 0.4 {
                let coord = util::index_to_coord_3d(index, CHUNK_SIZE);
                let mut lod_coord = util::shrink_coord_3d(&coord, 2);
                let mut lod_stride = CHUNK_SIZE / 2;
                for lod in &mut lods {
                    let index = util::coord_to_index_3d(&lod_coord, lod_stride);
                    if lod[index] {
                        break; // The LOD is already set, no need to continue.
                    } else {
                        lod[index] = true;
                        lod_coord = util::shrink_coord_3d(&lod_coord, 2);
                        lod_stride /= 2;
                    }
                }
            }
            packed_data.materials[index] = self.materials[index].pack();
        }

        // If the whole chunk is empty, return as such.
        if !lods[MAX_LOD - 1][0] {
            return PackedChunk::Empty { scale: self.scale };
        }

        // Pack the LODs into the minefield.
        for coord in util::coord_iter_3d(CHUNK_SIZE) {
            let index = util::coord_to_index_3d(&coord, CHUNK_SIZE);
            if self.materials[index].power > 0.4 {
                packed_data.minefield[index] = self.scale;
                continue;
            }
            let mut lod_coord = util::shrink_coord_3d(&coord, 2);
            let mut lod_stride = CHUNK_SIZE / 2;
            let mut current_lod = self.scale + 1;
            for lod in &lods {
                let lod_index = util::coord_to_index_3d(&lod_coord, lod_stride);
                if lod[lod_index] {
                    packed_data.minefield[index] = current_lod;
                    break;
                }
                lod_coord = util::shrink_coord_3d(&lod_coord, 2);
                lod_stride /= 2;
                current_lod += 1;
            }
        }

        PackedChunk::NonEmpty(packed_data)
    }
}

impl UnpackedChunkData {
    pub fn generate(chunk_coord: &util::Coord3D) -> UnpackedChunkData {
        let origin = util::scale_coord_3d(chunk_coord, CHUNK_SIZE);
        let mut data = UnpackedChunkData::new(0);

        let mountain_noise = functions::MountainNoise::new();
        let mut random = rand::thread_rng();
        let height =
            |x, y| (mountain_noise.get(x as f64 / 200.0, y as f64 / 200.0) * 80.0 + 160.0) as usize;
        let material = |random: &mut ThreadRng, height: usize| {
            let height = height as isize - 160;
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
                data.set_block(&(cx, cy, cz), MATERIALS[material_val].clone());
            }
        }

        data
    }
}
