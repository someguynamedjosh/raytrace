use cgmath::{Rad, Vector3};

use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::descriptor::descriptor_set::{DescriptorSet, PersistentDescriptorSet};
use vulkano::descriptor::pipeline_layout::PipelineLayout;
use vulkano::device::{Device, Queue};
use vulkano::format::Format;
use vulkano::image::{Dimensions, StorageImage};
use vulkano::pipeline::ComputePipeline;

use rand::{self, RngCore};
use noise::{NoiseFn, HybridMulti};

use std::sync::Arc;

use crate::util;
use shaders::{self, BasicRaytraceShaderLayout, CameraVectorPushConstants};

type WorldData = CpuAccessibleBuffer<[u16]>;
type WorldImage = StorageImage<Format>;
type BasicRaytracePipeline = ComputePipeline<PipelineLayout<BasicRaytraceShaderLayout>>;

type GenericImage = StorageImage<Format>;
type GenericDescriptorSet = dyn DescriptorSet + Sync + Send;

const CHUNK_BLOCK_WIDTH: u32 = 8;
const CHUNK_BLOCK_VOLUME: u32 = CHUNK_BLOCK_WIDTH * CHUNK_BLOCK_WIDTH * CHUNK_BLOCK_WIDTH;

const REGION_CHUNK_WIDTH: u32 = 8;
const REGION_BLOCK_WIDTH: u32 = REGION_CHUNK_WIDTH * CHUNK_BLOCK_WIDTH;

const ROOT_REGION_WIDTH: u32 = 8;
const ROOT_REGION_VOLUME: u32 = ROOT_REGION_WIDTH * ROOT_REGION_WIDTH * ROOT_REGION_WIDTH;
const ROOT_CHUNK_WIDTH: u32 = ROOT_REGION_WIDTH * REGION_CHUNK_WIDTH;
const ROOT_CHUNK_VOLUME: u32 = ROOT_CHUNK_WIDTH * ROOT_CHUNK_WIDTH * ROOT_CHUNK_WIDTH;
const ROOT_BLOCK_WIDTH: u32 = ROOT_CHUNK_WIDTH * CHUNK_BLOCK_WIDTH;

const ATLAS_CHUNK_WIDTH: u32 = 32; 
const ATLAS_BLOCK_WIDTH: u32 = ATLAS_CHUNK_WIDTH * CHUNK_BLOCK_WIDTH;
const ATLAS_CHUNK_VOLUME: u32 = ATLAS_CHUNK_WIDTH * ATLAS_CHUNK_WIDTH * ATLAS_CHUNK_WIDTH;

const NUM_UPLOAD_BUFFERS: usize = 32;

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

    chunk_upload_index: u16,

    upload_buffers: Vec<Arc<WorldData>>,
    upload_destinations: Vec<u16>,
    block_data_atlas: Arc<WorldImage>,

    chunk_map_data: Arc<WorldData>,
    chunk_map: Arc<WorldImage>,
    region_map_data: Arc<WorldData>,
    region_map: Arc<WorldImage>,

    world: World,

    basic_raytrace_pipeline: Arc<BasicRaytracePipeline>,
    basic_raytrace_descriptors: Arc<GenericDescriptorSet>,
}

struct Chunk {
    pub block_data: [u16; CHUNK_BLOCK_VOLUME as usize],
}

