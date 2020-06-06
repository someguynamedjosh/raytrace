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

pub trait CoordUtil {
    fn add(self, other: Self) -> Self;
    fn sub(self, other: Self) -> Self;
    fn inside(self, other: Self) -> bool;
}

impl CoordUtil for (usize, usize, usize) {
    fn add(self, other: Self) -> Self {
        (self.0 + other.0, self.1 + other.1, self.2 + other.2)
    }

    fn sub(self, other: Self) -> Self {
        (self.0 - other.0, self.1 - other.1, self.2 - other.2)
    }

    fn inside(self, other: Self) -> bool {
        self.0 <= other.0 && self.1 <= other.1 && self.2 <= other.2
    }
}

impl CoordUtil for (i32, i32, i32) {
    fn add(self, other: Self) -> Self {
        (self.0 + other.0, self.1 + other.1, self.2 + other.2)
    }

    fn sub(self, other: Self) -> Self {
        (self.0 - other.0, self.1 - other.1, self.2 - other.2)
    }

    fn inside(self, other: Self) -> bool {
        self.0 <= other.0 && self.1 <= other.1 && self.2 <= other.2
    }
}

impl CoordUtil for (isize, isize, isize) {
    fn add(self, other: Self) -> Self {
        (self.0 + other.0, self.1 + other.1, self.2 + other.2)
    }

    fn sub(self, other: Self) -> Self {
        (self.0 - other.0, self.1 - other.1, self.2 - other.2)
    }

    fn inside(self, other: Self) -> bool {
        self.0 <= other.0 && self.1 <= other.1 && self.2 <= other.2
    }
}

pub trait CoordConvertSigned<T> {
    fn sign(self) -> T;
}

impl CoordConvertSigned<SignedCoord3D> for Coord3D {
    fn sign(self) -> SignedCoord3D {
        (self.0 as isize, self.1 as isize, self.2 as isize)
    }
}

pub mod traits {
    pub use super::CoordConvertSigned;
    pub use super::CoordUtil;
}

/// Copies data from one array to another, assuming both arrays represent 3d data. data_size
/// dictates the dimensions of the area that will be copied. source_start specifies where in the
/// source data to start copying from. target_position specifies where in the target data to start
/// copying to. Panics if copying would go out of bounds. If you want the copy's size to be
/// automatically computed, use copy_3d_auto_clip instead. X coordinate is the least significant.
pub fn copy_3d<T: Copy>(
    data_size: (usize, usize, usize),
    source: &[T],
    source_stride: usize,
    source_start: (usize, usize, usize),
    target: &mut [T],
    target_stride: usize,
    target_position: (usize, usize, usize),
) {
    // Ensure the strides provided are correct.
    assert!(source.len() == source_stride * source_stride * source_stride);
    assert!(target.len() == target_stride * target_stride * target_stride);
    // Ensure the copy doesn't go out of bounds.
    assert!(source_start
        .add(data_size)
        .inside((source_stride, source_stride, source_stride,)));
    assert!(target_position
        .add(data_size)
        .inside((target_stride, target_stride, target_stride,)));
    let mut source_index = coord_to_index_3d(&source_start, source_stride);
    let mut target_index = coord_to_index_3d(&target_position, target_stride);
    for _z in 0..data_size.2 {
        for _y in 0..data_size.1 {
            for _x in 0..data_size.0 {
                target[target_index] = source[source_index];
                // Increasing the x coordinate by 1 increases the index by 1.
                source_index += 1;
                target_index += 1;
            }
            // Increasing the y coordinate by 1 increases the index by stride.
            source_index += source_stride - data_size.0;
            target_index += target_stride - data_size.0;
        }
        // Increasing the z coordinate by 1 increases the index by stride ^ 2.
        // The -data_size.1 works to counteract all the times stride was added for the y coordinate.
        source_index += source_stride * (source_stride - data_size.1);
        target_index += target_stride * (target_stride - data_size.1);
    }
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
        source_stride,
        source_start,
        target,
        target_stride,
        target_position,
    );
}

/// Functions like copy_3d_auto_clip except that source_size dictates the area that can be copied
/// from the source data. It should not be bigger than the actual source data.
pub fn copy_3d_bounded_auto_clip<T: Copy>(
    source: &[T],
    source_stride: usize,
    source_offset: SignedCoord3D,
    source_size: Coord3D,
    target: &mut [T],
    target_stride: usize,
) {
    assert!(
        source_size.0 <= source_stride
            && source_size.1 <= source_stride
            && source_size.2 <= source_stride
    );
    let data_size = source_stride.min(target_stride);
    let mut data_size = (
        data_size.min(source_size.0),
        data_size.min(source_size.1),
        data_size.min(source_size.2),
    );
    let mut source_start = (0, 0, 0);
    let mut target_position = (0, 0, 0);
    // If the source is placed at a negative coordinate, copying should start at the positive of
    // that coordinate. Otherwise, copying should target that coordinate.
    if source_offset.0 < 0 {
        source_start.0 = -source_offset.0 as usize;
        if source_start.0 >= data_size.0 {
            return;
        }
        data_size.0 -= source_start.0;
    } else {
        target_position.0 = source_offset.0 as usize;
    }
    if source_offset.1 < 0 {
        source_start.1 = -source_offset.1 as usize;
        if source_start.1 >= data_size.1 {
            return;
        }
        data_size.1 -= source_start.1;
    } else {
        target_position.1 = source_offset.1 as usize;
    }
    if source_offset.2 < 0 {
        source_start.2 = -source_offset.2 as usize;
        if source_start.2 >= data_size.2 {
            return;
        }
        data_size.2 -= source_start.2;
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
        source_stride,
        source_start,
        target,
        target_stride,
        target_position,
    );
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
