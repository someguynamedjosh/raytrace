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

use crate::shaders::{self, BasicRaytraceShaderLayout, CameraVectorPushConstants};
use crate::util;

type WorldData = CpuAccessibleBuffer<[u16]>;
type WorldImage = StorageImage<Format>;
type BasicRaytracePipeline = ComputePipeline<PipelineLayout<BasicRaytraceShaderLayout>>;

type GenericImage = StorageImage<Format>;
type GenericDescriptorSet = dyn DescriptorSet + Sync + Send;

const PIECE_BLOCK_WIDTH: u32 = 8; // pieces are 8x8x8 blocks
const CHUNK_PIECE_WIDTH: u32 = 8; // chunks are 8x8x8 pieces
const CHUNK_BLOCK_WIDTH: u32 = CHUNK_PIECE_WIDTH * PIECE_BLOCK_WIDTH;

const PIECE_BLOCK_VOLUME: u32 = PIECE_BLOCK_WIDTH * PIECE_BLOCK_WIDTH * PIECE_BLOCK_WIDTH;
const CHUNK_PIECE_VOLUME: u32 = CHUNK_PIECE_WIDTH * CHUNK_PIECE_WIDTH * CHUNK_PIECE_WIDTH;
const CHUNK_BLOCK_VOLUME: u32 = CHUNK_BLOCK_WIDTH * CHUNK_BLOCK_WIDTH * CHUNK_BLOCK_WIDTH;

const ROOT_CHUNK_WIDTH: u32 = 64; // root is 64x64x64 chunks.
const ATLAS_CHUNK_WIDTH: u32 = 4; // atlas is 4x4x4 chunks
const ATLAS_CHUNK_VOLUME: u32 = ATLAS_CHUNK_WIDTH * ATLAS_CHUNK_WIDTH * ATLAS_CHUNK_WIDTH;

// Positive Y (angle PI / 2) is forward
// Positive X is to the right
// Positive Z is up
// Heading starts at Positive X and goes clockwise (towards Positive Y).
// Pitch starts at zero and positive pitch looks up at Positive Z.
#[derive(Debug)]
pub struct Camera {
    pub origin: Vector3<f32>,
    pub heading: Rad<f32>,
    pub pitch: Rad<f32>,
}

pub struct Renderer {
    target_width: u32,
    target_height: u32,
    image_update_requested: bool,

    block_data: Arc<WorldData>,
    block_data_image: Arc<WorldImage>,
    piece_mip: Arc<WorldData>,
    piece_mip_image: Arc<WorldImage>,

    basic_raytrace_pipeline: Arc<BasicRaytracePipeline>,
    basic_raytrace_descriptors: Arc<GenericDescriptorSet>,
}

struct Chunk {
    pub block_data: [u16; CHUNK_BLOCK_VOLUME as usize],
    pub piece_mip: [u16; CHUNK_PIECE_VOLUME as usize],
}

impl Chunk {
    fn new() -> Chunk {
        Chunk {
            block_data: [0; CHUNK_BLOCK_VOLUME as usize],
            piece_mip: [0; CHUNK_PIECE_VOLUME as usize],
        }
    }
}

fn generate_chunk() -> Chunk {
    let mut chunk = Chunk::new();
    let mut index = 0;
    for z in 0..CHUNK_BLOCK_WIDTH {
        for y in 0..CHUNK_BLOCK_WIDTH {
            for x in 0..CHUNK_BLOCK_WIDTH {
                let (mipx, mipy, mipz) = (
                    x / PIECE_BLOCK_WIDTH,
                    y / PIECE_BLOCK_WIDTH,
                    z / PIECE_BLOCK_WIDTH,
                );
                let mip_index =
                    (((mipz * CHUNK_PIECE_WIDTH) + mipy) * CHUNK_PIECE_WIDTH + mipx) as usize;
                if z < 8 {
                    chunk.block_data[index] = 1;
                    chunk.piece_mip[mip_index] = 10;
                }
                if x % 32 < 16 && y % 32 < 16 && z < 24 {
                    if z < 16 {
                        chunk.block_data[index] = 2;
                        chunk.piece_mip[mip_index] = 10;
                    } else if x % 16 / 8 == y % 16 / 8 {
                        chunk.block_data[index] = 3;
                        chunk.piece_mip[mip_index] = 10;
                    } else if x % 8 / 4 == y % 8 / 4 {
                        chunk.block_data[index] = 4;
                        chunk.piece_mip[mip_index] = 10;
                    }
                } else if x % 16 == 7 && y % 16 == 7 && z < 16 {
                    chunk.block_data[index] = 5;
                    chunk.piece_mip[mip_index] = 10;
                } else if x % 16 == 10 && y % 16 == 10 && z < 16 {
                    chunk.block_data[index] = 6;
                    chunk.piece_mip[mip_index] = 10;
                }
                index += 1;
            }
        }
    }
    chunk
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
        let chunk = generate_chunk();

        let block_data = CpuAccessibleBuffer::from_iter(
            self.device.clone(),
            BufferUsage::all(),
            chunk.block_data.into_iter().map(|e| *e)
        )
        .unwrap();

        let piece_mip = CpuAccessibleBuffer::from_iter(
            self.device.clone(),
            BufferUsage::all(),
            chunk.piece_mip.into_iter().map(|e| *e)
        )
        .unwrap();

        let block_data_image = StorageImage::new(
            self.device.clone(),
            Dimensions::Dim3d {
                width: CHUNK_BLOCK_WIDTH,
                height: CHUNK_BLOCK_WIDTH,
                depth: CHUNK_BLOCK_WIDTH,
            },
            Format::R16Uint,
            Some(self.queue.family()),
        )
        .unwrap();

        let piece_mip_image = StorageImage::new(
            self.device.clone(),
            Dimensions::Dim3d {
                width: CHUNK_PIECE_WIDTH,
                height: CHUNK_PIECE_WIDTH,
                depth: CHUNK_PIECE_WIDTH,
            },
            Format::R16Uint,
            Some(self.queue.family()),
        )
        .unwrap();

        (block_data, block_data_image, piece_mip, piece_mip_image)
    }

    fn build(self) -> Renderer {
        let (target_width, target_height) = match self.target_image.dimensions() {
            Dimensions::Dim2d { width, height } => (width, height),
            _ => panic!("A non-2d image was passed as the target of a Renderer."),
        };

        let (block_data, block_data_image, piece_mip, piece_mip_image)= self.make_world();

        let basic_raytrace_shader = shaders::load_basic_raytrace_shader(self.device.clone());

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
                .add_image(block_data_image.clone())
                .unwrap()
                .add_image(piece_mip_image.clone())
                .unwrap()
                .add_image(self.target_image.clone())
                .unwrap()
                .build()
                .unwrap(),
        );

        Renderer {
            target_width,
            target_height,
            image_update_requested: true,

            block_data, 
            block_data_image, 
            piece_mip, 
            piece_mip_image,

            basic_raytrace_pipeline,
            basic_raytrace_descriptors,
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
                .copy_buffer_to_image(self.block_data.clone(), self.block_data_image.clone())
                .unwrap()
                .copy_buffer_to_image(self.piece_mip.clone(), self.piece_mip_image.clone())
                .unwrap();
            self.image_update_requested = false;
        }
        add_to
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
    }
}
