use ash::version::DeviceV1_0;
use ash::vk;

use cgmath::{Matrix3, SquareMatrix, Vector3};

use std::ffi::CString;
use std::rc::Rc;

use crate::game::Game;
use crate::util;

use super::command_buffer::CommandBuffer;
use super::constants::*;
use super::core::Core;
#[macro_use]
use crate::create_descriptor_collection_struct;
use super::descriptors::DescriptorPrototype;
use super::structures::{
    Buffer, DataDestination, ImageOptions, SampledImage, SamplerOptions, StorageImage,
};

struct Stage {
    core: Rc<Core>,
    vk_pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
}

impl Drop for Stage {
    fn drop(&mut self) {
        unsafe {
            self.core
                .device
                .destroy_pipeline_layout(self.pipeline_layout, None);
            self.core.device.destroy_pipeline(self.vk_pipeline, None);
        }
    }
}

pub struct Pipeline {
    core: Rc<Core>,

    x_shader_groups: u32,
    y_shader_groups: u32,

    command_buffers: Vec<CommandBuffer>,
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
    pub fn new(core: Rc<Core>, game: &Game) -> Pipeline {
        let frame_available_semaphore = core.create_semaphore("frame_available");
        let frame_complete_semaphore = core.create_semaphore("frame_complete");
        let frame_complete_fence = core.create_fence(true, "frame_complete");
        let swapchain_length = core.swapchain.swapchain_images.len() as u32;
        let command_buffers = CommandBuffer::create_multiple(core.clone(), swapchain_length);

        let swapchain_extent = core.swapchain.swapchain_extent;
        let x_shader_groups = swapchain_extent.width / SHADER_GROUP_SIZE;
        let y_shader_groups = swapchain_extent.height / SHADER_GROUP_SIZE;

        let mut render_data = RenderData::create(core.clone());
        render_data.initialize(game);
        let descriptor_collection = DescriptorCollection::create(core.clone(), &render_data);

        let denoise_stage = create_denoise_stage(core.clone(), &descriptor_collection);
        let finalize_stage = create_finalize_stage(core.clone(), &descriptor_collection);
        let raytrace_stage = create_raytrace_stage(core.clone(), &descriptor_collection);

        let mut pipeline = Pipeline {
            core,

            x_shader_groups,
            y_shader_groups,

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
        pipeline.record_command_buffers();
        pipeline
    }

    fn record_command_buffers(&mut self) {
        for (index, buffer) in self.command_buffers.iter().enumerate() {
            let swapchain_image = self.core.swapchain.swapchain_images[index];

            buffer.set_debug_name(&format!("primary_command_buffer_{}", index));

            buffer.begin();

            let layout = self.raytrace_stage.pipeline_layout;
            let set = self.descriptor_collection.raytrace.variants[0];
            buffer.bind_descriptor_set(layout, 0, set);
            buffer.bind_pipeline(self.raytrace_stage.vk_pipeline);
            buffer.dispatch(self.x_shader_groups, self.y_shader_groups, 1);

            buffer.transition_layout(
                &swapchain_image,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::GENERAL,
            );
            let layout = self.finalize_stage.pipeline_layout;
            let set = self.descriptor_collection.finalize.variants[0];
            buffer.bind_descriptor_set(layout, 0, set);
            let set = self.descriptor_collection.swapchain.variants[index];
            buffer.bind_descriptor_set(layout, 1, set);
            buffer.bind_pipeline(self.finalize_stage.vk_pipeline);
            buffer.dispatch(self.x_shader_groups, self.y_shader_groups, 1);

            buffer.transition_layout(
                &swapchain_image,
                vk::ImageLayout::GENERAL,
                vk::ImageLayout::PRESENT_SRC_KHR,
            );
            buffer.end();
        }
    }

    pub fn draw_frame(&mut self, game: &mut Game) {
        let (image_index, _is_suboptimal) = unsafe {
            self.core
                .swapchain
                .swapchain_loader
                .acquire_next_image(
                    self.core.swapchain.swapchain,
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
            p_command_buffers: &self.command_buffers[image_index as usize].get_vk_command_buffer(),
            signal_semaphore_count: 1,
            p_signal_semaphores: signal_semaphores.as_ptr(),
            ..Default::default()
        };

        unsafe {
            let wait_fence = self.frame_complete_fence;
            self.core
                .device
                .wait_for_fences(&[wait_fence], true, std::u64::MAX)
                .expect("Failed to wait for previous frame to finish rendering.");
            self.core
                .device
                .reset_fences(&[wait_fence])
                .expect("Failed to reset fence.");
        }

        let camera = game.borrow_camera();
        let util::TripleEulerVector { forward, up, right } =
            util::compute_triple_euler_vector(camera.heading, camera.pitch);

        let uniform_data = &mut self.render_data.raytrace_uniform_data;
        uniform_data.origin = camera.origin;
        uniform_data.forward = forward;
        uniform_data.up = up * 0.4;
        uniform_data.right = right * 0.4;
        // Modulus to prevent overflowing the seed.
        uniform_data.seed = (uniform_data.seed + 1) % BLUE_NOISE_SIZE;
        uniform_data.sun_angle = game.get_sun_angle();

        let mut buffer_content = self.render_data.raytrace_uniform_data_buffer.bind_all();
        buffer_content[0] = uniform_data.clone();
        drop(buffer_content);

        // Do this after we set the buffer so that it will only affect the next frame.
        let uniform_data = &mut self.render_data.raytrace_uniform_data;
        uniform_data.old_origin = uniform_data.origin;
        let current_transform_matrix = {
            // Multiplying {screenx * depth, screeny * depth, depth} by this gets pixel position in world space.
            let screen_to_world_space =
                Matrix3::from_cols(right.clone() * 0.4, up.clone() * 0.4, forward.clone());
            // Inverting it gives us world space to screen space.
            screen_to_world_space
                .invert()
                .expect("Screen space vectors should cover entire coordinate space.")
        };
        uniform_data.old_transform_c0 = current_transform_matrix[0].clone();
        uniform_data.old_transform_c1 = current_transform_matrix[1].clone();
        uniform_data.old_transform_c2 = current_transform_matrix[2].clone();

        unsafe {
            let wait_fence = self.frame_complete_fence;
            self.core
                .device
                .queue_submit(self.core.compute_queue, &[submit_info], wait_fence)
                .expect("Failed to submit command queue.");
        }

        let wait_semaphores = [self.frame_complete_semaphore];
        let swapchains = [self.core.swapchain.swapchain];
        let present_info = vk::PresentInfoKHR {
            wait_semaphore_count: 1,
            p_wait_semaphores: wait_semaphores.as_ptr(),
            swapchain_count: 1,
            p_swapchains: swapchains.as_ptr(),
            p_image_indices: &image_index,
            ..Default::default()
        };

        unsafe {
            self.core
                .swapchain
                .swapchain_loader
                .queue_present(self.core.present_queue, &present_info)
                .expect("Failed to present swapchain image.");
        }
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        unsafe {
            self.core
                .device
                .device_wait_idle()
                .expect("Failed to wait for device to finish rendering.");

            self.core
                .device
                .destroy_fence(self.frame_complete_fence, None);
            self.core
                .device
                .destroy_semaphore(self.frame_available_semaphore, None);
            self.core
                .device
                .destroy_semaphore(self.frame_complete_semaphore, None);
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug)]
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
#[derive(Clone, Debug)]
struct DenoisePushData {
    size: i32,
}

struct RenderData {
    core: Rc<Core>,

    world: SampledImage,
    lod_transitions: StorageImage,

    lighting_buffer: StorageImage,
    depth_buffer: StorageImage,
    normal_buffer: StorageImage,
    old_lighting_buffer: StorageImage,
    old_depth_buffer: StorageImage,
    old_normal_buffer: StorageImage,

    lighting_pong_buffer: StorageImage,
    albedo_buffer: StorageImage,
    emission_buffer: StorageImage,
    fog_color_buffer: StorageImage,

    blue_noise: SampledImage,

    raytrace_uniform_data: RaytraceUniformData,
    raytrace_uniform_data_buffer: Buffer<RaytraceUniformData>,
}

impl RenderData {
    fn create_framebuffer(core: Rc<Core>, name: &str, format: vk::Format) -> StorageImage {
        let dimensions = core.swapchain.swapchain_extent;
        let options = ImageOptions {
            typ: vk::ImageType::TYPE_2D,
            extent: vk::Extent3D {
                width: dimensions.width,
                height: dimensions.height,
                depth: 1,
            },
            format,
            usage: vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::STORAGE,
            ..Default::default()
        };
        StorageImage::create(core, name, &options)
    }

    fn create_world(core: Rc<Core>) -> SampledImage {
        let image_options = ImageOptions {
            typ: vk::ImageType::TYPE_3D,
            extent: vk::Extent3D {
                width: ROOT_BLOCK_WIDTH,
                height: ROOT_BLOCK_WIDTH,
                depth: ROOT_BLOCK_WIDTH,
            },
            format: vk::Format::R16_UINT,
            usage: vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            mip_levels: 10,
            ..Default::default()
        };
        let sampler_options = SamplerOptions {
            min_filter: vk::Filter::NEAREST,
            mag_filter: vk::Filter::NEAREST,
            address_mode: vk::SamplerAddressMode::CLAMP_TO_BORDER,
            border_color: vk::BorderColor::INT_OPAQUE_WHITE,
            unnormalized_coordinates: false,
            mipmap_mode: vk::SamplerMipmapMode::NEAREST,
        };
        SampledImage::create(core.clone(), "world", &image_options, &sampler_options)
    }

    fn create_lod_transitions(core: Rc<Core>) -> StorageImage {
        let image_options = ImageOptions {
            typ: vk::ImageType::TYPE_1D,
            extent: vk::Extent3D {
                width: ROOT_BLOCK_WIDTH,
                height: 1,
                depth: 1,
            },
            format: vk::Format::R16G16_UINT,
            usage: vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::STORAGE,
            ..Default::default()
        };
        let image = StorageImage::create(core, "lod_transitions", &image_options);
        let lod_divisors: [u16; 10] = [1, 2, 4, 8, 16, 32, 64, 128, 256, 512];
        let mut image_data = [0u16; ROOT_BLOCK_WIDTH as usize * 2];
        for coordinate in 0..ROOT_BLOCK_WIDTH as u16 {
            let pixel_index = (coordinate * 2) as usize;
            // Contain the highest LOD region which has been entered at each coordinate.
            // For example, if the world were only 8 blocks cubed the LODs would look like this:
            // 3, 0, 1, 0, 2, 0, 1, 0
            // The pattern can be found by checking at each element if the index is divisible by
            // 8, then 4, then 2, then 1, picking whichever LOD comes first.
            let mut highest_lod = 0;
            for lod in (1..8).rev() {
                if coordinate % lod_divisors[lod] == 0 {
                    highest_lod = lod;
                    break;
                }
            }
            image_data[pixel_index] = lod_divisors[highest_lod]; // Red channel
            image_data[pixel_index + 1] = highest_lod as u16; // Green channel
        }
        image.load_from_slice(&image_data);
        image
    }

    fn create_blue_noise(core: Rc<Core>) -> SampledImage {
        let image_options = ImageOptions {
            typ: vk::ImageType::TYPE_2D,
            extent: vk::Extent3D {
                width: BLUE_NOISE_WIDTH,
                height: BLUE_NOISE_HEIGHT,
                depth: 1,
            },
            format: vk::Format::R8G8B8A8_UNORM,
            usage: vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            ..Default::default()
        };
        let sampler_options = SamplerOptions {
            min_filter: vk::Filter::NEAREST,
            mag_filter: vk::Filter::NEAREST,
            address_mode: vk::SamplerAddressMode::CLAMP_TO_EDGE,
            unnormalized_coordinates: true,
            ..Default::default()
        };
        let tex =
            SampledImage::create(core.clone(), "blue_noise", &image_options, &sampler_options);
        tex.load_from_png_rgba8(include_bytes!("blue_noise_512.png"));
        tex
    }

    fn create_raytrace_uniform_data() -> RaytraceUniformData {
        RaytraceUniformData {
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
        }
    }

    fn create(core: Rc<Core>) -> RenderData {
        let rgba16_unorm = vk::Format::R16G16B16A16_UNORM;
        let rgba8_unorm = vk::Format::R8G8B8A8_UNORM;
        let r16_uint = vk::Format::R16_UINT;
        let r8_uint = vk::Format::R8_UINT;
        RenderData {
            core: core.clone(),

            world: Self::create_world(core.clone()),
            lod_transitions: Self::create_lod_transitions(core.clone()),

            lighting_buffer: Self::create_framebuffer(core.clone(), "lighting_buf", rgba16_unorm),
            depth_buffer: Self::create_framebuffer(core.clone(), "depth_buf", r16_uint),
            normal_buffer: Self::create_framebuffer(core.clone(), "normal_buf", r8_uint),
            old_lighting_buffer: Self::create_framebuffer(
                core.clone(),
                "old_lighting_buf",
                rgba16_unorm,
            ),
            old_depth_buffer: Self::create_framebuffer(core.clone(), "old_depth_buf", r16_uint),
            old_normal_buffer: Self::create_framebuffer(core.clone(), "old_normal_buf", r8_uint),

            lighting_pong_buffer: Self::create_framebuffer(
                core.clone(),
                "lighting_pong_buf",
                rgba16_unorm,
            ),
            albedo_buffer: Self::create_framebuffer(core.clone(), "albedo_buf", rgba8_unorm),
            emission_buffer: Self::create_framebuffer(core.clone(), "emission_buf", rgba8_unorm),
            fog_color_buffer: Self::create_framebuffer(core.clone(), "fog_color_buf", rgba8_unorm),

            blue_noise: Self::create_blue_noise(core.clone()),

            raytrace_uniform_data: Self::create_raytrace_uniform_data(),
            raytrace_uniform_data_buffer: Buffer::create(
                core.clone(),
                "raytrace_uniform_data",
                1,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
            ),
        }
    }

    fn make_lod_upload_buffer(&mut self, lod_level: u32, content: &[u16]) -> Buffer<u16> {
        let scale = (2u64).pow(lod_level).pow(3);
        let size = ROOT_BLOCK_VOLUME as u64 / scale;
        let mut buffer = Buffer::create(
            self.core.clone(),
            "lod0",
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
        );
        let mut bound_content = buffer.bind_all();
        for index in 0..size as usize {
            bound_content[index] = content[index];
        }
        drop(bound_content);
        println!("Created upload buffer for LOD{}", lod_level);
        buffer
    }

    fn upload_lod(&self, command_buf: &mut CommandBuffer, data_buf: &Buffer<u16>, lod_level: u32) {
        let scale = (2u32).pow(lod_level);
        let dimension = ROOT_BLOCK_WIDTH / scale;
        let extent = vk::Extent3D {
            width: dimension,
            height: dimension,
            depth: dimension,
        };
        command_buf.copy_buffer_to_image_mip(data_buf, &self.world, &extent, lod_level);
    }

    fn initialize(&mut self, game: &Game) {
        let world = game.borrow_world();
        let lod0_buf = self.make_lod_upload_buffer(0, &world.content_lod0);
        let lod1_buf = self.make_lod_upload_buffer(1, &world.content_lod1);
        let lod2_buf = self.make_lod_upload_buffer(2, &world.content_lod2);
        let lod3_buf = self.make_lod_upload_buffer(3, &world.content_lod3);
        let lod4_buf = self.make_lod_upload_buffer(4, &world.content_lod4);
        let lod5_buf = self.make_lod_upload_buffer(5, &world.content_lod5);
        let lod6_buf = self.make_lod_upload_buffer(6, &world.content_lod6);
        let lod7_buf = self.make_lod_upload_buffer(7, &world.content_lod7);
        let lod8_buf = self.make_lod_upload_buffer(8, &world.content_lod8);
        let lod9_buf = self.make_lod_upload_buffer(9, &world.content_lod9);
        let mut commands = CommandBuffer::create_single(self.core.clone());
        commands.begin_one_time_submit();
        commands.transition_layout_mipped(
            &self.world,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            10,
        );
        self.upload_lod(&mut commands, &lod0_buf, 0);
        self.upload_lod(&mut commands, &lod1_buf, 1);
        self.upload_lod(&mut commands, &lod2_buf, 2);
        self.upload_lod(&mut commands, &lod3_buf, 3);
        self.upload_lod(&mut commands, &lod4_buf, 4);
        self.upload_lod(&mut commands, &lod5_buf, 5);
        self.upload_lod(&mut commands, &lod6_buf, 6);
        self.upload_lod(&mut commands, &lod7_buf, 7);
        self.upload_lod(&mut commands, &lod8_buf, 8);
        self.upload_lod(&mut commands, &lod9_buf, 9);
        commands.transition_layout_mipped(
            &self.world,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::GENERAL,
            10,
        );
        let generic_layout_images = [
            &self.albedo_buffer,
            &self.depth_buffer,
            &self.emission_buffer,
            &self.fog_color_buffer,
            &self.lighting_buffer,
            &self.lighting_pong_buffer,
            &self.normal_buffer,
            &self.old_depth_buffer,
            &self.old_lighting_buffer,
            &self.old_normal_buffer,
        ];
        for image in generic_layout_images.iter() {
            commands.transition_layout(
                *image,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::GENERAL,
            );
        }
        commands.transition_layout(
            &self.lod_transitions,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::GENERAL,
        );
        commands.transition_layout(
            &self.blue_noise,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );
        commands.end();
        commands.blocking_execute_and_destroy();
        drop(lod0_buf);
        drop(lod1_buf);
        drop(lod2_buf);
        drop(lod3_buf);
        drop(lod4_buf);
        drop(lod5_buf);
        drop(lod6_buf);
        drop(lod7_buf);
        drop(lod8_buf);
        drop(lod9_buf);
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
    _core: Rc<Core>,
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
    _core: Rc<Core>,
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
    _core: Rc<Core>,
    render_data: &RenderData,
) -> Vec<Vec<DescriptorPrototype>> {
    vec![vec![
        render_data.world.create_dp(vk::ImageLayout::GENERAL),
        render_data.lod_transitions.create_dp(vk::ImageLayout::GENERAL),
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
    core: Rc<Core>,
    _render_data: &RenderData,
) -> Vec<Vec<DescriptorPrototype>> {
    let views = &core.swapchain.swapchain_image_views;
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

fn create_shader_module(
    core: Rc<Core>,
    shader_source: *const u8,
    length: usize,
) -> vk::ShaderModule {
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
    core: Rc<Core>,
    name: &str,
    shader_source: &[u8],
    entry_point: &str,
    descriptor_set_layouts: &[vk::DescriptorSetLayout],
    push_constant_ranges: &[vk::PushConstantRange],
) -> Stage {
    let shader_module =
        create_shader_module(core.clone(), shader_source.as_ptr(), shader_source.len());
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
        core,
        vk_pipeline: pipeline,
        pipeline_layout,
    }
}

fn create_denoise_stage(core: Rc<Core>, dc: &DescriptorCollection) -> Stage {
    let shader_source = include_bytes!("../../shaders/spirv/bilateral_denoise.comp.spirv");
    create_compute_shader_stage(
        core,
        "raytrace",
        shader_source,
        "main",
        &[dc.denoise.layout],
        &[vk::PushConstantRange {
            stage_flags: vk::ShaderStageFlags::COMPUTE,
            offset: 0,
            size: std::mem::size_of::<DenoisePushData>() as u32,
        }],
    )
}

fn create_finalize_stage(core: Rc<Core>, dc: &DescriptorCollection) -> Stage {
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

fn create_raytrace_stage(core: Rc<Core>, dc: &DescriptorCollection) -> Stage {
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
