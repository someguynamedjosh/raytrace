use rand::prelude::*;

use crate::render::{Material, MATERIALS};
use crate::util;

use super::functions;

// The index of the LOD that takes up an entire chunk.
pub const MAX_LOD: usize = 7;
pub const CHUNK_SIZE: usize = 1 << MAX_LOD; // 128
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

#[derive(Clone, PartialEq)]
pub struct PackedChunkData {
    pub minefield: Vec<u8>,
    pub materials: Vec<u32>,
}

impl PackedChunkData {
    fn new() -> PackedChunkData {
        PackedChunkData {
            minefield: vec![0; CHUNK_VOLUME],
            materials: vec![0; CHUNK_VOLUME],
        }
    }

    pub fn borrow_minefield(&self) -> &[u8] {
        &self.minefield[..]
    }

    pub fn borrow_materials(&self) -> &[u32] {
        &self.materials[..]
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

#[derive(Clone, PartialEq)]
pub struct UnpackedChunkData {
    pub materials: Vec<Material>,
    pub scale: u8,
}

impl UnpackedChunkData {
    /// Scale is what sized block one voxel in this chunk represents. 0 = smallest unit, 1 = double,
    /// 2 = quadruple, 3 = 8x and so on. It basically offsets every LOD value computed for the
    /// minefield in the pack function.
    fn new(scale: u8) -> UnpackedChunkData {
        UnpackedChunkData {
            materials: vec![Material::air(); CHUNK_VOLUME],
            scale,
        }
    }

    /// Neighborhood must be ordered so that X changes the fastest and Z changes the slowest, like
    /// in util::coord_iter_3d().
    pub fn from_smaller_chunks(neighborhood: &[&UnpackedChunkData]) -> UnpackedChunkData {
        let mut new_data = Self::new(neighborhood[0].scale + 1);
        new_data.replace_with_mip_of(neighborhood);
        new_data
    }

    pub fn replace_with_mip_of(&mut self, neighborhood: &[&UnpackedChunkData]) {
        debug_assert!(
            neighborhood.len() == 8,
            "Neighborhood must contain 8 chunks."
        );
        let smaller_scale = neighborhood[0].scale;
        for chunk in neighborhood {
            debug_assert!(
                chunk.scale == smaller_scale,
                "Scales of all component chunks must be equal, {} != {}.",
                chunk.scale,
                smaller_scale
            );
        }
        self.scale = neighborhood[0].scale + 1;
        for (chunk, offset) in neighborhood.iter().zip(util::coord_iter_3d(2)) {
            let offset = util::scale_coord_3d(&offset, CHUNK_SIZE / 2);
            self.incorporate_materials_from_smaller_chunk(&chunk.materials, &offset);
        }
    }

    pub fn get_scale(&self) -> u8 {
        self.scale
    }

    fn incorporate_materials_from_smaller_chunk(
        &mut self,
        materials: &[Material],
        offset: &util::Coord3D,
    ) {
        for coord in util::coord_iter_3d(CHUNK_SIZE / 2) {
            let source_coord = util::scale_coord_3d(&coord, 2);
            let source_index = util::coord_to_index_3d(&source_coord, CHUNK_SIZE);
            let target_coord = util::offset_coord_3d(&coord, offset);
            let mut material = Material::black();
            let mut power = 0;
            // Gives every index in a 2x2x2 neighborhood when added to the original index.
            for offset in [
                0,
                1,
                CHUNK_SIZE,
                CHUNK_SIZE + 1,
                CHUNK_SIZE * CHUNK_SIZE,
                CHUNK_SIZE * CHUNK_SIZE + 1,
                CHUNK_SIZE * CHUNK_SIZE + CHUNK_SIZE,
                CHUNK_SIZE * CHUNK_SIZE + CHUNK_SIZE + 1,
            ]
            .iter()
            {
                let source = &materials[source_index + offset];
                if source.solid {
                    material.add(source);
                    power += 1;
                }
            }
            if power > 3 {
                material.divide(power);
                self.materials[util::coord_to_index_3d(&target_coord, CHUNK_SIZE)] = material;
            } else {
                self.materials[util::coord_to_index_3d(&target_coord, CHUNK_SIZE)] = MATERIALS[0].clone();
            }
        }
    }

    fn set_block(&mut self, coord: &util::Coord3D, value: Material) {
        self.materials[util::coord_to_index_3d(coord, CHUNK_SIZE)] = value;
    }

    fn fill(&mut self, value: &Material) {
        for index in 0..CHUNK_VOLUME {
            self.materials[index] = value.clone();
        }
    }

    pub fn pack(&self) -> PackedChunkData {
        let mut data = PackedChunkData::new();
        self.pack_over(&mut data);
        data
    }

    pub fn pack_over(&self, packed_data: &mut PackedChunkData) {
        let mut lods = Vec::with_capacity(MAX_LOD);
        let mut lod_volume = CHUNK_VOLUME / 8;
        while lod_volume > 0 {
            lods.push(vec![false; lod_volume]);
            lod_volume /= 8;
        }

        for index in 0..CHUNK_VOLUME {
            // If there is a non-empty material at the index, mark the whole chunk as non-empty and
            // modify the LODs accordingly.
            if self.materials[index].solid {
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

        // If the whole chunk is empty, just fill the data.
        if !lods[MAX_LOD - 1][0] {
            for index in 0..CHUNK_VOLUME {
                packed_data.materials[index] = Material::air().pack();
                packed_data.minefield[index] = self.scale + MAX_LOD as u8;
            }
            return;
        }

        // Pack the LODs into the minefield.
        for index in 0..CHUNK_VOLUME {
            let coord = util::index_to_coord_3d(index, CHUNK_SIZE);
            if self.materials[index].solid {
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
    }
}

impl UnpackedChunkData {
    fn material(random: &mut ThreadRng, height: isize) -> usize {
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
    }

    pub fn empty() -> UnpackedChunkData {
        UnpackedChunkData::new(0)
    }

    pub fn generate(
        chunk_coord: &util::SignedCoord3D,
        heightmap: &super::Heightmap,
    ) -> UnpackedChunkData {
        let mut data = UnpackedChunkData::new(0);
        Self::generate_impl(&mut data, chunk_coord, heightmap);
        data
    }

    pub fn generate_over(
        data: &mut UnpackedChunkData,
        chunk_coord: &util::SignedCoord3D,
        heightmap: &super::Heightmap,
    ) {
        data.fill(&Material::air());
        Self::generate_impl(data, chunk_coord, heightmap);
    }

    fn generate_impl(
        data: &mut UnpackedChunkData,
        chunk_coord: &util::SignedCoord3D,
        heightmap: &super::Heightmap,
    ) {
        data.scale = 0;

        let origin = util::scale_signed_coord_3d(chunk_coord, CHUNK_SIZE as isize);

        let mut random = rand::thread_rng();

        if origin.2 + (CHUNK_SIZE as isize) < 12 {
            data.fill(&MATERIALS[2]);
        } else {
            for coord2d in util::coord_iter_2d(CHUNK_SIZE) {
                let height_val = heightmap.get(&coord2d);
                if height_val < origin.2 {
                    continue;
                }
                for z in origin.2..height_val.min(origin.2 + CHUNK_SIZE as isize) {
                    let material_val = Self::material(&mut random, z);
                    let cz = (z - origin.2) as usize;
                    data.set_block(&(coord2d.0, coord2d.1, cz), MATERIALS[material_val].clone());
                }
            }
        }
    }
}
