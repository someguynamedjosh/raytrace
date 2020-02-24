use ash::version::DeviceV1_0;
use ash::vk;
use std::ffi::CString;
use std::rc::Rc;

use crate::render::general::core::Core;

use super::descriptor_sets::DescriptorCollection;
use super::structs::DenoisePushData;

pub struct Stage {
    pub core: Rc<Core>,
    pub vk_pipeline: vk::Pipeline,
    pub pipeline_layout: vk::PipelineLayout,
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

pub fn create_denoise_stage(core: Rc<Core>, dc: &DescriptorCollection) -> Stage {
    let shader_source = include_bytes!("../../../shaders/spirv/bilateral_denoise.comp.spirv");
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

pub fn create_finalize_stage(core: Rc<Core>, dc: &DescriptorCollection) -> Stage {
    let shader_source = include_bytes!("../../../shaders/spirv/finalize.comp.spirv");
    create_compute_shader_stage(
        core,
        "finalize",
        shader_source,
        "main",
        &[dc.finalize.layout, dc.swapchain.layout],
        &[],
    )
}

pub fn create_raytrace_stage(core: Rc<Core>, dc: &DescriptorCollection) -> Stage {
    let shader_source = include_bytes!("../../../shaders/spirv/raytrace.comp.spirv");
    create_compute_shader_stage(
        core,
        "raytrace",
        shader_source,
        "main",
        &[dc.raytrace.layout],
        &[],
    )
}
