use winit::event_loop::EventLoop;

pub(self) mod commands;
pub(self) mod constants;
pub(self) mod core;
pub(self) mod debug;
pub(self) mod descriptors;
pub(self) mod pipeline;
pub(self) mod platform_specific;
pub(self) mod structures;
pub(self) mod util;

use self::core::Core;
use pipeline::Pipeline;

pub struct VulkanApp {
    core: Core,
    pipeline: Pipeline,
}

impl VulkanApp {
    pub fn new(event_loop: &EventLoop<()>) -> VulkanApp {
        let core = Core::new(event_loop);
        let pipeline = Pipeline::new(&core);
        VulkanApp { core, pipeline }
    }

    pub fn draw_frame(&mut self) {
        self.pipeline.draw_frame(&self.core);
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
