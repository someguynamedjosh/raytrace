use ash::version::DeviceV1_0;
use ash::version::InstanceV1_0;
use ash::vk;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

pub(self) mod constants;
pub(self) mod debug;
pub(self) mod init;
pub(self) mod platform_specific;
pub(self) mod util;

use constants::*;
use init::SwapChainInfo;

pub struct VulkanApp {
    _entry: ash::Entry,
    instance: ash::Instance,
    surface_loader: ash::extensions::khr::Surface,
    surface: vk::SurfaceKHR,
    debug_utils_loader: ash::extensions::ext::DebugUtils,
    debug_merssager: vk::DebugUtilsMessengerEXT,
    _physical_device: vk::PhysicalDevice,
    device: ash::Device,
    swapchain_info: SwapChainInfo,
    _graphics_queue: vk::Queue,
    _present_queue: vk::Queue,
    _window: Box<Window>,
}

impl VulkanApp {
    pub fn new(event_loop: &EventLoop<()>) -> VulkanApp {
        let entry = ash::Entry::new().unwrap();
        let instance = init::create_instance(&entry, WINDOW_TITLE);
        let (debug_utils_loader, debug_merssager) = debug::setup_debug_utils(&entry, &instance);
        let window = WindowBuilder::new()
            .with_title(WINDOW_TITLE)
            .with_inner_size((WINDOW_WIDTH, WINDOW_HEIGHT).into())
            .build(event_loop)
            .expect("Failed to create window.");
        let window = Box::new(window);
        let surface_info = init::create_surface(&entry, &instance, &window);
        let physical_device = init::pick_physical_device(&instance, &surface_info);
        let (device, family_indices) = init::create_logical_device(
            &instance,
            physical_device,
            &surface_info,
        );
        let swapchain_info = init::create_swapchain(&instance, &device, physical_device, &window, &surface_info, &family_indices);
        let graphics_queue =
            unsafe { device.get_device_queue(family_indices.compute.unwrap(), 0) };
        let present_queue =
            unsafe { device.get_device_queue(family_indices.present.unwrap(), 0) };

        // cleanup(); the 'drop' function will take care of it.
        VulkanApp {
            _entry: entry,
            instance,
            surface: surface_info.surface,
            surface_loader: surface_info.surface_loader,
            debug_utils_loader,
            debug_merssager,
            _physical_device: physical_device,
            device,
            swapchain_info,
            _graphics_queue: graphics_queue,
            _present_queue: present_queue,
            _window: window,
        }
    }

    pub fn draw_frame(&mut self) {
        // Drawing will be here
    }
}

impl Drop for VulkanApp {
    fn drop(&mut self) {
        unsafe {
            self.swapchain_info.swapchain_loader.destroy_swapchain(self.swapchain_info.swapchain, None);

            self.device.destroy_device(None);

            self.surface_loader.destroy_surface(self.surface, None);

            if ENABLE_DEBUG {
                self.debug_utils_loader
                    .destroy_debug_utils_messenger(self.debug_merssager, None);
            }
            self.instance.destroy_instance(None);
        }
    }
}

// Fix content -------------------------------------------------------------------------------
impl VulkanApp {
    pub fn main_loop(mut self, event_loop: EventLoop<()>) {
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
                self.draw_frame();
            }
            _ => (),
        })
    }
}
