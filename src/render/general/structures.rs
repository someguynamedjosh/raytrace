use ash::version::DeviceV1_0;
use ash::vk;
use image::GenericImageView;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};
use std::rc::Rc;

use super::command_buffer::CommandBuffer;
use super::core::Core;
use super::descriptors::DescriptorPrototype;

pub trait BufferWrapper {
    fn get_vk_buffer(&self) -> vk::Buffer;
}

impl BufferWrapper for vk::Buffer {
    fn get_vk_buffer(&self) -> vk::Buffer {
        *self
    }
}

pub trait ImageWrapper {
    fn get_vk_image(&self) -> vk::Image;
}

impl ImageWrapper for vk::Image {
    fn get_vk_image(&self) -> vk::Image {
        *self
    }
}

pub trait ImageViewWrapper {
    fn get_vk_image_view(&self) -> vk::ImageView;
}

impl ImageViewWrapper for vk::ImageView {
    fn get_vk_image_view(&self) -> vk::ImageView {
        *self
    }
}

pub trait SamplerWrapper {
    fn get_vk_sampler(&self) -> vk::Sampler;
}

impl SamplerWrapper for vk::Sampler {
    fn get_vk_sampler(&self) -> vk::Sampler {
        *self
    }
}

pub trait ExtentWrapper {
    fn get_vk_extent(&self) -> vk::Extent3D;
}

impl ExtentWrapper for vk::Extent3D {
    fn get_vk_extent(&self) -> vk::Extent3D {
        *self
    }
}

pub trait CoreReferenceWrapper {
    fn get_core(&self) -> Rc<Core>;
}

macro_rules! wrap_field {
    {$struct_name:ty, [$($type_parameter:ident),*], buffer} => {
        impl<$($type_parameter),*> BufferWrapper for $struct_name {
            fn get_vk_buffer(&self) -> vk::Buffer { self.buffer }
        }
    };
    {$struct_name:ty, [$($type_parameter:ident),*], image} => {
        impl<$($type_parameter),*> ImageWrapper for $struct_name {
            fn get_vk_image(&self) -> vk::Image { self.image }
        }
    };
    {$struct_name:ty, [$($type_parameter:ident),*], image_view} => {
        impl<$($type_parameter),*> ImageViewWrapper for $struct_name {
            fn get_vk_image_view(&self) -> vk::ImageView { self.image_view }
        }
    };
    {$struct_name:ty, [$($type_parameter:ident),*], sampler} => {
        impl<$($type_parameter),*> SamplerWrapper for $struct_name {
            fn get_vk_sampler(&self) -> vk::Sampler { self.sampler }
        }
    };
    {$struct_name:ty, [$($type_parameter:ident),*], extent } => {
        impl<$($type_parameter),*> ExtentWrapper for $struct_name {
            fn get_vk_extent(&self) -> vk::Extent3D { self.extent }
        }
    };
    {$struct_name:ty, [$($type_parameter:ident),*], core} => {
        impl<$($type_parameter),*> CoreReferenceWrapper for $struct_name {
            fn get_core(&self) -> Rc<Core> { self.core.clone() }
        }
    };
}

macro_rules! derive_wrappers {
    ($struct_name:ty, [$($item_name:ident),* $(,)*]) => {
        $(
            wrap_field!($struct_name, [], $item_name);
        )*
    };
    ($struct_name:ty, [$template_param_1:ident], [$($item_name:ident),* $(,)*]) => {
        $(
            wrap_field!($struct_name, [$template_param_1], $item_name);
        )*
    }
}

pub struct BufferView<'a, ItemType> {
    source_buffer: &'a Buffer<ItemType>,
    ptr: &'a mut [ItemType],
}

impl<'a, ItemType> BufferView<'a, ItemType> {
    pub fn iter(&self) -> std::slice::Iter<ItemType> {
        self.ptr.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<ItemType> {
        self.ptr.iter_mut()
    }

    pub fn as_slice_mut(&mut self) -> &mut [ItemType] {
        self.ptr
    }
}

impl<'a, ItemType> Index<usize> for BufferView<'a, ItemType> {
    type Output = ItemType;

