use cgmath::{Rad, Vector3};

use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::descriptor::descriptor_set::{DescriptorSet, PersistentDescriptorSet};
use vulkano::descriptor::pipeline_layout::PipelineLayout;
use vulkano::device::{Device, Queue};
use vulkano::format::Format;
use vulkano::image::{Dimensions, StorageImage};
use vulkano::pipeline::ComputePipeline;

use noise::{NoiseFn, Perlin};

use std::sync::Arc;

use crate::util;
use shaders::{self, BasicRaytraceShaderLayout, CameraVectorPushConstants};

type WorldData = CpuAccessibleBuffer<[u16]>;
type WorldImage = StorageImage<Format>;
type BasicRaytracePipeline = ComputePipeline<PipelineLayout<BasicRaytraceShaderLayout>>;

type GenericImage = StorageImage<Format>;
type GenericDescriptorSet = dyn DescriptorSet + Sync + Send;

const PIECE_BLOCK_WIDTH: u32 = 4; // pieces are 8x8x8 blocks
const CHUNK_PIECE_WIDTH: u32 = 4; // chunks are 8x8x8 pieces
const CHUNK_BLOCK_WIDTH: u32 = CHUNK_PIECE_WIDTH * PIECE_BLOCK_WIDTH;

const PIECE_BLOCK_VOLUME: u32 = PIECE_BLOCK_WIDTH * PIECE_BLOCK_WIDTH * PIECE_BLOCK_WIDTH;
const CHUNK_PIECE_VOLUME: u32 = CHUNK_PIECE_WIDTH * CHUNK_PIECE_WIDTH * CHUNK_PIECE_WIDTH;
const CHUNK_BLOCK_VOLUME: u32 = CHUNK_BLOCK_WIDTH * CHUNK_BLOCK_WIDTH * CHUNK_BLOCK_WIDTH;

const ROOT_CHUNK_WIDTH: u32 = 32; // root is 64x64x64 chunks.
const ROOT_BLOCK_WIDTH: u32 = ROOT_CHUNK_WIDTH * CHUNK_BLOCK_WIDTH;
const ROOT_CHUNK_VOLUME: u32 = ROOT_CHUNK_WIDTH * ROOT_CHUNK_WIDTH * ROOT_CHUNK_WIDTH;
const ATLAS_CHUNK_WIDTH: u32 = 16; // atlas is 4x4x4 chunks
const ATLAS_PIECE_WIDTH: u32 = ATLAS_CHUNK_WIDTH * CHUNK_PIECE_WIDTH;
const ATLAS_BLOCK_WIDTH: u32 = ATLAS_CHUNK_WIDTH * CHUNK_BLOCK_WIDTH;
const ATLAS_CHUNK_VOLUME: u32 = ATLAS_CHUNK_WIDTH * ATLAS_CHUNK_WIDTH * ATLAS_CHUNK_WIDTH;

const EMPTY_CHUNK_INDEX: u16 = 0xFFFF;
const UNLOADED_CHUNK_INDEX: u16 = 0xFFFE;
const REQUEST_LOAD_CHUNK_INDEX: u16 = 0xFFFD;

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

    chunk_upload_waiting: bool,
    chunk_upload_index: u16,

    block_data: Arc<WorldData>,
    block_data_atlas: Arc<WorldImage>,
    piece_mip: Arc<WorldData>,
    piece_mip_atlas: Arc<WorldImage>,

    root_data: Arc<WorldData>,
    root_image: Arc<WorldImage>,

    world: World,

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

enum WorldChunk {
    Ungenerated,
    Empty,
    Occupied(Box<Chunk>),
}

impl Default for WorldChunk {
    fn default() -> Self {
        WorldChunk::Ungenerated
    }
}

struct World {
    chunks: Vec<WorldChunk>,
}

impl World {
    fn new() -> World {
        let mut world = World {
            chunks: Vec::new()
        };
        for _ in 0..ROOT_CHUNK_VOLUME {
            world.chunks.push(WorldChunk::Ungenerated);
        }
        world.generate();
        world.finalize();
        world
    }

