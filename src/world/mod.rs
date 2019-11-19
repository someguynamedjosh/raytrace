use crate::render::constants::*;

use noise::{NoiseFn, HybridMulti};
use rand::{self, RngCore};

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

pub enum WorldChunk {
    Ungenerated,
    Empty,
    Occupied(Box<Chunk>),
}

impl Default for WorldChunk {
    fn default() -> Self {
        WorldChunk::Ungenerated
    }
}

pub struct World {
    pub chunks: Vec<WorldChunk>,
    pub regions: [bool; ROOT_REGION_VOLUME as usize],
}

impl World {
    pub fn new() -> World {
        let mut world = World {
            chunks: Vec::new(),
            regions: [false; ROOT_REGION_VOLUME as usize]
        };
        for _ in 0..ROOT_CHUNK_VOLUME {
            world.chunks.push(WorldChunk::Ungenerated);
        }
        world.generate();
        world.finalize();
        world
    }

    fn draw_block(&mut self, x: usize, y: usize, z: usize, value: u16) {
        let (cx, cy, cz) = (
            x / CHUNK_BLOCK_WIDTH as usize,
            y / CHUNK_BLOCK_WIDTH as usize,
            z / CHUNK_BLOCK_WIDTH as usize,
        );
        let (rx, ry, rz) = (
            cx / REGION_CHUNK_WIDTH as usize,
            cy / REGION_CHUNK_WIDTH as usize,
            cz / REGION_CHUNK_WIDTH as usize,
        );
        let (bx, by, bz) = (
            x % CHUNK_BLOCK_WIDTH as usize,
            y % CHUNK_BLOCK_WIDTH as usize,
            z % CHUNK_BLOCK_WIDTH as usize,
        );
        let chunk_index = (cz * ROOT_CHUNK_WIDTH as usize + cy) * ROOT_CHUNK_WIDTH as usize + cx;
        let region_index = (rz * ROOT_REGION_WIDTH as usize + ry) * ROOT_REGION_WIDTH as usize + rx;
        let block_index = (bz * CHUNK_BLOCK_WIDTH as usize + by) * CHUNK_BLOCK_WIDTH as usize + bx;
        if let WorldChunk::Ungenerated = self.chunks[chunk_index] {
            self.chunks[chunk_index] = WorldChunk::Occupied(Box::new(Chunk::new()));
        }
        if let WorldChunk::Occupied(chunk) = &mut self.chunks[chunk_index] {
            chunk.block_data[block_index] = value;
        }
        self.regions[region_index] = true;
    }

    fn generate(&mut self) {
        let mut perlin = HybridMulti::new();
        perlin.octaves = 4;
        perlin.frequency = 0.04;
        perlin.lacunarity = 4.0;
        perlin.persistence = 0.5;
        let mut micro = HybridMulti::new();
        micro.octaves = 1;
        micro.frequency = 30.0;
        micro.lacunarity = 2.0;
        micro.persistence = 1.0;
        let mut random = rand::thread_rng();
        let height = |x, y| {
            (perlin.get([x as f64 / 250.0, y as f64 / 250.0]) * 30.0 + 60.0) as usize
        };
        for x in 2..ROOT_BLOCK_WIDTH as usize {
            for y in 2..ROOT_BLOCK_WIDTH as usize {
                let mut h0 = height(x, y);
                let (mut h1, mut h2, mut h3, mut h4) = (height(x + 2, y + 2), height(x - 2, y + 2), height(x + 2, y - 2), height(x - 2, y - 2));
                h0 -= h0 % 4;
                h1 -= h1 % 4;
                h2 -= h2 % 4;
                h3 -= h3 % 4;
                h4 -= h4 % 4;
                let lip = h1 < h0 || h2 < h0 || h3 < h0 || h4 < h0;
                if lip {
                    h0 += 1;
                }
                if x == 200 && y == 200 {
                    h0 += 8;
                }
                for z in 0..h0 {
                    self.draw_block(x, y, z, if z == h0 - 1 { 
                        if lip {
                            1
                        } else {
                            4
                        }
                    } else { 
                        3 
                    });
                }
                if x > 15 && y > 15 && x < ROOT_BLOCK_WIDTH as usize - 15 && y < ROOT_BLOCK_WIDTH as usize - 15 && random.next_u32() % 10000 == 1 {
                    for z in h0..h0 + 4 {
                        self.draw_block(x, y, z, 3);
                        self.draw_block(x+1, y, z, 3);
                        self.draw_block(x, y+1, z, 3);
                        self.draw_block(x-1, y, z, 3);
                        self.draw_block(x, y-1, z, 3);
                    }
                    for dx in 0..11 { for dy in 0..11 { for dz in 0..11 {
                        let radius = (dx as isize - 5).abs() + (dy as isize - 5).abs() + (dz as isize - 5).abs();
                        if radius < 8 {
                            if dx == 5 || dy == 5 || dz == 5 {
                                if radius < 7 {
                                    self.draw_block(x + dx - 5, y + dy - 5, h0 + dz + 4, 3);
                                }
                            } else {
                                self.draw_block(x + dx - 5, y + dy - 5, h0 + dz + 4, 2);
                            }
                        }
                    }}}
                }
            }
        }
    }

    fn finalize(&mut self) {
        for i in 0..ROOT_CHUNK_VOLUME as usize {
            if let WorldChunk::Ungenerated = self.chunks[i] {
                self.chunks[i] = WorldChunk::Empty;
            }
        }
    }
}