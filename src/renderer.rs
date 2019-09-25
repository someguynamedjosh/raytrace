use cgmath::{Rad, Vector3};

use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{AutoCommandBuffer, AutoCommandBufferBuilder, DynamicState};
use vulkano::descriptor::descriptor_set::{DescriptorSet, PersistentDescriptorSet};
use vulkano::descriptor::pipeline_layout::{PipelineLayout, PipelineLayoutAbstract};
use vulkano::device::{Device, Queue};
use vulkano::format::Format;
use vulkano::framebuffer::{FramebufferAbstract, RenderPassAbstract, Subpass};
use vulkano::image::{Dimensions, StorageImage};
use vulkano::pipeline::vertex::SingleBufferDefinition;
use vulkano::pipeline::{ComputePipeline, GraphicsPipeline};
use vulkano::sampler::{Filter, MipmapMode, Sampler, SamplerAddressMode};

use std::sync::Arc;

use crate::shaders::{
    self, CameraVectorPushConstants, ComputeShader, ComputeShaderLayout, FragmentShader,
    VertexShader,
};
use crate::util;

type InputData = Arc<CpuAccessibleBuffer<[u32]>>;
type InputDataImage = Arc<StorageImage<Format>>;
type OutputImage = Arc<StorageImage<Format>>;

type VertexBuffer = Arc<CpuAccessibleBuffer<[Vertex]>>;
type IndexBuffer = Arc<CpuAccessibleBuffer<[u32]>>;

type CustomComputePipeline = Arc<ComputePipeline<PipelineLayout<ComputeShaderLayout>>>;
type CustomGraphicsPipeline = Arc<
    GraphicsPipeline<
        SingleBufferDefinition<Vertex>,
        Box<dyn PipelineLayoutAbstract + Send + Sync>,
        Arc<dyn RenderPassAbstract + Sync + Send>,
    >,
>;

const RENDER_OUTPUT_WIDTH: u32 = 512;
const RENDER_OUTPUT_HEIGHT: u32 = 512;
const WORLD_SIZE: usize = 128;
const L2_STEP: usize = 8;
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
    device: Arc<Device>,
    queue: Arc<Queue>,

    world_l1_data: InputData,
    world_l1_image: InputDataImage,
    world_l2_data: InputData,
    world_l2_image: InputDataImage,

    output_image: OutputImage,

    vertex_buffer: VertexBuffer,
    index_buffer: IndexBuffer,

    render_pass: Arc<dyn RenderPassAbstract + Sync + Send>,

    compute_pipeline: CustomComputePipeline,
    compute_descriptors: Arc<dyn DescriptorSet + Sync + Send>,
    graphics_pipeline: CustomGraphicsPipeline,
    graphics_descriptors: Arc<dyn DescriptorSet + Sync + Send>,
}

struct RenderBuilder {
    device: Arc<Device>,
    queue: Arc<Queue>,
    format: Format,
}

