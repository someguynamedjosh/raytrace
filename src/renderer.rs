use cgmath::{Rad, Vector3};

use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::descriptor::descriptor_set::{DescriptorSet, PersistentDescriptorSet};
use vulkano::descriptor::pipeline_layout::PipelineLayout;
use vulkano::device::{Device, Queue};
use vulkano::format::Format;
use vulkano::image::{Dimensions, StorageImage};
use vulkano::pipeline::ComputePipeline;

use std::sync::Arc;

use crate::shaders::{
    self, AssignLightmapsShaderLayout, BasicRaytraceShaderLayout, CameraVectorPushConstants,
    FinalizeShaderLayout, UpdateLightmapsShaderLayout,
};
use crate::util;

type WorldData = CpuAccessibleBuffer<[u16]>;
type WorldImage = StorageImage<Format>;
type LatResetBuffer = CpuAccessibleBuffer<[u32]>;
type BasicRaytracePipeline = ComputePipeline<PipelineLayout<BasicRaytraceShaderLayout>>;
type AssignLightmapsPipeline = ComputePipeline<PipelineLayout<AssignLightmapsShaderLayout>>;
type UpdateLightmapsPipeline = ComputePipeline<PipelineLayout<UpdateLightmapsShaderLayout>>;
type FinalizePipeline = ComputePipeline<PipelineLayout<FinalizeShaderLayout>>;

type GenericImage = StorageImage<Format>;
type GenericDescriptorSet = dyn DescriptorSet + Sync + Send;

const WORLD_SIZE: usize = 256;
const L2_STEP: usize = 16;
const L2_SIZE: usize = WORLD_SIZE / L2_STEP;

const LIGHTMAP_SIZE: u32 = L2_STEP as u32; // One lightmap covers LIGHTMAP_SIZE^3 voxels.

const LIGHTMAP_RES_L0: u32 = 8; // Pixels per voxel.
const LIGHTMAP_QUANTITY_L0: u32 = 32; // Number of lightmaps to create.
const LIGHTMAP_ATLAS_WIDTH_L0: u32 = 8; // Number of lightmaps to pack along the x axis of atlas.
                                        // Height is calculated from QUANTITY / WIDTH.

const LIGHTMAP_RES_L1: u32 = 4;
const LIGHTMAP_QUANTITY_L1: u32 = 128;
const LIGHTMAP_ATLAS_WIDTH_L1: u32 = 16;

const LIGHTMAP_RES_L2: u32 = 2;
const LIGHTMAP_QUANTITY_L2: u32 = 512;
const LIGHTMAP_ATLAS_WIDTH_L2: u32 = 32;

// Calculations show each lightmap atlas contains 24 million pixels. Assuming 32 bits per pixel,
// this makes all atlases consume 288MB.

// Enough to store LIGHTMAP_QUANTITY_L2 + 2 extra bytes, rounded up.
const LIGHTMAP_TABLE_SIZE: u32 = 1024;
// Value 0: lightmap is used? value 1-3: xyz of region where used.
const LIGHTMAP_DESCRIPTION_WIDTH: u32 = 16;
// Enough to hold indexes of all lightmaps + 1 extra value, rounded up.
const LIGHTMAP_UPDATE_QUEUE_LENGTH: u32 = 2048; 

// Positive Y (angle PI / 2) is forward
// Positive X is to the right
// Positive Z is up
// Heading starts at Positive X and goes clockwise (towards Positive Y).
// Pitch starts at zero and positive pitch looks up at Positive Z.
pub struct Camera {
    pub origin: Vector3<f32>,
    pub heading: Rad<f32>,
    pub pitch: Rad<f32>,
}

pub struct Renderer {
    target_width: u32,
    target_height: u32,
    image_update_requested: bool,

    world_l1_data: Arc<WorldData>,
    world_l1_image: Arc<WorldImage>,
    world_l2_data: Arc<WorldData>,
    world_l2_image: Arc<WorldImage>,

    lightmap_assignment_buffer: Arc<GenericImage>,
    lightmap_availability_table: Arc<GenericImage>,
    lat_reset_buffer: Arc<LatResetBuffer>,
    lightmap_operation_buffer: Arc<GenericImage>,
    lightmap_table: Arc<GenericImage>,
    lightmap_usage_buffer: Arc<GenericImage>,
    lightmap_update_queue: Arc<GenericImage>,