impl Chunk {
    fn new() -> Chunk {
        Chunk {
            block_data: [0; CHUNK_BLOCK_VOLUME as usize],
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
    regions: [bool; ROOT_REGION_VOLUME as usize],
}

impl World {
    fn new() -> World {
        let mut world = World {
            chunks: Vec::new(),
            regions: [false; ROOT_REGION_VOLUME as usize]
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
        let (rx, ry, rz) = (
            cx / REGION_CHUNK_WIDTH as usize,
            cy / REGION_CHUNK_WIDTH as usize,
            cz / REGION_CHUNK_WIDTH as usize,
        );
        let (bx, by, bz) = (
            x % CHUNK_BLOCK_WIDTH as usize,
            y % CHUNK_BLOCK_WIDTH as usize,
            z % CHUNK_BLOCK_WIDTH as usize,
        );
        let chunk_index = (cz * ROOT_CHUNK_WIDTH as usize + cy) * ROOT_CHUNK_WIDTH as usize + cx;
        let region_index = (rz * ROOT_REGION_WIDTH as usize + ry) * ROOT_REGION_WIDTH as usize + rx;
        let block_index = (bz * CHUNK_BLOCK_WIDTH as usize + by) * CHUNK_BLOCK_WIDTH as usize + bx;
        if let WorldChunk::Ungenerated = self.chunks[chunk_index] {
            self.chunks[chunk_index] = WorldChunk::Occupied(Box::new(Chunk::new()));
        }
        if let WorldChunk::Occupied(chunk) = &mut self.chunks[chunk_index] {
            chunk.block_data[block_index] = value;
        }
        self.regions[region_index] = true;
    }

    fn generate(&mut self) {
        let mut perlin = HybridMulti::new();
        perlin.octaves = 4;
        perlin.frequency = 0.4;
        perlin.lacunarity = 2.3;
        perlin.persistence = 0.6;
        let mut micro = HybridMulti::new();
        micro.octaves = 1;
        micro.frequency = 30.0;
        micro.lacunarity = 2.0;
        micro.persistence = 1.0;
        let mut random = rand::thread_rng();
        for x in 0..ROOT_BLOCK_WIDTH as usize {
            for y in 0..ROOT_BLOCK_WIDTH as usize {
                let coord = [x as f64 / 250.0, y as f64 / 250.0];
                let mut height = (perlin.get(coord) * 4.0 + micro.get(coord) * 0.0 + 20.0) as usize;
                if x == 200 && y == 200 {
                    height += 8;
                }
                for z in 0..height {
                    self.draw_block(x, y, z, if z == height - 1 { 1 } else { 3 });
                }
                if x > 15 && y > 15 && x < ROOT_BLOCK_WIDTH as usize - 15 && y < ROOT_BLOCK_WIDTH as usize - 15 && random.next_u32() % 10000 == 1 {
                    for z in height..height + 4 {
                        self.draw_block(x, y, z, 3);
                        self.draw_block(x+1, y, z, 3);
                        self.draw_block(x, y+1, z, 3);
                        self.draw_block(x-1, y, z, 3);
                        self.draw_block(x, y-1, z, 3);
                    }
                    for dx in 0..11 { for dy in 0..11 { for dz in 0..11 {
                        let radius = (dx as isize - 5).abs() + (dy as isize - 5).abs() + (dz as isize - 5).abs();
                        if radius < 8 {
                            if dx == 5 || dy == 5 || dz == 5 {
                                if radius < 7 {
                                    self.draw_block(x + dx - 5, y + dy - 5, height + dz + 4, 3);
                                }
                            } else {
                                self.draw_block(x + dx - 5, y + dy - 5, height + dz + 4, 2);
                            }
                        }
                    }}}
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
        World,
        Vec<Arc<WorldData>>,
        Arc<WorldImage>,
        Arc<WorldData>,
        Arc<WorldImage>,
        Arc<WorldData>,
        Arc<WorldImage>,
    ) {
        let world = World::new();

        let upload_buffers = (0..NUM_UPLOAD_BUFFERS).map(|_| {
            CpuAccessibleBuffer::from_iter(
                self.device.clone(),
                BufferUsage::all(),
                (0..CHUNK_BLOCK_VOLUME).map(|_| 0)
            )
            .unwrap()
        }).collect();

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

        let chunk_map_data = CpuAccessibleBuffer::from_iter(
            self.device.clone(), 
            BufferUsage::all(), 
            (0..ROOT_CHUNK_VOLUME).map(|_| UNLOADED_CHUNK_INDEX)
        )
        .unwrap();

        let chunk_map = StorageImage::new(
            self.device.clone(), 
            Dimensions::Dim3d {
                width: ROOT_CHUNK_WIDTH,
                height: ROOT_CHUNK_WIDTH,
                depth: ROOT_CHUNK_WIDTH
            }, 
            Format::R16Uint, 
            Some(self.queue.family())
        )
        .unwrap();

        let region_map_data = CpuAccessibleBuffer::from_iter(
            self.device.clone(), 
            BufferUsage::all(), 
            (0..ROOT_REGION_VOLUME as usize).map(|i| if world.regions[i] { 1 } else { EMPTY_CHUNK_INDEX })
        )
        .unwrap();

        let region_map = StorageImage::new(
            self.device.clone(), 
            Dimensions::Dim3d {
                width: ROOT_REGION_WIDTH,
                height: ROOT_REGION_WIDTH,
                depth: ROOT_REGION_WIDTH
            }, 
            Format::R16Uint, 
            Some(self.queue.family())
        )
        .unwrap();

        (world, upload_buffers, block_data_atlas, chunk_map_data, chunk_map, region_map_data, region_map)
    }

    fn build(self) -> Renderer {
        let (target_width, target_height) = match self.target_image.dimensions() {
            Dimensions::Dim2d { width, height } => (width, height),
            _ => panic!("A non-2d image was passed as the target of a Renderer."),
        };

        let (world, upload_buffers, block_data_atlas, chunk_map_data, chunk_map, region_map_data, region_map) = self.make_world();

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
                .add_image(chunk_map.clone())
                .unwrap()
                .add_image(region_map.clone())
                .unwrap()
                .add_image(self.target_image.clone())
                .unwrap()
                .build()
                .unwrap(),
        );

        Renderer {
            target_width,
            target_height,

            chunk_upload_index: 0,

            upload_buffers,
            upload_destinations: Vec::new(),
            block_data_atlas,
            chunk_map_data,
            chunk_map,
            region_map_data,
            region_map,

            world,

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
        for (source, destination) in self.upload_destinations.iter().enumerate() {
            let (x, y, z) = (
                *destination as u32 % ATLAS_CHUNK_WIDTH,
                *destination as u32 / ATLAS_CHUNK_WIDTH % ATLAS_CHUNK_WIDTH,
                *destination as u32 / ATLAS_CHUNK_WIDTH / ATLAS_CHUNK_WIDTH,
            );
            add_to = add_to
                .copy_buffer_to_image_dimensions(
                    self.upload_buffers[source].clone(),
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
                .unwrap();
        }
        self.upload_destinations.clear();
        add_to
            .copy_buffer_to_image(self.chunk_map_data.clone(), self.chunk_map.clone())
            .unwrap()
            .copy_buffer_to_image(self.region_map_data.clone(), self.region_map.clone())
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
            .copy_image_to_buffer(self.chunk_map.clone(), self.chunk_map_data.clone())
            .unwrap()
            .copy_image_to_buffer(self.region_map.clone(), self.region_map_data.clone())
            .unwrap()
    }

    pub fn read_feedback(&mut self) {
        let mut chunk_map = self.chunk_map_data.write().unwrap();
        let mut region_map = self.region_map_data.write().unwrap();
        let mut current_buffer = 0;
        for region_index in 0..ROOT_REGION_VOLUME as usize {
            let region_content = region_map[region_index];
            if region_content != REQUEST_LOAD_CHUNK_INDEX { continue; }
            let region_occupied = self.world.regions[region_index];
            if !region_occupied {
                region_map[region_index] = EMPTY_CHUNK_INDEX;
                continue;
            }
            let rx = region_index % ROOT_REGION_WIDTH as usize;
            let ry = region_index / ROOT_REGION_WIDTH as usize % ROOT_REGION_WIDTH as usize;
            let rz = region_index / ROOT_REGION_WIDTH as usize / ROOT_REGION_WIDTH as usize;
            let rx = rx * REGION_CHUNK_WIDTH as usize;
            let ry = ry * REGION_CHUNK_WIDTH as usize;
            let rz = rz * REGION_CHUNK_WIDTH as usize;
            let offset = (rz * ROOT_CHUNK_WIDTH as usize + ry) * ROOT_CHUNK_WIDTH as usize + rx;
            for x in 0..REGION_CHUNK_WIDTH as usize {
                for y in 0..REGION_CHUNK_WIDTH as usize {
                    for z in 0..REGION_CHUNK_WIDTH as usize {
                        let chunk_index = (z * ROOT_CHUNK_WIDTH as usize + y) * ROOT_CHUNK_WIDTH as usize + x + offset;
                        if chunk_map[chunk_index] != REQUEST_LOAD_CHUNK_INDEX { continue; }
                        if let WorldChunk::Occupied(chunk) = &mut self.world.chunks[chunk_index] {
                            chunk_map[chunk_index] = self.chunk_upload_index;
                            self.upload_destinations.push(self.chunk_upload_index);
                            self.chunk_upload_index += 1;
                            let mut upload_buffer = self.upload_buffers[current_buffer].write().unwrap();
                            for block_index in 0..CHUNK_BLOCK_VOLUME as usize {
                                upload_buffer[block_index] = chunk.block_data[block_index];
                            }
                            current_buffer += 1;
                            if current_buffer == NUM_UPLOAD_BUFFERS {
                                return;
                            }
                        } else {
                            chunk_map[chunk_index] = EMPTY_CHUNK_INDEX;
                        }
                    }
                }
            }
            region_map[region_index] = 1;
        }
    }
}
