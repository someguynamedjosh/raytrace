use ash::extensions::{ext::DebugUtils, khr::Surface};
use ash::version::DeviceV1_0;
use ash::version::EntryV1_0;
use ash::version::InstanceV1_0;
use ash::vk;
use std::ffi::CString;
use std::os::raw::{c_char, c_void};
use std::ptr;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;
use winit::dpi::PhysicalSize;

use crate::render::constants::*;
use crate::render::util;

use super::core::{Core, QueueFamilyIndices, SwapChainInfo};
use super::debug;
use super::platform_specific;

impl Core {
    pub fn new(event_loop: &EventLoop<()>) -> Core {
        let entry = ash::Entry::new().unwrap();
        let instance = create_instance(&entry, WINDOW_TITLE);
        let (ext_debug_utils, debug_messenger) = debug::setup_debug_utils(&entry, &instance);
        let window = WindowBuilder::new()
            .with_title(WINDOW_TITLE)
            .with_inner_size(PhysicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
            .build(event_loop)
            .expect("Failed to create window.");
        let window = Box::new(window);
        let surface_info = create_surface(&entry, &instance, &window);
        let physical_device = pick_physical_device(&instance, &surface_info);
        let memory_properties =
            unsafe { instance.get_physical_device_memory_properties(physical_device) };
        let (device, queue_family_indices) =
            create_logical_device(&instance, physical_device, &surface_info);
        let command_pool = create_command_pool(
            &device,
            &ext_debug_utils,
            queue_family_indices.compute.unwrap(),
        );
        let swapchain = create_swapchain(
            &instance,
            &device,
            &ext_debug_utils,
            physical_device,
            &window,
            &surface_info,
            &queue_family_indices,
        );
        let compute_queue =
            unsafe { device.get_device_queue(queue_family_indices.compute.unwrap(), 0) };
        let present_queue =
            unsafe { device.get_device_queue(queue_family_indices.present.unwrap(), 0) };

        Core {
            entry,
            instance,
            queue_family_indices,
            surface: surface_info.surface,
            ext_surface: surface_info.ext_surface,
            ext_debug_utils,
            debug_messenger,
            physical_device,
            memory_properties,
            device,
            swapchain,
            compute_queue,
            present_queue,
            command_pool,
            window,
        }
    }
}

pub struct SurfaceInfo {
    pub ext_surface: Surface,
    pub surface: vk::SurfaceKHR,
}

pub struct SwapChainSupportInfo {
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    pub formats: Vec<vk::SurfaceFormatKHR>,
    pub present_modes: Vec<vk::PresentModeKHR>,
}

pub fn create_instance(entry: &ash::Entry, window_title: &str) -> ash::Instance {
    if ENABLE_DEBUG && !check_validation_layer_support(entry) {
        panic!("Validation layers requested, but not available!");
    }

    let app_name = CString::new(window_title).unwrap();
    let engine_name = CString::new("Vulkan Engine").unwrap();
    let app_info = vk::ApplicationInfo {
        p_application_name: app_name.as_ptr(),
        s_type: vk::StructureType::APPLICATION_INFO,
        p_next: ptr::null(),
        application_version: APPLICATION_VERSION,
        p_engine_name: engine_name.as_ptr(),
        engine_version: ENGINE_VERSION,
        api_version: API_VERSION,
    };

    // This create info used to debug issues in vk::createInstance and vk::destroyInstance.
    let debug_utils_create_info = debug::build_debug_utils_create_info();

    let extension_names = platform_specific::required_extension_names();

    let validation_layer_names: Vec<CString> = VALIDATION_LAYERS
        .iter()
        .map(|layer_name| CString::new(*layer_name).unwrap())
        .collect();
    let validation_layer_name_pointers: Vec<*const i8> = validation_layer_names
        .iter()
        .map(|layer_name| layer_name.as_ptr())
        .collect();

    let create_info = vk::InstanceCreateInfo {
        s_type: vk::StructureType::INSTANCE_CREATE_INFO,
        p_next: if ENABLE_DEBUG {
            &debug_utils_create_info as *const vk::DebugUtilsMessengerCreateInfoEXT as *const c_void
        } else {
            ptr::null()
        },
        flags: vk::InstanceCreateFlags::empty(),
        p_application_info: &app_info,
        pp_enabled_layer_names: if ENABLE_DEBUG {
            validation_layer_name_pointers.as_ptr()
        } else {
            ptr::null()
        },
        enabled_layer_count: if ENABLE_DEBUG {
            validation_layer_name_pointers.len()
        } else {
            0
        } as u32,
        pp_enabled_extension_names: extension_names.as_ptr(),
        enabled_extension_count: extension_names.len() as u32,
    };

    let instance: ash::Instance = unsafe {
        entry
            .create_instance(&create_info, None)
            .expect("Failed to create Vulkan instance!")
    };

    instance
}

pub fn check_validation_layer_support(entry: &ash::Entry) -> bool {
    // if support validation layer, then return true

    let layer_properties = entry
        .enumerate_instance_layer_properties()
        .expect("Failed to enumerate Instance Layers Properties");

    if layer_properties.len() <= 0 {
        eprintln!("No available layers.");
        return false;
    }

    for required_layer_name in VALIDATION_LAYERS.iter() {
        let mut is_layer_found = false;

        for layer_property in layer_properties.iter() {
            let test_layer_name = util::convert_raw_cstring(&layer_property.layer_name);
            if (*required_layer_name) == test_layer_name {
                is_layer_found = true;
                break;
            }
        }

        if is_layer_found == false {
            return false;
        }
    }

    true
}

pub fn create_surface(
    entry: &ash::Entry,
    instance: &ash::Instance,
    window: &winit::window::Window,
) -> SurfaceInfo {
    let surface = unsafe {
        platform_specific::create_surface(entry, instance, window)
            .expect("Failed to create surface.")
    };
    let ext_surface = Surface::new(entry, instance);

    SurfaceInfo {
        ext_surface,
        surface,
    }
}

pub fn pick_physical_device(
    instance: &ash::Instance,
    surface_info: &SurfaceInfo,
) -> vk::PhysicalDevice {
    let physical_devices = unsafe {
        instance
            .enumerate_physical_devices()
            .expect("Failed to enumerate physical devices!")
    };

    let result = physical_devices.iter().find(|physical_device| {
        let is_suitable = is_physical_device_suitable(instance, **physical_device, surface_info);

        if is_suitable {
            unsafe {
                let device_properties = instance.get_physical_device_properties(**physical_device);
                let device_name = util::convert_raw_cstring(&device_properties.device_name);
                println!("Using GPU: {}", device_name);
            }
        }

        is_suitable
    });

    match result {
        Some(p_physical_device) => *p_physical_device,
        None => panic!("Failed to find a suitable GPU!"),
    }
}

pub fn is_physical_device_suitable(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    surface_info: &SurfaceInfo,
) -> bool {
    let indices = find_queue_family(instance, physical_device, surface_info);

    let is_queue_family_supported = indices.is_complete();
    let is_device_extension_supported = check_device_extension_support(instance, physical_device);
    let is_swapchain_supported = if is_device_extension_supported {
        let swapchain_support = query_swapchain_support(physical_device, surface_info);
        !swapchain_support.formats.is_empty() && !swapchain_support.present_modes.is_empty()
    } else {
        false
    };

    is_queue_family_supported && is_device_extension_supported && is_swapchain_supported
}

pub fn create_logical_device(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    surface_info: &SurfaceInfo,
) -> (ash::Device, QueueFamilyIndices) {
    let indices = find_queue_family(instance, physical_device, surface_info);

    use std::collections::HashSet;
    let mut queue_families = HashSet::new();
    queue_families.insert(indices.compute.unwrap());
    queue_families.insert(indices.present.unwrap());

    let queue_priorities = [1.0_f32];
    let mut queue_create_infos = vec![];
    for &queue_family in queue_families.iter() {
        let queue_create_info = vk::DeviceQueueCreateInfo {
            s_type: vk::StructureType::DEVICE_QUEUE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::DeviceQueueCreateFlags::empty(),
            queue_family_index: queue_family,
            p_queue_priorities: queue_priorities.as_ptr(),
            queue_count: queue_priorities.len() as u32,
        };
        queue_create_infos.push(queue_create_info);
    }

    let physical_device_features = vk::PhysicalDeviceFeatures::default();

    let requred_validation_layer_raw_names: Vec<CString> = VALIDATION_LAYERS
        .iter()
        .map(|layer_name| CString::new(*layer_name).unwrap())
        .collect();
    let enable_layer_names: Vec<*const c_char> = requred_validation_layer_raw_names
        .iter()
        .map(|layer_name| layer_name.as_ptr())
        .collect();

    let device_extension_cstrings: Vec<CString> = DEVICE_EXTENSIONS
        .iter()
        .map(|extension_name| CString::new(*extension_name).unwrap())
        .collect();
    let device_extension_cstring_pointers: Vec<*const c_char> = device_extension_cstrings
        .iter()
        .map(|extension_name_cstring| extension_name_cstring.as_ptr())
        .collect();

    let device_create_info = vk::DeviceCreateInfo {
        s_type: vk::StructureType::DEVICE_CREATE_INFO,
        p_next: ptr::null(),
        flags: vk::DeviceCreateFlags::empty(),
        queue_create_info_count: queue_create_infos.len() as u32,
        p_queue_create_infos: queue_create_infos.as_ptr(),
        enabled_layer_count: if ENABLE_DEBUG {
            enable_layer_names.len()
        } else {
            0
        } as u32,
        pp_enabled_layer_names: if ENABLE_DEBUG {
            enable_layer_names.as_ptr()
        } else {
            ptr::null()
        },
        enabled_extension_count: device_extension_cstring_pointers.len() as u32,
        pp_enabled_extension_names: device_extension_cstring_pointers.as_ptr(),
        p_enabled_features: &physical_device_features,
    };

    let device: ash::Device = unsafe {
        instance
            .create_device(physical_device, &device_create_info, None)
            .expect("Failed to create logical Device!")
    };

    if ENABLE_DEBUG {
        println!("Validation layers enabled!");
    }

    (device, indices)
}

pub fn find_queue_family(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    surface_info: &SurfaceInfo,
) -> QueueFamilyIndices {
    let queue_families =
        unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

    let mut queue_family_indices = QueueFamilyIndices::default();

    let mut index = 0;
    for queue_family in queue_families.iter() {
        if queue_family.queue_count > 0
            && queue_family.queue_flags.contains(vk::QueueFlags::COMPUTE)
            && queue_family.queue_flags.contains(vk::QueueFlags::TRANSFER)
        {
            queue_family_indices.compute = Some(index);
        }

        let is_present_support = unsafe {
            surface_info
                .ext_surface
                .get_physical_device_surface_support(
                    physical_device,
                    index as u32,
                    surface_info.surface,
                )
        };
        if queue_family.queue_count > 0 && is_present_support {
            queue_family_indices.present = Some(index);
        }

        if queue_family_indices.is_complete() {
            break;
        }

        index += 1;
    }

    queue_family_indices
}

pub fn check_device_extension_support(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
) -> bool {
    let available_extensions = unsafe {
        instance
            .enumerate_device_extension_properties(physical_device)
            .expect("Failed to get device extension properties.")
    };

    let mut available_extension_names = vec![];

    for extension in available_extensions.iter() {
        let extension_name = util::convert_raw_cstring(&extension.extension_name);

        available_extension_names.push(extension_name);
    }

    use std::collections::HashSet;
    let mut required_extensions = HashSet::new();
    for extension in DEVICE_EXTENSIONS.iter() {
        required_extensions.insert(extension.to_string());
    }

    for extension_name in available_extension_names.iter() {
        required_extensions.remove(extension_name);
    }

    return required_extensions.is_empty();
}

pub fn query_swapchain_support(
    physical_device: vk::PhysicalDevice,
    surface_info: &SurfaceInfo,
) -> SwapChainSupportInfo {
    unsafe {
        let capabilities = surface_info
            .ext_surface
            .get_physical_device_surface_capabilities(physical_device, surface_info.surface)
            .expect("Failed to query for surface capabilities.");
        let formats = surface_info
            .ext_surface
            .get_physical_device_surface_formats(physical_device, surface_info.surface)
            .expect("Failed to query for surface formats.");
        let present_modes = surface_info
            .ext_surface
            .get_physical_device_surface_present_modes(physical_device, surface_info.surface)
            .expect("Failed to query for surface present mode.");

        SwapChainSupportInfo {
            capabilities,
            formats,
            present_modes,
        }
    }
}

fn create_command_pool(
    device: &ash::Device,
    debug_utils: &DebugUtils,
    queue_family_index: u32,
) -> vk::CommandPool {
    let create_info = vk::CommandPoolCreateInfo {
        queue_family_index,
        ..Default::default()
    };
    let pool = unsafe {
        device
            .create_command_pool(&create_info, None)
            .expect("Failed to create command pool.")
    };
    debug::set_debug_name(device, debug_utils, pool, "primary_command_pool");
    pool
}

pub fn create_swapchain(
    instance: &ash::Instance,
    device: &ash::Device,
    debug_utils: &ash::extensions::ext::DebugUtils,
    physical_device: vk::PhysicalDevice,
    window: &winit::window::Window,
    surface_info: &SurfaceInfo,
    queue_family: &QueueFamilyIndices,
) -> SwapChainInfo {
    let swapchain_support = query_swapchain_support(physical_device, surface_info);

    let surface_format = choose_swapchain_format(&swapchain_support.formats);
    let present_mode = choose_swapchain_present_mode(&swapchain_support.present_modes);
    let extent = choose_swapchain_extent(&swapchain_support.capabilities, window);

    let image_count = swapchain_support.capabilities.min_image_count + 1;
    let image_count = if swapchain_support.capabilities.max_image_count > 0 {
        image_count.min(swapchain_support.capabilities.max_image_count)
    } else {
        image_count
    };

    let (image_sharing_mode, queue_family_index_count, queue_family_indices) =
        if queue_family.compute != queue_family.present {
            (
                vk::SharingMode::CONCURRENT,
                2,
                vec![queue_family.compute.unwrap(), queue_family.present.unwrap()],
            )
        } else {
            (vk::SharingMode::EXCLUSIVE, 0, vec![])
        };

    let swapchain_create_info = vk::SwapchainCreateInfoKHR {
        s_type: vk::StructureType::SWAPCHAIN_CREATE_INFO_KHR,
        p_next: ptr::null(),
        flags: vk::SwapchainCreateFlagsKHR::empty(),
        surface: surface_info.surface,
        min_image_count: image_count,
        image_color_space: surface_format.color_space,
        image_format: surface_format.format,
        image_extent: extent,
        image_usage: vk::ImageUsageFlags::STORAGE,
        image_sharing_mode,
        p_queue_family_indices: queue_family_indices.as_ptr(),
        queue_family_index_count,
        pre_transform: swapchain_support.capabilities.current_transform,
        composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
        present_mode,
        clipped: vk::TRUE,
        old_swapchain: vk::SwapchainKHR::null(),
        image_array_layers: 1,
    };

    let swapchain_loader = ash::extensions::khr::Swapchain::new(instance, device);
    let swapchain = unsafe {
        swapchain_loader
            .create_swapchain(&swapchain_create_info, None)
            .expect("Failed to create Swapchain!")
    };

    let swapchain_images = unsafe {
        swapchain_loader
            .get_swapchain_images(swapchain)
            .expect("Failed to get Swapchain Images.")
    };

    let mut swapchain_image_views = vec![];
    for (index, image) in swapchain_images.iter().enumerate() {
        debug::set_debug_name(
            device,
            debug_utils,
            *image,
            &format!("swapchain_img_{}", index),
        );
        let create_info = vk::ImageViewCreateInfo {
            image: *image,
            view_type: vk::ImageViewType::TYPE_2D,
            format: surface_format.format,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
                ..Default::default()
            },
            ..Default::default()
        };
        swapchain_image_views.push(unsafe {
            let view = device
                .create_image_view(&create_info, None)
                .expect("Failed to create image view for swapchain image.");
            debug::set_debug_name(
                device,
                debug_utils,
                view,
                &format!("swapchain_img_view_{}", index),
            );
            view
        });
    }

