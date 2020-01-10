use ash::version::DeviceV1_0;
use ash::vk;

use cgmath::Vector3;

use std::ffi::CString;

use super::commands as cmd;
use super::constants::*;
use super::core::Core;
#[macro_use]
use crate::create_descriptor_collection_struct;
use super::descriptors::DescriptorPrototype;
use super::structures::{Buffer, Image, ObjectBuffer, SampledImage};

struct Stage {
    vk_pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
}

impl Stage {
    unsafe fn destroy(&mut self, core: &Core) {
        core.device
            .destroy_pipeline_layout(self.pipeline_layout, None);
        core.device.destroy_pipeline(self.vk_pipeline, None);
    }
}

pub struct Pipeline {
    command_buffers: Vec<vk::CommandBuffer>,
    frame_available_semaphore: vk::Semaphore,
    frame_complete_semaphore: vk::Semaphore,
    frame_complete_fence: vk::Fence,
    render_data: RenderData,
    descriptor_collection: DescriptorCollection,

    denoise_stage: Stage,
    finalize_stage: Stage,
    raytrace_stage: Stage,
}

impl Pipeline {
    pub fn new(core: &Core) -> Pipeline {
        let (frame_available_semaphore, frame_complete_semaphore) = create_semaphores(core);
        let (frame_complete_fence,) = create_fences(core);
        let command_buffers = create_command_buffers(core);

        let render_data = RenderData::create(core);
        let descriptor_collection = DescriptorCollection::create(core, &render_data);

        let change_layouts = cmd::create_buffer(core, "change_layouts");
        cmd::begin(core, change_layouts);
        let images = [
            &render_data.albedo_buffer,
            &render_data.block_data_atlas,
            &render_data.chunk_map,
            &render_data.depth_buffer,
            &render_data.emission_buffer,
            &render_data.fog_color_buffer,
            &render_data.lighting_buffer,
            &render_data.lighting_pong_buffer,
            &render_data.normal_buffer,
            &render_data.old_depth_buffer,
            &render_data.old_lighting_buffer,
            &render_data.old_normal_buffer,
            &render_data.region_map,
        ];
        for img in images.iter() {
            cmd::transition_layout(
                core,
                change_layouts,
                img.image,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::GENERAL,
            );
        }
        cmd::end(core, change_layouts);
        cmd::execute_and_destroy(core, change_layouts);

        let denoise_stage = create_denoise_stage(core, &descriptor_collection);
        let finalize_stage = create_finalize_stage(core, &descriptor_collection);
        let raytrace_stage = create_raytrace_stage(core, &descriptor_collection);

        let mut pipeline = Pipeline {
            command_buffers,
            frame_available_semaphore,
            frame_complete_semaphore,
            frame_complete_fence,
            render_data,
            descriptor_collection,

            denoise_stage,
            finalize_stage,
            raytrace_stage,
        };
        pipeline.record_command_buffers(core);
        pipeline
    }

    fn record_command_buffers(&mut self, core: &Core) {
        for (index, buffer) in self.command_buffers.iter().enumerate() {
            let swapchain_image = core.swapchain_info.swapchain_images[index];
            let buffer = *buffer;
            cmd::begin(core, buffer);
            cmd::transition_layout(
                core,
                buffer,
                swapchain_image,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::GENERAL,
            );
            let layout = self.raytrace_stage.pipeline_layout;
            let set = self.descriptor_collection.raytrace.variants[0];
            cmd::bind_descriptor_set(core, buffer, set, layout, 0);
            cmd::bind_pipeline(core, buffer, self.raytrace_stage.vk_pipeline);
            unsafe {
                core.device.cmd_dispatch(buffer, 30, 30, 1);
            }
            cmd::transition_layout(
                core,
                buffer,
                swapchain_image,
                vk::ImageLayout::GENERAL,
                vk::ImageLayout::PRESENT_SRC_KHR,
            );
            cmd::end(core, buffer);
        }
    }

