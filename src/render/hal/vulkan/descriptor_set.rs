use std::ptr;
use std::sync::Arc;

use ash::vk;

use crate::render::hal::{BindingType, DescriptorSetLayoutCreateInfo, ShaderStages};
use crate::render::hal::vulkan::renderer::Renderer;

pub struct DescriptorSetLayout {
    pub(crate) layout: vk::DescriptorSetLayout,

    renderer: Arc<Renderer>,
}

fn convert_binding_type(binding: &BindingType) -> vk::DescriptorType {
    match binding {
        BindingType::UniformBuffer => vk::DescriptorType::UNIFORM_BUFFER,
        BindingType::StagingBuffer => vk::DescriptorType::STORAGE_BUFFER,
        BindingType::Texture => vk::DescriptorType::SAMPLED_IMAGE,
        BindingType::Sampler => vk::DescriptorType::SAMPLER,
    }
}

fn convert_shader_stage(stage: ShaderStages) -> vk::ShaderStageFlags {
    let mut flags = vk::ShaderStageFlags::empty();
    if stage.contains(ShaderStages::Vertex) {
        flags |= vk::ShaderStageFlags::VERTEX;
    }
    if stage.contains(ShaderStages::Fragment) {
        flags |= vk::ShaderStageFlags::FRAGMENT;
    }
    if stage.contains(ShaderStages::Compute) {
        flags |= vk::ShaderStageFlags::COMPUTE;
    }
    flags
}

impl DescriptorSetLayout {
    pub fn new(renderer: Arc<Renderer>, create_info: &DescriptorSetLayoutCreateInfo) -> Self {
        let bindings = create_info.bindings.iter().map(|b| {
            vk::DescriptorSetLayoutBinding {
                binding: b.binding,
                descriptor_type: convert_binding_type(&b.typ),
                descriptor_count: 1,
                stage_flags: convert_shader_stage(b.stage),
                p_immutable_samplers: ptr::null(),
                _marker: Default::default(),
            }
        }).collect::<Vec<_>>();

        let flags = vk::DescriptorSetLayoutCreateFlags::default();

        let layout_create_info = vk::DescriptorSetLayoutCreateInfo::default()
            .bindings(&bindings)
            .flags(flags);

        let layout = unsafe { renderer.device.create_descriptor_set_layout(&layout_create_info, None).unwrap() };

        DescriptorSetLayout { layout, renderer }
    }
}

impl Drop for DescriptorSetLayout {
    fn drop(&mut self) {
        unsafe { self.renderer.device.destroy_descriptor_set_layout(self.layout, None); }
    }
}

struct DescriptorSet {}

impl DescriptorSet {
    // pub fn new(renderer: Arc<Renderer>) -> Self {}
}