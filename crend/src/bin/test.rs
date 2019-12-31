extern crate crend;

use glfw::{ClientApiHint, WindowHint, WindowMode};

fn main() {
    let mut glfw_instance = glfw::init(glfw::FAIL_ON_ERRORS).expect("Failed to init GLFW.");

    glfw_instance.window_hint(WindowHint::ClientApi(ClientApiHint::NoApi));
    glfw_instance.window_hint(WindowHint::Resizable(false));
    let (window, _events) = glfw_instance
        .create_window(512, 512, "Test window", WindowMode::Windowed)
        .expect("Failed to create GLFW window.");

    let required_extensions = glfw_instance
        .get_required_instance_extensions()
        .unwrap_or_default();
    
    crend::init(&required_extensions);

    while !window.should_close() {
        glfw_instance.poll_events();
    }
}
