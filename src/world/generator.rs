use crate::render::constants::*;
use crate::world::Region;
use super::functions;

use rand::{self, prelude::*};

pub fn generate(region: &mut Region, position: (u32, u32, u32)) -> bool {
    println!("GENERATING REGION...... {:?}", position);
    let offset = (
        position.0 * REGION_BLOCK_WIDTH,
        position.1 * REGION_BLOCK_WIDTH,
        position.2 * REGION_BLOCK_WIDTH,
    );
    let mountain_noise = functions::MountainNoise::new();
    let mut random = rand::thread_rng();
    let height = |x, y| {
        (mountain_noise.get(x as f64 / 200.0, y as f64 / 200.0) * 80.0 + 10.0) as u32
    };
    let material = |random: &mut ThreadRng, height| {
        if height < 12 {
            1
        } else if height < 30 {
            let threshold = height - 12;
            if random.next_u32() % (30 - 12) < threshold as u32 {
                4
            } else {
                1
            }
        } else if height < 35 {
            4
        } else if height < 60 {
            let threshold = height - 35;
            if random.next_u32() % (60 - 35) < threshold as u32 {
                5
            } else {
                4
            }
        } else {
            5
        }
    };

    let mut not_empty = false;

    for x in 0..REGION_BLOCK_WIDTH {
        for y in 0..REGION_BLOCK_WIDTH {
            let height = height(x + offset.0, y + offset.1);
            if height > offset.2 {
                let height = height - offset.2;
                for z in 0..height.min(REGION_BLOCK_WIDTH) {
                    region.set_block((x, y, z), material(&mut random, z + offset.2));
                }
                not_empty = true;
            }
        }
    }

    not_empty
}