    lightmap_atlas_l0: Arc<GenericImage>,
    lightmap_atlas_l1: Arc<GenericImage>,
    lightmap_atlas_l2: Arc<GenericImage>,

    basic_raytrace_pipeline: Arc<BasicRaytracePipeline>,
    basic_raytrace_descriptors: Arc<GenericDescriptorSet>,
    assign_lightmaps_pipeline: Arc<AssignLightmapsPipeline>,
    assign_lightmaps_descriptors: Arc<GenericDescriptorSet>,
    update_lightmaps_pipeline: Arc<UpdateLightmapsPipeline>,
    update_lightmaps_descriptors: Arc<GenericDescriptorSet>,
    finalize_pipeline: Arc<FinalizePipeline>,
    finalize_descriptors: Arc<GenericDescriptorSet>,
}

struct RenderBuilder {
    device: Arc<Device>,
    queue: Arc<Queue>,
    target_image: Arc<GenericImage>,
}

impl RenderBuilder {
    fn make_world(
        &self,
    ) -> (
        Arc<WorldData>,
        Arc<WorldImage>,
        Arc<WorldData>,
        Arc<WorldImage>,
    ) {
        let world_l1_data = CpuAccessibleBuffer::from_iter(
            self.device.clone(),
            BufferUsage::all(),
            (0..WORLD_SIZE * WORLD_SIZE * WORLD_SIZE).map(|_| 0u16),
        )
        .unwrap();

        let world_l2_data = CpuAccessibleBuffer::from_iter(
            self.device.clone(),
            BufferUsage::all(),
            (0..L2_SIZE * L2_SIZE * L2_SIZE).map(|_| 0u16),
        )
        .unwrap();

        let mut target = world_l1_data.write().unwrap();
        let mut l2 = world_l2_data.write().unwrap();
        let mut index = 0;
        for z in 0..WORLD_SIZE {
            for y in 0..WORLD_SIZE {
                for x in 0..WORLD_SIZE {
                    let offset = if x % 15 > 10 && y % 15 > 10 { 30 } else { 0 };
                    let (l2x, l2y, l2z) = (x / L2_STEP, y / L2_STEP, z / L2_STEP);
                    let l2_index = ((l2z * L2_SIZE) + l2y) * L2_SIZE + l2x;
                    if z < (x + y + offset) / 4 {
                        target[index] = 10;
                        l2[l2_index] = 10;
                    }
                    if x == 0 && y == 10 {
                        target[index] = 10;
                        l2[l2_index] = 10;
                    }
                    if y == 0 && x == 30 {
                        target[index] = 10;
                        l2[l2_index] = 10;
                    }
                    index += 1;
                }
            }
        }
        drop(target);
        drop(l2);

        let world_l1_image = StorageImage::new(
            self.device.clone(),
            Dimensions::Dim3d {
                width: WORLD_SIZE as u32,
                height: WORLD_SIZE as u32,
                depth: WORLD_SIZE as u32,
            },
            Format::R16Uint,
            Some(self.queue.family()),
        )
        .unwrap();

        let world_l2_image = StorageImage::new(
            self.device.clone(),
            Dimensions::Dim3d {
                width: L2_SIZE as u32,
                height: L2_SIZE as u32,
                depth: L2_SIZE as u32,
            },
            Format::R16Uint,
            Some(self.queue.family()),
        )
        .unwrap();

        (world_l1_data, world_l1_image, world_l2_data, world_l2_image)
    }

    fn make_lightmap_atlas(
        &self,
        pixels_per_voxel: u32,
        quantity: u32,
        atlas_width: u32,
    ) -> Arc<GenericImage> {
        let lightmap_resolution = pixels_per_voxel * LIGHTMAP_SIZE;
        // *3 because we need seperate sets of layers for x, y, and z axes.
        let layers_per_lightmap = LIGHTMAP_SIZE * 3;
        let atlas = StorageImage::new(
            self.device.clone(),
            Dimensions::Dim3d {
                width: lightmap_resolution * atlas_width,
                height: lightmap_resolution * (quantity / atlas_width),
                depth: layers_per_lightmap,
            },
            Format::R32Uint,
            Some(self.queue.family()),
        )
        .unwrap();
        atlas
    }