    fn draw_block(&mut self, x: usize, y: usize, z: usize, value: u16) {
        let (cx, cy, cz) = (
            x / CHUNK_BLOCK_WIDTH as usize,
            y / CHUNK_BLOCK_WIDTH as usize,
            z / CHUNK_BLOCK_WIDTH as usize,
        );
        let (bx, by, bz) = (
            x % CHUNK_BLOCK_WIDTH as usize,
            y % CHUNK_BLOCK_WIDTH as usize,
            z % CHUNK_BLOCK_WIDTH as usize,
        );
        let (px, py, pz) = (
            bx / PIECE_BLOCK_WIDTH as usize,
            by / PIECE_BLOCK_WIDTH as usize,
            bz / PIECE_BLOCK_WIDTH as usize,
        );
        let chunk_index = (cz * ROOT_CHUNK_WIDTH as usize + cy) * ROOT_CHUNK_WIDTH as usize + cx;
        let block_index = (bz * CHUNK_BLOCK_WIDTH as usize + by) * CHUNK_BLOCK_WIDTH as usize + bx;
        let piece_index = (pz * CHUNK_PIECE_WIDTH as usize + py) * CHUNK_PIECE_WIDTH as usize + px;
        if let WorldChunk::Ungenerated = self.chunks[chunk_index] {
            self.chunks[chunk_index] = WorldChunk::Occupied(Box::new(Chunk::new()));
        }
        if let WorldChunk::Occupied(chunk) = &mut self.chunks[chunk_index] {
            chunk.block_data[block_index] = value;
            chunk.piece_mip[piece_index] += 1;
        }
    }

    fn generate(&mut self) {
        let perlin = Perlin::new();
        for x in 0..ROOT_BLOCK_WIDTH as usize {
            for y in 0..ROOT_BLOCK_WIDTH as usize {
                let height = (perlin.get([x as f64 / 250.0, y as f64 / 250.0]) * 7.0 + 10.0) as usize;
                for z in 0..height {
                    self.draw_block(x, y, z, 1);
                }
            }
        }
    }

    fn finalize(&mut self) {
        for i in 0..ROOT_CHUNK_VOLUME as usize {
            if let WorldChunk::Ungenerated = self.chunks[i] {
                self.chunks[i] = WorldChunk::Empty;
            }
        }
    }
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
        let chunk = Chunk::new();

        let block_data = CpuAccessibleBuffer::from_iter(
            self.device.clone(),
            BufferUsage::all(),
            chunk.block_data.into_iter().map(|e| *e),
        )
        .unwrap();

        let piece_mip = CpuAccessibleBuffer::from_iter(
            self.device.clone(),
            BufferUsage::all(),
            chunk.piece_mip.into_iter().map(|e| *e),
        )
        .unwrap();

        let block_data_atlas = StorageImage::new(
            self.device.clone(),
            Dimensions::Dim3d {
                width: ATLAS_BLOCK_WIDTH,
                height: ATLAS_BLOCK_WIDTH,
                depth: ATLAS_BLOCK_WIDTH,
            },
            Format::R16Uint,
            Some(self.queue.family()),
        )
        .unwrap();

        let piece_mip_atlas = StorageImage::new(
            self.device.clone(),
            Dimensions::Dim3d {
                width: ATLAS_PIECE_WIDTH,
                height: ATLAS_PIECE_WIDTH,
                depth: ATLAS_PIECE_WIDTH,
            },
            Format::R16Uint,
            Some(self.queue.family()),
        )
        .unwrap();

