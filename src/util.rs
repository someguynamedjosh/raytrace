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

pub type Coord2D = (usize, usize);
pub type SignedCoord2D = (isize, isize);

/// X is least significant (changes the fastest).
pub fn index_to_coord_2d(index: usize, stride: usize) -> Coord2D {
    (index % stride, index / stride % stride)
}

/// X is least significant (changes the fastest).
pub fn coord_to_index_2d(coord: &Coord2D, stride: usize) -> usize {
    coord.1 * stride + coord.0
}

pub fn scale_coord_2d(coord: &Coord2D, scale: usize) -> Coord2D {
    (coord.0 * scale, coord.1 * scale)
}

/// X is least significant (changes the fastest).
pub fn index_to_signed_coord_2d(index: isize, stride: isize) -> SignedCoord2D {
    (index % stride, index / stride % stride)
}

/// X is least significant (changes the fastest).
pub fn signed_coord_to_index_2d(coord: &SignedCoord2D, stride: isize) -> isize {
    coord.1 * stride + coord.0
}

pub fn scale_signed_coord_2d(coord: &SignedCoord2D, scale: isize) -> SignedCoord2D {
    (coord.0 * scale, coord.1 * scale)
}

pub struct CoordIter2D {
    coord: (usize, usize),
    size: usize,
    first: bool,
}

impl Iterator for CoordIter2D {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        if self.first {
            self.first = false;
            return Some(self.coord.clone());
        }
        self.coord.0 += 1;
        if self.coord.0 == self.size {
            self.coord.0 = 0;
            self.coord.1 += 1;
            if self.coord.1 == self.size {
                return None;
            }
        }
        Some(self.coord.clone())
    }
}

/// X is least significant (changes 1:1 instead of 1:size or 1:size^2).
pub fn coord_iter_2d(size: usize) -> CoordIter2D {
    CoordIter2D {
        coord: (0, 0),
        size,
        first: true,
    }
}

pub type Coord3D = (usize, usize, usize);
pub type SignedCoord3D = (isize, isize, isize);

/// X is least significant (changes 1:1 instead of 1:size or 1:size^2).
pub fn index_to_coord_3d(index: usize, stride: usize) -> Coord3D {
    (
        index % stride,
        index / stride % stride,
        index / stride / stride,
    )
}

// TODO: Remove in favor of tuple util functions
/// X is least significant (changes 1:1 instead of 1:size or 1:size^2).
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

pub struct CoordIter3D {
    coord: (usize, usize, usize),
    size: usize,
    first: bool,
}

impl Iterator for CoordIter3D {
    type Item = (usize, usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        if self.first {
            self.first = false;
            return Some(self.coord.clone());
        }
        self.coord.0 += 1;
        if self.coord.0 == self.size {
            self.coord.0 = 0;
            self.coord.1 += 1;
            if self.coord.1 == self.size {
                self.coord.1 = 0;
                self.coord.2 += 1;
                if self.coord.2 == self.size {
                    return None;
                }
            }
        }
        Some(self.coord.clone())
    }
}

/// X is least significant (changes the fastest).
pub fn coord_iter_3d(size: usize) -> CoordIter3D {
    CoordIter3D {
        coord: (0, 0, 0),
        size,
        first: true,
    }
}

pub struct RingBufferAverage<ElementType> {
    elements: Vec<ElementType>,
    current_index: usize,
}

impl<ElementType> RingBufferAverage<ElementType>
where
    ElementType: std::ops::Add<ElementType, Output = ElementType>
        + std::ops::Div<ElementType, Output = ElementType>
        + std::cmp::Ord
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

    pub fn max(&self) -> ElementType {
        let max = self
            .elements
            .iter()
            .fold(Default::default(), |max: ElementType, item| max.max(*item));
        max
    }

    pub fn push_sample(&mut self, sample: ElementType) {
        self.elements[self.current_index] = sample;
        self.current_index = (self.current_index + 1) % self.elements.len();
    }
}

pub trait CoordUtil<ElementType> {
    fn add(self, other: Self) -> Self;
    fn sub(self, other: Self) -> Self;
    fn scale(self, factor: ElementType) -> Self;
    fn shrink(self, factor: ElementType) -> Self;
    /// Applies modulus to each coordinate.
    fn wrap(self, bounds: Self) -> Self;
    fn ewmin(self, other: Self) -> Self;
    fn ewmax(self, other: Self) -> Self;
    fn inside(self, other: Self) -> bool;
    /// Increasing the x position by one increases the index by 1. Increasing the z position
    /// increases the index the most.
    fn to_index(self, dims: Self) -> ElementType;
}

use std::cmp::Ord;
use std::ops::{Add, Div, Mul, Rem, Sub};

