use cgmath::InnerSpace;
use winit::event::VirtualKeyCode;

use crate::render::Camera;
use crate::util;
use crate::world::{self, World};

use std::env;

pub mod control;

use control::ControlSet;

pub struct Game {
    camera: Camera,
    world: World,
    controls: ControlSet,

    sun_angle: f32,
}

impl Game {
    fn make_controls() -> ControlSet {
        let mut set = ControlSet::new();
        set.add_control("up", VirtualKeyCode::E);
        set.add_control("down", VirtualKeyCode::Q);
        set.add_control("left", VirtualKeyCode::A);
        set.add_control("right", VirtualKeyCode::D);
        set.add_control("forward", VirtualKeyCode::W);
        set.add_control("backward", VirtualKeyCode::S);

        set.add_control("sunup", VirtualKeyCode::R);
        set.add_control("sundown", VirtualKeyCode::F);
        set
    }

    pub fn new() -> Game {
        let mut result = Game {
            camera: Camera::new(),
            world: world::make_world(),
            controls: Self::make_controls(),
            sun_angle: 0.0,
        };
        result.camera.origin.x = 446.0;
        result.camera.origin.y = 283.0;
        result.camera.origin.z = 39.0;
        result.camera.heading.0 = -5.26;
        result.camera.pitch.0 = -0.20;
        result.sun_angle = 1.3;
        result
    }

    // Called after all controls have been updated.
    pub fn tick(&mut self, dt: f32) {
    }

    pub fn on_mouse_move(&mut self, x: f64, y: f64) {
    }

    pub fn borrow_world(&self) -> &World {
        &self.world
    }

    pub fn borrow_world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn borrow_camera(&self) -> &Camera {
        &self.camera
    }

    pub fn borrow_controls(&self) -> &ControlSet {
        &self.controls
    }

    pub fn borrow_controls_mut(&mut self) -> &mut ControlSet {
        &mut self.controls
    }

    pub fn get_sun_angle(&self) -> f32 {
        self.sun_angle
    }
}