    fn build_lat(&self) -> (Arc<GenericImage>, Arc<LatResetBuffer>) {
        let lightmap_availability_table = StorageImage::new(
            self.device.clone(),
            Dimensions::Dim2d {
                width: LIGHTMAP_TABLE_SIZE,
                height: 3,
            },
            Format::R32Uint,
            Some(self.queue.family()),
        )
        .unwrap();
        let mut reset_data = Vec::new();
        for lightmap_resolution in 0..3 {
            let atlas_size = match lightmap_resolution {
                0 => LIGHTMAP_QUANTITY_L0,
                1 => LIGHTMAP_QUANTITY_L1,
                2 => LIGHTMAP_QUANTITY_L2,
                _ => unreachable!(),
            };
            for index in 0..LIGHTMAP_TABLE_SIZE - 2 {
                reset_data.push(index);
            }
            // Elements 0..LIGHTMAP_AVAILABILITY_TABLE_SIZE represent indexes of available
            // lightmaps of whatever our current resolution is.
            reset_data.push(0);
            reset_data.push(atlas_size);
        }
        let lat_reset_buffer = CpuAccessibleBuffer::from_iter(
            self.device.clone(),
            BufferUsage::all(),
            reset_data.into_iter(),
        )
        .unwrap();
        (lightmap_availability_table, lat_reset_buffer)
    }

