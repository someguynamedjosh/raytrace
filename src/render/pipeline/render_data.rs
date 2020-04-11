use ash::vk;
use std::rc::Rc;

use crate::game::Game;
use crate::render::constants::*;
use crate::render::general::command_buffer::CommandBuffer;
use crate::render::general::core::Core;
use crate::render::general::structures::{
    Buffer, DataDestination, ImageOptions, SampledImage, SamplerOptions, StorageImage,
};
use crate::util;
use crate::world::{World, CHUNK_SIZE};

use super::structs::RaytraceUniformData;

pub struct RenderData {
    pub core: Rc<Core>,

    pub world: SampledImage,
    pub minefield: SampledImage,
    pub world_lod1: SampledImage,
    pub minefield_lod1: SampledImage,

    pub lighting_buffer: StorageImage,
    pub completed_buffer: StorageImage,
    pub depth_buffer: StorageImage,
    pub normal_buffer: StorageImage,

    pub lighting_pong_buffer: StorageImage,
    pub albedo_buffer: StorageImage,
    pub emission_buffer: StorageImage,
    pub fog_color_buffer: StorageImage,

    pub blue_noise: SampledImage,

    pub raytrace_uniform_data: RaytraceUniformData,
    pub raytrace_uniform_data_buffer: Buffer<RaytraceUniformData>,
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
                width: ROOT_BLOCK_WIDTH as u32,
                height: ROOT_BLOCK_WIDTH as u32,
                depth: ROOT_BLOCK_WIDTH as u32,
            },
            format: vk::Format::R16_UINT,
            usage: vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            ..Default::default()
        };
        let sampler_options = SamplerOptions {
            min_filter: vk::Filter::NEAREST,
            mag_filter: vk::Filter::NEAREST,
            address_mode: vk::SamplerAddressMode::CLAMP_TO_BORDER,
            border_color: vk::BorderColor::INT_OPAQUE_BLACK,
            unnormalized_coordinates: false,
            mipmap_mode: vk::SamplerMipmapMode::NEAREST,
        };
        SampledImage::create(core.clone(), "world", &image_options, &sampler_options)
    }

    fn create_world_lod1(core: Rc<Core>) -> SampledImage {
        let image_options = ImageOptions {
            typ: vk::ImageType::TYPE_3D,
            extent: vk::Extent3D {
                width: ROOT_BLOCK_WIDTH as u32,
                height: ROOT_BLOCK_WIDTH as u32,
                depth: ROOT_BLOCK_WIDTH as u32,
            },
            format: vk::Format::R32_UINT,
            usage: vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            ..Default::default()
        };
        let sampler_options = SamplerOptions {
            min_filter: vk::Filter::NEAREST,
            mag_filter: vk::Filter::NEAREST,
            address_mode: vk::SamplerAddressMode::CLAMP_TO_BORDER,
            border_color: vk::BorderColor::INT_OPAQUE_BLACK,
            unnormalized_coordinates: false,
            mipmap_mode: vk::SamplerMipmapMode::NEAREST,
        };
        SampledImage::create(core.clone(), "world_lod1", &image_options, &sampler_options)
    }

    fn create_minefield(core: Rc<Core>, label: &str) -> SampledImage {
        let image_options = ImageOptions {
            typ: vk::ImageType::TYPE_3D,
            extent: vk::Extent3D {
                width: ROOT_BLOCK_WIDTH as u32,
                height: ROOT_BLOCK_WIDTH as u32,
                depth: ROOT_BLOCK_WIDTH as u32,
            },
            format: vk::Format::R8_UINT,
            usage: vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            ..Default::default()
        };
        let sampler_options = SamplerOptions {
            min_filter: vk::Filter::NEAREST,
            mag_filter: vk::Filter::NEAREST,
            address_mode: vk::SamplerAddressMode::CLAMP_TO_BORDER,
            border_color: vk::BorderColor::INT_OPAQUE_BLACK,
            unnormalized_coordinates: true,
            ..Default::default()
        };
        SampledImage::create(core.clone(), label, &image_options, &sampler_options)
    }

    fn create_blue_noise(core: Rc<Core>) -> SampledImage {
        let image_options = ImageOptions {
            typ: vk::ImageType::TYPE_2D,
            extent: vk::Extent3D {
                width: BLUE_NOISE_WIDTH as u32,
                height: BLUE_NOISE_HEIGHT as u32,
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

    pub fn create(core: Rc<Core>) -> RenderData {
        let rgba16_unorm = vk::Format::R16G16B16A16_UNORM;
        let rgba8_unorm = vk::Format::R8G8B8A8_UNORM;
        let r16_uint = vk::Format::R16_UINT;
        let r8_uint = vk::Format::R8_UINT;
        RenderData {
            core: core.clone(),

            world: Self::create_world(core.clone()),
            minefield: Self::create_minefield(core.clone(), "minefield"),
            world_lod1: Self::create_world_lod1(core.clone()),
            minefield_lod1: Self::create_minefield(core.clone(), "minefield_lod1"),

            lighting_buffer: Self::create_framebuffer(core.clone(), "lighting_buf", rgba16_unorm),
            completed_buffer: Self::create_framebuffer(core.clone(), "completed_buf", rgba16_unorm),
            depth_buffer: Self::create_framebuffer(core.clone(), "depth_buf", r16_uint),
            normal_buffer: Self::create_framebuffer(core.clone(), "normal_buf", r8_uint),

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

    fn make_world_upload_buffers(
        &mut self,
        world: &mut World,
    ) -> (Buffer<u16>, Buffer<u8>, Buffer<u8>) {
        let mut blocks_buffer = Buffer::create(
            self.core.clone(),
            "blocks",
            ROOT_BLOCK_VOLUME as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
        );
        let mut minefield_buffer = Buffer::create(
            self.core.clone(),
            "minefield",
            ROOT_BLOCK_VOLUME as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
        );

        const ROOT_CHUNK_WIDTH: usize = ROOT_BLOCK_WIDTH / CHUNK_SIZE;
        let mut blocks_buffer_data = blocks_buffer.bind_all();
        let mut minefield_buffer_data = minefield_buffer.bind_all();
        for chunk_coord in util::coord_iter_3d(ROOT_CHUNK_WIDTH) {
            let chunk = world.borrow_chunk(&util::offset_coord_3d(
                &chunk_coord,
                &(
                    ROOT_CHUNK_WIDTH / 2,
                    ROOT_CHUNK_WIDTH / 2,
                    ROOT_CHUNK_WIDTH / 2,
                ),
            ));
            chunk.copy_blocks(
                blocks_buffer_data.as_slice_mut(),
                ROOT_BLOCK_WIDTH,
                &util::scale_coord_3d(&chunk_coord, CHUNK_SIZE),
            );
            chunk.copy_minefield(
                minefield_buffer_data.as_slice_mut(),
                ROOT_BLOCK_WIDTH,
                &util::scale_coord_3d(&chunk_coord, CHUNK_SIZE),
            );
        }
        drop(blocks_buffer_data);
        drop(minefield_buffer_data);

        let mut minefield_lod1_buffer = Buffer::create(
            self.core.clone(),
            "minefield_lod1",
            ROOT_BLOCK_VOLUME as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
        );
        let mut minefield_lod1_buffer_data = minefield_lod1_buffer.bind_all();
        for chunk_coord in util::coord_iter_3d(ROOT_CHUNK_WIDTH) {
            let mip = world.borrow_lod1_mip(&chunk_coord);
            mip.copy_minefield(
                minefield_lod1_buffer_data.as_slice_mut(),
                ROOT_BLOCK_WIDTH,
                &util::scale_coord_3d(&chunk_coord, CHUNK_SIZE),
            );
        }
        drop(minefield_lod1_buffer_data);

        (blocks_buffer, minefield_buffer, minefield_lod1_buffer)
    }

    pub fn initialize(&mut self, game: &mut Game) {
        let world = game.borrow_world_mut();
        let (lod0_buf, minefield, minefield_lod1) = self.make_world_upload_buffers(world);
        let commands = CommandBuffer::create_single(self.core.clone());
        commands.begin_one_time_submit();
        commands.transition_layout(
            &self.world,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        );
        commands.copy_buffer_to_image(&lod0_buf, &self.world, &self.world);
        commands.transition_layout(
            &self.world,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::GENERAL,
        );
        commands.transition_layout(
            &self.minefield,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        );
        commands.copy_buffer_to_image(&minefield, &self.minefield, &self.minefield);
        commands.transition_layout(
            &self.minefield,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::GENERAL,
        );
        commands.transition_layout(
            &self.minefield_lod1,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        );
        commands.copy_buffer_to_image(&minefield_lod1, &self.minefield_lod1, &self.minefield_lod1);
        commands.transition_layout(
            &self.minefield_lod1,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::GENERAL,
        );
        let generic_layout_images = [
            &self.albedo_buffer,
            &self.completed_buffer,
            &self.depth_buffer,
            &self.emission_buffer,
            &self.fog_color_buffer,
            &self.lighting_buffer,
            &self.lighting_pong_buffer,
            &self.normal_buffer,
        ];
        for image in generic_layout_images.iter() {
            commands.transition_layout(
                *image,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::GENERAL,
            );
        }
        commands.transition_layout(
            &self.blue_noise,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );
        commands.end();
        commands.blocking_execute_and_destroy();
    }
}
