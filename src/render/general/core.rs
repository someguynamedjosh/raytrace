use ash::extensions::{ext::DebugUtils, khr::Surface};
use ash::version::DeviceV1_0;
use ash::version::InstanceV1_0;
use ash::vk::{self, Handle};
use winit::window::Window;

use crate::render::constants::*;

use super::debug;

pub struct Core {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub physical_device: vk::PhysicalDevice,
    pub device: ash::Device,
    pub ext_surface: Surface,
    pub ext_debug_utils: DebugUtils,

    pub surface: vk::SurfaceKHR,
    pub debug_messenger: vk::DebugUtilsMessengerEXT,
    pub memory_properties: vk::PhysicalDeviceMemoryProperties,
    pub swapchain: SwapChainInfo,
    pub window: Box<Window>,

    pub queue_family_indices: QueueFamilyIndices,
    pub compute_queue: vk::Queue,
    pub present_queue: vk::Queue,
    pub command_pool: vk::CommandPool,
}

impl Core {
    pub fn create_fence(&self, start_signaled: bool, debug_name: &str) -> vk::Fence {
        let create_info = vk::FenceCreateInfo {
            flags: if start_signaled {
                vk::FenceCreateFlags::SIGNALED
            } else {
                Default::default()
            },
            ..Default::default()
        };
        let fence = unsafe {
            self.device
                .create_fence(&create_info, None)
                .expect("Failed to create fence.")
        };
        self.set_debug_name(fence, debug_name);
        fence
    }

    pub fn create_semaphore(&self, debug_name: &str) -> vk::Semaphore {
        let create_info = vk::SemaphoreCreateInfo {
            ..Default::default()
        };
        let semaphore = unsafe {
            self.device
                .create_semaphore(&create_info, None)
                .expect("Failed to create semaphore.")
        };
        self.set_debug_name(semaphore, debug_name);
        semaphore
    }

    pub fn find_compatible_memory_type(
        &self,
        memory_type_bits: u32,
        required_flags: vk::MemoryPropertyFlags,
    ) -> u32 {
        for index in 0..self.memory_properties.memory_type_count {
            // Skip over memory types that memory_type_bits does not allow.
            if memory_type_bits & (1 << index) == 0 {
                continue;
            }
            let properties = self.memory_properties.memory_types[index as usize];
            // Skip over memory types that don't have the required flags.
            if (properties.property_flags & required_flags) != required_flags {
                continue;
            }
            return index;
        }
        panic!("Could not find appropriate memory type!");
    }

    pub fn set_debug_name<VkObject: Handle>(&self, object: VkObject, name: &str) {
        debug::set_debug_name(&self.device, &self.ext_debug_utils, object, name);
    }
}

impl Drop for Core {
    fn drop(&mut self) {
        unsafe {
            for view in &self.swapchain.swapchain_image_views {
                self.device.destroy_image_view(*view, None);
            }

            self.swapchain
                .swapchain_loader
                .destroy_swapchain(self.swapchain.swapchain, None);

            self.device.destroy_command_pool(self.command_pool, None);

            self.device.destroy_device(None);

            self.ext_surface.destroy_surface(self.surface, None);

            if ENABLE_DEBUG {
                self.ext_debug_utils
                    .destroy_debug_utils_messenger(self.debug_messenger, None);
            }
            self.instance.destroy_instance(None);
        }
    }
}

#[derive(Default)]
pub struct QueueFamilyIndices {
    pub compute: Option<u32>,
    pub present: Option<u32>,
}

impl QueueFamilyIndices {
    pub fn is_complete(&self) -> bool {
        self.compute.is_some() && self.present.is_some()
    }
}

pub struct SwapChainInfo {
    pub swapchain_loader: ash::extensions::khr::Swapchain,
    pub swapchain: vk::SwapchainKHR,
    pub swapchain_images: Vec<vk::Image>,
    pub swapchain_format: vk::Format,
    pub swapchain_extent: vk::Extent2D,
    pub swapchain_image_views: Vec<vk::ImageView>,
}
