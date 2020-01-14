use ash::version::DeviceV1_0;
use ash::vk;

use image::GenericImageView;

use std::marker::PhantomData;
use std::ops::{Index, IndexMut};
use std::rc::Rc;

use super::command_buffer::CommandBuffer;
use super::core::Core;
use super::descriptors::DescriptorPrototype;

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

pub struct StorageImage {
    core: Rc<Core>,
    pub image: vk::Image,
    pub image_view: vk::ImageView,
    pub extent: vk::Extent3D,
    memory: vk::DeviceMemory,
}

impl StorageImage {
    pub fn create(
        core: Rc<Core>,
        name: &str,
        typ: vk::ImageType,
        extent: vk::Extent3D,
        format: vk::Format,
        usage: vk::ImageUsageFlags,
    ) -> Self {
        Self::create_mipped(core, name, typ, extent, format, usage, 1)
    }

    pub fn create_mipped(
        core: Rc<Core>,
        name: &str,
        typ: vk::ImageType,
        extent: vk::Extent3D,
        format: vk::Format,
        usage: vk::ImageUsageFlags,
        mip_levels: u32,
    ) -> Self {
        let create_info = vk::ImageCreateInfo {
            image_type: typ,
            extent,
            format,
            samples: vk::SampleCountFlags::TYPE_1,
            mip_levels,
            array_layers: 1,
            usage,
            tiling: vk::ImageTiling::OPTIMAL,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };
        let image = unsafe {
            core.device
                .create_image(&create_info, None)
                .expect("Failed to create buffer.")
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
            view_type: match typ {
                vk::ImageType::TYPE_1D => vk::ImageViewType::TYPE_1D,
                vk::ImageType::TYPE_2D => vk::ImageViewType::TYPE_2D,
                vk::ImageType::TYPE_3D => vk::ImageViewType::TYPE_3D,
                _ => unreachable!("Encountered unspecified ImageType."),
            },
            format,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: mip_levels,
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

        Self {
            core,
            image,
            image_view,
            extent,
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

pub struct SampledImage {
    core: Rc<Core>,
    pub image: vk::Image,
    pub image_view: vk::ImageView,
    pub sampler: vk::Sampler,
    memory: vk::DeviceMemory,
    extent: vk::Extent3D,
}

impl SampledImage {
    pub fn create(
        core: Rc<Core>,
        name: &str,
        typ: vk::ImageType,
        extent: vk::Extent3D,
        format: vk::Format,
        usage: vk::ImageUsageFlags,
    ) -> Self {
        Self::create_mipped(core, name, typ, extent, format, usage, 1)
    }

    pub fn create_mipped(
        core: Rc<Core>,
        name: &str,
        typ: vk::ImageType,
        extent: vk::Extent3D,
        format: vk::Format,
        usage: vk::ImageUsageFlags,
        mip_levels: u32
    ) -> Self {
        let create_info = vk::ImageCreateInfo {
            image_type: typ,
            extent,
            format,
            samples: vk::SampleCountFlags::TYPE_1,
            mip_levels,
            array_layers: 1,
            usage,
            tiling: vk::ImageTiling::OPTIMAL,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };
        let image = unsafe {
            core.device
                .create_image(&create_info, None)
                .expect("Failed to create buffer.")
        };
        core.set_debug_name(image, &format!("{}", name));

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
            view_type: match typ {
                vk::ImageType::TYPE_1D => vk::ImageViewType::TYPE_1D,
                vk::ImageType::TYPE_2D => vk::ImageViewType::TYPE_2D,
                vk::ImageType::TYPE_3D => vk::ImageViewType::TYPE_3D,
                _ => unreachable!("Encountered unspecified ImageType."),
            },
            format,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: mip_levels,
                base_array_layer: 0,
                layer_count: 1,
            },
            ..Default::default()
        };
        let image_view = unsafe {
            core.device
                .create_image_view(&image_view_create_info, None)
                .expect("Failed to create image view for sampled image.")
        };
        core.set_debug_name(image_view, &format!("{}_view", name));

        let sampler_create_info = vk::SamplerCreateInfo {
            mag_filter: vk::Filter::NEAREST,
            min_filter: vk::Filter::NEAREST,
            // TODO: Allow customization.
            address_mode_u: vk::SamplerAddressMode::CLAMP_TO_BORDER,
            address_mode_v: vk::SamplerAddressMode::CLAMP_TO_BORDER,
            address_mode_w: vk::SamplerAddressMode::CLAMP_TO_BORDER,
            border_color: vk::BorderColor::INT_OPAQUE_WHITE,
            unnormalized_coordinates: if mip_levels == 1 { vk::TRUE } else { vk::FALSE },
            compare_enable: vk::FALSE,
            mipmap_mode: vk::SamplerMipmapMode::NEAREST,
            min_lod: 0.0,
            max_lod: if mip_levels == 1 { 0.0 } else { mip_levels as f32 },
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
            extent,
        }
    }

    pub fn create_dp(&self, layout: vk::ImageLayout) -> DescriptorPrototype {
        DescriptorPrototype::CombinedImageSampler(self.image_view, layout, self.sampler)
    }

    pub fn load_from_png(&mut self, bytes: &[u8]) {
        let size = self.extent.width * self.extent.height * self.extent.depth * 4;
        let data = image::load_from_memory_with_format(bytes, image::ImageFormat::PNG)
            .expect("Failed to decode PNG data.");
        let mut buffer = Buffer::<u8>::create(
            self.core.clone(),
            "texture_upload_buffer",
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

        let upload_commands = CommandBuffer::create_single(self.core.clone());
        upload_commands.set_debug_name("texture_upload_commands");
        upload_commands.begin_one_time_submit();
        upload_commands.transition_layout(
            self,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        );
        upload_commands.copy_buffer_to_image(&buffer, self, self);
        upload_commands.transition_layout(
            self,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );
        upload_commands.end();
        upload_commands.blocking_execute_and_destroy();
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

pub trait ImageWrapper {
    fn get_vk_image(&self) -> vk::Image;
}

impl ImageWrapper for StorageImage {
    fn get_vk_image(&self) -> vk::Image {
        self.image
    }
}

impl ImageWrapper for SampledImage {
    fn get_vk_image(&self) -> vk::Image {
        self.image
    }
}

impl ImageWrapper for vk::Image {
    fn get_vk_image(&self) -> vk::Image {
        *self
    }
}

pub trait ImageViewWrapper {
    fn get_vk_image_view(&self) -> vk::ImageView;
}

impl ImageViewWrapper for StorageImage {
    fn get_vk_image_view(&self) -> vk::ImageView {
        self.image_view
    }
}

impl ImageViewWrapper for SampledImage {
    fn get_vk_image_view(&self) -> vk::ImageView {
        self.image_view
    }
}

impl ImageViewWrapper for vk::ImageView {
    fn get_vk_image_view(&self) -> vk::ImageView {
        *self
    }
}

pub trait BufferWrapper {
    fn get_vk_buffer(&self) -> vk::Buffer;
}

impl<ItemType> BufferWrapper for Buffer<ItemType> {
    fn get_vk_buffer(&self) -> vk::Buffer {
        self.buffer
    }
}

impl BufferWrapper for vk::Buffer {
    fn get_vk_buffer(&self) -> vk::Buffer {
        *self
    }
}

pub trait SamplerWrapper {
    fn get_vk_sampler(&self) -> vk::Sampler;
}

impl SamplerWrapper for SampledImage {
    fn get_vk_sampler(&self) -> vk::Sampler {
        self.sampler
    }
}

impl SamplerWrapper for vk::Sampler {
    fn get_vk_sampler(&self) -> vk::Sampler {
        *self
    }
}

pub trait ExtentWrapper {
    fn get_vk_extent(&self) -> vk::Extent3D;
}

impl ExtentWrapper for StorageImage {
    fn get_vk_extent(&self) -> vk::Extent3D {
        self.extent
    }
}

impl ExtentWrapper for SampledImage {
    fn get_vk_extent(&self) -> vk::Extent3D {
        self.extent
    }
}

impl ExtentWrapper for vk::Extent3D {
    fn get_vk_extent(&self) -> vk::Extent3D {
        *self
    }
}
