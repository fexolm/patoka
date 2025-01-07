extern crate patoka;

use std::sync::Arc;

use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

use patoka::render::hal::*;
use patoka::render::hal::RendererCreateInfo;
use patoka::render::hal::vulkan::command_list::CommandList;
use patoka::render::hal::vulkan::renderer::Renderer;
use patoka::render::hal::vulkan::sync::{Fence, Semaphore};

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let window = Arc::new(WindowBuilder::new()
        .with_title("Patoka Game")
        .with_inner_size(winit::dpi::LogicalSize::new(800f32, 600f32))
        .build(&event_loop).unwrap());

    let renderer = {
        let create_info = RendererCreateInfo {};
        Renderer::new(window, &create_info).unwrap()
    };

    let command_list = {
        let create_info = CommandListCreateInfo {};
        CommandList::new(renderer.clone(), &create_info)
    };

    let render_fence = {
        let create_info = FenceCreateInfo {};
        Fence::new(renderer.clone(), &create_info)
    };

    let swapchain_semaphore = {
        let create_info = SemaphoreCreateInfo {};
        Semaphore::new(renderer.clone(), &create_info)
    };

    let render_semaphore = {
        let create_info = SemaphoreCreateInfo {};
        Semaphore::new(renderer.clone(), &create_info)
    };

    let mut frame = 0;

    loop {
        render_fence.wait();
        render_fence.reset();

        let img = renderer.acquire_next_framebuffer(&swapchain_semaphore);

        command_list.reset();
        command_list.begin();

        command_list.flash_screen(&img, frame);

        command_list.end();

        renderer.submit(&command_list, &[&swapchain_semaphore], &[&render_semaphore], &render_fence);

        renderer.present(&render_semaphore, &img);
        frame += 1;
    }
}