    fn index(&self, index: usize) -> &Self::Output {
        &self.ptr[index]
    }
}

impl<'a, ItemType> IndexMut<usize> for BufferView<'a, ItemType> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.ptr[index]
    }
}

impl<'a, ItemType> Drop for BufferView<'a, ItemType> {
    fn drop(&mut self) {
        unsafe {
            self.source_buffer.unbind();
        }
    }
}

pub struct Buffer<ItemType> {
    core: Rc<Core>,
    pub buffer: vk::Buffer,
    memory: vk::DeviceMemory,
    size: u64,
    num_items: u64,
    content: PhantomData<ItemType>,
}
derive_wrappers!(Buffer<ItemType>, [ItemType], [core, buffer]);

impl<ItemType> Buffer<ItemType> {
    pub fn create(core: Rc<Core>, name: &str, num_items: u64, usage: vk::BufferUsageFlags) -> Self {
        let size = num_items * std::mem::size_of::<ItemType>() as u64;
        let create_info = vk::BufferCreateInfo {
            size,
            usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };
        let buffer = unsafe {
            core.device
                .create_buffer(&create_info, None)
                .expect("Failed to create buffer.")
        };
        core.set_debug_name(buffer, name);

        let memory_requirements = unsafe { core.device.get_buffer_memory_requirements(buffer) };
        let memory_allocation_info = vk::MemoryAllocateInfo {
            allocation_size: memory_requirements.size,
            memory_type_index: core.find_compatible_memory_type(
                memory_requirements.memory_type_bits,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            ),
            ..Default::default()
        };
        let memory = unsafe {
            core.device
                .allocate_memory(&memory_allocation_info, None)
                .expect("Failed to allocate memory for buffer.")
        };
        unsafe {
            core.device
                .bind_buffer_memory(buffer, memory, 0)
                .expect("Failed to bind buffer to device memory.");
        }
        core.set_debug_name(memory, &format!("{}_memory", name));

        Self {
            core,
            buffer,
            memory,
            size,
            num_items,
            content: PhantomData,
        }
    }

    pub fn create_dp(&self) -> DescriptorPrototype {
        DescriptorPrototype::UniformBuffer(self.buffer, 0, self.size)
    }

    pub fn bind_all(&mut self) -> BufferView<ItemType> {
        let slice = unsafe {
            let ptr = self
                .core
                .device
                .map_memory(self.memory, 0, self.size, Default::default())
                .expect("Failed to bind memory.") as *mut ItemType;
            std::slice::from_raw_parts_mut(ptr, self.num_items as usize)
        };
        BufferView {
            source_buffer: self,
            ptr: slice,
        }
    }

    unsafe fn unbind(&self) {
        self.core.device.unmap_memory(self.memory)
    }

    pub fn fill(&mut self, value: &ItemType)
    where
        ItemType: Clone,
    {
        let mut range = self.bind_all();
        for item in range.iter_mut() {
            *item = value.clone();
        }
    }
}

impl<ItemType> Drop for Buffer<ItemType> {
    fn drop(&mut self) {
        unsafe {
            self.core.device.destroy_buffer(self.buffer, None);
            self.core.device.free_memory(self.memory, None);
        }
    }
}

pub struct ImageOptions {
    pub typ: vk::ImageType,
    pub extent: vk::Extent3D,
    pub format: vk::Format,
    pub usage: vk::ImageUsageFlags,
    pub mip_levels: u32,
}

impl Default for ImageOptions {
    fn default() -> Self {
        Self {
            typ: Default::default(),
            extent: Default::default(),
            format: Default::default(),
            usage: Default::default(),
            mip_levels: 1,
        }
    }
}

