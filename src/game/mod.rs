use cgmath::{InnerSpace, Rad, Vector3};
use winit::VirtualKeyCode;

use crate::render::Camera;
use crate::util;
use crate::world::World;

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
        set
    }

    pub fn new() -> Game {
        let mut result = Game {
            camera: Camera::new(),
            world: World::new(),
            controls: Self::make_controls(),
            sun_angle: 0.0,
        };
        result.camera.origin.x = 40.0;
        result.camera.origin.y = 40.0;
        result.camera.origin.z = 80.0;
        result
    }

    // Called after all controls have been updated.
    pub fn tick(&mut self, dt: f32) {
        self.sun_angle += dt * 0.5;

        let dx: f32 = if self.controls.is_held("left") {
            -1.0
        } else if self.controls.is_held("right") {
            1.0
        } else {
            0.0
        };
        let dy: f32 = if self.controls.is_held("backward") {
            -1.0
        } else if self.controls.is_held("forward") {
            1.0
        } else {
            0.0
        };
        let dz: f32 = if self.controls.is_held("down") {
            -1.0
        } else if self.controls.is_held("up") {
            1.0
        } else {
            0.0
        };
        let amount = dt * 50.0;
        let util::TripleEulerVector { forward, up, right } =
            util::compute_triple_euler_vector(self.camera.heading, self.camera.pitch);
        let forward = forward.normalize();
        let up = up.normalize();
        let right = right.normalize();
        self.camera.origin += amount * forward * dy + amount * up * dz + amount * right * dx;
    }

    pub fn on_mouse_move(&mut self, x: f64, y: f64) {
        self.camera.heading.0 = (-x / 80.0) as f32;
        self.camera.pitch.0 = ((256.0 - y) / 200.0) as f32;
    }

    pub fn borrow_world(&self) -> &World {
        &self.world
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