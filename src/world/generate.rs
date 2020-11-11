use super::{functions, Heightmap, UnpackedChunkData};
use crate::render::{constants::*, Material, MATERIALS};
use crate::util::{self, prelude::*};
use lazy_static::lazy_static;
use rand::prelude::*;

lazy_static! {
    static ref MOUNTAIN_NOISE: functions::MountainNoise2 = functions::MountainNoise2::new();
}

const SCALE: f64 = 0600.0;

fn height(x: isize, y: isize) -> isize {
    (MOUNTAIN_NOISE.get(x as f64 / SCALE, y as f64 / SCALE) * SCALE * 0.2 + 10.0) as isize
}

pub fn generate_heightmap(
    data: &mut Heightmap,
    chunk_coord: &util::SignedCoord2D,
) {
    let origin = util::scale_signed_coord_2d(chunk_coord, CHUNK_SIZE as isize);

    let mut index = 0;
    for (x, y) in util::coord_iter_2d(CHUNK_SIZE) {
        let (x, y) = (x as isize, y as isize);
        data.data[index] = height(origin.0 + x, origin.1 + y);
        index += 1;
    }
}

fn material(random: &mut ThreadRng, height: isize) -> usize {
    if height < 20 {
        2
    } else if height < 80 {
        let threshold = (height - 20) as u32;
        if random.next_u32() % (80 - 20) < threshold {
            5
        } else {
            2
        }
    } else if height < 160 {
        let threshold = (height - 80) as u32;
        if random.next_u32() % (160 - 80) < threshold {
            6
        } else {
            5
        }
    } else {
        6
    }
}

pub fn generate_chunk(
    data: &mut UnpackedChunkData,
    chunk_coord: &util::SignedCoord3D,
    heightmap: &super::Heightmap,
) {
    let size = CHUNK_SIZE as isize;
    let origin = chunk_coord.scale(size);

    let mut random = rand::thread_rng();

    if origin.2 + size < 12 {
        data.fill(&MATERIALS[2]);
    } else {
        for coord2d in util::coord_iter_2d(CHUNK_SIZE) {
            let height_val = heightmap.get(&coord2d);
            if height_val < origin.2 {
                for cz in 0..CHUNK_SIZE {
                    data.set_block(&(coord2d.0, coord2d.1, cz), Material::air());
                }
                continue;
            }
            for lz in 0..CHUNK_SIZE {
                let z = origin.2 + lz as isize;
                if z >= height_val {
                    data.set_block(&(coord2d.0, coord2d.1, lz), Material::air());
                    continue;
                }
                let material_val = material(&mut random, z);
                data.set_block(&(coord2d.0, coord2d.1, lz), MATERIALS[material_val].clone());
            }
        }
    }
}
