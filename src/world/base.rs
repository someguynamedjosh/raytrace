use array_macro::array;
use std::collections::HashMap;

use crate::util;

use super::{Heightmap, UnpackedChunkData};

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
    lods: Vec<HashMap<util::SignedCoord3D, UnpackedChunkData>>,
    heightmaps: HashMap<util::SignedCoord2D, Heightmap>,
    temp_chunks: [UnpackedChunkData; 32],
    temp_chunks_in_use: usize,
}

impl World {
    pub fn new() -> World {
        World {
            // Just create the first LOD for now. Other LODs can be created on demand.
            lods: vec![HashMap::new()],
            heightmaps: HashMap::new(),
            temp_chunks: array![UnpackedChunkData::empty(); 32],
            temp_chunks_in_use: 0,
        }
    }

    fn get_heightmap(&mut self, chunk_coord: &util::SignedCoord2D) -> &Heightmap {
        if !self.heightmaps.contains_key(chunk_coord) {
            self.heightmaps
                .insert(chunk_coord.clone(), Heightmap::generate(chunk_coord));
        }
        self.heightmaps.get(chunk_coord).unwrap()
    }

    fn temp_generate_chunk(&mut self, chunk_coord: &util::SignedCoord3D, lod: usize) -> usize {
        if lod == 0 {
            let coord2d = (chunk_coord.0, chunk_coord.1);
            self.get_heightmap(&coord2d);
            let heightmap_ref = self.heightmaps.get(&coord2d).unwrap();
            let data_ref = &mut self.temp_chunks[self.temp_chunks_in_use];
            UnpackedChunkData::generate_over(data_ref, chunk_coord, heightmap_ref);
            self.temp_chunks_in_use += 1;
            self.temp_chunks_in_use - 1
        } else {
            let next_lod_coord = util::scale_signed_coord_3d(chunk_coord, 2);
            self.temp_chunks_in_use += 1;
            let mut neighborhood = Vec::with_capacity(8);
            for offset in util::coord_iter_3d(2) {
                let offset = util::coord_to_signed_coord(&offset);
                let coord = util::offset_signed_coord_3d(&next_lod_coord, &offset);
                neighborhood.push(self.temp_generate_chunk(&coord, lod - 1));
            }
            self.temp_chunks_in_use -= 8;
            let target_chunk =
                &mut self.temp_chunks[self.temp_chunks_in_use - 1] as *mut UnpackedChunkData;
            let mut real_neighborhood = Vec::with_capacity(8);
            for index in neighborhood {
                real_neighborhood.push(&self.temp_chunks[index]);
            }
            unsafe {
                // This is safe because target_chunk appears in the array before any of the
                // neighborhood chunks.
                (*target_chunk).replace_with_mip_of(&real_neighborhood[..]);
            }
            self.temp_chunks_in_use - 1
        }
    }

    fn checked_generate_chunk(&mut self, chunk_coord: &util::SignedCoord3D, lod: usize) {
        if self.lods[lod].contains_key(chunk_coord) {
            return;
        }
        if lod == 0 {
            let heightmap_ref = self.get_heightmap(&(chunk_coord.0, chunk_coord.1));
            let chunk = UnpackedChunkData::generate(chunk_coord, heightmap_ref);
            self.lods[0].insert(chunk_coord.clone(), chunk);
        } else {
            // Coordinate of this "chunk" in the next LOD down.
            let next_lod_coord = util::scale_signed_coord_3d(chunk_coord, 2);
            let mut neighborhood = Vec::new();
            // Ensure that all the chunks we will need are generated.
            for offset in util::coord_iter_3d(2) {
                let offset = util::coord_to_signed_coord(&offset);
                let coord = util::offset_signed_coord_3d(&next_lod_coord, &offset);
                self.checked_generate_chunk(&coord, lod - 1);
            }
            for offset in util::coord_iter_3d(2) {
                let offset = util::coord_to_signed_coord(&offset);
                let coord = util::offset_signed_coord_3d(&next_lod_coord, &offset);
                let chunk = self.lods[lod - 1].get(&coord).unwrap();
                neighborhood.push(chunk);
            }
            let new_data = UnpackedChunkData::from_smaller_chunks(&neighborhood);
            // For now, this is here to prevent the program from filling up all the RAM. In the
            // future, we will need a system which dynamically stores stuff to the disk when memory
            // is getting too full.
            for offset in util::coord_iter_3d(2) {
                let offset = util::coord_to_signed_coord(&offset);
                let coord = util::offset_signed_coord_3d(&next_lod_coord, &offset);
                self.lods[lod - 1].remove(&coord);
            }
            self.lods[lod].insert(chunk_coord.clone(), new_data);
        }
    }

    pub fn borrow_chunk(
        &mut self,
        chunk_coord: &util::SignedCoord3D,
        lod: usize,
    ) -> &UnpackedChunkData {
        let index = self.temp_generate_chunk(chunk_coord, lod);
        self.temp_chunks_in_use -= 1;
        &self.temp_chunks[index]
    }
}
