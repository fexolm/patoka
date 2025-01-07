use std::sync::Arc;

use ash::vk;

use crate::render::hal::CommandListCreateInfo;
use crate::render::hal::vulkan::FRAME_OVERLAP;
use crate::render::hal::vulkan::renderer::VulkanRenderer;

pub struct VulkanCommandList {
    command_buffers: Vec<vk::CommandBuffer>,
    renderer: Arc<VulkanRenderer>,
}
impl<'r, 'w> VulkanCommandList {
    pub(crate) fn new(renderer: Arc<VulkanRenderer>, info: &CommandListCreateInfo) -> Self {
        let command_buffers = {
            let alloc_info = vk::CommandBufferAllocateInfo::default()
                .command_pool(renderer.command_pool)
                .command_buffer_count(FRAME_OVERLAP as u32)
                .level(vk::CommandBufferLevel::PRIMARY);

            unsafe { renderer.device.allocate_command_buffers(&alloc_info).unwrap() }
        };

        Self { command_buffers, renderer }
    }
}
