pub mod base;
pub use base::{World};
pub(self) mod functions;

pub fn make_world() -> World {
    World::new()
}