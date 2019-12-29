pub mod base;
pub use base::{Chunk, Region, World};
pub(self) mod functions;
mod generator;

pub fn make_world() -> World {
    World::new(Box::new(generator::generate))
}