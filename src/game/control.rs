use winit::VirtualKeyCode;
use std::collections::HashMap;

struct Control {
    last_state: bool,
    this_state: bool,
}

pub struct ControlSet {
    controls: Vec<Control>,
    by_name: HashMap<String, usize>,
    by_code: HashMap<VirtualKeyCode, usize>,
}

impl ControlSet {
    pub fn new() -> ControlSet {
        ControlSet {
            controls: Vec::new(),
            by_name: HashMap::new(),
            by_code: HashMap::new(),
        }
    }

    pub fn add_control(&mut self, name: &str, binding: VirtualKeyCode) {
        let index = self.controls.len();
        self.controls.push(Control {
            last_state: false,
            this_state: false,
        });
        self.by_name.insert(name.to_owned(), index);
        self.by_code.insert(binding, index);
    }

    // Call this before passing in any key events.
    pub fn tick(&mut self) {
        for control in &mut self.controls {
            control.last_state = control.this_state;
        }
    }

    pub fn on_pressed(&mut self, code: VirtualKeyCode) {
        if let Some(index) = self.by_code.get(&code) {
            self.controls[*index].this_state = true;
        }
    }

    pub fn on_released(&mut self, code: VirtualKeyCode) {
        if let Some(index) = self.by_code.get(&code) {
            self.controls[*index].this_state = false;
        }
    }

    // True if the control is currently being held down.
    pub fn is_held(&self, name: &str) -> bool {
        if let Some(index) = self.by_name.get(name) {
            self.controls[*index].this_state
        } else {
            false
        }
    }

    // True if the control was just pressed this frame.
    pub fn is_pressed(&self, name: &str) -> bool {
        if let Some(index) = self.by_name.get(name) {
            let control = &self.controls[*index];
            control.this_state && !control.last_state
        } else {
            false
        }
    }

    // True if the control was just released this frame.
    pub fn is_released(&self, name: &str) -> bool {
        if let Some(index) = self.by_name.get(name) {
            let control = &self.controls[*index];
            !control.this_state && control.last_state
        } else {
            false
        }
    }
}