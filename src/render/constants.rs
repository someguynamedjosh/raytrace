use ash::vk_make_version;

// Core constants.
pub const APPLICATION_VERSION: u32 = vk_make_version!(1, 0, 0);
pub const ENGINE_VERSION: u32 = vk_make_version!(1, 0, 0);
pub const API_VERSION: u32 = vk_make_version!(1, 0, 92);

pub const WINDOW_TITLE: &str = "Hello world";
pub const WINDOW_WIDTH: u32 = 1024;
pub const WINDOW_HEIGHT: u32 = 1024;
pub const ENABLE_DEBUG: bool = cfg!(debug_assertions);
pub const VALIDATION_LAYERS: &[&str] = &["VK_LAYER_KHRONOS_validation"];
pub const DEVICE_EXTENSIONS: &[&str] = &["VK_KHR_swapchain"];

// Pipeline constants.
pub const BLUE_NOISE_WIDTH: u32 = 512;
pub const BLUE_NOISE_HEIGHT: u32 = 512;
pub const BLUE_NOISE_CHANNELS: u32 = 4;
pub const BLUE_NOISE_SIZE: u32 = BLUE_NOISE_WIDTH * BLUE_NOISE_HEIGHT * BLUE_NOISE_CHANNELS;

pub const ROOT_BLOCK_WIDTH: u32 = 256;
pub const ROOT_BLOCK_VOLUME: u32 = ROOT_BLOCK_WIDTH * ROOT_BLOCK_WIDTH * ROOT_BLOCK_WIDTH;

pub const NUM_UPLOAD_BUFFERS: usize = 32;
pub const SHADER_GROUP_SIZE: u32 = 8; // Each compute shader works on 8x8 groups.