fn create_image_resources(
    core: Rc<Core>,
    name: &str,
    options: &ImageOptions,
) -> (vk::Image, vk::ImageView, vk::DeviceMemory) {
    let create_info = vk::ImageCreateInfo {
        image_type: options.typ,
        extent: options.extent,
        format: options.format,
        samples: vk::SampleCountFlags::TYPE_1,
        mip_levels: options.mip_levels,
        array_layers: 1,
        usage: options.usage,
        tiling: vk::ImageTiling::OPTIMAL,
        sharing_mode: vk::SharingMode::EXCLUSIVE,
        ..Default::default()
    };
    let image = unsafe {
        core.device
            .create_image(&create_info, None)
            .expect("Failed to create image.")
    };
    core.set_debug_name(image, name);

    let memory_requirements = unsafe { core.device.get_image_memory_requirements(image) };
    let memory_allocation_info = vk::MemoryAllocateInfo {
        allocation_size: memory_requirements.size,
        memory_type_index: core.find_compatible_memory_type(
            memory_requirements.memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        ),
        ..Default::default()
    };
    let memory = unsafe {
        core.device
            .allocate_memory(&memory_allocation_info, None)
            .expect("Failed to allocate memory for image.")
    };
    core.set_debug_name(memory, &format!("{}_memory", name));
    unsafe {
        core.device
            .bind_image_memory(image, memory, 0)
            .expect("Failed to bind image to device memory.");
    }

    let image_view_create_info = vk::ImageViewCreateInfo {
        image,
        view_type: match options.typ {
            vk::ImageType::TYPE_1D => vk::ImageViewType::TYPE_1D,
            vk::ImageType::TYPE_2D => vk::ImageViewType::TYPE_2D,
            vk::ImageType::TYPE_3D => vk::ImageViewType::TYPE_3D,
            _ => unreachable!("Encountered invalid ImageType."),
        },
        format: options.format,
        subresource_range: vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: options.mip_levels,
            base_array_layer: 0,
            layer_count: 1,
        },
        ..Default::default()
    };
    let image_view = unsafe {
        core.device
            .create_image_view(&image_view_create_info, None)
            .expect("Failed to create image view for storage image.")
    };
    core.set_debug_name(image_view, &format!("{}_view", name));

    (image, image_view, memory)
}

pub struct StorageImage {
    core: Rc<Core>,
    pub image: vk::Image,
    pub image_view: vk::ImageView,
    pub extent: vk::Extent3D,
    memory: vk::DeviceMemory,
}
derive_wrappers!(StorageImage, [core, image, image_view, extent]);

impl StorageImage {
    pub fn create(core: Rc<Core>, name: &str, options: &ImageOptions) -> Self {
        let (image, image_view, memory) = create_image_resources(core.clone(), name, options);
        Self {
            core,
            image,
            image_view,
            extent: options.extent,
            memory,
        }
    }

    pub fn create_dp(&self, layout: vk::ImageLayout) -> DescriptorPrototype {
        DescriptorPrototype::StorageImage(self.image_view, layout)
    }
}

impl Drop for StorageImage {
    fn drop(&mut self) {
        unsafe {
            self.core.device.destroy_image(self.image, None);
            self.core.device.destroy_image_view(self.image_view, None);
            self.core.device.free_memory(self.memory, None);
        }
    }
}

pub struct SamplerOptions {
    pub min_filter: vk::Filter,
    pub mag_filter: vk::Filter,
    pub address_mode: vk::SamplerAddressMode,
    pub border_color: vk::BorderColor,
    pub unnormalized_coordinates: bool,
    pub mipmap_mode: vk::SamplerMipmapMode,
}

impl Default for SamplerOptions {
    fn default() -> Self {
        Self {
            min_filter: vk::Filter::NEAREST,
            mag_filter: vk::Filter::NEAREST,
            address_mode: vk::SamplerAddressMode::CLAMP_TO_EDGE,
            border_color: vk::BorderColor::FLOAT_OPAQUE_BLACK,
            unnormalized_coordinates: false,
            mipmap_mode: vk::SamplerMipmapMode::NEAREST,
        }
    }
}

pub struct SampledImage {
    core: Rc<Core>,
    pub image: vk::Image,
    pub image_view: vk::ImageView,
    pub sampler: vk::Sampler,
    pub extent: vk::Extent3D,
    memory: vk::DeviceMemory,
}
derive_wrappers!(SampledImage, [core, image, image_view, sampler, extent]);

