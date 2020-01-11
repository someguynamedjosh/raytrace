use ash::version::DeviceV1_0;
use ash::vk;

use std::rc::Rc;

use super::core::Core;

pub enum DescriptorPrototype {
    StorageImage(vk::ImageView, vk::ImageLayout),
    CombinedImageSampler(vk::ImageView, vk::ImageLayout, vk::Sampler),
    UniformBuffer(vk::Buffer, u64, u64),
}

impl DescriptorPrototype {
    fn matches(&self, other: &Self) -> bool {
        match self {
            Self::StorageImage(..) => {
                if let Self::StorageImage(..) = other {
                    true
                } else {
                    false
                }
            }
            Self::CombinedImageSampler(..) => {
                if let Self::CombinedImageSampler(..) = other {
                    true
                } else {
                    false
                }
            }
            Self::UniformBuffer(..) => {
                if let Self::UniformBuffer(..) = other {
                    true
                } else {
                    false
                }
            }
        }
    }

    fn get_descriptor_type(&self) -> vk::DescriptorType {
        match self {
            Self::StorageImage(..) => vk::DescriptorType::STORAGE_IMAGE,
            Self::CombinedImageSampler(..) => vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            Self::UniformBuffer(..) => vk::DescriptorType::UNIFORM_BUFFER,
        }
    }

    fn create_descriptor_set_layout_binding(&self, index: u32) -> vk::DescriptorSetLayoutBinding {
        vk::DescriptorSetLayoutBinding {
            binding: index,
            descriptor_type: self.get_descriptor_type(),
            descriptor_count: 1,
            stage_flags: vk::ShaderStageFlags::COMPUTE,
            ..Default::default()
        }
    }

    fn create_descriptor_payload(&self) -> DescriptorPayload {
        match *self {
            Self::StorageImage(image_view, image_layout) => {
                DescriptorPayload::ImageInfo(vk::DescriptorImageInfo {
                    image_view,
                    image_layout,
                    ..Default::default()
                })
            }
            Self::CombinedImageSampler(image_view, image_layout, sampler) => {
                DescriptorPayload::ImageInfo(vk::DescriptorImageInfo {
                    image_view,
                    image_layout,
                    sampler,
                    ..Default::default()
                })
            }
            Self::UniformBuffer(buffer, offset, range) => {
                DescriptorPayload::BufferInfo(vk::DescriptorBufferInfo {
                    buffer,
                    offset,
                    range,
                    ..Default::default()
                })
            }
        }
    }
}

#[derive(Debug)]
enum DescriptorPayload {
    ImageInfo(vk::DescriptorImageInfo),
    BufferInfo(vk::DescriptorBufferInfo),
}

impl DescriptorPayload {
    fn add_to_write_op(&self, write_op: &mut vk::WriteDescriptorSet) {
        match self {
            Self::ImageInfo(image_info) => write_op.p_image_info = image_info,
            Self::BufferInfo(buffer_info) => write_op.p_buffer_info = buffer_info,
        }
    }
}

pub struct DescriptorData {
    pub layout: vk::DescriptorSetLayout,
    pub variants: Vec<vk::DescriptorSet>,
}

fn variants_match(variants: &Vec<Vec<DescriptorPrototype>>) -> bool {
    let mut iterator = variants.iter();
    iterator.next();
    for variant in iterator {
        for (item, other_item) in variants[0].iter().zip(variant.iter()) {
            if !item.matches(other_item) {
                return false;
            }
        }
    }
    true
}

struct DescriptorTypeAccumulator {
    totals: Vec<u32>,
}

impl DescriptorTypeAccumulator {
    fn new() -> DescriptorTypeAccumulator {
        DescriptorTypeAccumulator {
            totals: (0..13).map(|_| 0).collect(),
        }
    }

    fn index(&self, typ: vk::DescriptorType) -> usize {
        match typ {
            vk::DescriptorType::ACCELERATION_STRUCTURE_NV => 0,
            vk::DescriptorType::COMBINED_IMAGE_SAMPLER => 1,
            vk::DescriptorType::INLINE_UNIFORM_BLOCK_EXT => 2,
            vk::DescriptorType::INPUT_ATTACHMENT => 3,
            vk::DescriptorType::SAMPLED_IMAGE => 4,
            vk::DescriptorType::SAMPLER => 5,
            vk::DescriptorType::STORAGE_BUFFER => 6,
            vk::DescriptorType::STORAGE_BUFFER_DYNAMIC => 7,
            vk::DescriptorType::STORAGE_IMAGE => 8,
            vk::DescriptorType::STORAGE_TEXEL_BUFFER => 9,
            vk::DescriptorType::UNIFORM_BUFFER => 10,
            vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC => 11,
            vk::DescriptorType::UNIFORM_TEXEL_BUFFER => 12,
            _ => unreachable!("Encountered invalid descriptor type."),
        }
    }

