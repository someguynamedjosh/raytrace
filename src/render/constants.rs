use ash::vk_make_version;

pub const APPLICATION_VERSION: u32 = vk_make_version!(1, 0, 0);
pub const ENGINE_VERSION: u32 = vk_make_version!(1, 0, 0);
pub const API_VERSION: u32 = vk_make_version!(1, 0, 92);

pub const WINDOW_TITLE: &str = "Hello world";
pub const WINDOW_WIDTH: u32 = 512;
pub const WINDOW_HEIGHT: u32 = 512;
pub const ENABLE_DEBUG: bool = cfg!(enable_debug_asserts);
pub const VALIDATION_LAYERS: &[&str] = &["VK_LAYER_KHRONOS_validation"];
pub const DEVICE_EXTENSIONS: &[&str] = &["VK_KHR_swapchain"];

pub const MAX_FRAMES_IN_FLIGHT: usize = 2;
