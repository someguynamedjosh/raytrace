use ash::version::DeviceV1_0;
use ash::vk;

use image::GenericImageView;

use std::marker::PhantomData;

use super::commands as cmd;
use super::core::Core;
use super::descriptors::DescriptorPrototype;

pub struct Buffer {
    pub buffer: vk::Buffer,
    memory: vk::DeviceMemory,
    size: u64,
}

impl Buffer {
    pub fn create(core: &Core, name: &str, size: u64, usage: vk::BufferUsageFlags) -> Buffer {
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

        Buffer {
            buffer,
            memory,
            size,
        }
    }

    pub unsafe fn bind_all<PtrType>(&mut self, core: &Core) -> *mut PtrType {
        core.device
            .map_memory(self.memory, 0, self.size, Default::default())
            .expect("Failed to bind memory.") as *mut PtrType
    }

    pub unsafe fn unbind(&mut self, core: &Core) {
        core.device.unmap_memory(self.memory)
    }

    pub fn destroy(&mut self, core: &Core) {
        unsafe {
            core.device.destroy_buffer(self.buffer, None);
            core.device.free_memory(self.memory, None);
        }
    }
}

pub struct ObjectBuffer<ObjectType> {
    pub buffer: vk::Buffer,
    memory: vk::DeviceMemory,
    size: u64,
    buffer_data: PhantomData<ObjectType>,
}

impl<ObjectType> ObjectBuffer<ObjectType> {
    pub fn create(
        core: &Core,
        name: &str,
        usage: vk::BufferUsageFlags,
    ) -> Self {
        let size = std::mem::size_of::<ObjectType>() as u64;
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
            buffer,
            memory,
            size,
            buffer_data: PhantomData,
        }
    }

    pub fn create_dp(&self) -> DescriptorPrototype {
        DescriptorPrototype::UniformBuffer(self.buffer, 0, self.size)
    }

    pub unsafe fn bind_all(&mut self, core: &Core) -> *mut ObjectType {
        core.device
            .map_memory(self.memory, 0, self.size, Default::default())
            .expect("Failed to bind memory.") as *mut ObjectType
    }

    pub unsafe fn unbind(&mut self, core: &Core) {
        core.device.unmap_memory(self.memory)
    }

    pub fn destroy(&mut self, core: &Core) {
        unsafe {
            core.device.destroy_buffer(self.buffer, None);
            core.device.free_memory(self.memory, None);
        }
    }
}

pub struct Image {
    pub image: vk::Image,
    pub image_view: vk::ImageView,
    memory: vk::DeviceMemory,
}

impl Image {
    pub fn create(
        core: &Core,
        name: &str,
        typ: vk::ImageType,
        extent: vk::Extent3D,
        format: vk::Format,
    ) -> Image {
        let create_info = vk::ImageCreateInfo {
            image_type: typ,
            extent,
            format,
            samples: vk::SampleCountFlags::TYPE_1,
            mip_levels: 1,
            array_layers: 1,
            // TODO: Better usage.
            usage: vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::STORAGE,
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
                level_count: 1,
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

        Image {
            image,
            image_view,
            memory,
        }
    }

    pub fn create_dp(&self, layout: vk::ImageLayout) -> DescriptorPrototype {
        DescriptorPrototype::StorageImage(self.image_view, layout)
    }

    pub fn destroy(&mut self, core: &Core) {
        unsafe {
            core.device.destroy_image(self.image, None);
            core.device.destroy_image_view(self.image_view, None);
            core.device.free_memory(self.memory, None);
        }
    }
}

pub struct SampledImage {
    pub image: vk::Image,
    pub image_view: vk::ImageView,
    pub sampler: vk::Sampler,
    memory: vk::DeviceMemory,
    extent: vk::Extent3D,
}

impl SampledImage {
    pub fn create(
        core: &Core,
        name: &str,
        typ: vk::ImageType,
        extent: vk::Extent3D,
        format: vk::Format,
    ) -> SampledImage {
        let create_info = vk::ImageCreateInfo {
            image_type: typ,
            extent,
            format,
            samples: vk::SampleCountFlags::TYPE_1,
            mip_levels: 1,
            array_layers: 1,
            // TODO: Better usage.
            usage: vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
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
                level_count: 1,
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
            address_mode_u: vk::SamplerAddressMode::CLAMP_TO_BORDER,
            address_mode_v: vk::SamplerAddressMode::CLAMP_TO_BORDER,
            address_mode_w: vk::SamplerAddressMode::CLAMP_TO_BORDER,
            border_color: vk::BorderColor::FLOAT_OPAQUE_BLACK,
            unnormalized_coordinates: vk::TRUE, // Make coords in the range 0-(width) instead of 0-1
            compare_enable: vk::FALSE,
            ..Default::default()
        };
        let sampler = unsafe {
            core.device
                .create_sampler(&sampler_create_info, None)
                .expect("Failed to create sampler for sampled image.")
        };
        core.set_debug_name(sampler, &format!("{}_sampler", name));

        SampledImage {
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

    pub fn load_from_png(&mut self, core: &Core, bytes: &[u8]) {
        let size = self.extent.width * self.extent.height * self.extent.depth * 4;
        let data = image::load_from_memory_with_format(bytes, image::ImageFormat::PNG)
            .expect("Failed to decode PNG data.");
        let mut buffer = Buffer::create(
            core,
            "texture_upload_buffer",
            size as u64,
            vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST,
        );
        unsafe {
            let buffer_ptr = buffer.bind_all::<u8>(core);
            for (index, pixel) in data.pixels().enumerate() {
                // RGBA
                *buffer_ptr.offset(index as isize * 4 + 0) = (pixel.2).0[0];
                *buffer_ptr.offset(index as isize * 4 + 1) = (pixel.2).0[1];
                *buffer_ptr.offset(index as isize * 4 + 2) = (pixel.2).0[2];
                *buffer_ptr.offset(index as isize * 4 + 3) = (pixel.2).0[3];
            }
            buffer.unbind(core);
        }
        let upload_commands = cmd::create_buffer(core, "texture_upload_queue");
        cmd::begin_one_time_submit(core, upload_commands);
        cmd::transition_layout(
            core,
            upload_commands,
            self.image,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        );
        cmd::copy_buffer_to_image(
            core,
            upload_commands,
            buffer.buffer,
            self.image,
            self.extent,
        );
        cmd::transition_layout(
            core,
            upload_commands,
            self.image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );
        cmd::end(core, upload_commands);
        cmd::execute_and_destroy(core, upload_commands);
        buffer.destroy(core);
    }

    pub fn destroy(&mut self, core: &Core) {
        unsafe {
            core.device.destroy_sampler(self.sampler, None);
            core.device.destroy_image_view(self.image_view, None);
            core.device.destroy_image(self.image, None);
            core.device.free_memory(self.memory, None);
        }
    }
}