    fn increment(&mut self, typ: vk::DescriptorType, amount: u32) {
        let index = self.index(typ);
        self.totals[index] += amount;
    }

    fn all_totals(&self) -> Vec<(vk::DescriptorType, u32)> {
        vec![
            (vk::DescriptorType::ACCELERATION_STRUCTURE_NV, self.totals[0]),
            (vk::DescriptorType::COMBINED_IMAGE_SAMPLER, self.totals[1]),
            (vk::DescriptorType::INLINE_UNIFORM_BLOCK_EXT, self.totals[2]),
            (vk::DescriptorType::INPUT_ATTACHMENT, self.totals[3]),
            (vk::DescriptorType::SAMPLED_IMAGE, self.totals[4]),
            (vk::DescriptorType::SAMPLER, self.totals[5]),
            (vk::DescriptorType::STORAGE_BUFFER, self.totals[6]),
            (vk::DescriptorType::STORAGE_BUFFER_DYNAMIC, self.totals[7]),
            (vk::DescriptorType::STORAGE_IMAGE, self.totals[8]),
            (vk::DescriptorType::STORAGE_TEXEL_BUFFER, self.totals[9]),
            (vk::DescriptorType::UNIFORM_BUFFER, self.totals[10]),
            (vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC, self.totals[11]),
            (vk::DescriptorType::UNIFORM_TEXEL_BUFFER, self.totals[12]),
        ]
    }
}

pub type PrototypeGenerator<Data> = Box<dyn Fn(Rc<Core>, &Data) -> Vec<Vec<DescriptorPrototype>>>;

pub fn generate_descriptor_pool<Data>(
    prototype_generators: &[PrototypeGenerator<Data>],
    names: &[&str],
    core: Rc<Core>,
    data: &Data,
) -> (vk::DescriptorPool, Vec<DescriptorData>) {
    let prototypes: Vec<_> = prototype_generators
        .into_iter()
        .map(|generator| {
            let variants = generator(core.clone(), data);
            debug_assert!(variants_match(&variants));
            variants
        })
        .collect();

    let empty_variant = vec![];
    let mut counter = DescriptorTypeAccumulator::new();
    let mut total_descriptor_sets = 0;
    let mut total_descriptors = 0;
    let layout_info: Vec<_> = prototypes
        .iter()
        .enumerate()
        .map(|(index, variants)| {
            total_descriptor_sets += variants.len() as u32;
            let arbitrary_variant = if variants.len() == 0 {
                &empty_variant // If there are no variants, then just make an empty layout.
            } else {
                &variants[0]
            };
            total_descriptors += variants.len() * arbitrary_variant.len();

            for item in arbitrary_variant {
                counter.increment(item.get_descriptor_type(), variants.len() as u32);
            }
            let bindings: Vec<_> = arbitrary_variant
                .iter()
                .enumerate()
                .map(|(index, item)| item.create_descriptor_set_layout_binding(index as u32))
                .collect();
            let create_info = vk::DescriptorSetLayoutCreateInfo {
                binding_count: bindings.len() as u32,
                p_bindings: bindings.as_ptr(),
                ..Default::default()
            };
            let layout = unsafe {
                core.device
                    .create_descriptor_set_layout(&create_info, None)
                    .expect("Failed to create descriptor set layout.")
            };
            core.set_debug_name(layout, &format!("{}_ds_layout", names[index]));
            (layout, variants.len())
        })
        .collect();
    let mut pool_sizes = vec![];
    for (typ, total) in counter.all_totals() {
        if total == 0 {
            continue; // Don't bother specifying that we need none of a particular type.
        }
        pool_sizes.push(vk::DescriptorPoolSize {
            ty: typ,
            descriptor_count: total,
            ..Default::default()
        });
    }
    let pool_create_info = vk::DescriptorPoolCreateInfo {
        pool_size_count: pool_sizes.len() as u32,
        p_pool_sizes: pool_sizes.as_ptr(),
        max_sets: total_descriptor_sets,
        ..Default::default()
    };
    let descriptor_pool = unsafe {
        core.device
            .create_descriptor_pool(&pool_create_info, None)
            .expect("Failed to create descriptor pool.")
    };
    core.set_debug_name(descriptor_pool, "primary_descriptor_pool");

    let mut request_layouts = vec![];
    for (layout, quantity) in &layout_info {
        for _ in 0..*quantity {
            request_layouts.push(*layout);
        }
    }
    debug_assert!(request_layouts.len() as u32 == total_descriptor_sets);
    let allocate_info = vk::DescriptorSetAllocateInfo {
        descriptor_pool,
        descriptor_set_count: total_descriptor_sets,
        p_set_layouts: request_layouts.as_ptr(),
        ..Default::default()
    };
    let mut descriptor_sets = unsafe {
        core.device
            .allocate_descriptor_sets(&allocate_info)
            .expect("Failed to create descriptor sets.")
    };

    let variants_flat_iter = prototypes.iter().map(|variants| variants.iter()).flatten();
    let mut payload_holder = Vec::with_capacity(total_descriptors);
    let mut writes = Vec::with_capacity(total_descriptors);
    for (variant_index, variant) in variants_flat_iter.enumerate() {
        for (item_index, item) in variant.iter().enumerate() {
            let payload = item.create_descriptor_payload();
            let payload_index = payload_holder.len();
            payload_holder.push(payload);
            let mut write_op = vk::WriteDescriptorSet {
                dst_set: descriptor_sets[variant_index],
                dst_binding: item_index as u32,
                descriptor_count: 1,
                descriptor_type: item.get_descriptor_type(),
                ..Default::default()
            };
            payload_holder[payload_index].add_to_write_op(&mut write_op);
            writes.push(write_op);
        }
    }
    unsafe {
        core.device.update_descriptor_sets(&writes, &[]);
    }

    let mut descriptor_datas = vec![];
    for (layout_index, (layout, quantity)) in layout_info.into_iter().enumerate() {
        let variants: Vec<_> = descriptor_sets.drain(0..quantity).collect();
        for (variant_index, variant) in variants.iter().enumerate() {
            core.set_debug_name(
                *variant,
                &format!("{}_ds_variant_{}", names[layout_index], variant_index),
            );
        }
        descriptor_datas.push(DescriptorData { layout, variants });
    }

    (descriptor_pool, descriptor_datas)
}

