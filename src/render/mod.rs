use winit::event_loop::EventLoop;

use crate::game::Game;

pub(self) mod commands;
pub mod constants;
pub(self) mod core;
pub(self) mod debug;
pub(self) mod descriptors;
pub(self) mod pipeline;
pub(self) mod platform_specific;
pub(self) mod structures;
pub(self) mod util;

use self::core::Core;
use pipeline::Pipeline;

// Positive Y (angle PI / 2) is forward
// Positive X is to the right
// Positive Z is up
// Heading starts at Positive X and goes clockwise (towards Positive Y).
// Pitch starts at zero and positive pitch looks up at Positive Z.
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


pub struct VulkanApp {
    core: Core,
    pipeline: Pipeline,
    game: Game,
}

impl VulkanApp {
    pub fn new(event_loop: &EventLoop<()>) -> VulkanApp {
        let core = Core::new(event_loop);
        let pipeline = Pipeline::new(&core);
        VulkanApp { 
            core, 
            pipeline,
            game: Game::new(),
        }
    }

    pub fn on_mouse_move(&mut self, x: f64, y: f64) {
        self.game.on_mouse_move(x, y);
    }

    pub fn tick(&mut self, dt: f32) {
        self.game.tick(dt as f32)
    }

    pub fn draw_frame(&mut self) {
        self.pipeline.draw_frame(&self.core, &mut self.game);
    }
}

impl Drop for VulkanApp {
    fn drop(&mut self) {
        unsafe {
            self.pipeline.destroy(&self.core);
            self.core.destroy();
        }
    }
}
