use rand::prelude::*;
use std::collections::HashMap;

use crate::render::constants::*;
use crate::util;

use super::{functions, Chunk, ChunkMip};

/*
0:512
1:256
2:128
3:64
4:32
5:16
6:8
7:4
8:2
9:1
*/

pub struct World {
    chunks: HashMap<util::Coord3D, Chunk>,
    lod1_mips: HashMap<util::Coord3D, ChunkMip>,
}

impl World {
    pub fn new() -> World {
        let mut world = World {
            chunks: HashMap::new(),
            lod1_mips: HashMap::new(),
        };
        world
    }

    pub fn borrow_chunk(&mut self, chunk_coord: &util::Coord3D) -> &Chunk {
        if !self.chunks.contains_key(chunk_coord) {
            self.chunks.insert(chunk_coord.clone(), Chunk::generate(chunk_coord));
        }
        self.chunks.get(chunk_coord).unwrap()
    }

    pub fn borrow_lod1_mip(&mut self, chunk_coord: &util::Coord3D) -> &ChunkMip {
        if !self.lod1_mips.contains_key(chunk_coord) {
            let lod0_coord = util::scale_coord_3d(chunk_coord, 2);
            let mut neighborhood = Vec::new();
            for offset in util::coord_iter_3d(2) {
                self.borrow_chunk(&util::offset_coord_3d(&lod0_coord, &offset));
            }
            for offset in util::coord_iter_3d(2) {
                // Unwrap is safe because we just ensured they all exist with borrow_chunk above.
                let chunk = self.chunks.get(&util::offset_coord_3d(&lod0_coord, &offset)).unwrap();
                neighborhood.push(chunk);
            }
            let mip = ChunkMip::from_chunks(&neighborhood[..]);
            self.lod1_mips.insert(chunk_coord.clone(), mip);
        }
        self.lod1_mips.get(chunk_coord).unwrap()
    }
}
