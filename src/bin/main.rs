extern crate patoka;

use std::sync::Arc;

use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

use patoka::render::hal::*;
use patoka::render::hal::RendererCreateInfo;
use patoka::render::hal::vulkan::renderer::VulkanRenderer;

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let window = Arc::new(WindowBuilder::new()
        .with_title("Patoka Game")
        .with_inner_size(winit::dpi::LogicalSize::new(800f32, 600f32))
        .build(&event_loop).unwrap());

    let renderer = {
        let create_info = RendererCreateInfo {};
        VulkanRenderer::new(window, &create_info).unwrap()
    };

    let command_list = {
        let create_info = CommandListCreateInfo {};
        renderer.create_command_list(&create_info)
    };
}
