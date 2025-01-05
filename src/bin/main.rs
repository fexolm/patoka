extern crate patoka;

use patoka::render::hal::*;
use patoka::render::hal::{Renderer, RendererCreateInfo};
use patoka::render::hal::vulkan::renderer::VulkanRenderer;

fn main() {
    let create_info = RendererCreateInfo {
        title: "Sample app".to_string(),
        window_size: (800, 600),
    };
    let renderer = VulkanRenderer::new(&create_info);
}
