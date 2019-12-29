use crate::render::constants::*;
use array_macro::array;
use std::collections::HashMap;

pub struct Chunk {
    pub block_data: [u16; CHUNK_BLOCK_VOLUME as usize],
}

impl Chunk {
    fn new() -> Chunk {
        Chunk {
            block_data: [0; CHUNK_BLOCK_VOLUME as usize],
        }
    }
}

pub struct Region {
    pub chunks: [Option<Box<Chunk>>; REGION_CHUNK_VOLUME as usize],
}

impl Region {
    fn new() -> Region {
        Region {
            chunks: array![None; (REGION_CHUNK_VOLUME as usize)],
        }
    }

    pub fn set_block(&mut self, coord: (u32, u32, u32), value: u16) {
        debug_assert!(
            coord.0 < REGION_BLOCK_WIDTH,
            "X must be in region boundaries."
        );
        debug_assert!(
            coord.1 < REGION_BLOCK_WIDTH,
            "Y must be in region boundaries."
        );
        debug_assert!(
            coord.2 < REGION_BLOCK_WIDTH,
            "Z must be in region boundaries."
        );

        let chunk_coord = (
            coord.0 / CHUNK_BLOCK_WIDTH,
            coord.1 / CHUNK_BLOCK_WIDTH,
            coord.2 / CHUNK_BLOCK_WIDTH,
        );
        let chunk_index = (chunk_coord.2 * REGION_CHUNK_WIDTH + chunk_coord.1) * REGION_CHUNK_WIDTH
            + chunk_coord.0;

        let block_coord = (
            coord.0 % CHUNK_BLOCK_WIDTH,
            coord.1 % CHUNK_BLOCK_WIDTH,
            coord.2 % CHUNK_BLOCK_WIDTH,
        );
        let block_index = (block_coord.2 * CHUNK_BLOCK_WIDTH + block_coord.1) * CHUNK_BLOCK_WIDTH 
            + block_coord.0;

        if let Some(chunk) = &mut self.chunks[chunk_index as usize] {
            chunk.block_data[block_index as usize] = value;
        } else {
            let mut new_chunk = Chunk::new();
            new_chunk.block_data[block_index as usize] = value;
            self.chunks[chunk_index as usize] = Some(Box::new(new_chunk));
        }
    }
}

pub type RegionGenerator = dyn FnMut(&mut Region, (u32, u32, u32)) -> bool;

pub struct World {
    regions: HashMap<(u32, u32, u32), Option<Box<Region>>>,
    generator: Box<RegionGenerator>,
}

impl World {
    pub fn new(generator: Box<RegionGenerator>) -> World {
        World {
            regions: HashMap::new(),
            generator,
        }
    }

    pub fn borrow_region(&mut self, coord: (u32, u32, u32)) -> Option<&Box<Region>> {
        if !self.regions.contains_key(&coord) {
            let mut new_region = Region::new();
            let not_empty = (self.generator)(&mut new_region, coord.clone());
            if not_empty {
                self.regions
                    .insert(coord.clone(), Some(Box::new(new_region)));
            } else {
                self.regions.insert(coord.clone(), None);
            }
        }
        self.regions
            .get(&coord)
            .expect("Region should have been generated previously in this function.")
            .as_ref()
    }
}