        (block_data, block_data_atlas, piece_mip, piece_mip_atlas)
    }

    fn build(self) -> Renderer {
        let (target_width, target_height) = match self.target_image.dimensions() {
            Dimensions::Dim2d { width, height } => (width, height),
            _ => panic!("A non-2d image was passed as the target of a Renderer."),
        };

        let (block_data, block_data_atlas, piece_mip, piece_mip_atlas) = self.make_world();

        let root_data = CpuAccessibleBuffer::from_iter(
            self.device.clone(),
            BufferUsage::all(),
            (0..ROOT_CHUNK_VOLUME).map(|_| UNLOADED_CHUNK_INDEX),
        )
        .unwrap();
        let root_image = StorageImage::new(
            self.device.clone(),
            Dimensions::Dim3d {
                width: ROOT_CHUNK_WIDTH,
                height: ROOT_CHUNK_WIDTH,
                depth: ROOT_CHUNK_WIDTH,
            },
            Format::R16Uint,
            Some(self.queue.family()),
        )
        .unwrap();

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
                .add_image(block_data_atlas.clone())
                .unwrap()
                .add_image(piece_mip_atlas.clone())
                .unwrap()
                .add_image(root_image.clone())
                .unwrap()
                .add_image(self.target_image.clone())
                .unwrap()
                .build()
                .unwrap(),
        );

        Renderer {
            target_width,
            target_height,

            chunk_upload_waiting: false,
            chunk_upload_index: 0,

            block_data,
            block_data_atlas,
            piece_mip,
            piece_mip_atlas,

            root_data,
            root_image,

            world: World::new(),

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
        if self.chunk_upload_waiting {
            let (x, y, z) = (
                self.chunk_upload_index as u32 % ATLAS_CHUNK_WIDTH,
                self.chunk_upload_index as u32 / ATLAS_CHUNK_WIDTH % ATLAS_CHUNK_WIDTH,
                self.chunk_upload_index as u32 / ATLAS_CHUNK_WIDTH / ATLAS_CHUNK_WIDTH,
            );
            add_to = add_to
                .copy_buffer_to_image_dimensions(
                    self.block_data.clone(),
                    self.block_data_atlas.clone(),
                    [
                        x * CHUNK_BLOCK_WIDTH,
                        y * CHUNK_BLOCK_WIDTH,
                        z * CHUNK_BLOCK_WIDTH,
                    ],
                    [CHUNK_BLOCK_WIDTH, CHUNK_BLOCK_WIDTH, CHUNK_BLOCK_WIDTH],
                    0,
                    0,
                    0,
                )
                .unwrap()
                .copy_buffer_to_image_dimensions(
                    self.piece_mip.clone(),
                    self.piece_mip_atlas.clone(),
                    [
                        x * CHUNK_PIECE_WIDTH,
                        y * CHUNK_PIECE_WIDTH,
                        z * CHUNK_PIECE_WIDTH,
                    ],
                    [CHUNK_PIECE_WIDTH, CHUNK_PIECE_WIDTH, CHUNK_PIECE_WIDTH],
                    0,
                    0,
                    0,
                )
                .unwrap();
            self.chunk_upload_waiting = false;
        }
        add_to
            .copy_buffer_to_image(self.root_data.clone(), self.root_image.clone())
            .unwrap()
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
            .copy_image_to_buffer(self.root_image.clone(), self.root_data.clone())
            .unwrap()
    }

    pub fn read_feedback(&mut self) {
        let mut content = self.root_data.write().unwrap();
        for i in 0..ROOT_CHUNK_VOLUME as usize {
            if content[i] == REQUEST_LOAD_CHUNK_INDEX {
                if let WorldChunk::Occupied(chunk) = &mut self.world.chunks[i] {
                    if self.chunk_upload_waiting {
                        continue;
                    }
                    self.chunk_upload_index += 1;
                    self.chunk_upload_waiting = true;
                    content[i] = self.chunk_upload_index;
                    let mut block_data = self.block_data.write().unwrap();
                    let mut piece_mip = self.piece_mip.write().unwrap();
                    for block_index in 0..CHUNK_BLOCK_VOLUME as usize {
                        block_data[block_index] = chunk.block_data[block_index];
                    }
                    for piece_index in 0..CHUNK_PIECE_VOLUME as usize {
                        piece_mip[piece_index] = chunk.piece_mip[piece_index];
                    }
                } else {
                    content[i] = EMPTY_CHUNK_INDEX;
                }
            }
        }
    }
}
