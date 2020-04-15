use lazy_static::lazy_static;

use crate::util;

use super::functions;

pub struct Heightmap {
    data: Vec<isize>,
}

lazy_static! {
    static ref MOUNTAIN_NOISE: functions::MountainNoise2 = functions::MountainNoise2::new();
}

impl Heightmap {
    pub fn get(&self, coord: &(usize, usize)) -> isize {
        self.data[util::coord_to_index_2d(&coord, super::CHUNK_SIZE)]
    }

    fn height(x: isize, y: isize) -> isize {
        (MOUNTAIN_NOISE.get(x as f64 / 200.0, y as f64 / 200.0) * 400.0 + 10.0) as isize
    }

    pub fn generate(chunk_coord: &util::SignedCoord2D) -> Heightmap {
        let origin = util::scale_signed_coord_2d(chunk_coord, super::CHUNK_SIZE as isize);
        let mut heightmap = Heightmap {
            data: Vec::with_capacity(super::CHUNK_SIZE * super::CHUNK_SIZE)
        };

        for (x, y) in util::coord_iter_2d(super::CHUNK_SIZE) {
            heightmap.data.push(Self::height(origin.0 + x as isize, origin.1 + y as isize));
        }

        heightmap
    }
}