    pub fn draw_frame(&mut self, core: &Core) {
        let (image_index, _is_suboptimal) = unsafe {
            core.swapchain_info
                .swapchain_loader
                .acquire_next_image(
                    core.swapchain_info.swapchain,
                    std::u64::MAX,
                    self.frame_available_semaphore,
                    vk::Fence::null(),
                )
                .expect("Failed to acquire next swapchain image.")
        };

        let wait_semaphores = [self.frame_available_semaphore];
        let signal_semaphores = [self.frame_complete_semaphore];
        let wait_stage_mask = [vk::PipelineStageFlags::ALL_COMMANDS];
        let submit_info = vk::SubmitInfo {
            wait_semaphore_count: 1,
            p_wait_semaphores: wait_semaphores.as_ptr(),
            p_wait_dst_stage_mask: wait_stage_mask.as_ptr(),
            command_buffer_count: 1,
            p_command_buffers: &self.command_buffers[image_index as usize],
            signal_semaphore_count: 1,
            p_signal_semaphores: signal_semaphores.as_ptr(),
            ..Default::default()
        };

        unsafe {
            let wait_fence = self.frame_complete_fence;
            core.device
                .wait_for_fences(&[wait_fence], true, std::u64::MAX)
                .expect("Failed to wait for previous frame to finish rendering.");
            core.device
                .reset_fences(&[wait_fence])
                .expect("Failed to reset fence.");
            core.device
                .queue_submit(core.compute_queue, &[submit_info], wait_fence)
                .expect("Failed to submit command queue.");
        }

        let wait_semaphores = [self.frame_complete_semaphore];
        let swapchains = [core.swapchain_info.swapchain];
        let present_info = vk::PresentInfoKHR {
            wait_semaphore_count: 1,
            p_wait_semaphores: wait_semaphores.as_ptr(),
            swapchain_count: 1,
            p_swapchains: swapchains.as_ptr(),
            p_image_indices: &image_index,
            ..Default::default()
        };

        unsafe {
            core.swapchain_info
                .swapchain_loader
                .queue_present(core.present_queue, &present_info)
                .expect("Failed to present swapchain image.");
        }
    }

    pub unsafe fn destroy(&mut self, core: &Core) {
        core.device
            .device_wait_idle()
            .expect("Failed to wait for device to finish rendering.");

        self.denoise_stage.destroy(core);
        self.finalize_stage.destroy(core);
        self.raytrace_stage.destroy(core);

        self.descriptor_collection.destroy(core);
        self.render_data.destroy(core);

        core.device.destroy_fence(self.frame_complete_fence, None);
        core.device
            .destroy_semaphore(self.frame_available_semaphore, None);
        core.device
            .destroy_semaphore(self.frame_complete_semaphore, None);
    }
}

fn create_semaphores(core: &Core) -> (vk::Semaphore, vk::Semaphore) {
    let create_info = Default::default();

    (
        unsafe {
            core.device
                .create_semaphore(&create_info, None)
                .expect("Failed to create semaphore.")
        },
        unsafe {
            core.device
                .create_semaphore(&create_info, None)
                .expect("Failed to create semaphore.")
        },
    )
}

fn create_fences(core: &Core) -> (vk::Fence,) {
    let create_info = vk::FenceCreateInfo {
        // Start the fences signalled so we don't wait on the first couple of frames.
        flags: vk::FenceCreateFlags::SIGNALED,
        ..Default::default()
    };

    let fence = unsafe {
        core.device
            .create_fence(&create_info, None)
            .expect("Failed to create fence.")
    };
    core.set_debug_name(fence, "wait_for_frame_end");
    (fence,)
}

fn create_command_buffers(core: &Core) -> Vec<vk::CommandBuffer> {
    // TODO: debug names.
    let allocate_info = vk::CommandBufferAllocateInfo {
        command_buffer_count: core.swapchain_info.swapchain_images.len() as u32,
        command_pool: core.command_pool,
        level: vk::CommandBufferLevel::PRIMARY,
        ..Default::default()
    };
    unsafe {
        core.device
            .allocate_command_buffers(&allocate_info)
            .expect("Failed to allocate command buffers.")
    }
}

#[repr(C)]
struct RaytraceUniformData {
    sun_angle: f32,
    seed: u32,
    _padding0: u64,
    origin: Vector3<f32>,
    _padding1: u32,
    forward: Vector3<f32>,
    _padding2: u32,
    up: Vector3<f32>,
    _padding3: u32,
    right: Vector3<f32>,
    _padding4: u32,
    old_origin: Vector3<f32>,
    _padding5: u32,
    old_transform_c0: Vector3<f32>,
    _padding6: u32,
    old_transform_c1: Vector3<f32>,
    _padding7: u32,
    old_transform_c2: Vector3<f32>,
    _padding8: u32,
    region_offset: Vector3<i32>,
    _padding9: u32,
}

#[repr(C)]
struct DenoisePushData {
    size: i32,
}

struct RenderData {
    upload_buffers: Vec<Buffer>,
    upload_destinations: Vec<u16>,
    block_data_atlas: Image,

    chunk_map: Image,
    region_map: Image,

