use std::sync::Arc;

use ash::vk;

use crate::render::hal::CommandListCreateInfo;
use crate::render::hal::vulkan::FRAME_OVERLAP;
use crate::render::hal::vulkan::image::Image;
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

    fn transition_image_layout(&self, image: &dyn Image, old_layout: vk::ImageLayout, new_layout: vk::ImageLayout) {
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
                .image(image.get_image());

            let barriers = [image_barrier];

            let dependency_info = vk::DependencyInfo::default()
                .image_memory_barriers(&barriers);

            self.renderer.device.cmd_pipeline_barrier2(self.get_raw(), &dependency_info);
        }
    }

    pub fn flash_screen(&self, image: &dyn Image, frame_num: usize) {
        self.transition_image_layout(image, vk::ImageLayout::UNDEFINED, vk::ImageLayout::GENERAL);
        let flash = ((frame_num as f32) / 120f32).sin().abs();
        let clear_value = vk::ClearColorValue { float32: [0f32, 0f32, flash, 1f32] };
        unsafe {
            self.renderer.device.cmd_clear_color_image(self.get_raw(), image.get_image(), vk::ImageLayout::GENERAL,
                                                       &clear_value, &[Self::subresource_range(vk::ImageAspectFlags::COLOR)])
        };
        self.transition_image_layout(image, vk::ImageLayout::GENERAL, vk::ImageLayout::PRESENT_SRC_KHR);
    }
}
