use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

mod render;

fn main() {
    let event_loop = EventLoop::new();
    let mut vulkan_app = render::VulkanApp::new(&event_loop);
    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
            WindowEvent::KeyboardInput { input, .. } => match input {
                KeyboardInput {
                    virtual_keycode,
                    state,
                    ..
                } => match (virtual_keycode, state) {
                    (Some(VirtualKeyCode::Escape), ElementState::Pressed) => {
                        *control_flow = ControlFlow::Exit
                    }
                    _ => {}
                },
            },
            _ => {}
        },
        Event::MainEventsCleared => {
            vulkan_app.draw_frame();
        }
        _ => (),
    })
}