    lighting_buffer: Image,
    depth_buffer: Image,
    normal_buffer: Image,
    old_lighting_buffer: Image,
    old_depth_buffer: Image,
    old_normal_buffer: Image,

    lighting_pong_buffer: Image,
    albedo_buffer: Image,
    emission_buffer: Image,
    fog_color_buffer: Image,

    blue_noise: SampledImage,

    raytrace_uniform_data: RaytraceUniformData,
    raytrace_uniform_data_buffer: ObjectBuffer<RaytraceUniformData>,
}

impl RenderData {
    fn make_framebuffer(core: &Core, name: &str, format: vk::Format) -> Image {
        let dimensions = core.swapchain_info.swapchain_extent;
        Image::create(
            core,
            name,
            vk::ImageType::TYPE_2D,
            vk::Extent3D {
                width: dimensions.width,
                height: dimensions.height,
                depth: 1,
            },
            format,
        )
    }

    fn create(core: &Core) -> RenderData {
        let rgba16_unorm = vk::Format::R16G16B16A16_UNORM;
        let rgba8_unorm = vk::Format::R8G8B8A8_UNORM;
        let r16_uint = vk::Format::R16_UINT;
        let r8_uint = vk::Format::R8_UINT;
        RenderData {
            upload_buffers: (0..NUM_UPLOAD_BUFFERS)
                .map(|index| {
                    Buffer::create(
                        core,
                        &format!("upload_buffer_{}", index),
                        CHUNK_BLOCK_VOLUME as u64,
                        vk::BufferUsageFlags::TRANSFER_SRC,
                    )
                })
                .collect(),
            upload_destinations: vec![],
            block_data_atlas: Image::create(
                core,
                "block_data_atlas",
                vk::ImageType::TYPE_3D,
                vk::Extent3D {
                    width: ATLAS_BLOCK_WIDTH,
                    height: ATLAS_BLOCK_WIDTH,
                    depth: ATLAS_BLOCK_WIDTH,
                },
                vk::Format::R16_UINT,
            ),

            chunk_map: Image::create(
                core,
                "chunk_map",
                vk::ImageType::TYPE_3D,
                vk::Extent3D {
                    width: ROOT_CHUNK_WIDTH,
                    height: ROOT_CHUNK_WIDTH,
                    depth: ROOT_CHUNK_WIDTH,
                },
                vk::Format::R16_UINT,
            ),
            region_map: Image::create(
                core,
                "region_map",
                vk::ImageType::TYPE_3D,
                vk::Extent3D {
                    width: ROOT_REGION_WIDTH,
                    height: ROOT_REGION_WIDTH,
                    depth: ROOT_REGION_WIDTH,
                },
                vk::Format::R16_UINT,
            ),

            lighting_buffer: Self::make_framebuffer(core, "lighting_buf", rgba16_unorm),
            depth_buffer: Self::make_framebuffer(core, "depth_buf", r16_uint),
            normal_buffer: Self::make_framebuffer(core, "normal_buf", r8_uint),
            old_lighting_buffer: Self::make_framebuffer(core, "old_lighting_buf", rgba16_unorm),
            old_depth_buffer: Self::make_framebuffer(core, "old_depth_buf", r16_uint),
            old_normal_buffer: Self::make_framebuffer(core, "old_normal_buf", r8_uint),

            lighting_pong_buffer: Self::make_framebuffer(core, "lighting_pong_buf", rgba16_unorm),
            albedo_buffer: Self::make_framebuffer(core, "albedo_buf", rgba8_unorm),
            emission_buffer: Self::make_framebuffer(core, "emission_buf", rgba8_unorm),
            fog_color_buffer: Self::make_framebuffer(core, "fog_color_buf", rgba8_unorm),

            blue_noise: {
                let mut tex = SampledImage::create(
                    core,
                    "blue_noise",
                    vk::ImageType::TYPE_2D,
                    vk::Extent3D {
                        width: BLUE_NOISE_WIDTH,
                        height: BLUE_NOISE_HEIGHT,
                        depth: 1,
                    },
                    vk::Format::R8G8B8A8_UNORM,
                );
                tex.load_from_png(core, include_bytes!("blue_noise_512.png"));
                tex
            },

            raytrace_uniform_data: RaytraceUniformData {
                sun_angle: 0.0,
                seed: 0,
                origin: [0.0, 0.0, 0.0].into(),
                forward: [0.0, 0.0, 0.0].into(),
                up: [0.0, 0.0, 0.0].into(),
                right: [0.0, 0.0, 0.0].into(),
                old_origin: [0.0, 0.0, 0.0].into(),
                old_transform_c0: [0.0, 0.0, 0.0].into(),
                old_transform_c1: [0.0, 0.0, 0.0].into(),
                old_transform_c2: [0.0, 0.0, 0.0].into(),
                region_offset: [0, 0, 0].into(),
                _padding0: 0,
                _padding1: 0,
                _padding2: 0,
                _padding3: 0,
                _padding4: 0,
                _padding5: 0,
                _padding6: 0,
                _padding7: 0,
                _padding8: 0,
                _padding9: 0,
            },
            raytrace_uniform_data_buffer: ObjectBuffer::create(
                core,
                "raytrace_uniform_data",
                vk::BufferUsageFlags::UNIFORM_BUFFER,
            ),
        }
    }