impl SampledImage {
    pub fn create(
        core: Rc<Core>,
        name: &str,
        image_options: &ImageOptions,
        sampler_options: &SamplerOptions,
    ) -> Self {
        let (image, image_view, memory) = create_image_resources(core.clone(), name, image_options);

        let sampler_create_info = vk::SamplerCreateInfo {
            mag_filter: sampler_options.min_filter,
            min_filter: sampler_options.mag_filter,
            // TODO: Allow customization.
            address_mode_u: sampler_options.address_mode,
            address_mode_v: sampler_options.address_mode,
            address_mode_w: sampler_options.address_mode,
            border_color: sampler_options.border_color,
            unnormalized_coordinates: if sampler_options.unnormalized_coordinates {
                vk::TRUE
            } else {
                vk::FALSE
            },
            compare_enable: vk::FALSE,
            mipmap_mode: sampler_options.mipmap_mode,
            min_lod: 0.0,
            max_lod: if image_options.mip_levels == 1 {
                0.0
            } else {
                image_options.mip_levels as f32
            },
            ..Default::default()
        };
        let sampler = unsafe {
            core.device
                .create_sampler(&sampler_create_info, None)
                .expect("Failed to create sampler for sampled image.")
        };
        core.set_debug_name(sampler, &format!("{}_sampler", name));

        Self {
            core,
            image,
            image_view,
            sampler,
            memory,
            extent: image_options.extent,
        }
    }

    pub fn create_dp(&self, layout: vk::ImageLayout) -> DescriptorPrototype {
        DescriptorPrototype::CombinedImageSampler(self.image_view, layout, self.sampler)
    }
}

impl Drop for SampledImage {
    fn drop(&mut self) {
        unsafe {
            self.core.device.destroy_sampler(self.sampler, None);
            self.core.device.destroy_image_view(self.image_view, None);
            self.core.device.destroy_image(self.image, None);
            self.core.device.free_memory(self.memory, None);
        }
    }
}

pub trait DataDestination: CoreReferenceWrapper {
    fn load_from_slice<ElementType: Copy>(&self, data: &[ElementType]) {
        let mut buffer = Buffer::create(
            self.get_core(),
            "slice_upload_buffer",
            data.len() as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
        );
        let mut buffer_content = buffer.bind_all();
        for index in 0..data.len() {
            buffer_content[index] = data[index];
        }
        drop(buffer_content);
        self.load_from_buffer(&buffer);
        drop(buffer);
    }

    fn load_from_png_rgba8(&self, data: &[u8]) {
        let data = image::load_from_memory_with_format(data, image::ImageFormat::PNG)
            .expect("Failed to decode PNG data.");
        let size = data.width() * data.height() * 4;
        let mut buffer = Buffer::<u8>::create(
            self.get_core(),
            "png_upload_buffer",
            size as u64,
            vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST,
        );
        let mut buffer_content = buffer.bind_all();
        for (index, pixel) in data.pixels().enumerate() {
            // RGBA
            buffer_content[index as usize * 4 + 0] = (pixel.2).0[0];
            buffer_content[index as usize * 4 + 1] = (pixel.2).0[1];
            buffer_content[index as usize * 4 + 2] = (pixel.2).0[2];
            buffer_content[index as usize * 4 + 3] = (pixel.2).0[3];
        }
        drop(buffer_content);
        self.load_from_buffer(&buffer);
        drop(buffer);
    }

    fn load_from_buffer(&self, buffer: &impl BufferWrapper);
}

impl<GenericType> DataDestination for GenericType
where
    GenericType: ImageWrapper + ExtentWrapper + CoreReferenceWrapper,
{
    fn load_from_buffer(&self, buffer: &impl BufferWrapper) {
        let load_commands = CommandBuffer::create_single(self.get_core());
        load_commands.set_debug_name("load_commands");
        load_commands.begin_one_time_submit();
        load_commands.transition_layout(
            self,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        );
        load_commands.copy_buffer_to_image(buffer, self, self);
        load_commands.end();
        load_commands.blocking_execute_and_destroy();
    }
}
