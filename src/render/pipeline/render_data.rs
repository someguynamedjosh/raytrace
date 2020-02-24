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

use super::structs::RaytraceUniformData;

pub struct RenderData {
    pub core: Rc<Core>,

    pub world: SampledImage,
    pub minefield: SampledImage,

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
            border_color: vk::BorderColor::INT_OPAQUE_BLACK,
            unnormalized_coordinates: false,
            mipmap_mode: vk::SamplerMipmapMode::NEAREST,
        };
        SampledImage::create(core.clone(), "world", &image_options, &sampler_options)
    }

    fn create_minefield(core: Rc<Core>) -> SampledImage {
        let image_options = ImageOptions {
            typ: vk::ImageType::TYPE_3D,
            extent: vk::Extent3D {
                width: ROOT_BLOCK_WIDTH,
                height: ROOT_BLOCK_WIDTH,
                depth: ROOT_BLOCK_WIDTH,
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
        SampledImage::create(core.clone(), "minefield", &image_options, &sampler_options)
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

    pub fn create(core: Rc<Core>) -> RenderData {
        let rgba16_unorm = vk::Format::R16G16B16A16_UNORM;
        let rgba8_unorm = vk::Format::R8G8B8A8_UNORM;
        let r16_uint = vk::Format::R16_UINT;
        let r8_uint = vk::Format::R8_UINT;
        RenderData {
            core: core.clone(),

            world: Self::create_world(core.clone()),
            minefield: Self::create_minefield(core.clone()),

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

    fn make_minefield_data(&self, game: &Game) -> Buffer<u8> {
        let world = game.borrow_world();
        let mut buffer = Buffer::create(
            self.core.clone(),
            "minefield_data",
            ROOT_BLOCK_VOLUME as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
        );
        let mut bound_content = buffer.bind_all();
        for index in 0..ROOT_BLOCK_VOLUME {
            let (x, y, z) = util::index_to_coord_3d(index, ROOT_BLOCK_WIDTH);
            bound_content[index as usize] = world.min_lod_at_coord(x, y, z);
        }
        drop(bound_content);
        println!("Created upload buffer for minefield.");
        buffer
    }

    pub fn initialize(&mut self, game: &Game) {
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
        let minefield = self.make_minefield_data(game);
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
