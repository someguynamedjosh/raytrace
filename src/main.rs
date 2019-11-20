// Copyright (c) 2016 The vulkano developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

// Welcome to the triangle example!
//
// This is the only example that is entirely detailed. All the other examples avoid code
// duplication by using helper functions.
//
// This example assumes that you are already more or less familiar with graphics programming
// and that you want to learn Vulkan. This means that for example it won't go into details about
// what a vertex or a shader is.

use vulkano::command_buffer::{AutoCommandBufferBuilder, DynamicState};
use vulkano::framebuffer::{Framebuffer, FramebufferAbstract, RenderPassAbstract};
use vulkano::image::SwapchainImage;
use vulkano::pipeline::viewport::Viewport;
use vulkano::swapchain::{self, AcquireError, SwapchainCreationError};
use vulkano::sync::{self, FlushError, GpuFuture};

use winit::{ElementState, Event, KeyboardInput, VirtualKeyCode, Window, WindowEvent};

use std::env;
use std::sync::Arc;

mod game;
mod render;
mod util;
mod world;

use render::{InitResult, Presenter, Renderer};

const SAMPLE_SIZE: usize = 2000;
const WARMUP_TIME: usize = 200;

fn main() {
    let args: Vec<_> = env::args().collect();
    let mut capture_counter = if args.len() > 1 {
        SAMPLE_SIZE + WARMUP_TIME
    } else {
        0
    };

    let InitResult {
        device,
        queue,
        surface,
        mut events_loop,
        mut swapchain,
        swapchain_images,
    } = render::init();
    let window = surface.window();
    println!("Vulkan started.");

    let mut game = game::Game::new();
    println!("Game initialized.");

    let presenter = Presenter::new(
        device.clone(),
        queue.clone(),
        (512, 512),
        swapchain.format(),
    );
    let mut renderer = Renderer::new(
        device.clone(),
        queue.clone(),
        presenter.get_presented_image(),
        &game,
    );
    println!("Renderer initialized.");

    // Dynamic viewports allow us to recreate just the viewport when the window is resized
    // Otherwise we would have to recreate the whole pipeline.
    let mut dynamic_state = DynamicState {
        line_width: None,
        viewports: None,
        scissors: None,
    };

    let mut framebuffers = window_size_dependent_setup(
        &swapchain_images,
        presenter.get_render_pass(),
        &mut dynamic_state,
    );
    let mut recreate_swapchain = false;
    let mut previous_frame_end = Box::new(sync::now(device.clone())) as Box<dyn GpuFuture>;

    let mut total_frames = 0;
    let mut total_frame_time = 0;
    let mut frame_start = std::time::Instant::now();
    loop {
        previous_frame_end.cleanup_finished();
        if recreate_swapchain {
            let dimensions = if let Some(dimensions) = window.get_inner_size() {
                let dimensions: (u32, u32) =
                    dimensions.to_physical(window.get_hidpi_factor()).into();
                [dimensions.0, dimensions.1]
            } else {
                return;
            };

            let (new_swapchain, new_images) = match swapchain.recreate_with_dimension(dimensions) {
                Ok(r) => r,
                // This error tends to happen when the user is manually resizing the window.
                // Simply restarting the loop is the easiest way to fix this issue.
                Err(SwapchainCreationError::UnsupportedDimensions) => continue,
                Err(err) => panic!("{:?}", err),
            };

            swapchain = new_swapchain;
            framebuffers = window_size_dependent_setup(
                &new_images,
                presenter.get_render_pass(),
                &mut dynamic_state,
            );

            recreate_swapchain = false;
        }

        let (image_num, acquire_future) =
            match swapchain::acquire_next_image(swapchain.clone(), None) {
                Ok(r) => r,
                Err(AcquireError::OutOfDate) => {
                    recreate_swapchain = true;
                    continue;
                }
                Err(err) => panic!("{:?}", err),
            };

        let mut builder =
            AutoCommandBufferBuilder::primary_one_time_submit(device.clone(), queue.family())
                .unwrap();
        builder = renderer.add_render_commands(builder, &game);
        let builder = presenter.add_present_commands(
            builder,
            &dynamic_state,
            framebuffers[image_num].clone(),
        );
        let command_buffer = builder.build().unwrap();

        let future = previous_frame_end
            .join(acquire_future)
            .then_execute(queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(queue.clone(), swapchain.clone(), image_num)
            .then_signal_fence_and_flush();

        let mut done = false;
        game.borrow_controls_mut().tick();
        if capture_counter == 0 {
            events_loop.poll_events(|ev| match ev {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => done = true,
                Event::WindowEvent {
                    event: WindowEvent::CursorMoved { position, .. },
                    ..
                } => game.on_mouse_move(position.x, position.y),
                Event::WindowEvent {
                    event:
                        WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(code),
                                    ..
                                },
                            ..
                        },
                    ..
                } => match code {
                    VirtualKeyCode::Escape => done = true,
                    _ => game.borrow_controls_mut().on_pressed(code),
                },
                Event::WindowEvent {
                    event:
                        WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Released,
                                    virtual_keycode: Some(code),
                                    ..
                                },
                            ..
                        },
                    ..
                } => game.borrow_controls_mut().on_released(code),
                Event::WindowEvent {
                    event: WindowEvent::Resized(_),
                    ..
                } => recreate_swapchain = true,
                _ => (),
            }); 
        } 
        if done {
            return;
        }
        let dt = frame_start.elapsed().as_millis() as f32 / 1000.0;
        game.tick(dt);
        frame_start = std::time::Instant::now();

        match future {
            Ok(future) => {
                future.wait(None).unwrap();
                previous_frame_end = Box::new(sync::now(device.clone())) as Box<_>;
            }
            Err(FlushError::OutOfDate) => {
                recreate_swapchain = true;
                previous_frame_end = Box::new(sync::now(device.clone())) as Box<_>;
            }
            Err(e) => {
                println!("{:?}", e);
                previous_frame_end = Box::new(sync::now(device.clone())) as Box<_>;
            }
        }

        if capture_counter > 0 {
            capture_counter -= 1;
            if capture_counter < SAMPLE_SIZE {
                renderer.capture();
            }
            if capture_counter == 0 {
                renderer.finish_capture();
                return;
            }
        }

        renderer.read_feedback(&game);

        total_frame_time += (dt * 1000.0) as i64;
        total_frames += 1;
        println!(
            "Frame took {}ms, average {} per frame.",
            frame_start.elapsed().as_millis(),
            total_frame_time / (total_frames)
        );
    }
}

/// This method is called once during initialization, then again whenever the window is resized
fn window_size_dependent_setup(
    images: &[Arc<SwapchainImage<Window>>],
    render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
    dynamic_state: &mut DynamicState,
) -> Vec<Arc<dyn FramebufferAbstract + Send + Sync>> {
    let dimensions = images[0].dimensions();

    let viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: [dimensions[0] as f32, dimensions[1] as f32],
        depth_range: 0.0..1.0,
    };
    dynamic_state.viewports = Some(vec![viewport]);

    images
        .iter()
        .map(|image| {
            Arc::new(
                Framebuffer::start(render_pass.clone())
                    .add(image.clone())
                    .unwrap()
                    .build()
                    .unwrap(),
            ) as Arc<dyn FramebufferAbstract + Send + Sync>
        })
        .collect::<Vec<_>>()
}
