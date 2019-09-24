use vulkano::device::{Device};

use std::sync::Arc;

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

pub fn load_compute(device: Arc<Device>) -> cs::Shader {
    cs::Shader::load(device).unwrap()
}

pub fn load_vertex(device: Arc<Device>) -> vs::Shader {
    vs::Shader::load(device).unwrap()
}

pub fn load_fragment(device: Arc<Device>) -> fs::Shader {
    fs::Shader::load(device).unwrap()
}