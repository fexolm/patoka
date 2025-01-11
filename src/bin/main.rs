extern crate patoka;

use std::sync::Arc;

use ash::vk;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

use patoka::include_bytes_align_as;
use patoka::render::hal::*;
use patoka::render::hal::RendererCreateInfo;
use patoka::render::hal::vulkan::command_list::CommandList;
use patoka::render::hal::vulkan::descriptor_set::{DescriptorSet, DescriptorSetLayout};
use patoka::render::hal::vulkan::image::Texture;
use patoka::render::hal::vulkan::pipeline::{ComputePipeline, PipelineLayout};
use patoka::render::hal::vulkan::renderer::Renderer;
use patoka::render::hal::vulkan::shader::Shader;
use patoka::render::hal::vulkan::sync::{Fence, Semaphore};

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let window = Arc::new(WindowBuilder::new()
        .with_title("Patoka Game")
        .with_inner_size(winit::dpi::LogicalSize::new(800f32, 600f32))
        .build(&event_loop).unwrap());

    let renderer = {
        let create_info = RendererCreateInfo {};
        Renderer::new(window, create_info).unwrap()
    };

    let mut command_list = {
        let create_info = CommandListCreateInfo {};
        CommandList::new(renderer.clone(), create_info)
    };

    let render_fence = {
        Fence::new(renderer.clone())
    };

    let swapchain_semaphore = {
        Semaphore::new(renderer.clone())
    };

    let render_semaphore = {
        Semaphore::new(renderer.clone())
    };

    let texture = {
        let extent = vk::Extent3D { width: 800, height: 600, depth: 1 };
        let usage = vk::ImageUsageFlags::TRANSFER_SRC
            | vk::ImageUsageFlags::TRANSFER_DST
            | vk::ImageUsageFlags::STORAGE
            | vk::ImageUsageFlags::COLOR_ATTACHMENT;
        Texture::new(renderer.clone(), vk::Format::R16G16B16A16_SFLOAT, extent, usage, vk::ImageAspectFlags::COLOR)
    };

    let draw_image_descriptor_layout = {
        let create_info = DescriptorSetLayoutCreateInfo {
            bindings: vec![DescriptorSetBinding {
                stage: ShaderStages::Compute,
                typ: BindingType::Texture,
                binding: 0,
            }],
        };
        DescriptorSetLayout::new(renderer.clone(), create_info)
    };

    let descriptor_set = DescriptorSet::new(renderer.clone(), draw_image_descriptor_layout.clone());

    let shader_code: &'static [u32] = include_bytes_align_as!(u32, "shaders/gradient.spv");
    let shader = {
        let create_info = ShaderCreateInfo {
            code: shader_code,
        };
        Shader::new(renderer.clone(), create_info)
    };

    let pipeline_layout = {
        let create_info = PipelineLayoutCreateInfo {
            sets: vec![draw_image_descriptor_layout.clone()],
        };

        PipelineLayout::new(renderer.clone(), create_info)
    };

    let pipeline = {
        let create_info = ComputePipelineCreateInfo {
            shader: shader.clone(),
            pipeline_layout: pipeline_layout.clone(),
            entrypoint: c"main",
        };

        ComputePipeline::new(renderer.clone(), create_info)
    };

    loop {
        render_fence.wait();
        render_fence.reset();

        renderer.start_frame(&swapchain_semaphore);

        descriptor_set.write_texture(0, &texture);

        command_list.reset();
        command_list.begin();
        command_list.transition_texture_layout(&texture, vk::ImageLayout::UNDEFINED, vk::ImageLayout::GENERAL);
        command_list.bind_compute_pipeline(pipeline.clone());
        command_list.bind_descriptor_set(pipeline_layout.clone(), descriptor_set.clone());
        command_list.dispatch_compute_pipeline(800 / 16, 600 / 16, 1);

        command_list.copy_to_framebuffer(&texture);

        command_list.end();

        renderer.submit(&command_list, &[&swapchain_semaphore], &[&render_semaphore], &render_fence);

        renderer.present(&render_semaphore);
    }
}
