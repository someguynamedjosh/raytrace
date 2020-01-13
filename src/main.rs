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
    let mut frame_time_accumulator: u128 = 0;
    let mut elapsed_frames: u32 = 0;
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
            let micros = frame_timer.elapsed().as_micros();
            frame_timer = Instant::now();
            elapsed_frames += 1;
            // Give the engine some time to "warm up", like getting memory in cache etc.
            if elapsed_frames > 10 {
                frame_time_accumulator += micros;
            }

            if frame_time_accumulator > 10 * 1000 * 1000 {
                println!(
                    "Final result: {} frames in {}us",
                    elapsed_frames - 10,
                    frame_time_accumulator
                );
                println!(
                    "Time per frame: {}us",
                    frame_time_accumulator / (elapsed_frames as u128 - 10)
                );
                *control_flow = ControlFlow::Exit;
            }

            game.tick(0.0);
            pipeline.draw_frame(&mut game);
            game.borrow_controls_mut().tick();
        }
        _ => (),
    });
}
