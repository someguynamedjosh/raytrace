use std::time::Instant;

use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

mod game;
mod render;
mod world;
mod util;

fn main() {
    let event_loop = EventLoop::new();
    let mut vulkan_app = render::VulkanApp::new(&event_loop);
    let mut frame_timer = Instant::now();
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
                        *control_flow = ControlFlow::Exit;
                    }
                    _ => {}
                },
            },
            WindowEvent::CursorMoved { position, .. } => {
                vulkan_app.on_mouse_move(position.x, -position.y)
            }
            _ => {}
        },
        Event::MainEventsCleared => {
            let delta_time = frame_timer.elapsed().as_millis() as f64 / 1000.0;
            frame_timer = Instant::now();
            vulkan_app.tick(delta_time as f32);
            vulkan_app.draw_frame();
        }
        _ => (),
    });
}
