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

use crate::shaders::{self, ComputeShader, ComputeShaderLayout, FragmentShader, VertexShader};

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

#[derive(Clone, Debug, Default)]
struct Vertex {
    position: [f32; 2],
}

vulkano::impl_vertex!(Vertex, position);

pub struct Renderer {
    device: Arc<Device>,
    queue: Arc<Queue>,

    input_data: InputData,
    input_data_image: InputDataImage,

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
    fn make_input_data(&self) -> (InputData, InputDataImage) {
        let input_data = CpuAccessibleBuffer::from_iter(
            self.device.clone(),
            BufferUsage::all(),
            (0..64 * 64 * 64).map(|_| 0u32),
        )
        .unwrap();

        let mut target = input_data.write().unwrap();
        let mut index = 0;
        for z in 0..64 {
            for y in 0..64 {
                for x in 0..64 {
                    if (63 - z) < (x + y) / 4 {
                        target[index] = 10;
                    }
                    index += 1;
                }
            }
        }
        drop(target);

        let input_data_image = StorageImage::new(
            self.device.clone(),
            Dimensions::Dim3d {
                width: 64,
                height: 64,
                depth: 64,
            },
            Format::R32Uint,
            Some(self.queue.family()),
        )
        .unwrap();

        (input_data, input_data_image)
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
        let (input_data, input_data_image) = self.make_input_data();
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
                .add_image(input_data_image.clone())
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

            input_data,
            input_data_image,

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
        state: &DynamicState,
        output: Arc<dyn FramebufferAbstract + Send + Sync>,
    ) -> AutoCommandBuffer {
        let clear_values = vec![[0.0, 0.0, 1.0, 1.0].into()];
        AutoCommandBufferBuilder::primary_one_time_submit(self.device.clone(), self.queue.family())
            .unwrap()
            .clear_color_image(self.output_image.clone(), [0.0, 0.0, 1.0, 1.0].into())
            .unwrap()
            .copy_buffer_to_image(self.input_data.clone(), self.input_data_image.clone())
            .unwrap()
            .dispatch(
                [RENDER_OUTPUT_WIDTH / 8, RENDER_OUTPUT_HEIGHT / 8, 1],
                self.compute_pipeline.clone(),
                self.compute_descriptors.clone(),
                (),
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
