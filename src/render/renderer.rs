use cgmath::{Matrix3, Rad, SquareMatrix, Vector3, Zero};

use image::{GenericImageView, ImageFormat};

use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::descriptor::descriptor_set::{DescriptorSet, PersistentDescriptorSet};
use vulkano::descriptor::pipeline_layout::PipelineLayout;
use vulkano::device::{Device, Queue};
use vulkano::format::Format;
use vulkano::image::{Dimensions, ImmutableImage, StorageImage};
use vulkano::pipeline::ComputePipeline;
use vulkano::sampler::{Filter, MipmapMode, Sampler, SamplerAddressMode};

use std::sync::Arc;

use super::constants::*;
use crate::game::Game;
use crate::util;
use shaders::{
    self, BasicRaytraceShaderLayout, BilateralDenoisePushData, BilateralDenoiseShaderLayout,
    FinalizeShaderLayout, RaytracePushData,
};

type WorldData = CpuAccessibleBuffer<[u16]>;
type WorldImage = StorageImage<Format>;
type BasicRaytracePipeline = ComputePipeline<PipelineLayout<BasicRaytraceShaderLayout>>;
type BilateralDenoisePipeline = ComputePipeline<PipelineLayout<BilateralDenoiseShaderLayout>>;
type FinalizePipeline = ComputePipeline<PipelineLayout<FinalizeShaderLayout>>;

type GenericImage = StorageImage<Format>;
type GenericDescriptorSet = dyn DescriptorSet + Sync + Send;

const NUM_UPLOAD_BUFFERS: usize = 32;

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

impl Camera {
    pub fn new() -> Camera {
        Camera {
            origin: Vector3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            heading: Rad(1.0),
            pitch: Rad(0.0),
        }
    }
}

pub struct Renderer {
    target_image: Arc<GenericImage>,
    target_width: u32,
    target_height: u32,

    current_seed: u32,
    chunk_upload_index: u16,
    // A matrix that transforms world space to the previous frame's screen space
    previous_screen_space_transform: Matrix3<f32>,
    // Required for translating world space coordinates before applying screen space transform.
    previous_camera_origin: Vector3<f32>,

    upload_buffers: Vec<Arc<WorldData>>,
    upload_destinations: Vec<u16>,
    block_data_atlas: Arc<WorldImage>,

    chunk_map_data: Arc<WorldData>,
    chunk_map: Arc<WorldImage>,
    region_map_data: Arc<WorldData>,
    region_map: Arc<WorldImage>,

    basic_raytrace_pipeline: Arc<BasicRaytracePipeline>,
    basic_raytrace_descriptors: Arc<GenericDescriptorSet>,
    bilateral_denoise_pipeline: Arc<BilateralDenoisePipeline>,
    bilateral_denoise_ping_descriptors: Arc<GenericDescriptorSet>,
    bilateral_denoise_pong_descriptors: Arc<GenericDescriptorSet>,
    finalize_pipeline: Arc<FinalizePipeline>,
    finalize_descriptors: Arc<GenericDescriptorSet>,
    //    lighting_pong_buffer: Arc<GenericImage>,
    //    albedo_buffer: Arc<GenericImage>,
    //    emission_buffer: Arc<GenericImage>,
    //    normal_buffer: Arc<GenericImage>,
    lighting_buffer: Arc<GenericImage>,
    depth_buffer: Arc<GenericImage>,
    normal_buffer: Arc<GenericImage>,
    old_lighting_buffer: Arc<GenericImage>,
    old_depth_buffer: Arc<GenericImage>,
    old_normal_buffer: Arc<GenericImage>,
}

struct RenderBuilder<'a> {
    device: Arc<Device>,
    queue: Arc<Queue>,
    target_image: Arc<GenericImage>,
    game: &'a Game,
}

