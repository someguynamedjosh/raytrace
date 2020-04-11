use ash::version::DeviceV1_0;
use ash::vk;
use std::rc::Rc;

use crate::render::general::core::Core;
#[macro_use]
use crate::create_descriptor_collection_struct;
use crate::render::general::descriptors::DescriptorPrototype;

use super::render_data::RenderData;

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
            render_data.depth_buffer.create_dp(vk::ImageLayout::GENERAL),
            render_data.normal_buffer.create_dp(vk::ImageLayout::GENERAL),
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
        render_data.albedo_buffer.create_dp(vk::ImageLayout::GENERAL),
        render_data.emission_buffer.create_dp(vk::ImageLayout::GENERAL),
        render_data.fog_color_buffer.create_dp(vk::ImageLayout::GENERAL),
        //
        render_data.lighting_buffer.create_dp(vk::ImageLayout::GENERAL),
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
        render_data.minefield.create_dp(vk::ImageLayout::GENERAL),
        //
        render_data.albedo_buffer.create_dp(vk::ImageLayout::GENERAL),
        render_data.emission_buffer.create_dp(vk::ImageLayout::GENERAL),
        render_data.fog_color_buffer.create_dp(vk::ImageLayout::GENERAL),
        //
        render_data.lighting_buffer.create_dp(vk::ImageLayout::GENERAL),
        render_data.completed_buffer.create_dp(vk::ImageLayout::GENERAL),
        render_data.normal_buffer.create_dp(vk::ImageLayout::GENERAL),
        render_data.depth_buffer.create_dp(vk::ImageLayout::GENERAL),
        //
        render_data.blue_noise.create_dp(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL),
        render_data.raytrace_uniform_data_buffer.create_dp(),
        //
        render_data.world_lod1.create_dp(vk::ImageLayout::GENERAL),
        render_data.minefield_lod1.create_dp(vk::ImageLayout::GENERAL),
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
