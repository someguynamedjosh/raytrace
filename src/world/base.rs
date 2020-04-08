use rand::prelude::*;
use std::collections::HashMap;

use crate::render::constants::*;
use crate::util;

use super::{functions, Chunk};

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
    chunks: HashMap<(usize, usize, usize), Chunk>,
}

impl World {
    pub fn new() -> World {
        let mut world = World {
            chunks: HashMap::new(),
        };
        world
    }

    pub fn borrow_chunk(&mut self, chunk_coord: &(usize, usize, usize)) -> &Chunk {
        if !self.chunks.contains_key(chunk_coord) {
            self.chunks.insert(chunk_coord.clone(), Chunk::generate(chunk_coord));
        }
        self.chunks.get(chunk_coord).unwrap()
    }
}
