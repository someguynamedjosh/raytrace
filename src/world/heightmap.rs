use crate::util;

pub struct Heightmap {
    pub(super) data: Vec<isize>,
}

impl Heightmap {
    pub fn new() -> Heightmap {
        Heightmap {
            data: vec![0; super::CHUNK_SIZE * super::CHUNK_SIZE],
        }
    }

    pub fn get(&self, coord: &(usize, usize)) -> isize {
        self.data[util::coord_to_index_2d(&coord, super::CHUNK_SIZE)]
    }
}
