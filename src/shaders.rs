use vulkano::device::Device;

use std::sync::Arc;

// Unfortunately the shader! macro does not trigger a recompile whenever source code changes.
fn _watchdog() {
    let _source = include_bytes!("../shaders/compute.comp");
    let _source = include_bytes!("../shaders/vertex.vert");
    let _source = include_bytes!("../shaders/fragment.frag");
}

mod basic_raytrace {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "shaders/compute.comp"
    }
}

// Shaders.
mod screen_vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/vertex.vert"
    }
}

mod screen_fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/fragment.frag"
    }
}

pub use basic_raytrace::ty::CameraVectors as CameraVectorPushConstants;
pub use basic_raytrace::Layout as BasicRaytraceShaderLayout;
pub use basic_raytrace::Shader as BasicRaytraceShader;
pub use screen_fs::Layout as ScreenFragmentShaderLayout;
pub use screen_fs::Shader as ScreenFragmentShader;
pub use screen_vs::Layout as ScreenVertexShaderLayout;
pub use screen_vs::Shader as ScreenVertexShader;

pub fn load_basic_raytrace_shader(device: Arc<Device>) -> BasicRaytraceShader {
    basic_raytrace::Shader::load(device).unwrap()
}

pub fn load_screen_vertex_shader(device: Arc<Device>) -> ScreenVertexShader {
    screen_vs::Shader::load(device).unwrap()
}

pub fn load_screen_fragment_shader(device: Arc<Device>) -> ScreenFragmentShader {
    screen_fs::Shader::load(device).unwrap()
}
