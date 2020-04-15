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
}

impl World {
    pub fn new() -> World {
        World {
            // Just create the first LOD for now. Other LODs can be created on demand.
            lods: vec![HashMap::new()],
            heightmaps: HashMap::new(),
        }
    }

    fn get_heightmap(&mut self, chunk_coord: &util::SignedCoord2D) -> &Heightmap {
        if !self.heightmaps.contains_key(chunk_coord) {
            self.heightmaps
                .insert(chunk_coord.clone(), Heightmap::generate(chunk_coord));
        }
        self.heightmaps.get(chunk_coord).unwrap()
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
        if self.lods.len() <= lod {
            for _ in self.lods.len()..(lod + 1) {
                self.lods.push(HashMap::new());
            }
        }
        // Ensure the chunk exists. (This function returns early if the chunk already exists.)
        self.checked_generate_chunk(chunk_coord, lod);
        self.lods[lod].get(chunk_coord).unwrap()
    }
}
