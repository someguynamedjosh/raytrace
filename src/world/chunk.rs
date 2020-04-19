use crate::render::Material;
use crate::util;

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
    pub fn new() -> PackedChunkData {
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

#[derive(Clone, PartialEq)]
pub struct UnpackedChunkData {
    pub materials: Vec<Material>,
    pub scale: u8,
}

impl UnpackedChunkData {
    /// Scale is what sized block one voxel in this chunk represents. 0 = smallest unit, 1 = double,
    /// 2 = quadruple, 3 = 8x and so on. It basically offsets every LOD value computed for the
    /// minefield in the pack function.
    pub fn new(scale: u8) -> UnpackedChunkData {
        UnpackedChunkData {
            materials: vec![Material::air(); CHUNK_VOLUME],
            scale,
        }
    }

    pub fn set_block(&mut self, coord: &util::Coord3D, value: Material) {
        self.materials[util::coord_to_index_3d(coord, CHUNK_SIZE)] = value;
    }

    pub fn fill(&mut self, value: &Material) {
        for index in 0..CHUNK_VOLUME {
            self.materials[index] = value.clone();
        }
    }

    pub fn pack_into(&self, packed_data: &mut PackedChunkData) {
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