/// Usage:
/// create_descriptor_collection_struct!{
///     name: [Name of the created struct],
///     aux_data_type: [Data type passed as the second argument to generator functions.]
///     items: {
///         [name of item to be used in struct] = [name of generator function to create it from],
///         //e.g.:
///         world_data = generate_world_data_ds_protos,
///         ...
///     }
/// }
/// 
/// The result can be used like this:
/// let collection = CollectionStructName::create(reference_to_core, reference_to_aux_data);
/// collection.world_data.layout; // Layout of world data descriptor sets.
/// // The first descriptor set from the first prototype generated by generate_world_data_ds_protos
/// collection.world_data.variants[0]; 
/// collection.destroy(reference_to_core); // Cleans up descriptor pool and all descriptor layouts.
#[macro_export]
macro_rules! create_descriptor_collection_struct {
    {
        name: $struct_name:ident, 
        aux_data_type: $aux_data_type:ty,
        items: { 
            $($field_name:ident = $generator_name:ident),* 
            $(,)*
        } $(,)*
    } => { 
        struct $struct_name {
            core: std::rc::Rc<crate::render::core::Core>,
            pool: vk::DescriptorPool,
            $($field_name : crate::render::descriptors::DescriptorData),*
        }

        impl $struct_name {
            fn create(core: std::rc::Rc<crate::render::core::Core>, aux_data: &$aux_data_type) -> Self {
                let generators = [$(
                    Box::new($generator_name) 
                    as crate::render::descriptors::PrototypeGenerator<$aux_data_type>
                ),*];
                let names = [$(stringify!($field_name)),*];
                let (pool, datas) = crate::render::descriptors::generate_descriptor_pool(
                    &generators, &names, core.clone(), aux_data
                );
                let mut datas_consumer = datas.into_iter();
                $struct_name {
                    core,
                    pool,
                    $($field_name : datas_consumer.next().unwrap()),*
                }
            }
        }

        impl Drop for $struct_name {
            fn drop(&mut self) {
                unsafe {
                    $(self.core.device.destroy_descriptor_set_layout(
                        self.$field_name.layout, 
                        None
                    );)*
                    self.core.device.destroy_descriptor_pool(self.pool, None);
                }
            }
        }
    }
}
