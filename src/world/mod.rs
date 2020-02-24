pub mod base;
pub(self) mod functions;

pub use base::World;

pub fn make_world() -> World {
    World::new()
}
