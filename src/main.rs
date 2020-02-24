use std::time::Instant;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

mod game;
mod render;
mod util;
mod world;

fn main() {
    let mut game = game::Game::new();
    let event_loop = EventLoop::new();
    let (_core, mut pipeline) = render::create_instance(&event_loop, &game);
    let mut frame_timer = Instant::now();
    let mut performance_buffer = util::RingBufferAverage::new(16);
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
                    (Some(code), ElementState::Pressed) => {
                        game.borrow_controls_mut().on_pressed(code);
                    }
                    (Some(code), ElementState::Released) => {
                        game.borrow_controls_mut().on_released(code);
                    }
                    _ => {}
                },
            },
            WindowEvent::CursorMoved { position, .. } => game.on_mouse_move(position.x, position.y),
            _ => {}
        },
        Event::MainEventsCleared => {
            let millis = frame_timer.elapsed().as_millis();
            frame_timer = Instant::now();

            performance_buffer.push_sample(millis);
            println!("Average frame time: {}ms", performance_buffer.average());
            game.tick((millis as f64 / 1000.0) as f32);
            pipeline.draw_frame(&mut game);
            game.borrow_controls_mut().tick();
        }
        _ => (),
    });
}
