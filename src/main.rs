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

use cgmath::{Rad, Vector3, InnerSpace};

use vulkano::command_buffer::DynamicState;
use vulkano::framebuffer::{Framebuffer, FramebufferAbstract, RenderPassAbstract};
use vulkano::image::SwapchainImage;
use vulkano::pipeline::viewport::Viewport;
use vulkano::swapchain::{self, AcquireError, SwapchainCreationError};
use vulkano::sync::{self, FlushError, GpuFuture};

use winit::{ElementState, Event, KeyboardInput, VirtualKeyCode, Window, WindowEvent};

use std::sync::Arc;

mod init;
mod renderer;
mod shaders;
mod util;

fn main() {
    let init::InitResult {
        device,
        queue,
        surface,
        mut events_loop,
        mut swapchain,
        swapchain_images,
    } = init::init();
    let window = surface.window();

    let mut renderer = renderer::Renderer::new(device.clone(), queue.clone(), swapchain.format());

    // Dynamic viewports allow us to recreate just the viewport when the window is resized
    // Otherwise we would have to recreate the whole pipeline.
    let mut dynamic_state = DynamicState {
        line_width: None,
        viewports: None,
        scissors: None,
    };

    let mut framebuffers = window_size_dependent_setup(
        &swapchain_images,
        renderer.get_render_pass(),
        &mut dynamic_state,
    );
    let mut recreate_swapchain = false;
    let mut previous_frame_end = Box::new(sync::now(device.clone())) as Box<dyn GpuFuture>;

    let mut camera = renderer::Camera {
        origin: Vector3 {
            x: 160.0,
            y: 160.0,
            z: 160.0,
        },
        heading: Rad(1.5),
        pitch: Rad(-1.0),
    };
    let mut camera_movement = Vector3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    let mut total_frames = 0;
    let mut total_frame_time = 0;
    loop {
        let frame_start = std::time::Instant::now();
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
                renderer.get_render_pass(),
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

        let command_buffer = renderer.create_command_buffer(
            &camera,
            &dynamic_state,
            framebuffers[image_num].clone(),
        );

        let future = previous_frame_end
            .join(acquire_future)
            .then_execute(queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(queue.clone(), swapchain.clone(), image_num)
            .then_signal_fence_and_flush();

        match future {
            Ok(future) => {
                previous_frame_end = Box::new(future) as Box<_>;
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

        // Note that in more complex programs it is likely that one of `acquire_next_image`,
        // `command_buffer::submit`, or `present` will block for some time. This happens when the
        // GPU's queue is full and the driver has to wait until the GPU finished some work.
        //
        // Unfortunately the Vulkan API doesn't provide any way to not wait or to detect when a
        // wait would happen. Blocking may be the desired behavior, but if you don't want to
        // block you should spawn a separate thread dedicated to submissions.

        // Handling the window events in order to close the program when the user wants to close
        // it.
        let mut done = false;
        events_loop.poll_events(|ev| match ev {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => done = true,
            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                camera.heading.0 = (-position.x / 80.0) as f32;
                camera.pitch.0 = ((256.0 - position.y) / 200.0) as f32;
            }
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
                VirtualKeyCode::W => camera_movement.y = 1.0,
                VirtualKeyCode::S => camera_movement.y = -1.0,
                VirtualKeyCode::D => camera_movement.x = 1.0,
                VirtualKeyCode::A => camera_movement.x = -1.0,
                VirtualKeyCode::E => camera_movement.z = 1.0,
                VirtualKeyCode::Q => camera_movement.z = -1.0,
                VirtualKeyCode::Escape => done = true,
                _ => (),
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
            } => match code {
                VirtualKeyCode::W => camera_movement.y = 0.0,
                VirtualKeyCode::S => camera_movement.y = 0.0,
                VirtualKeyCode::D => camera_movement.x = 0.0,
                VirtualKeyCode::A => camera_movement.x = 0.0,
                VirtualKeyCode::E => camera_movement.z = 0.0,
                VirtualKeyCode::Q => camera_movement.z = 0.0,
                _ => (),
            },
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => recreate_swapchain = true,
            _ => (),
        });
        if done {
            return;
        }
        let elapsed = frame_start.elapsed().as_millis() as f32 / 1000.0;
        total_frame_time += frame_start.elapsed().as_millis();
        total_frames += 1;
        camera.origin += {
            let amount = elapsed * 15.0;
            let util::TripleEulerVector { forward, up, right } =
                util::compute_triple_euler_vector(camera.heading, camera.pitch);
            let forward = forward.normalize();
            let up = up.normalize();
            let right = right.normalize();
            amount * forward * camera_movement.y
                + amount * up * camera_movement.z
                + amount * right * camera_movement.x
        };
        println!("Frame took {}ms, average {} per frame.", frame_start.elapsed().as_millis(), total_frame_time / (total_frames));
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
