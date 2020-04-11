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
        let args: Vec<_> = env::args().collect();
        let mut result = Game {
            camera: Camera::new(),
            world: world::make_world(),
            controls: Self::make_controls(),
            sun_angle: 0.0,
        };
        if args.len() > 1 {
            result.camera.origin.x = args[1].parse().unwrap();
            result.camera.origin.y = args[2].parse().unwrap();
            result.camera.origin.z = args[3].parse().unwrap();
            result.camera.heading.0 = args[4].parse().unwrap();
            result.camera.pitch.0 = args[5].parse().unwrap();
            result.sun_angle = args[6].parse().unwrap();
        } else {
            result.camera.origin.x = 0.0;
            result.camera.origin.y = 0.0;
            result.camera.origin.z = 0.0;
        }
        result
    }

    // Called after all controls have been updated.
    pub fn tick(&mut self, dt: f32) {
        if self.controls.is_held("sunup") {
            self.sun_angle += dt * 1.0;
        } else if self.controls.is_held("sundown") {
            self.sun_angle -= dt * 1.0;
        }

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
