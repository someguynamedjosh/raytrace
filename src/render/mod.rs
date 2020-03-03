use std::rc::Rc;
use winit::event_loop::EventLoop;

mod GEN_MATERIALS;
pub mod constants;
pub(self) mod general;
pub(self) mod pipeline;
pub(self) mod util;

pub use general::core::Core;
pub use pipeline::Pipeline;
pub use GEN_MATERIALS::*;

// Positive Y (angle PI / 2) is forward
// Positive X is to the right
// Positive Z is up
// Heading starts at Positive X and goes clockwise (towards Positive Y).
// Pitch starts at zero and positive pitch looks up at Positive Z.
#[derive(Debug)]
pub struct Camera {
    pub origin: cgmath::Vector3<f32>,
    pub heading: cgmath::Rad<f32>,
    pub pitch: cgmath::Rad<f32>,
}

impl Camera {
    pub fn new() -> Camera {
        Camera {
            origin: [0.0; 3].into(),
            heading: cgmath::Rad(1.0),
            pitch: cgmath::Rad(0.0),
        }
    }
}

pub fn create_instance(
    event_loop: &EventLoop<()>,
    game: &crate::game::Game,
) -> (Rc<Core>, Pipeline) {
    let core = Rc::new(Core::new(event_loop));
    let pipeline = Pipeline::new(core.clone(), game);
    (core, pipeline)
}
