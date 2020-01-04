use winit::event_loop::EventLoop;

mod render;

fn main() {
    let event_loop = EventLoop::new();
    let vulkan_app = render::VulkanApp::new(&event_loop);
    vulkan_app.main_loop(event_loop);
}