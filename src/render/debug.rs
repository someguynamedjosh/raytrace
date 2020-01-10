use ash::extensions::ext::DebugUtils;
use ash::vk::{self, Handle};

use colored::*;

use std::ffi::{c_void, CStr, CString};
use std::ptr;

use super::constants::*;

fn name_of_type(typ: vk::ObjectType) -> &'static str {
    match typ {
        vk::ObjectType::ACCELERATION_STRUCTURE_NV => "NV::AccelerationStructure",
        vk::ObjectType::BUFFER => "Buffer",
        vk::ObjectType::BUFFER_VIEW => "BufferView",
        vk::ObjectType::COMMAND_BUFFER => "CommandBuffer",
        vk::ObjectType::COMMAND_POOL => "CommandPool",
        vk::ObjectType::DEBUG_REPORT_CALLBACK_EXT => "EXT::DebugReportCallback",
        vk::ObjectType::DEBUG_UTILS_MESSENGER_EXT => "EXT::DebugUtilsMessenger",
        vk::ObjectType::DESCRIPTOR_POOL => "DescriptorPool",
        vk::ObjectType::DESCRIPTOR_SET => "DescriptorSet",
        vk::ObjectType::DESCRIPTOR_SET_LAYOUT => "DescriptorSetLayout",
        vk::ObjectType::DESCRIPTOR_UPDATE_TEMPLATE => "DescriptorUpdateTemplate",
        vk::ObjectType::DEVICE => "Device",
        vk::ObjectType::DEVICE_MEMORY => "DeviceMemory",
        vk::ObjectType::DISPLAY_KHR => "KHR::Display",
        vk::ObjectType::DISPLAY_MODE_KHR => "KHR::DisplayMode",
        vk::ObjectType::EVENT => "Event",
        vk::ObjectType::FENCE => "Fence",
        vk::ObjectType::FRAMEBUFFER => "Framebuffer",
        vk::ObjectType::IMAGE => "Image",
        vk::ObjectType::IMAGE_VIEW => "ImageView",
        vk::ObjectType::INDIRECT_COMMANDS_LAYOUT_NVX => "NVX::IndirectCommandsLayout",
        vk::ObjectType::INSTANCE => "Instance",
        vk::ObjectType::OBJECT_TABLE_NVX => "NVS::ObjectTable",
        vk::ObjectType::PHYSICAL_DEVICE => "PhysicalDevice",
        vk::ObjectType::PIPELINE => "Pipeline",
        vk::ObjectType::PIPELINE_CACHE => "PipelineCache",
        vk::ObjectType::PIPELINE_LAYOUT => "PipelineLayout",
        vk::ObjectType::QUERY_POOL => "QueryPool",
        vk::ObjectType::QUEUE => "Queue",
        vk::ObjectType::RENDER_PASS => "RenderPass",
        vk::ObjectType::SAMPLER => "Sampler",
        vk::ObjectType::SAMPLER_YCBCR_CONVERSION => "SamplerYcbcrConversion",
        vk::ObjectType::SEMAPHORE => "Semaphore",
        vk::ObjectType::SHADER_MODULE => "ShaderModule",
        vk::ObjectType::SURFACE_KHR => "KHR::Surface",
        vk::ObjectType::SWAPCHAIN_KHR => "KHR::Swapchain",
        vk::ObjectType::VALIDATION_CACHE_EXT => "EXT::ValidationCache",
        _ => "Unknown", // Includes vk::ObjectType::UNKNOWN
    }
}

unsafe extern "system" fn vulkan_debug_utils_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut c_void,
) -> vk::Bool32 {
    let severity = match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => "[Verbose]",
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => "[Warning]",
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => "[Error]",
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => "[Info]",
        _ => "[Unknown]",
    };
    let types = match message_type {
        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL => "[General]",
        vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => "[Performance]",
        vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION => "[Validation]",
        _ => "[Unknown]",
    };
    let message_cstring = CStr::from_ptr((*p_callback_data).p_message).to_owned();
    let message = message_cstring.to_string_lossy().to_owned();

    let mut formatted_error = if message_type.contains(vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION) {
        if message.contains("The Vulkan spec states:") {
            let vulkan_spec_quote_start = message
                .find("The Vulkan spec states:")
                .expect("Malformed validation message: {}");
            let spec_url_start = message
                .find("https://")
                .expect("Malformed validation message.");
            let error_text = &message[0..vulkan_spec_quote_start];
            let error_text = error_text.replace(": ", ":\n > ");
            let doc_quote = &message[vulkan_spec_quote_start..(spec_url_start - 2)];
            let url = &message[spec_url_start..(message.len() - 2)];
            format!(" > {}\nEXPLANATION:\n{}\n{}", error_text, doc_quote, url)
        } else {
            message.into()
        }
    } else {
        message.into()
    };

    let header = format!("[Debug]{}{}", severity, types);
    let header = if message_severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::ERROR) {
        header.bright_red().bold()
    } else if message_severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::WARNING) {
        header.bright_yellow().bold()
    } else if message_severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::INFO) {
        header.cyan()
    } else if message_severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE) {
        header.blue()
    } else {
        header.white()
    };
    println!("{}\n{}", header, formatted_error);

    vk::FALSE
}

pub fn set_debug_name<VkObject: Handle>(
    device: &ash::Device,
    debug_utils: &DebugUtils,
    object: VkObject,
    name: &str,
) {
    let name = CString::new(name).unwrap();
    let name_info = vk::DebugUtilsObjectNameInfoEXT {
        s_type: vk::StructureType::DEBUG_UTILS_OBJECT_NAME_INFO_EXT,
        object_type: VkObject::TYPE,
        object_handle: object.as_raw(),
        p_object_name: name.as_ptr(),
        ..Default::default()
    };
    unsafe {
        debug_utils
            .debug_utils_set_object_name(device.handle(), &name_info)
            .expect("Failed to set debug name.");
    }
}

pub fn setup_debug_utils(
    entry: &ash::Entry,
    instance: &ash::Instance,
) -> (ash::extensions::ext::DebugUtils, vk::DebugUtilsMessengerEXT) {
    let debug_utils_loader = ash::extensions::ext::DebugUtils::new(entry, instance);

    if ENABLE_DEBUG {
        let messenger_create_info = build_debug_utils_create_info();

        let utils_messenger = unsafe {
            debug_utils_loader
                .create_debug_utils_messenger(&messenger_create_info, None)
                .expect("Debug Utils Callback")
        };

        (debug_utils_loader, utils_messenger)
    } else {
        (debug_utils_loader, ash::vk::DebugUtilsMessengerEXT::null())
    }
}

pub fn build_debug_utils_create_info() -> vk::DebugUtilsMessengerCreateInfoEXT {
    vk::DebugUtilsMessengerCreateInfoEXT {
        s_type: vk::StructureType::DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT,
        p_next: ptr::null(),
        flags: vk::DebugUtilsMessengerCreateFlagsEXT::empty(),
        // TODO: Maybe command line flags to turn these on / off?
        message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::WARNING |
            // vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE |
            // vk::DebugUtilsMessageSeverityFlagsEXT::INFO |
            vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
        message_type: vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
            | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
            | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
        pfn_user_callback: Some(vulkan_debug_utils_callback),
        p_user_data: ptr::null_mut(),
    }
}
