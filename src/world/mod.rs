mod base;
mod chunk;
pub(self) mod functions;

pub use base::World;
pub use chunk::*;

pub fn make_world() -> World {
    World::new()
}
