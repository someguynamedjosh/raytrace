use vulkano::device::Device;

use std::sync::Arc;

// Unfortunately the shader! macro does not trigger a recompile whenever source code changes.
fn _watchdog() {
    let _source = include_bytes!("../glsl/basic_raytrace.comp");
    let _source = include_bytes!("../glsl/bilateral_denoise.comp");
    let _source = include_bytes!("../glsl/screen.vert");
    let _source = include_bytes!("../glsl/screen.frag");
}

mod basic_raytrace {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "glsl/basic_raytrace.comp"
    }
}
pub use basic_raytrace::ty::PushData as RaytracePushData;
pub use basic_raytrace::Layout as BasicRaytraceShaderLayout;
pub use basic_raytrace::Shader as BasicRaytraceShader;

mod bilateral_denoise {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "glsl/bilateral_denoise.comp"
    }
}
pub use bilateral_denoise::Layout as BilateralDenoiseShaderLayout;
pub use bilateral_denoise::Shader as BilateralDenoiseShader;

mod screen_vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "glsl/screen.vert"
    }
}
pub use screen_fs::Layout as ScreenFragmentShaderLayout;
pub use screen_fs::Shader as ScreenFragmentShader;

mod screen_fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "glsl/screen.frag"
    }
}
pub use screen_vs::Layout as ScreenVertexShaderLayout;
pub use screen_vs::Shader as ScreenVertexShader;

pub fn load_basic_raytrace_shader(device: Arc<Device>) -> BasicRaytraceShader {
    basic_raytrace::Shader::load(device).unwrap()
}

pub fn load_bilateral_denoise_shader(device: Arc<Device>) -> BilateralDenoiseShader {
    bilateral_denoise::Shader::load(device).unwrap()
}

pub fn load_screen_vertex_shader(device: Arc<Device>) -> ScreenVertexShader {
    screen_vs::Shader::load(device).unwrap()
}

pub fn load_screen_fragment_shader(device: Arc<Device>) -> ScreenFragmentShader {
    screen_fs::Shader::load(device).unwrap()
}