impl RenderBuilder {
    fn make_world(&self) -> (InputData, InputDataImage, InputData, InputDataImage) {
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
                    let l2_index = ((z / 8 * L2_SIZE) + (y / 8)) * L2_SIZE + x / 8;
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

    fn make_output_image(&self) -> (OutputImage, Arc<Sampler>) {
        // The image that the compute shader will write from and the graphics pipeline will read from.
        let output_image = StorageImage::new(
            self.device.clone(),
            Dimensions::Dim2d {
                width: RENDER_OUTPUT_WIDTH,
                height: RENDER_OUTPUT_HEIGHT,
            },
            Format::R8G8B8A8Unorm,
            Some(self.queue.family()),
        )
        .unwrap();

        let sampler = Sampler::new(
            self.device.clone(),
            Filter::Nearest,
            Filter::Nearest,
            MipmapMode::Nearest,
            SamplerAddressMode::ClampToEdge,
            SamplerAddressMode::ClampToEdge,
            SamplerAddressMode::ClampToEdge,
            0.0,
            1.0,
            0.0,
            0.0,
        )
        .unwrap();

        (output_image, sampler)
    }

    fn make_quad(&self) -> (VertexBuffer, IndexBuffer) {
        // Create a vertex buffer containing a full screen quad.
        let vertex_buffer = CpuAccessibleBuffer::from_iter(
            self.device.clone(),
            BufferUsage::all(),
            [
                Vertex {
                    position: [1.0, 1.0],
                },
                Vertex {
                    position: [-1.0, 1.0],
                },
                Vertex {
                    position: [-1.0, -1.0],
                },
                Vertex {
                    position: [1.0, -1.0],
                },
            ]
            .iter()
            .cloned(),
        )
        .unwrap();

        // Indexes buffer used when drawing the quad.
        let index_buffer = CpuAccessibleBuffer::from_iter(
            self.device.clone(),
            BufferUsage::index_buffer(),
            [0u32, 1u32, 2u32, 2u32, 3u32, 0u32].iter().cloned(),
        )
        .unwrap();

        (vertex_buffer, index_buffer)
    }

    fn load_shaders(&self) -> (ComputeShader, VertexShader, FragmentShader) {
        (
            shaders::load_compute(self.device.clone()),
            shaders::load_vertex(self.device.clone()),
            shaders::load_fragment(self.device.clone()),
        )
    }

    fn build(self) -> Renderer {
        let (world_l1_data, world_l1_image, world_l2_data, world_l2_image) = self.make_world();
        let (output_image, output_sampler) = self.make_output_image();
        let (vertex_buffer, index_buffer) = self.make_quad();
        let (compute_shader, vertex_shader, fragment_shader) = self.load_shaders();

        let render_pass: Arc<dyn RenderPassAbstract + Sync + Send> = Arc::new(
            vulkano::single_pass_renderpass!(
                self.device.clone(),
                attachments: {
                    color: {
                        load: Clear,
                        store: Store,
                        format: self.format,
                        samples: 1,
                    }
                },
                pass: {
                    color: [color],
                    depth_stencil: {}
                }
            )
            .unwrap(),
        );

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
                .add_image(output_image.clone())
                .unwrap()
                .build()
                .unwrap(),
        );

        let graphics_pipeline = Arc::new(
            GraphicsPipeline::start()
                .vertex_input_single_buffer::<Vertex>()
                .vertex_shader(vertex_shader.main_entry_point(), ())
                .triangle_list()
                .viewports_dynamic_scissors_irrelevant(1)
                .fragment_shader(fragment_shader.main_entry_point(), ())
                .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
                .build(self.device.clone())
                .unwrap(),
        );

        let graphics_descriptors: Arc<dyn DescriptorSet + Sync + Send> = Arc::new(
            PersistentDescriptorSet::start(graphics_pipeline.clone(), 0)
                .add_sampled_image(output_image.clone(), output_sampler.clone())
                .unwrap()
                .build()
                .unwrap(),
        );

        Renderer {
            device: self.device,
            queue: self.queue,

            world_l1_data,
            world_l1_image,
            world_l2_data,
            world_l2_image,

            output_image,

            vertex_buffer,
            index_buffer,

            render_pass,

            compute_pipeline,
            compute_descriptors,
            graphics_pipeline,
            graphics_descriptors,
        }
    }
}

impl Renderer {
    pub fn new(device: Arc<Device>, queue: Arc<Queue>, format: Format) -> Renderer {
        RenderBuilder {
            device,
            queue,
            format,
        }
        .build()
    }

    pub fn get_render_pass(&self) -> Arc<dyn RenderPassAbstract + Sync + Send> {
        self.render_pass.clone()
    }

    pub fn create_command_buffer(
        &mut self,
        camera: &Camera,
        state: &DynamicState,
        output: Arc<dyn FramebufferAbstract + Send + Sync>,
    ) -> AutoCommandBuffer {
        let clear_values = vec![[0.0, 0.0, 1.0, 1.0].into()];
        let camera_pos = camera.origin;
        let util::TripleEulerVector { forward, up, right } =
            util::compute_triple_euler_vector(camera.heading, camera.pitch);
        AutoCommandBufferBuilder::primary_one_time_submit(self.device.clone(), self.queue.family())
            .unwrap()
            .clear_color_image(self.output_image.clone(), [0.0, 0.0, 1.0, 1.0].into())
            .unwrap()
            .copy_buffer_to_image(self.world_l1_data.clone(), self.world_l1_image.clone())
            .unwrap()
            .copy_buffer_to_image(self.world_l2_data.clone(), self.world_l2_image.clone())
            .unwrap()
            .dispatch(
                [RENDER_OUTPUT_WIDTH / 8, RENDER_OUTPUT_HEIGHT / 8, 1],
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
            .begin_render_pass(output, false, clear_values)
            .unwrap()
            .draw_indexed(
                self.graphics_pipeline.clone(),
                state,
                self.vertex_buffer.clone(),
                self.index_buffer.clone(),
                self.graphics_descriptors.clone(),
                (),
            )
            .unwrap()
            .end_render_pass()
            .unwrap()
            .build()
            .unwrap()
    }
}
