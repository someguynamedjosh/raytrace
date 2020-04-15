mod base;
mod chunk;
pub(self) mod functions;
mod heightmap;

pub use base::World;
pub use chunk::*;
pub use heightmap::*;

pub fn make_world() -> World {
    World::new()
}
