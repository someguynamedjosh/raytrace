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
}
