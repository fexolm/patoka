use std::sync::Arc;

use ash::vk;
use vk_mem::{Alloc, Allocation, AllocationCreateInfo, MemoryUsage};

use crate::render::hal::vulkan::renderer::Renderer;

pub trait Image {
    unsafe fn get_image_view(&self) -> vk::ImageView;
    unsafe fn get_image(&self) -> vk::Image;
}
pub struct Framebuffer {
    pub(super) image_view: vk::ImageView,
    pub(super) image: vk::Image,
    pub(super) idx: u32,
}

pub struct Texture {
    pub(super) image: vk::Image,
    pub(super) image_view: vk::ImageView,
    pub(super) allocation: Allocation,
    pub(super) extent: vk::Extent3D,
    pub(super) format: vk::Format,
    renderer: Arc<Renderer>,
}

impl Texture {
    pub fn new(renderer: Arc<Renderer>, format: vk::Format, extent: vk::Extent3D, usage: vk::ImageUsageFlags, aspect_flags: vk::ImageAspectFlags) -> Self {
        let image_create_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .extent(extent)
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(usage);

        let allocation_info = AllocationCreateInfo {
            usage: MemoryUsage::AutoPreferDevice,
            required_flags: vk::MemoryPropertyFlags::DEVICE_LOCAL,
            ..Default::default()
        };

        let (image, allocation) = unsafe { renderer.allocator.create_image(&image_create_info, &allocation_info).unwrap() };

        let imageview_create_info = vk::ImageViewCreateInfo::default()
            .view_type(vk::ImageViewType::TYPE_2D)
            .image(image)
            .format(format)
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1)
                    .aspect_mask(aspect_flags)
            );

        let image_view = unsafe { renderer.device.create_image_view(&imageview_create_info, None).unwrap() };

        Texture { image, image_view, allocation, extent, format, renderer }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe { self.renderer.device.destroy_image_view(self.image_view, None); }
        unsafe { self.renderer.allocator.destroy_image(self.image, &mut self.allocation) };
    }
}
impl Image for Framebuffer {
    unsafe fn get_image_view(&self) -> vk::ImageView {
        self.image_view
    }
    unsafe fn get_image(&self) -> vk::Image {
        self.image
    }
}

