mod base;
mod chunk;
mod chunk_storage;
pub(self) mod functions;
mod heightmap;

pub use base::World;
pub use chunk::*;
pub use chunk_storage::*;
pub use heightmap::*;

pub fn make_world() -> World {
    World::new()
}
