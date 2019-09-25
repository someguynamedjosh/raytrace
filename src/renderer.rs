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

use crate::shaders::{self, CameraVectorPushConstants, BasicRaytraceShader, BasicRaytraceShaderLayout};
use crate::util;

type RendererInputData = CpuAccessibleBuffer<[u32]>;
type RendererInputImage = StorageImage<Format>;
type RendererComputePipeline = ComputePipeline<PipelineLayout<BasicRaytraceShaderLayout>>;

type GenericImage = StorageImage<Format>;

const WORLD_SIZE: usize = 256;
const L2_STEP: usize = 16;
const L2_SIZE: usize = WORLD_SIZE / L2_STEP;

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

#[derive(Clone, Debug, Default)]
struct Vertex {
    position: [f32; 2],
}

vulkano::impl_vertex!(Vertex, position);

pub struct Renderer {
    target_width: u32,
    target_height: u32,
    image_update_requested: bool,

    world_l1_data: Arc<RendererInputData>,
    world_l1_image: Arc<RendererInputImage>,
    world_l2_data: Arc<RendererInputData>,
    world_l2_image: Arc<RendererInputImage>,

    compute_pipeline: Arc<RendererComputePipeline>,
    compute_descriptors: Arc<dyn DescriptorSet + Sync + Send>,
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
        Arc<RendererInputData>,
        Arc<RendererInputImage>,
        Arc<RendererInputData>,
        Arc<RendererInputImage>,
    ) {
        let world_l1_data = CpuAccessibleBuffer::from_iter(
            self.device.clone(),
            BufferUsage::all(),
            (0..WORLD_SIZE * WORLD_SIZE * WORLD_SIZE).map(|_| 0u32),
        )
        .unwrap();

        let world_l2_data = CpuAccessibleBuffer::from_iter(
            self.device.clone(),
            BufferUsage::all(),
            (0..L2_SIZE * L2_SIZE * L2_SIZE).map(|_| 0u32),
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
            Format::R32Uint,
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
            Format::R32Uint,
            Some(self.queue.family()),
        )
        .unwrap();

        (world_l1_data, world_l1_image, world_l2_data, world_l2_image)
    }

    fn load_shaders(&self) -> (BasicRaytraceShader,) {
        (shaders::load_basic_raytrace_shader(self.device.clone()),)
    }

    fn build(self) -> Renderer {
        let (world_l1_data, world_l1_image, world_l2_data, world_l2_image) = self.make_world();
        let (compute_shader,) = self.load_shaders();

        let compute_pipeline = Arc::new(
            ComputePipeline::new(self.device.clone(), &compute_shader.main_entry_point(), &())
                .unwrap(),
        );

        let compute_descriptors: Arc<dyn DescriptorSet + Sync + Send> = Arc::new(
            PersistentDescriptorSet::start(compute_pipeline.clone(), 0)
                .add_image(world_l1_image.clone())
                .unwrap()
                .add_image(world_l2_image.clone())
                .unwrap()
                .add_image(self.target_image.clone())
                .unwrap()
                .build()
                .unwrap(),
        );

        let (target_width, target_height) = match self.target_image.dimensions() {
            Dimensions::Dim2d{width, height} => (width, height),
            _ => panic!("A non-2d image was passed as the target of a Renderer.")
        };

        Renderer {
            target_width,
            target_height,
            image_update_requested: true,

            world_l1_data,
            world_l1_image,
            world_l2_data,
            world_l2_image,

            compute_pipeline,
            compute_descriptors,
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
        &self,
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
                .unwrap();
        }
        add_to
            .dispatch(
                [self.target_width / 8, self.target_height / 8, 1],
                self.compute_pipeline.clone(),
                self.compute_descriptors.clone(),
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
    }
}
