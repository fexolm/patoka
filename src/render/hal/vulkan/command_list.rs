use ash::vk;

use crate::render::hal::{CommandList, CommandListCreateInfo};
use crate::render::hal::vulkan::FRAME_OVERLAP;
use crate::render::hal::vulkan::renderer::VulkanRenderer;

pub struct VulkanCommandList<'r, 'w> {
    command_buffers: Vec<vk::CommandBuffer>,
    renderer: &'r VulkanRenderer<'w>,
}
impl<'r, 'w> VulkanCommandList<'r, 'w> {
    pub(crate) fn new(renderer: &'r VulkanRenderer<'w>, info: &CommandListCreateInfo) -> Box<Self> {
        let command_buffers = {
            let alloc_info = vk::CommandBufferAllocateInfo::default()
                .command_pool(renderer.command_pool)
                .command_buffer_count(FRAME_OVERLAP as u32)
                .level(vk::CommandBufferLevel::PRIMARY);

            unsafe { renderer.device.allocate_command_buffers(&alloc_info).unwrap() }
        };

        Box::new(Self { command_buffers, renderer })
    }
}

impl<'r, 'w> Drop for VulkanCommandList<'r, 'w> {
    fn drop(&mut self) {}
}

impl<'r, 'w> CommandList for VulkanCommandList<'r, 'w> {}