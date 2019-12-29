use cgmath::{Rad, Vector3};

pub struct TripleEulerVector {
    pub forward: Vector3<f32>,
    pub up: Vector3<f32>,
    pub right: Vector3<f32>,
}

pub fn compute_triple_euler_vector(heading: Rad<f32>, pitch: Rad<f32>) -> TripleEulerVector {
    let forward = Vector3 {
        x: heading.0.cos() * pitch.0.cos(),
        y: heading.0.sin() * pitch.0.cos(),
        z: pitch.0.sin(),
    };
    let up = Vector3 {
        x: heading.0.cos() * (pitch.0 + std::f32::consts::FRAC_PI_2).cos(),
        y: heading.0.sin() * (pitch.0 + std::f32::consts::FRAC_PI_2).cos(),
        z: (pitch.0 + std::f32::consts::FRAC_PI_2).sin(),
    };
    let right = forward.cross(up);
    TripleEulerVector { forward, up, right }
}

pub fn index_to_coord_2d(index: u32, stride: u32) -> (u32, u32) {
    (
        index % stride,
        index / stride % stride,
    )
}

pub fn coord_to_index_2d(coord: &(u32, u32), stride: u32) -> u32 {
    coord.1 * stride + coord.0
}

pub fn scale_coord_2d(coord: &(u32, u32), scale: u32) -> (u32, u32) {
    (coord.0 * scale, coord.1 * scale)
}

pub fn coord_iter_2d(size: u32) -> impl Iterator<Item = (u32, u32)> {
    let coord_iter = 0..size;
    coord_iter.flat_map(move |y| (0..size).map(move |x| (x, y)))
}

pub fn index_to_coord_3d(index: u32, stride: u32) -> (u32, u32, u32) {
    (
        index % stride,
        index / stride % stride,
        index / stride / stride,
    )
}

pub fn coord_to_index_3d(coord: &(u32, u32, u32), stride: u32) -> u32 {
    (coord.2 * stride + coord.1) * stride + coord.0
}

pub fn scale_coord_3d(coord: &(u32, u32, u32), scale: u32) -> (u32, u32, u32) {
    (coord.0 * scale, coord.1 * scale, coord.2 * scale)
}

pub fn coord_iter_3d(size: u32) -> impl Iterator<Item = (u32, u32, u32)> {
    let coord_iter = 0..size;
    let coord_iter = coord_iter.flat_map(move |z| (0..size).map(move |y| (y, z)));
    coord_iter.flat_map(move |yz| (0..size).map(move |x| (x, yz.0, yz.1)))
}
