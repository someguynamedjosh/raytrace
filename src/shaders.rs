use vulkano::device::Device;

use std::sync::Arc;

// Unfortunately the shader! macro does not trigger a recompile whenever source code changes.
fn _watchdog() {
    let _source = include_bytes!("../shaders/compute.comp");
    let _source = include_bytes!("../shaders/vertex.vert");
    let _source = include_bytes!("../shaders/fragment.frag");
}

mod cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "shaders/compute.comp"
    }
}

// Shaders.
mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/vertex.vert"
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/fragment.frag"
    }
}

pub use cs::Layout as ComputeShaderLayout;
pub use cs::Shader as ComputeShader;
pub use fs::Layout as FragmentShaderLayout;
pub use fs::Shader as FragmentShader;
pub use vs::Layout as VertexShaderLayout;
pub use vs::Shader as VertexShader;

pub fn load_compute(device: Arc<Device>) -> ComputeShader {
    cs::Shader::load(device).unwrap()
}

pub fn load_vertex(device: Arc<Device>) -> VertexShader {
    vs::Shader::load(device).unwrap()
}

pub fn load_fragment(device: Arc<Device>) -> FragmentShader {
    fs::Shader::load(device).unwrap()
}