    fn build(self) -> Renderer {
        let (target_width, target_height) = match self.target_image.dimensions() {
            Dimensions::Dim2d { width, height } => (width, height),
            _ => panic!("A non-2d image was passed as the target of a Renderer."),
        };

        let (world_l1_data, world_l1_image, world_l2_data, world_l2_image) = self.make_world();

        let position_buffer = StorageImage::new(
            self.device.clone(),
            self.target_image.dimensions(),
            Format::R16G16B16A16Sfloat,
            Some(self.queue.family()),
        )
        .unwrap();
        let hit_result_buffer = StorageImage::new(
            self.device.clone(),
            self.target_image.dimensions(),
            Format::R16Uint,
            Some(self.queue.family()),
        )
        .unwrap();

        let lightmap_assignment_buffer = StorageImage::new(
            self.device.clone(),
            Dimensions::Dim3d {
                width: L2_SIZE as u32,
                height: L2_SIZE as u32,
                depth: L2_SIZE as u32,
            },
            Format::R32Uint,
            Some(self.queue.family()),
        )
        .unwrap();
        let (lightmap_availability_table, lat_reset_buffer) = self.build_lat();
        let lightmap_operation_buffer = StorageImage::new(
            self.device.clone(),
            Dimensions::Dim3d {
                width: L2_SIZE as u32,
                height: L2_SIZE as u32,
                depth: L2_SIZE as u32,
            },
            Format::R32Uint,
            Some(self.queue.family()),
        )
        .unwrap();
        let lightmap_table = StorageImage::new(
            self.device.clone(),
            Dimensions::Dim3d {
                width: LIGHTMAP_TABLE_SIZE,
                height: LIGHTMAP_DESCRIPTION_WIDTH,
                depth: 4u32, // Only going to use 3 but powers of two.
            },
            Format::R32Uint,
            Some(self.queue.family()),
        )
        .unwrap();
        let lightmap_usage_buffer = StorageImage::new(
            self.device.clone(),
            Dimensions::Dim1d { width: 16u32 },
            Format::R32Uint,
            Some(self.queue.family()),
        )
        .unwrap();
        let lightmap_update_queue = StorageImage::new(
            self.device.clone(),
            Dimensions::Dim1d { width: LIGHTMAP_UPDATE_QUEUE_LENGTH },
            Format::R32Uint,
            Some(self.queue.family()),
        )
        .unwrap();

        let lightmap_atlas_l0 = self.make_lightmap_atlas(
            LIGHTMAP_RES_L0,
            LIGHTMAP_QUANTITY_L0,
            LIGHTMAP_ATLAS_WIDTH_L0,
        );
        let lightmap_atlas_l1 = self.make_lightmap_atlas(
            LIGHTMAP_RES_L1,
            LIGHTMAP_QUANTITY_L1,
            LIGHTMAP_ATLAS_WIDTH_L1,
        );
        let lightmap_atlas_l2 = self.make_lightmap_atlas(
            LIGHTMAP_RES_L2,
            LIGHTMAP_QUANTITY_L2,
            LIGHTMAP_ATLAS_WIDTH_L2,
        );

        let basic_raytrace_shader = shaders::load_basic_raytrace_shader(self.device.clone());
        let assign_lightmaps_shader = shaders::load_assign_lightmaps_shader(self.device.clone());
        let update_lightmaps_shader = shaders::load_update_lightmaps_shader(self.device.clone());
        let finalize_shader = shaders::load_finalize_shader(self.device.clone());

        let basic_raytrace_pipeline = Arc::new(
            ComputePipeline::new(
                self.device.clone(),
                &basic_raytrace_shader.main_entry_point(),
                &(),
            )
            .unwrap(),
        );
        let basic_raytrace_descriptors: Arc<dyn DescriptorSet + Sync + Send> = Arc::new(
            PersistentDescriptorSet::start(basic_raytrace_pipeline.clone(), 0)
                .add_image(world_l1_image.clone())
                .unwrap()
                .add_image(world_l2_image.clone())
                .unwrap()
                .add_image(position_buffer.clone())
                .unwrap()
                .add_image(hit_result_buffer.clone())
                .unwrap()
                .add_image(lightmap_operation_buffer.clone())
                .unwrap()
                .add_image(lightmap_usage_buffer.clone())
                .unwrap()
                .build()
                .unwrap(),
        );

        let assign_lightmaps_pipeline = Arc::new(
            ComputePipeline::new(
                self.device.clone(),
                &assign_lightmaps_shader.main_entry_point(),
                &(),
            )
            .unwrap(),
        );
        let assign_lightmaps_descriptors: Arc<dyn DescriptorSet + Sync + Send> = Arc::new(
            PersistentDescriptorSet::start(assign_lightmaps_pipeline.clone(), 0)
                .add_image(lightmap_operation_buffer.clone())
                .unwrap()
                .add_image(lightmap_availability_table.clone())
                .unwrap()
                .add_image(lightmap_assignment_buffer.clone())
                .unwrap()
                .add_image(lightmap_table.clone())
                .unwrap()
                .add_image(lightmap_update_queue.clone())
                .unwrap()
                .build()
                .unwrap(),
        );

        let update_lightmaps_pipeline = Arc::new(
            ComputePipeline::new(
                self.device.clone(),
                &update_lightmaps_shader.main_entry_point(),
                &(),
            )
            .unwrap(),
        );
        let update_lightmaps_descriptors: Arc<dyn DescriptorSet + Sync + Send> = Arc::new(
            PersistentDescriptorSet::start(update_lightmaps_pipeline.clone(), 0)
                .add_image(world_l1_image.clone())
                .unwrap()
                .add_image(world_l2_image.clone())
                .unwrap()
                .add_image(lightmap_table.clone())
                .unwrap()
                .add_image(lightmap_update_queue.clone())
                .unwrap()
                .add_image(lightmap_atlas_l0.clone())
                .unwrap()
                .add_image(lightmap_atlas_l1.clone())
                .unwrap()
                .add_image(lightmap_atlas_l2.clone())
                .unwrap()
                .build()
                .unwrap(),
        );

        let finalize_pipeline = Arc::new(
            ComputePipeline::new(
                self.device.clone(),
                &finalize_shader.main_entry_point(),
                &(),
            )
            .unwrap(),
        );
        let finalize_descriptors: Arc<dyn DescriptorSet + Sync + Send> = Arc::new(
            PersistentDescriptorSet::start(finalize_pipeline.clone(), 0)
                .add_image(position_buffer.clone())
                .unwrap()
                .add_image(hit_result_buffer.clone())
                .unwrap()
                .add_image(lightmap_assignment_buffer.clone())
                .unwrap()
                .add_image(lightmap_table.clone())
                .unwrap()
                .add_image(lightmap_usage_buffer.clone())
                .unwrap()
                .add_image(self.target_image.clone())
                .unwrap()
                .add_image(lightmap_atlas_l0.clone())
                .unwrap()
                .add_image(lightmap_atlas_l1.clone())
                .unwrap()
                .add_image(lightmap_atlas_l2.clone())
                .unwrap()
                .build()
                .unwrap(),
        );

        Renderer {
            target_width,
            target_height,
            image_update_requested: true,

            world_l1_data,
            world_l1_image,
            world_l2_data,
            world_l2_image,

            lightmap_assignment_buffer,
            lightmap_availability_table,
            lat_reset_buffer,
            lightmap_operation_buffer,
            lightmap_table,
            lightmap_usage_buffer,
            lightmap_update_queue,

            lightmap_atlas_l0,
            lightmap_atlas_l1,
            lightmap_atlas_l2,

            basic_raytrace_pipeline,
            basic_raytrace_descriptors,
            assign_lightmaps_pipeline,
            assign_lightmaps_descriptors,
            update_lightmaps_pipeline,
            update_lightmaps_descriptors,
            finalize_pipeline,
            finalize_descriptors,
        }
    }
}

