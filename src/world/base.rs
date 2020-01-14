use super::functions;
use crate::render::constants::*;
use crate::util;
use rand::prelude::*;

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
    pub content_lod0: Vec<u16>,
    pub content_lod1: Vec<u16>,
    pub content_lod2: Vec<u16>,
    pub content_lod3: Vec<u16>,
    pub content_lod4: Vec<u16>,
    pub content_lod5: Vec<u16>,
    pub content_lod6: Vec<u16>,
    pub content_lod7: Vec<u16>,
    pub content_lod8: Vec<u16>,
    pub content_lod9: Vec<u16>,
}

impl World {
    pub fn new() -> World {
        let mut world = World {
            content_lod0: vec![0; ROOT_BLOCK_VOLUME as usize],
            content_lod1: vec![0; ROOT_BLOCK_VOLUME as usize / 8],
            content_lod2: vec![0; ROOT_BLOCK_VOLUME as usize / 64],
            content_lod3: vec![0; ROOT_BLOCK_VOLUME as usize / 512],
            content_lod4: vec![0; ROOT_BLOCK_VOLUME as usize / 4096],
            content_lod5: vec![0; ROOT_BLOCK_VOLUME as usize / 4096 / 8],
            content_lod6: vec![0; ROOT_BLOCK_VOLUME as usize / 4096 / 64],
            content_lod7: vec![0; ROOT_BLOCK_VOLUME as usize / 4096 / 512],
            content_lod8: vec![0; ROOT_BLOCK_VOLUME as usize / 4096 / 4096],
            content_lod9: vec![0; ROOT_BLOCK_VOLUME as usize / 4096 / 4096 / 8],
        };
        world.generate();
        world
    }

    #[inline]
    fn set_block(&mut self, x: u32, y: u32, z: u32, value: u16) {
        let index = util::coord_to_index_3d(&(x, y, z), ROOT_BLOCK_WIDTH) as usize;
        let was_empty = self.content_lod0[index] == 0;
        self.content_lod0[index] = value;
        if !was_empty || value == 0 {
            return; // Don't set any of the lower-res LODs.
        }
        let scale = 2;
        let coord = (x / scale, y / scale, z / scale);
        self.content_lod1[util::coord_to_index_3d(&coord, ROOT_BLOCK_WIDTH / scale) as usize] = 2;
        let scale = 4;
        let coord = (x / scale, y / scale, z / scale);
        self.content_lod2[util::coord_to_index_3d(&coord, ROOT_BLOCK_WIDTH / scale) as usize] = 2;
        let scale = 8;
        let coord = (x / scale, y / scale, z / scale);
        self.content_lod3[util::coord_to_index_3d(&coord, ROOT_BLOCK_WIDTH / scale) as usize] = 2;
        let scale = 16;
        let coord = (x / scale, y / scale, z / scale);
        self.content_lod4[util::coord_to_index_3d(&coord, ROOT_BLOCK_WIDTH / scale) as usize] = 2;
        let scale = 32;
        let coord = (x / scale, y / scale, z / scale);
        self.content_lod5[util::coord_to_index_3d(&coord, ROOT_BLOCK_WIDTH / scale) as usize] = 2;
        let scale = 64;
        let coord = (x / scale, y / scale, z / scale);
        self.content_lod6[util::coord_to_index_3d(&coord, ROOT_BLOCK_WIDTH / scale) as usize] = 2;
        let scale = 128;
        let coord = (x / scale, y / scale, z / scale);
        self.content_lod7[util::coord_to_index_3d(&coord, ROOT_BLOCK_WIDTH / scale) as usize] = 2;
        let scale = 256;
        let coord = (x / scale, y / scale, z / scale);
        self.content_lod8[util::coord_to_index_3d(&coord, ROOT_BLOCK_WIDTH / scale) as usize] = 2;
        let scale = 512;
        let coord = (x / scale, y / scale, z / scale);
        self.content_lod9[util::coord_to_index_3d(&coord, ROOT_BLOCK_WIDTH / scale) as usize] = 2;
    }

    fn generate(&mut self) {
        let mountain_noise = functions::MountainNoise::new();
        let mut random = rand::thread_rng();
        let height =
            |x, y| (mountain_noise.get(x as f64 / 200.0, y as f64 / 200.0) * 80.0 + 10.0) as u32;
        let material = |random: &mut ThreadRng, height| {
            if height < 12 {
                2
            } else if height < 30 {
                let threshold = height - 12;
                if random.next_u32() % (30 - 12) < threshold as u32 {
                    5
                } else {
                    2
                }
            } else if height < 35 {
                5
            } else if height < 60 {
                let threshold = height - 35;
                if random.next_u32() % (60 - 35) < threshold as u32 {
                    6
                } else {
                    5
                }
            } else {
                6
            }
        };

        for (x, y) in util::coord_iter_2d(ROOT_BLOCK_WIDTH) {
            if x == 0 {
                println!("Generating {} of {}", y, ROOT_BLOCK_WIDTH);
            }
            let height = height(x, y);
            for z in 0..height {
                self.set_block(x, y, z, material(&mut random, z));
            }
        }
    }
}