impl<'a> RenderBuilder<'a> {
    fn make_world(
        &self,
    ) -> (
        Vec<Arc<WorldData>>,
        Arc<WorldImage>,
        Arc<WorldData>,
        Arc<WorldImage>,
        Arc<WorldData>,
        Arc<WorldImage>,
    ) {
        let world = self.game.borrow_world();

        let upload_buffers = (0..NUM_UPLOAD_BUFFERS)
            .map(|_| {
                CpuAccessibleBuffer::from_iter(
                    self.device.clone(),
                    BufferUsage::all(),
                    (0..CHUNK_BLOCK_VOLUME).map(|_| 0),
                )
                .unwrap()
            })
            .collect();

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
            (0..ROOT_CHUNK_VOLUME).map(|_| UNLOADED_CHUNK_INDEX),
        )
        .unwrap();

        let chunk_map = StorageImage::new(
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

        let region_map_data = CpuAccessibleBuffer::from_iter(
            self.device.clone(),
            BufferUsage::all(),
            (0..ROOT_REGION_VOLUME as usize).map(|_| 1),
        )
        .unwrap();

        let region_map = StorageImage::new(
            self.device.clone(),
            Dimensions::Dim3d {
                width: ROOT_REGION_WIDTH,
                height: ROOT_REGION_WIDTH,
                depth: ROOT_REGION_WIDTH,
            },
            Format::R16Uint,
            Some(self.queue.family()),
        )
        .unwrap();
        println!("Buffers created.");

        (
            upload_buffers,
            block_data_atlas,
            chunk_map_data,
            chunk_map,
            region_map_data,
            region_map,
        )
    }

    fn make_render_buffer(&self, size: (u32, u32), format: Format) -> Arc<GenericImage> {
        StorageImage::new(
            self.device.clone(),
            Dimensions::Dim2d {
                width: size.0,
                height: size.1,
            },
            format,
            Some(self.queue.family()),
        )
        .unwrap()
    }

    fn load_blue_noise(&self) -> (Arc<ImmutableImage<Format>>, Arc<Sampler>) {
        let file = include_bytes!("blue_noise_512.png");
        let data = image::load_from_memory_with_format(file, ImageFormat::PNG).unwrap();
        let mut pixels = Vec::with_capacity(512 * 512 * 4);
        for pixel in data.pixels() {
            let color = (pixel.2).0;
            pixels.push(color[0]);
            pixels.push(color[1]);
            pixels.push(color[2]);
            pixels.push(color[3]);
        }

        // TODO: Properly wait for the completion of the texture upload.
        let image = ImmutableImage::from_iter(
            pixels.into_iter(),
            Dimensions::Dim2d {
                width: 512,
                height: 512,
            },
            Format::R8G8B8A8Srgb,
            self.queue.clone(),
        )
        .unwrap()
        .0;
        let sampler = Sampler::new(
            self.device.clone(),
            Filter::Nearest,
            Filter::Nearest,
            MipmapMode::Nearest,
            SamplerAddressMode::Repeat,
            SamplerAddressMode::Repeat,
            SamplerAddressMode::ClampToEdge,
            0.0,
            1.0,
            0.0,
            0.0,
        )
        .unwrap();

        (image, sampler)
    }

