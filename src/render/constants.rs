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
pub const BLUE_NOISE_WIDTH: usize = 512;
pub const BLUE_NOISE_HEIGHT: usize = 512;
pub const BLUE_NOISE_CHANNELS: usize = 4;
pub const BLUE_NOISE_SIZE: usize = BLUE_NOISE_WIDTH * BLUE_NOISE_HEIGHT * BLUE_NOISE_CHANNELS;

// The LOD that takes up an entire chunk.
pub const MAX_CHUNK_LOD: usize = 6;
pub const CHUNK_SIZE: usize = 1 << MAX_CHUNK_LOD; // 64
pub const CHUNK_VOLUME: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
// This should always be a multiple of 2
pub const ROOT_CHUNK_SIZE: usize = 4;
pub const ROOT_BLOCK_SIZE: usize = CHUNK_SIZE * ROOT_CHUNK_SIZE;
pub const ROOT_BLOCK_VOLUME: usize = ROOT_BLOCK_SIZE * ROOT_BLOCK_SIZE * ROOT_BLOCK_SIZE;
// Slices are used to upload new terrain data to the GPU.
pub const SLICE_SIZE: usize = 16;
pub const SLICES_PER_CHUNK: usize = CHUNK_SIZE / SLICE_SIZE;

pub const SHADER_GROUP_SIZE: usize = 8; // Each compute shader works on 8x8 groups.
