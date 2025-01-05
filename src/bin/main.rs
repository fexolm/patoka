extern crate patoka;

use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

use patoka::render::hal::*;
use patoka::render::hal::{Renderer, RendererCreateInfo};
use patoka::render::hal::vulkan::renderer::VulkanRenderer;

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("Patoka Game")
        .with_inner_size(winit::dpi::LogicalSize::new(800f32, 600f32))
        .build(&event_loop).unwrap();

    let create_info = RendererCreateInfo {};
    let renderer = VulkanRenderer::new(&window, &create_info);
}