    fn destroy(&mut self, core: &Core) {
        for upload_buffer in &mut self.upload_buffers {
            upload_buffer.destroy(core);
        }
        self.block_data_atlas.destroy(core);

        self.chunk_map.destroy(core);
        self.region_map.destroy(core);

        self.lighting_buffer.destroy(core);
        self.depth_buffer.destroy(core);
        self.normal_buffer.destroy(core);
        self.old_lighting_buffer.destroy(core);
        self.old_depth_buffer.destroy(core);
        self.old_normal_buffer.destroy(core);

        self.lighting_pong_buffer.destroy(core);
        self.albedo_buffer.destroy(core);
        self.emission_buffer.destroy(core);
        self.fog_color_buffer.destroy(core);

        self.blue_noise.destroy(core);
        self.raytrace_uniform_data_buffer.destroy(core);
    }
}

create_descriptor_collection_struct! {
    name: DescriptorCollection,
    aux_data_type: RenderData,
    items: {
        denoise = generate_denoise_ds_prototypes,
        finalize = generate_finalize_ds_prototypes,
        raytrace = generate_raytrace_ds_prototypes,
        swapchain = generate_swapchain_ds_prototypes,
    }
}

#[rustfmt::skip] // It keeps trying to spread my beautiful descriptors over 3 lines :(
fn generate_denoise_ds_prototypes(
    _core: &Core,
    render_data: &RenderData,
) -> Vec<Vec<DescriptorPrototype>> {
    vec![
        vec![
            render_data.lighting_buffer.create_dp(vk::ImageLayout::GENERAL),
            render_data.normal_buffer.create_dp(vk::ImageLayout::GENERAL),
            render_data.depth_buffer.create_dp(vk::ImageLayout::GENERAL),
            //
            render_data.lighting_pong_buffer.create_dp(vk::ImageLayout::GENERAL),
        ],
        vec![
            render_data.lighting_pong_buffer.create_dp(vk::ImageLayout::GENERAL),
            render_data.normal_buffer.create_dp(vk::ImageLayout::GENERAL),
            render_data.depth_buffer.create_dp(vk::ImageLayout::GENERAL),
            //
            render_data.lighting_buffer.create_dp(vk::ImageLayout::GENERAL),
        ],
    ]
}

#[rustfmt::skip]
fn generate_finalize_ds_prototypes(
    _core: &Core,
    render_data: &RenderData,
) -> Vec<Vec<DescriptorPrototype>> {
    vec![vec![
        render_data.lighting_buffer.create_dp(vk::ImageLayout::GENERAL),
        render_data.albedo_buffer.create_dp(vk::ImageLayout::GENERAL),
        render_data.emission_buffer.create_dp(vk::ImageLayout::GENERAL),
        render_data.fog_color_buffer.create_dp(vk::ImageLayout::GENERAL),
        render_data.depth_buffer.create_dp(vk::ImageLayout::GENERAL),
        //
        render_data.blue_noise.create_dp(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL),
    ]]
}

#[rustfmt::skip]
fn generate_raytrace_ds_prototypes(
    _core: &Core,
    render_data: &RenderData,
) -> Vec<Vec<DescriptorPrototype>> {
    vec![vec![
        render_data.block_data_atlas.create_dp(vk::ImageLayout::GENERAL),
        render_data.chunk_map.create_dp(vk::ImageLayout::GENERAL),
        render_data.region_map.create_dp(vk::ImageLayout::GENERAL),
        //
        render_data.lighting_buffer.create_dp(vk::ImageLayout::GENERAL),
        render_data.albedo_buffer.create_dp(vk::ImageLayout::GENERAL),
        render_data.emission_buffer.create_dp(vk::ImageLayout::GENERAL),
        render_data.fog_color_buffer.create_dp(vk::ImageLayout::GENERAL),
        render_data.normal_buffer.create_dp(vk::ImageLayout::GENERAL),
        render_data.depth_buffer.create_dp(vk::ImageLayout::GENERAL),
        //
        render_data.old_lighting_buffer.create_dp(vk::ImageLayout::GENERAL),
        render_data.old_normal_buffer.create_dp(vk::ImageLayout::GENERAL),
        render_data.old_depth_buffer.create_dp(vk::ImageLayout::GENERAL),
        //
        render_data.blue_noise.create_dp(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL),
        render_data.raytrace_uniform_data_buffer.create_dp(),
    ]]
}

