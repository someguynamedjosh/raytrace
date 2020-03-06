use rand::prelude::*;

use crate::render::constants::*;
use crate::render::{Material, MATERIALS};
use crate::util;

use super::functions;

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

    pub material_lod1: Vec<Material>,
    pub material_lod2: Vec<Material>,
    pub material_lod3: Vec<Material>,
    pub material_lod4: Vec<Material>,
    pub material_lod5: Vec<Material>,
    pub material_lod6: Vec<Material>,
    pub material_lod7: Vec<Material>,
    pub material_lod8: Vec<Material>,
    pub material_lod9: Vec<Material>,

    pub materialbuf_lod1: Vec<u64>,
    pub materialbuf_lod2: Vec<u64>,
    pub materialbuf_lod3: Vec<u64>,
    pub materialbuf_lod4: Vec<u64>,
    pub materialbuf_lod5: Vec<u64>,
    pub materialbuf_lod6: Vec<u64>,
    pub materialbuf_lod7: Vec<u64>,
    pub materialbuf_lod8: Vec<u64>,
    pub materialbuf_lod9: Vec<u64>,
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

            material_lod1: vec![Material::black(); ROOT_BLOCK_VOLUME as usize / 8],
            material_lod2: vec![Material::black(); ROOT_BLOCK_VOLUME as usize / 64],
            material_lod3: vec![Material::black(); ROOT_BLOCK_VOLUME as usize / 512],
            material_lod4: vec![Material::black(); ROOT_BLOCK_VOLUME as usize / 4096],
            material_lod5: vec![Material::black(); ROOT_BLOCK_VOLUME as usize / 4096 / 8],
            material_lod6: vec![Material::black(); ROOT_BLOCK_VOLUME as usize / 4096 / 64],
            material_lod7: vec![Material::black(); ROOT_BLOCK_VOLUME as usize / 4096 / 512],
            material_lod8: vec![Material::black(); ROOT_BLOCK_VOLUME as usize / 4096 / 4096],
            material_lod9: vec![Material::black(); ROOT_BLOCK_VOLUME as usize / 4096 / 4096 / 8],

            materialbuf_lod1: vec![0; ROOT_BLOCK_VOLUME as usize / 8],
            materialbuf_lod2: vec![0; ROOT_BLOCK_VOLUME as usize / 64],
            materialbuf_lod3: vec![0; ROOT_BLOCK_VOLUME as usize / 512],
            materialbuf_lod4: vec![0; ROOT_BLOCK_VOLUME as usize / 4096],
            materialbuf_lod5: vec![0; ROOT_BLOCK_VOLUME as usize / 4096 / 8],
            materialbuf_lod6: vec![0; ROOT_BLOCK_VOLUME as usize / 4096 / 64],
            materialbuf_lod7: vec![0; ROOT_BLOCK_VOLUME as usize / 4096 / 512],
            materialbuf_lod8: vec![0; ROOT_BLOCK_VOLUME as usize / 4096 / 4096],
            materialbuf_lod9: vec![0; ROOT_BLOCK_VOLUME as usize / 4096 / 4096 / 8],
        };
        world.generate();
        world
    }

    pub fn min_lod_at_coord(&self, x: u32, y: u32, z: u32) -> u8 {
        let coord = (x, y, z);
        let index = util::coord_to_index_3d(&coord, ROOT_BLOCK_WIDTH) as usize;
        if self.content_lod0[index] > 0 {
            return 0;
        }
        let coord = util::shrink_coord_3d(&coord, 2);
        let index = util::coord_to_index_3d(&coord, ROOT_BLOCK_WIDTH / 2) as usize;
        if self.content_lod1[index] > 0 {
            return 1;
        }
        let coord = util::shrink_coord_3d(&coord, 2);
        let index = util::coord_to_index_3d(&coord, ROOT_BLOCK_WIDTH / 4) as usize;
        if self.content_lod2[index] > 0 {
            return 2;
        }
        let coord = util::shrink_coord_3d(&coord, 2);
        let index = util::coord_to_index_3d(&coord, ROOT_BLOCK_WIDTH / 8) as usize;
        if self.content_lod3[index] > 0 {
            return 3;
        }
        let coord = util::shrink_coord_3d(&coord, 2);
        let index = util::coord_to_index_3d(&coord, ROOT_BLOCK_WIDTH / 16) as usize;
        if self.content_lod4[index] > 0 {
            return 4;
        }
        let coord = util::shrink_coord_3d(&coord, 2);
        let index = util::coord_to_index_3d(&coord, ROOT_BLOCK_WIDTH / 32) as usize;
        if self.content_lod5[index] > 0 {
            return 5;
        }
        let coord = util::shrink_coord_3d(&coord, 2);
        let index = util::coord_to_index_3d(&coord, ROOT_BLOCK_WIDTH / 64) as usize;
        if self.content_lod6[index] > 0 {
            return 6;
        }
        let coord = util::shrink_coord_3d(&coord, 2);
        let index = util::coord_to_index_3d(&coord, ROOT_BLOCK_WIDTH / 128) as usize;
        if self.content_lod7[index] > 0 {
            return 7;
        }
        let coord = util::shrink_coord_3d(&coord, 2);
        let index = util::coord_to_index_3d(&coord, ROOT_BLOCK_WIDTH / 256) as usize;
        if self.content_lod8[index] > 0 {
            return 8;
        }
        9
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

    fn compute_material_for(&mut self, x: u32, y: u32, z: u32, lod: u32) -> &Material {
        let lod_size = ROOT_BLOCK_WIDTH >> lod;
        let target_index = util::coord_to_index_3d(&(x, y, z), lod_size) as usize;
        let (sx, sy, sz) = (x * 2, y * 2, z * 2);
        let source_indices = [
            util::coord_to_index_3d(&(sx + 0, sy + 0, sz + 0), lod_size << 1) as usize,
            util::coord_to_index_3d(&(sx + 1, sy + 0, sz + 0), lod_size << 1) as usize,
            util::coord_to_index_3d(&(sx + 0, sy + 1, sz + 0), lod_size << 1) as usize,
            util::coord_to_index_3d(&(sx + 1, sy + 1, sz + 0), lod_size << 1) as usize,
            util::coord_to_index_3d(&(sx + 0, sy + 0, sz + 1), lod_size << 1) as usize,
            util::coord_to_index_3d(&(sx + 1, sy + 0, sz + 1), lod_size << 1) as usize,
            util::coord_to_index_3d(&(sx + 0, sy + 1, sz + 1), lod_size << 1) as usize,
            util::coord_to_index_3d(&(sx + 1, sy + 1, sz + 1), lod_size << 1) as usize,
        ];

        fn add_mat(target: &mut Material, source: &Material) {
            target.albedo.0 += source.albedo.0;
            target.albedo.1 += source.albedo.1;
            target.albedo.2 += source.albedo.2;
            target.emission.0 += source.emission.0;
            target.emission.1 += source.emission.1;
            target.emission.2 += source.emission.2;
        }

        fn pack_mat(source: &Material, divisor: f32) -> u64 {
            let ar = (source.albedo.0 / divisor * 255.0) as u64;
            let ag = (source.albedo.1 / divisor * 255.0) as u64;
            let ab = (source.albedo.2 / divisor * 255.0) as u64;
            let er = (source.emission.0 / divisor / 4.0 * 255.0) as u64;
            let eg = (source.emission.1 / divisor / 4.0 * 255.0) as u64;
            let eb = (source.emission.2 / divisor / 4.0 * 255.0) as u64;
            (ar << 40) | (ag << 32) | (ab << 24) | (er << 16) | (eg << 8) | (eb << 0)
        }

        if lod == 1 {
            for index in &source_indices {
                let target = match lod {
                    1 => &mut self.material_lod1[target_index],
                    2 => &mut self.material_lod2[target_index],
                    3 => &mut self.material_lod3[target_index],
                    4 => &mut self.material_lod4[target_index],
                    5 => &mut self.material_lod5[target_index],
                    6 => &mut self.material_lod6[target_index],
                    7 => &mut self.material_lod7[target_index],
                    8 => &mut self.material_lod8[target_index],
                    9 => &mut self.material_lod9[target_index],
                    _ => unreachable!(),
                };
                add_mat(target, &MATERIALS[self.content_lod0[*index] as usize]);
            }
        } else {
            for index in &source_indices {
                let (x, y, z) = util::index_to_coord_3d(*index as u32, lod_size << 1);
                let material = self.compute_material_for(x, y, z, lod - 1).clone();

                let target = match lod {
                    1 => &mut self.material_lod1[target_index],
                    2 => &mut self.material_lod2[target_index],
                    3 => &mut self.material_lod3[target_index],
                    4 => &mut self.material_lod4[target_index],
                    5 => &mut self.material_lod5[target_index],
                    6 => &mut self.material_lod6[target_index],
                    7 => &mut self.material_lod7[target_index],
                    8 => &mut self.material_lod8[target_index],
                    9 => &mut self.material_lod9[target_index],
                    _ => unreachable!(),
                };
                add_mat(target, &material);
            }
        }

        let material = match lod {
            1 => &self.material_lod1[target_index],
            2 => &self.material_lod2[target_index],
            3 => &self.material_lod3[target_index],
            4 => &self.material_lod4[target_index],
            5 => &self.material_lod5[target_index],
            6 => &self.material_lod6[target_index],
            7 => &self.material_lod7[target_index],
            8 => &self.material_lod8[target_index],
            9 => &self.material_lod9[target_index],
            _ => unreachable!(),
        };
        let storage_target = match lod {
            1 => &mut self.materialbuf_lod1[target_index],
            2 => &mut self.materialbuf_lod2[target_index],
            3 => &mut self.materialbuf_lod3[target_index],
            4 => &mut self.materialbuf_lod4[target_index],
            5 => &mut self.materialbuf_lod5[target_index],
            6 => &mut self.materialbuf_lod6[target_index],
            7 => &mut self.materialbuf_lod7[target_index],
            8 => &mut self.materialbuf_lod8[target_index],
            9 => &mut self.materialbuf_lod9[target_index],
            _ => unreachable!(),
        };
        *storage_target = pack_mat(material, (8.0f32).powi(lod as i32));
        material
    }

    fn compute_material_lods(&mut self) {
        self.compute_material_for(0, 0, 0, 9);
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
            let height = height(x, y);
            for z in 0..height {
                self.set_block(x, y, z, material(&mut random, z));
            }
        }

        self.compute_material_lods();
    }
}