    fn build(self) -> Renderer {
        let (target_width, target_height) = match self.target_image.dimensions() {
            Dimensions::Dim2d { width, height } => (width, height),
            _ => panic!("A non-2d image was passed as the target of a Renderer."),
        };

        let (
            upload_buffers,
            block_data_atlas,
            chunk_map_data,
            chunk_map,
            region_map_data,
            region_map,
        ) = self.make_world();

        let rbuf_size = (target_width, target_height);
        let lighting_buffer = self.make_render_buffer(rbuf_size, Format::R16G16B16A16Unorm);
        let old_lighting_buffer = self.make_render_buffer(rbuf_size, Format::R16G16B16A16Unorm);
        let lighting_pong_buffer = self.make_render_buffer(rbuf_size, Format::R16G16B16A16Unorm);
        let albedo_buffer = self.make_render_buffer(rbuf_size, Format::R8G8B8A8Unorm);
        let emission_buffer = self.make_render_buffer(rbuf_size, Format::R8G8B8A8Unorm);
        let depth_buffer = self.make_render_buffer(rbuf_size, Format::R16Uint);
        let old_depth_buffer = self.make_render_buffer(rbuf_size, Format::R16Uint);
        let normal_buffer = self.make_render_buffer(rbuf_size, Format::R8Uint);
        let old_normal_buffer = self.make_render_buffer(rbuf_size, Format::R8Uint);
        let fog_color_buffer = self.make_render_buffer(rbuf_size, Format::R8G8B8A8Unorm);

        let (blue_noise, blue_noise_sampler) = self.load_blue_noise();

        let basic_raytrace_shader = shaders::load_basic_raytrace_shader(self.device.clone());
        let bilateral_denoise_shader = shaders::load_bilateral_denoise_shader(self.device.clone());
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
                .add_image(block_data_atlas.clone())
                .unwrap()
                .add_image(chunk_map.clone())
                .unwrap()
                .add_image(region_map.clone())
                .unwrap()
                .add_image(lighting_buffer.clone())
                .unwrap()
                .add_image(depth_buffer.clone())
                .unwrap()
                .add_image(normal_buffer.clone())
                .unwrap()
                .add_image(albedo_buffer.clone())
                .unwrap()
                .add_image(emission_buffer.clone())
                .unwrap()
                .add_sampled_image(blue_noise.clone(), blue_noise_sampler.clone())
                .unwrap()
                .add_image(old_lighting_buffer.clone())
                .unwrap()
                .add_image(old_depth_buffer.clone())
                .unwrap()
                .add_image(old_normal_buffer.clone())
                .unwrap()
                .add_image(fog_color_buffer.clone())
                .unwrap()
                .build()
                .unwrap(),
        );

        let bilateral_denoise_pipeline = Arc::new(
            ComputePipeline::new(
                self.device.clone(),
                &bilateral_denoise_shader.main_entry_point(),
                &(),
            )
            .unwrap(),
        );
        let bilateral_denoise_ping_descriptors: Arc<dyn DescriptorSet + Sync + Send> = Arc::new(
            PersistentDescriptorSet::start(bilateral_denoise_pipeline.clone(), 0)
                .add_image(lighting_buffer.clone())
                .unwrap()
                .add_image(depth_buffer.clone())
                .unwrap()
                .add_image(normal_buffer.clone())
                .unwrap()
                .add_image(lighting_pong_buffer.clone())
                .unwrap()
                .build()
                .unwrap(),
        );
        let bilateral_denoise_pong_descriptors: Arc<dyn DescriptorSet + Sync + Send> = Arc::new(
            PersistentDescriptorSet::start(bilateral_denoise_pipeline.clone(), 0)
                .add_image(lighting_pong_buffer.clone())
                .unwrap()
                .add_image(depth_buffer.clone())
                .unwrap()
                .add_image(normal_buffer.clone())
                .unwrap()
                .add_image(lighting_buffer.clone())
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
                .add_image(lighting_buffer.clone())
                .unwrap()
                .add_image(albedo_buffer.clone())
                .unwrap()
                .add_image(emission_buffer.clone())
                .unwrap()
                .add_image(depth_buffer.clone())
                .unwrap()
                .add_image(fog_color_buffer.clone())
                .unwrap()
                .add_sampled_image(blue_noise.clone(), blue_noise_sampler.clone())
                .unwrap()
                .add_image(self.target_image.clone())
                .unwrap()
                .build()
                .unwrap(),
        );

        println!("Pipeline created.");

        Renderer {
            target_image: self.target_image,
            target_width,
            target_height,

            current_seed: 0,
            chunk_upload_index: 0,
            previous_screen_space_transform: Matrix3::zero(),
            previous_camera_origin: [0.0, 0.0, 0.0].into(),

            upload_buffers,
            upload_destinations: Vec::new(),
            block_data_atlas,
            chunk_map_data,
            chunk_map,
            region_map_data,
            region_map,

            basic_raytrace_pipeline,
            basic_raytrace_descriptors,
            bilateral_denoise_pipeline,
            bilateral_denoise_ping_descriptors,
            bilateral_denoise_pong_descriptors,
            finalize_pipeline,
            finalize_descriptors,

            depth_buffer,
            lighting_buffer,
            normal_buffer,
            old_depth_buffer,
            old_lighting_buffer,
            old_normal_buffer,
        }
    }
}

