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

/// X is least significant (changes the fastest).
pub fn index_to_coord_2d(index: usize, stride: usize) -> (usize, usize) {
    (index % stride, index / stride % stride)
}

/// X is least significant (changes the fastest).
pub fn coord_to_index_2d(coord: &(usize, usize), stride: usize) -> usize {
    coord.1 * stride + coord.0
}

pub fn scale_coord_2d(coord: &(usize, usize), scale: usize) -> (usize, usize) {
    (coord.0 * scale, coord.1 * scale)
}

/// X is least significant (changes the fastest).
pub fn coord_iter_2d(size: usize) -> impl Iterator<Item = (usize, usize)> {
    let coord_iter = 0..size;
    coord_iter.flat_map(move |y| (0..size).map(move |x| (x, y)))
}

pub type Coord3D = (usize, usize, usize);
pub type SignedCoord3D = (isize, isize, isize);

/// X is least significant (changes the fastest).
pub fn index_to_coord_3d(index: usize, stride: usize) -> Coord3D {
    (
        index % stride,
        index / stride % stride,
        index / stride / stride,
    )
}

/// X is least significant (changes the fastest).
pub fn coord_to_index_3d(coord: &Coord3D, stride: usize) -> usize {
    (coord.2 * stride + coord.1) * stride + coord.0
}

pub fn offset_coord_3d(coord: &Coord3D, offset: &Coord3D) -> Coord3D {
    (coord.0 + offset.0, coord.1 + offset.1, coord.2 + offset.2)
}

pub fn scale_coord_3d(coord: &Coord3D, scale: usize) -> Coord3D {
    (coord.0 * scale, coord.1 * scale, coord.2 * scale)
}

pub fn shrink_coord_3d(coord: &Coord3D, divisor: usize) -> Coord3D {
    (coord.0 / divisor, coord.1 / divisor, coord.2 / divisor)
}

pub fn offset_signed_coord_3d(coord: &SignedCoord3D, offset: &SignedCoord3D) -> SignedCoord3D {
    (coord.0 + offset.0, coord.1 + offset.1, coord.2 + offset.2)
}

pub fn scale_signed_coord_3d(coord: &SignedCoord3D, scale: isize) -> SignedCoord3D {
    (coord.0 * scale, coord.1 * scale, coord.2 * scale)
}

pub fn coord_to_signed_coord(coord: &Coord3D) -> SignedCoord3D {
    (coord.0 as isize, coord.1 as isize, coord.2 as isize)
}

pub fn shrink_signed_coord_3d(coord: &SignedCoord3D, divisor: isize) -> SignedCoord3D {
    (coord.0 / divisor, coord.1 / divisor, coord.2 / divisor)
}

/// X is least significant (changes the fastest).
pub fn coord_iter_3d(size: usize) -> impl Iterator<Item = Coord3D> {
    let coord_iter = 0..size;
    let coord_iter = coord_iter.flat_map(move |z| (0..size).map(move |y| (y, z)));
    coord_iter.flat_map(move |yz| (0..size).map(move |x| (x, yz.0, yz.1)))
}

pub struct RingBufferAverage<ElementType> {
    elements: Vec<ElementType>,
    current_index: usize,
}

impl<ElementType> RingBufferAverage<ElementType>
where
    ElementType: std::ops::Add<ElementType, Output = ElementType>
        + std::ops::Div<ElementType, Output = ElementType>
        + Default
        + Copy,
    u64: Into<ElementType>,
{
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0);
        let mut vec = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            vec.push(Default::default());
        }
        Self {
            elements: vec,
            current_index: 0,
        }
    }

    pub fn average(&self) -> ElementType {
        let sum = self
            .elements
            .iter()
            .fold(Default::default(), |sum: ElementType, item| sum + *item);
        sum / (self.elements.len() as u64).into()
    }

    pub fn push_sample(&mut self, sample: ElementType) {
        self.elements[self.current_index] = sample;
        self.current_index = (self.current_index + 1) % self.elements.len();
    }
}