impl<T> CoordUtil<T> for (T, T)
where
    T: Copy
        + Add<Output = T>
        + Sub<Output = T>
        + Mul<Output = T>
        + Div<Output = T>
        + Rem<Output = T>
        + Ord,
{
    fn add(self, other: Self) -> Self {
        (self.0 + other.0, self.1 + other.1)
    }

    fn sub(self, other: Self) -> Self {
        (self.0 - other.0, self.1 - other.1)
    }

    fn scale(self, factor: T) -> Self {
        (self.0 * factor, self.1 * factor)
    }

    fn shrink(self, factor: T) -> Self {
        (self.0 / factor, self.1 / factor)
    }

    fn wrap(self, other: Self) -> Self {
        (self.0 % other.0, self.1 % other.1)
    }

    fn ewmin(self, other: Self) -> Self {
        (self.0.min(other.0), self.1.min(other.1))
    }

    fn ewmax(self, other: Self) -> Self {
        (self.0.max(other.0), self.1.max(other.1))
    }

    fn inside(self, other: Self) -> bool {
        self.0 <= other.0 && self.1 <= other.1
    }

    fn to_index(self, dims: Self) -> T {
        self.1 * dims.0 + self.0
    }
}

impl<T> CoordUtil<T> for (T, T, T)
where
    T: Copy
        + Add<Output = T>
        + Sub<Output = T>
        + Mul<Output = T>
        + Div<Output = T>
        + Rem<Output = T>
        + Ord,
{
    fn add(self, other: Self) -> Self {
        (self.0 + other.0, self.1 + other.1, self.2 + other.2)
    }

    fn sub(self, other: Self) -> Self {
        (self.0 - other.0, self.1 - other.1, self.2 - other.2)
    }

    fn scale(self, factor: T) -> Self {
        (self.0 * factor, self.1 * factor, self.2 * factor)
    }

    fn shrink(self, factor: T) -> Self {
        (self.0 / factor, self.1 / factor, self.2 / factor)
    }

    fn wrap(self, other: Self) -> Self {
        (self.0 % other.0, self.1 % other.1, self.2 % other.2)
    }

    fn ewmin(self, other: Self) -> Self {
        (
            self.0.min(other.0),
            self.1.min(other.1),
            self.2.min(other.2),
        )
    }

    fn ewmax(self, other: Self) -> Self {
        (
            self.0.max(other.0),
            self.1.max(other.1),
            self.2.max(other.2),
        )
    }

    fn inside(self, other: Self) -> bool {
        self.0 <= other.0 && self.1 <= other.1 && self.2 <= other.2
    }

    fn to_index(self, dims: Self) -> T {
        (self.2 * dims.1 + self.1) * dims.0 + self.0
    }
}

pub trait CoordConvertSigned<T> {
    fn signed(self) -> T;
}

impl CoordConvertSigned<SignedCoord3D> for Coord3D {
    fn signed(self) -> SignedCoord3D {
        (self.0 as isize, self.1 as isize, self.2 as isize)
    }
}

pub trait CoordRepeat<T> {
    fn repeat(self) -> T;
}