impl Renderer {
    pub fn new(
        device: Arc<Device>,
        queue: Arc<Queue>,
        target_image: Arc<GenericImage>,
        game: &Game,
    ) -> Renderer {
        RenderBuilder {
            device,
            queue,
            target_image,
            game,
        }
        .build()
    }

    pub fn add_render_commands(
        &mut self,
        mut add_to: AutoCommandBufferBuilder,
        game: &Game,
    ) -> AutoCommandBufferBuilder {
        let camera = game.borrow_camera();
        let camera_pos = camera.origin;
        let util::TripleEulerVector { forward, up, right } =
            util::compute_triple_euler_vector(camera.heading, camera.pitch);
        let up = up * 0.3;
        let right = right * 0.3;
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

        self.current_seed = (self.current_seed + 1) % (512 * 512);
        let completed_buffer = add_to
            .copy_buffer_to_image(self.chunk_map_data.clone(), self.chunk_map.clone())
            .unwrap()
            .copy_buffer_to_image(self.region_map_data.clone(), self.region_map.clone())
            .unwrap()
            .dispatch(
                [self.target_width / 8, self.target_height / 8, 1],
                self.basic_raytrace_pipeline.clone(),
                self.basic_raytrace_descriptors.clone(),
                RaytracePushData {
                    _dummy0: [0; 4],
                    _dummy1: [0; 4],
                    _dummy2: [0; 4],
                    _dummy3: [0; 4],
                    _dummy4: [0; 4],
                    _dummy5: [0; 4],
                    _dummy6: [0; 4],
                    origin: camera_pos.clone().into(),
                    forward: forward.clone().into(),
                    right: right.clone().into(),
                    up: up.clone().into(),
                    old_origin: self.previous_camera_origin.clone().into(),
                    old_transform_c0: self.previous_screen_space_transform.x.clone().into(),
                    old_transform_c1: self.previous_screen_space_transform.y.clone().into(),
                    old_transform_c2: self.previous_screen_space_transform.z.clone().into(),
                    sun_angle: game.get_sun_angle(),
                    seed: self.current_seed,
                },
            )
            .unwrap()
            .copy_image(
                self.depth_buffer.clone(),
                [0, 0, 0],
                0,
                0,
                self.old_depth_buffer.clone(),
                [0, 0, 0],
                0,
                0,
                [512, 512, 1],
                1,
            )
            .unwrap()
            .copy_image(
                self.lighting_buffer.clone(),
                [0, 0, 0],
                0,
                0,
                self.old_lighting_buffer.clone(),
                [0, 0, 0],
                0,
                0,
                [512, 512, 1],
                1,
            )
            .unwrap()
            .copy_image(
                self.normal_buffer.clone(),
                [0, 0, 0],
                0,
                0,
                self.old_normal_buffer.clone(),
                [0, 0, 0],
                0,
                0,
                [512, 512, 1],
                1,
            )
            .unwrap()
            .dispatch(
                [self.target_width / 8, self.target_height / 8, 1],
                self.bilateral_denoise_pipeline.clone(),
                self.bilateral_denoise_ping_descriptors.clone(),
                BilateralDenoisePushData { size: 1 },
            )
            .unwrap()
            .dispatch(
                [self.target_width / 8, self.target_height / 8, 1],
                self.bilateral_denoise_pipeline.clone(),
                self.bilateral_denoise_pong_descriptors.clone(),
                BilateralDenoisePushData { size: 2 },
            )
            .unwrap()
            .dispatch(
                [self.target_width / 8, self.target_height / 8, 1],
                self.bilateral_denoise_pipeline.clone(),
                self.bilateral_denoise_ping_descriptors.clone(),
                BilateralDenoisePushData { size: 3 },
            )
            .unwrap()
            .dispatch(
                [self.target_width / 8, self.target_height / 8, 1],
                self.bilateral_denoise_pipeline.clone(),
                self.bilateral_denoise_pong_descriptors.clone(),
                BilateralDenoisePushData { size: 2 },
            )
            .unwrap()
            .dispatch(
                [self.target_width / 8, self.target_height / 8, 1],
                self.finalize_pipeline.clone(),
                self.finalize_descriptors.clone(),
                BilateralDenoisePushData { size: 1 },
            )
            .unwrap()
            .copy_image_to_buffer(self.chunk_map.clone(), self.chunk_map_data.clone())
            .unwrap()
            .copy_image_to_buffer(self.region_map.clone(), self.region_map_data.clone())
            .unwrap();

        self.previous_screen_space_transform = {
            // The arguments are ordered weirdly, column increases from top to bottom.
            // Multiplying {screenx, screeny, depth} by this gets pixel position in world space.
            let screen_to_world_space =
                Matrix3::from_cols(right.clone(), up.clone(), forward.clone());
            // Inverting it gives us world space to screen space.
            screen_to_world_space
                .invert()
                .expect("Screen space vectors should cover entire coordinate space.")
        };
        self.previous_camera_origin = camera.origin.clone();
        completed_buffer
    }