impl Renderer {
    pub fn new(
        device: Arc<Device>,
        queue: Arc<Queue>,
        target_image: Arc<GenericImage>,
    ) -> Renderer {
        RenderBuilder {
            device,
            queue,
            target_image,
        }
        .build()
    }

    pub fn add_render_commands(
        &mut self,
        mut add_to: AutoCommandBufferBuilder,
        camera: &Camera,
    ) -> AutoCommandBufferBuilder {
        let camera_pos = camera.origin;
        let util::TripleEulerVector { forward, up, right } =
            util::compute_triple_euler_vector(camera.heading, camera.pitch);
        if self.image_update_requested {
            add_to = add_to
                .copy_buffer_to_image(self.world_l1_data.clone(), self.world_l1_image.clone())
                .unwrap()
                .copy_buffer_to_image(self.world_l2_data.clone(), self.world_l2_image.clone())
                .unwrap()
                .clear_color_image(self.lightmap_assignment_buffer.clone(), [0x3000u32].into())
                .unwrap()
                .copy_buffer_to_image(
                    self.lat_reset_buffer.clone(),
                    self.lightmap_availability_table.clone(),
                )
                .unwrap()
                .clear_color_image(self.lightmap_table.clone(), [0x0u32].into())
                .unwrap();
            self.image_update_requested = false;
        }
        add_to
            .clear_color_image(self.lightmap_operation_buffer.clone(), [3u32].into())
            .unwrap()
            .clear_color_image(self.lightmap_usage_buffer.clone(), [0u32].into())
            .unwrap()
            .clear_color_image(self.lightmap_update_queue.clone(), [0u32].into())
            .unwrap()
            // Do initial raytrace to determine which voxels are on screen.
            .dispatch(
                [self.target_width / 8, self.target_height / 8, 1],
                self.basic_raytrace_pipeline.clone(),
                self.basic_raytrace_descriptors.clone(),
                CameraVectorPushConstants {
                    _dummy0: [0; 4],
                    _dummy1: [0; 4],
                    _dummy2: [0; 4],
                    origin: [camera_pos.x, camera_pos.y, camera_pos.z],
                    forward: [forward.x, forward.y, forward.z],
                    right: [right.x * 0.3, right.y * 0.3, right.z * 0.3],
                    up: [up.x * 0.3, up.y * 0.3, up.z * 0.3],
                },
            )
            .unwrap()
            .dispatch(
                [L2_SIZE as u32 / 4, L2_SIZE as u32 / 4, L2_SIZE as u32 / 4],
                self.assign_lightmaps_pipeline.clone(),
                self.assign_lightmaps_descriptors.clone(),
                (),
            )
            .unwrap()
            .dispatch(
                [LIGHTMAP_UPDATE_QUEUE_LENGTH / 64, 1, 1],
                self.update_lightmaps_pipeline.clone(),
                self.update_lightmaps_descriptors.clone(),
                (),
            )
            .unwrap()
            // Combine computed data into final image.
            .dispatch(
                [self.target_width / 8, self.target_height / 8, 1],
                self.finalize_pipeline.clone(),
                self.finalize_descriptors.clone(),
                (),
            )
            .unwrap()
    }
}
