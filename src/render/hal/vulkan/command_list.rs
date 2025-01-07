use std::sync::Arc;

use ash::vk;
use ash::vk::Offset3D;

use crate::render::hal::CommandListCreateInfo;
use crate::render::hal::vulkan::FRAME_OVERLAP;
use crate::render::hal::vulkan::image::Texture;
use crate::render::hal::vulkan::renderer::Renderer;

pub struct CommandList {
    command_buffers: [vk::CommandBuffer; FRAME_OVERLAP],
    renderer: Arc<Renderer>,
}
impl CommandList {
    pub fn new(renderer: Arc<Renderer>, info: &CommandListCreateInfo) -> Self {
        let command_buffers = {
            let alloc_info = vk::CommandBufferAllocateInfo::default()
                .command_pool(renderer.command_pool)
                .command_buffer_count(FRAME_OVERLAP as u32)
                .level(vk::CommandBufferLevel::PRIMARY);

            unsafe { renderer.device.allocate_command_buffers(&alloc_info).unwrap().as_slice().try_into().unwrap() }
        };

        Self { command_buffers, renderer }
    }

    pub(crate) fn get_raw(&self) -> vk::CommandBuffer {
        let frame = self.renderer.current_frame();
        self.command_buffers[frame]
    }

    pub fn reset(&self) {
        let reset_flags = vk::CommandBufferResetFlags::default();
        unsafe { self.renderer.device.reset_command_buffer(self.get_raw(), reset_flags).unwrap() };
    }

    pub fn begin(&self) {
        let info = vk::CommandBufferBeginInfo::default();
        unsafe { self.renderer.device.begin_command_buffer(self.get_raw(), &info).unwrap(); }
    }

    pub fn end(&self) {
        unsafe { self.renderer.device.end_command_buffer(self.get_raw()).unwrap() };
    }

    fn subresource_range(aspect_mask: vk::ImageAspectFlags) -> vk::ImageSubresourceRange {
        vk::ImageSubresourceRange::default()
            .aspect_mask(aspect_mask)
            .base_mip_level(0)
            .level_count(vk::REMAINING_MIP_LEVELS)
            .base_array_layer(0)
            .layer_count(vk::REMAINING_ARRAY_LAYERS)
    }

    fn transition_image_layout(&self, image: vk::Image, old_layout: vk::ImageLayout, new_layout: vk::ImageLayout) {
        unsafe {
            let aspect_mask = if new_layout == vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL {
                vk::ImageAspectFlags::DEPTH
            } else {
                vk::ImageAspectFlags::COLOR
            };

            let image_barrier = vk::ImageMemoryBarrier2::default()
                .src_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                .src_access_mask(vk::AccessFlags2::MEMORY_WRITE)
                .dst_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                .dst_access_mask(vk::AccessFlags2::MEMORY_WRITE | vk::AccessFlags2::MEMORY_READ)
                .old_layout(old_layout)
                .new_layout(new_layout)
                .subresource_range(Self::subresource_range(aspect_mask))
                .image(image);

            let barriers = [image_barrier];

            let dependency_info = vk::DependencyInfo::default()
                .image_memory_barriers(&barriers);

            self.renderer.device.cmd_pipeline_barrier2(self.get_raw(), &dependency_info);
        }
    }

    fn copy_image_to_image(&self, source: vk::Image, dest: vk::Image, src_size: vk::Extent2D, dst_size: vk::Extent2D) {
        let blit_regions = [vk::ImageBlit2::default()
            .src_offsets([
                Offset3D::default(),
                Offset3D { x: src_size.width as i32, y: src_size.height as i32, z: 1 }
            ])
            .dst_offsets([
                Offset3D::default(),
                Offset3D { x: dst_size.width as i32, y: dst_size.height as i32, z: 1 }
            ])
            .src_subresource(vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_array_layer: 0,
                layer_count: 1,
                mip_level: 0,
            })
            .dst_subresource(vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_array_layer: 0,
                layer_count: 1,
                mip_level: 0,
            })];

        let blit_info = vk::BlitImageInfo2::default()
            .dst_image(dest)
            .dst_image_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .src_image(source)
            .src_image_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
            .filter(vk::Filter::LINEAR)
            .regions(&blit_regions);

        unsafe { self.renderer.device.cmd_blit_image2(self.get_raw(), &blit_info) }
    }

    pub fn flash_screen(&self, texture: &Texture, frame_num: usize) {
        self.transition_image_layout(texture.image, vk::ImageLayout::UNDEFINED, vk::ImageLayout::GENERAL);
        let flash = ((frame_num as f32) / 120f32).sin().abs();
        let clear_value = vk::ClearColorValue { float32: [0f32, 0f32, flash, 1f32] };
        unsafe {
            self.renderer.device.cmd_clear_color_image(self.get_raw(), texture.image, vk::ImageLayout::GENERAL,
                                                       &clear_value, &[Self::subresource_range(vk::ImageAspectFlags::COLOR)])
        };
    }

    pub fn copy_to_framebuffer(&self, texture: &Texture) {
        self.transition_image_layout(texture.image, vk::ImageLayout::GENERAL, vk::ImageLayout::TRANSFER_SRC_OPTIMAL);
        self.transition_image_layout(self.renderer.get_current_swapchain_img(), vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL);
        self.copy_image_to_image(texture.image, self.renderer.get_current_swapchain_img(), vk::Extent2D { width: 800, height: 600 }, vk::Extent2D { width: 800, height: 600 });
        self.transition_image_layout(self.renderer.get_current_swapchain_img(), vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::PRESENT_SRC_KHR);
    }
}