    pub fn read_feedback(&mut self, game: &mut Game) {
        let mut chunk_map = self.chunk_map_data.write().unwrap();
        let mut region_map = self.region_map_data.write().unwrap();
        let mut current_buffer = 0;
        for region_index in 0..ROOT_REGION_VOLUME {
            let region_content = region_map[region_index as usize];
            if region_content != REQUEST_LOAD_CHUNK_INDEX {
                continue;
            }
            let region_coord = util::index_to_coord_3d(region_index, ROOT_REGION_WIDTH);
            let possible_region = game.borrow_world_mut().borrow_region(region_coord);
            let region_data = if let Some(data) = possible_region {
                data
            } else {
                region_map[region_index as usize] = EMPTY_CHUNK_INDEX;
                continue;
            };
            region_map[region_index as usize] = 1;
            let chunk_coord = util::scale_coord_3d(&region_coord, REGION_CHUNK_WIDTH);
            // The index of the first chunk in the region.
            let region_offset = util::coord_to_index_3d(&chunk_coord, ROOT_CHUNK_WIDTH);
            for local_coord in util::coord_iter_3d(REGION_CHUNK_WIDTH) {
                let local_index = util::coord_to_index_3d(&local_coord, REGION_CHUNK_WIDTH);
                let global_index = util::coord_to_index_3d(&local_coord, ROOT_CHUNK_WIDTH) + region_offset;
                if chunk_map[global_index as usize] != REQUEST_LOAD_CHUNK_INDEX {
                    continue;
                }
                let chunk_data = if let Some(data) = &region_data.chunks[local_index as usize] {
                    data
                } else {
                    chunk_map[global_index as usize] = EMPTY_CHUNK_INDEX;
                    continue;
                };
                chunk_map[global_index as usize] = self.chunk_upload_index;
                self.upload_destinations.push(self.chunk_upload_index);
                self.chunk_upload_index += 1;
                let mut upload_buffer = self.upload_buffers[current_buffer].write().unwrap();
                for block_index in 0..CHUNK_BLOCK_VOLUME as usize {
                    upload_buffer[block_index] = chunk_data.block_data[block_index];
                }
                current_buffer += 1;
                if current_buffer == NUM_UPLOAD_BUFFERS {
                    println!("Uploaded {} chunks.", current_buffer);
                    return;
                }
            }
        }
        println!("Uploaded {} chunks.", current_buffer);
    }

    pub fn capture(&mut self) {
        unimplemented!();
    }

    pub fn finish_capture(&mut self) {
        unimplemented!();
    }
}