impl<T: Copy> CoordRepeat<(T, T, T)> for T {
    fn repeat(self) -> (T, T, T) {
        (self, self, self)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Axis {
    X,
    Y,
    Z,
}

pub mod prelude {
    pub use super::Axis;
    pub use super::{Coord2D, Coord3D, SignedCoord2D, SignedCoord3D};
    pub use super::{CoordConvertSigned, CoordRepeat, CoordUtil};
}

/// Copies data from one array to another, assuming both arrays represent 3d data. data_size
/// dictates the dimensions of the area that will be copied. source_start specifies where in the
/// source data to start copying from. target_position specifies where in the target data to start
/// copying to. Panics if copying would go out of bounds. If you want the copy's size to be
/// automatically computed, use copy_3d_auto_clip instead. X coordinate is the least significant.
pub fn copy_3d<T: Copy>(
    data_size: (usize, usize, usize),
    source: &[T],
    source_dims: Coord3D,
    source_start: (usize, usize, usize),
    target: &mut [T],
    target_dims: Coord3D,
    target_position: (usize, usize, usize),
) {
    // Ensure the strides provided are correct.
    assert!(source.len() == source_dims.0 * source_dims.1 * source_dims.2);
    assert!(target.len() == target_dims.0 * target_dims.1 * target_dims.2);
    // Ensure the copy doesn't go out of bounds.
    assert!(source_start.add(data_size).inside(source_dims));
    assert!(target_position.add(data_size).inside(target_dims));
    let mut source_index = source_start.to_index(source_dims);
    let mut target_index = target_position.to_index(target_dims);
    for _z in 0..data_size.2 {
        for _y in 0..data_size.1 {
            for _x in 0..data_size.0 {
                target[target_index] = source[source_index];
                // Increasing the x coordinate by 1 increases the index by 1.
                source_index += 1;
                target_index += 1;
            }
            // Increasing the y coordinate by 1 increases the index by dims.0.
            source_index += source_dims.0 - data_size.0;
            target_index += target_dims.0 - data_size.0;
        }
        // Increasing the z coordinate by 1 increases the index by dims.0 * dims.1.
        // The -data_size.1 works to counteract all the times dims.0 was added for the y coordinate.
        source_index += source_dims.0 * (source_dims.1 - data_size.1);
        target_index += target_dims.0 * (target_dims.1 - data_size.1);
    }
}

#[test]
fn test_copy_3d() {
    use array_macro::array;
    use rand::prelude::*;
    let source_dims = (4, 4, 6);
    let source = array![|_| rand::thread_rng().next_u32(); 96];
    let mut target = array![0; 125];
    copy_3d(
        (2, 2, 2),
        &source,
        source_dims,
        (1, 2, 2),
        &mut target[..],
        (5, 5, 5),
        (3, 2, 1),
    );
    assert!(source[(1, 2, 2).to_index(source_dims)] == target[(3, 2, 1).to_index((5, 5, 5))]);
    assert!(source[(1, 2, 3).to_index(source_dims)] == target[(3, 2, 2).to_index((5, 5, 5))]);
}

/// Copies all the data from source to target in the area that they overlap. The target data is
/// considered to be placed at (0, 0, 0), and the source data can be thought as being placed inside
/// the target data at source_offset.
pub fn copy_3d_auto_clip<T: Copy>(
    source: &[T],
    source_stride: usize,
    source_offset: SignedCoord3D,
    target: &mut [T],
    target_stride: usize,
) {
    let data_size = source_stride.min(target_stride);
    let mut data_size = (data_size, data_size, data_size);
    let mut source_start = (0, 0, 0);
    let mut target_position = (0, 0, 0);
    // If the source is placed at a negative coordinate, copying should start at the positive of
    // that coordinate. Otherwise, copying should target that coordinate.
    if source_offset.0 < 0 {
        source_start.0 = -source_offset.0 as usize;
    } else {
        target_position.0 = source_offset.0 as usize;
    }
    if source_offset.1 < 0 {
        source_start.1 = -source_offset.1 as usize;
    } else {
        target_position.1 = source_offset.1 as usize;
    }
    if source_offset.2 < 0 {
        source_start.2 = -source_offset.2 as usize;
    } else {
        target_position.2 = source_offset.2 as usize;
    }
    // Shrink the boundaries if copying would end up going out of bounds.
    data_size.0 = data_size
        .0
        .min(source_stride - source_start.0)
        .min(target_stride - target_position.0);
    data_size.1 = data_size
        .1
        .min(source_stride - source_start.1)
        .min(target_stride - target_position.1);
    data_size.2 = data_size
        .2
        .min(source_stride - source_start.2)
        .min(target_stride - target_position.2);
    if data_size == (0, 0, 0) {
        return;
    }
    // Do the actual copy
    copy_3d(
        data_size,
        source,
        source_stride.repeat(),
        source_start,
        target,
        target_stride.repeat(),
        target_position,
    );
}

#[test]
fn test_copy_3d_auto_clip() {
    use array_macro::array;
    use rand::prelude::*;
    let source = array![|_| rand::thread_rng().next_u32(); 64];
    let mut target = array![0; 64];
    copy_3d_auto_clip(&source, 4, (3, 2, 2), &mut target[..], 4);
    assert!(source[coord_to_index_3d(&(0, 0, 0), 4)] == target[coord_to_index_3d(&(3, 2, 2), 4)]);
    assert!(source[coord_to_index_3d(&(0, 0, 1), 4)] == target[coord_to_index_3d(&(3, 2, 3), 4)]);
}

/// Copies a region of size `size` from source to target, assuming both are 3D arrays. source_start
/// and target_start are the coordinates to start copying at and start copying to. Negative
/// target coordinates and an overly large size are both valid, these values will be clipped to
/// create a valid copy based on the area that actually has both source data to copy from and target
/// data to copy to. The data that was originally located at source_start can be found at
/// target_start
pub fn copy_3d_bounded_auto_clip<T: Copy>(
    size: Coord3D,
    source: &[T],
    source_dims: Coord3D,
    mut source_start: Coord3D,
    target: &mut [T],
    target_dims: Coord3D,
    target_start: SignedCoord3D,
) {
    let mut data_size = source_dims.ewmin(target_dims).ewmin(size);
    // Where to *actually* copy the data to.
    let mut target_position = (0, 0, 0);
    // If the target starts at a negative coordinate, source_start should be increased by the
    // negated coordinate.
    if target_start.0 < 0 {
        let neg = -target_start.0 as usize;
        source_start.0 += neg;
        if neg >= data_size.0 {
            return;
        }
        data_size.0 -= neg;
    } else {
        target_position.0 = target_start.0 as usize;
        if target_position.0 >= target_dims.0 {
            return;
        }
    }
    if target_start.1 < 0 {
        let neg = -target_start.1 as usize;
        source_start.1 += neg;
        if neg >= data_size.1 {
            return;
        }
        data_size.1 -= neg;
    } else {
        target_position.1 = target_start.1 as usize;
        if target_position.1 >= target_dims.1 {
            return;
        }
    }
    if target_start.2 < 0 {
        let neg = -target_start.2 as usize;
        source_start.2 += neg;
        if neg >= data_size.2 {
            return;
        }
        data_size.2 -= neg;
    } else {
        target_position.2 = target_start.2 as usize;
        if target_position.2 >= target_dims.2 {
            return;
        }
    }
    // Shrink the boundaries if copying would end up going out of bounds.
    data_size = data_size
        .ewmin(source_dims.sub(source_start))
        .ewmin(target_dims.sub(target_position));
    if data_size.0 == 0 || data_size.1 == 0 || data_size.2 == 0 {
        return;
    }
    // Do the actual copy
    copy_3d(
        data_size,
        source,
        source_dims,
        source_start,
        target,
        target_dims,
        target_position,
    );
}

#[test]
fn test_copy_3d_bounded_auto_clip() {
    use array_macro::array;
    use rand::prelude::*;
    let source = array![|_| rand::thread_rng().next_u32(); 64];
    let mut target = array![0; 64];
    copy_3d_bounded_auto_clip(
        (1, 1, 2),
        &source,
        (4, 4, 4),
        (0, 0, 0),
        &mut target[..],
        (4, 4, 4),
        (3, 2, 2),
    );
    assert!(source[coord_to_index_3d(&(0, 0, 0), 4)] == target[coord_to_index_3d(&(3, 2, 2), 4)]);
    assert!(source[coord_to_index_3d(&(0, 0, 1), 4)] == target[coord_to_index_3d(&(3, 2, 3), 4)]);
    assert!(target[coord_to_index_3d(&(3, 3, 2), 4)] == 0);
}

pub fn fill_slice_3d<T: Copy>(
    value: T,
    target: &mut [T],
    target_stride: usize,
    slice_start: Coord3D,
    slice_size: Coord3D,
) {
    // Ensure the stride provided is correct.
    assert!(target.len() == target_stride * target_stride * target_stride);
    // Ensure the fill doesn't go out of bounds.
    assert!(slice_start
        .add(slice_size)
        .inside((target_stride, target_stride, target_stride,)));
    let mut target_index = coord_to_index_3d(&slice_start, target_stride);
    for _z in 0..slice_size.2 {
        for _y in 0..slice_size.1 {
            for _x in 0..slice_size.0 {
                target[target_index] = value;
                // Increasing the x coordinate by 1 increases the index by 1.
                target_index += 1;
            }
            // Increasing the y coordinate by 1 increases the index by stride.
            target_index += target_stride - slice_size.0;
        }
        // Increasing the z coordinate by 1 increases the index by stride ^ 2.
        // The -slice_size.1 works to counteract all the times stride was added for the y coordinate.
        target_index += target_stride * (target_stride - slice_size.1);
    }
}

/// Fills an area of a 3d data array with a single value. slice_start is the corner where the
/// filling should begin. It can be negative, which will cause it to automatically be truncated.
/// slice_size is how much to fill starting at slice_start. It can also be too big for the data
/// and will also be automatically truncated.
pub fn fill_slice_3d_auto_clip<T: Copy>(
    value: T,
    target: &mut [T],
    target_stride: usize,
    slice_start: SignedCoord3D,
    mut slice_size: Coord3D,
) {
    let mut real_slice_start = (0, 0, 0);
    if slice_start.0 < 0 {
        slice_size.0 -= -slice_start.0 as usize;
    } else {
        real_slice_start.0 = slice_start.0 as usize;
    }
    if slice_start.1 < 0 {
        slice_size.1 -= -slice_start.1 as usize;
    } else {
        real_slice_start.1 = slice_start.1 as usize;
    }
    if slice_start.2 < 0 {
        slice_size.2 -= -slice_start.2 as usize;
    } else {
        real_slice_start.2 = slice_start.2 as usize;
    }
    // Shrink the boundaries if filling would end up going out of bounds.
    slice_size.0 = slice_size.0.min(target_stride - real_slice_start.0);
    slice_size.1 = slice_size.1.min(target_stride - real_slice_start.1);
    slice_size.2 = slice_size.2.min(target_stride - real_slice_start.2);
    // Do the actual operation
    fill_slice_3d(value, target, target_stride, real_slice_start, slice_size);
}
