use winit::event_loop::EventLoop;

pub(self) mod constants;
pub(self) mod core;
pub(self) mod platform_specific;
pub(self) mod util;

use self::core::Core;

pub struct VulkanApp {
    core: Core,
}

impl VulkanApp {
    pub fn new(event_loop: &EventLoop<()>) -> VulkanApp {
        let core = Core::new(event_loop);
        VulkanApp { core }
    }

    pub fn draw_frame(&mut self) {
        // Drawing will be here
    }
}

impl Drop for VulkanApp {
    fn drop(&mut self) {
        self.core.destroy();
    }
}