fn generate_swapchain_ds_prototypes(
    core: &Core,
    _render_data: &RenderData,
) -> Vec<Vec<DescriptorPrototype>> {
    let views = &core.swapchain_info.swapchain_image_views;
    views
        .iter()
        .map(|image_view| {
            vec![DescriptorPrototype::StorageImage(
                *image_view,
                vk::ImageLayout::GENERAL,
            )]
        })
        .collect()
}

fn create_shader_module(core: &Core, shader_source: *const u8, length: usize) -> vk::ShaderModule {
    let shader_module_create_info = vk::ShaderModuleCreateInfo {
        code_size: length,
        p_code: shader_source as *const u32,
        ..Default::default()
    };
    unsafe {
        core.device
            .create_shader_module(&shader_module_create_info, None)
            .expect("Failed to create shader module.")
    }
}

fn create_compute_shader_stage(
    core: &Core,
    name: &str,
    shader_source: &[u8],
    entry_point: &str,
    descriptor_set_layouts: &[vk::DescriptorSetLayout],
    push_constant_ranges: &[vk::PushConstantRange],
) -> Stage {
    let shader_module = create_shader_module(core, shader_source.as_ptr(), shader_source.len());
    let entry_point_cstring = CString::new(entry_point).unwrap();
    let vk_stage = vk::PipelineShaderStageCreateInfo {
        module: shader_module,
        p_name: entry_point_cstring.as_ptr(),
        stage: vk::ShaderStageFlags::COMPUTE,
        ..Default::default()
    };

    let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo {
        set_layout_count: descriptor_set_layouts.len() as u32,
        p_set_layouts: descriptor_set_layouts.as_ptr(),
        push_constant_range_count: push_constant_ranges.len() as u32,
        p_push_constant_ranges: push_constant_ranges.as_ptr(),
        ..Default::default()
    };
    let pipeline_layout = unsafe {
        core.device
            .create_pipeline_layout(&pipeline_layout_create_info, None)
            .expect("Failed to create pipeline layout.")
    };
    core.set_debug_name(pipeline_layout, &format!("{}_layout", name));

    let pipeline_create_info = vk::ComputePipelineCreateInfo {
        stage: vk_stage,
        layout: pipeline_layout,
        ..Default::default()
    };
    let pipeline = unsafe {
        core.device
            .create_compute_pipelines(vk::PipelineCache::null(), &[pipeline_create_info], None)
            .expect("Failed to create compute pipeline.")[0]
    };
    core.set_debug_name(pipeline, name);

    unsafe {
        core.device.destroy_shader_module(shader_module, None);
    }
    Stage {
        vk_pipeline: pipeline,
        pipeline_layout,
    }
}

fn create_denoise_stage(core: &Core, dc: &DescriptorCollection) -> Stage {
    let shader_source = include_bytes!("../../shaders/spirv/bilateral_denoise.comp.spirv");
    create_compute_shader_stage(
        core,
        "raytrace",
        shader_source,
        "main",
        &[dc.raytrace.layout],
        &[vk::PushConstantRange {
            stage_flags: vk::ShaderStageFlags::COMPUTE,
            offset: 0,
            size: std::mem::size_of::<DenoisePushData>() as u32,
        }],
    )
}

fn create_finalize_stage(core: &Core, dc: &DescriptorCollection) -> Stage {
    let shader_source = include_bytes!("../../shaders/spirv/finalize.comp.spirv");
    create_compute_shader_stage(
        core,
        "finalize",
        shader_source,
        "main",
        &[dc.finalize.layout, dc.swapchain.layout],
        &[],
    )
}

fn create_raytrace_stage(core: &Core, dc: &DescriptorCollection) -> Stage {
    let shader_source = include_bytes!("../../shaders/spirv/raytrace.comp.spirv");
    create_compute_shader_stage(
        core,
        "raytrace",
        shader_source,
        "main",
        &[dc.raytrace.layout],
        &[],
    )
}
