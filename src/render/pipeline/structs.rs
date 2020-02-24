use cgmath::Vector3;

#[repr(C)]
#[derive(Clone, Debug)]
pub struct RaytraceUniformData {
    pub sun_angle: f32,
    pub seed: u32,
    pub _padding0: u64,
    pub origin: Vector3<f32>,
    pub _padding1: u32,
    pub forward: Vector3<f32>,
    pub _padding2: u32,
    pub up: Vector3<f32>,
    pub _padding3: u32,
    pub right: Vector3<f32>,
    pub _padding4: u32,
    pub old_origin: Vector3<f32>,
    pub _padding5: u32,
    pub old_transform_c0: Vector3<f32>,
    pub _padding6: u32,
    pub old_transform_c1: Vector3<f32>,
    pub _padding7: u32,
    pub old_transform_c2: Vector3<f32>,
    pub _padding8: u32,
    pub region_offset: Vector3<i32>,
    pub _padding9: u32,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct DenoisePushData {
    pub size: i32,
}