    SwapChainInfo {
        swapchain_loader,
        swapchain,
        swapchain_format: surface_format.format,
        swapchain_extent: extent,
        swapchain_images,
        swapchain_image_views,
    }
}

pub fn choose_swapchain_format(
    available_formats: &Vec<vk::SurfaceFormatKHR>,
) -> vk::SurfaceFormatKHR {
    for available_format in available_formats {
        if available_format.format == vk::Format::B8G8R8A8_UNORM
            && available_format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        {
            return available_format.clone();
        }
    }

    return available_formats.first().unwrap().clone();
}

pub fn choose_swapchain_present_mode(
    available_present_modes: &Vec<vk::PresentModeKHR>,
) -> vk::PresentModeKHR {
    for &available_present_mode in available_present_modes.iter() {
        if available_present_mode == vk::PresentModeKHR::MAILBOX {
            return available_present_mode;
        }
    }

    vk::PresentModeKHR::FIFO
}

pub fn choose_swapchain_extent(
    capabilities: &vk::SurfaceCapabilitiesKHR,
    window: &winit::window::Window,
) -> vk::Extent2D {
    if capabilities.current_extent.width != u32::max_value() {
        capabilities.current_extent
    } else {
        use num::clamp;

        let window_size = window.inner_size();

        vk::Extent2D {
            width: clamp(
                window_size.width as u32,
                capabilities.min_image_extent.width,
                capabilities.max_image_extent.width,
            ),
            height: clamp(
                window_size.height as u32,
                capabilities.min_image_extent.height,
                capabilities.max_image_extent.height,
            ),
        }
    }
}
