use std::ptr;
use std::sync::Arc;

use ash::vk;

use crate::render::hal::{BindingType, DescriptorSetLayoutCreateInfo, ShaderStages};
use crate::render::hal::vulkan::image::Texture;
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
    pub fn new(renderer: Arc<Renderer>, create_info: &DescriptorSetLayoutCreateInfo) -> Arc<Self> {
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

        Arc::new(DescriptorSetLayout { layout, renderer })
    }
}

impl Drop for DescriptorSetLayout {
    fn drop(&mut self) {
        unsafe { self.renderer.device.destroy_descriptor_set_layout(self.layout, None); }
    }
}

pub struct DescriptorSet {
    pub(crate) descriptor_set: vk::DescriptorSet,

    renderer: Arc<Renderer>,
}

impl DescriptorSet {
    pub fn new(renderer: Arc<Renderer>, layout: &DescriptorSetLayout) -> Self {
        let layouts = [layout.layout];
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(renderer.descriptor_pool)
            .set_layouts(&layouts);
        let descriptor_set = unsafe { renderer.device.allocate_descriptor_sets(&alloc_info).unwrap()[0] };

        DescriptorSet { descriptor_set, renderer }
    }

    pub fn write_texture(&self, binding: u32, texture: &Texture) {
        let img_infos = [vk::DescriptorImageInfo::default()
            .image_view(texture.image_view)
            .image_layout(vk::ImageLayout::GENERAL)];

        let writes = [vk::WriteDescriptorSet::default()
            .dst_binding(binding)
            .dst_set(self.descriptor_set)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .image_info(&img_infos)];

        unsafe { self.renderer.device.update_descriptor_sets(&writes, &[]); }
    }
}

impl Drop for DescriptorSet {
    fn drop(&mut self) {
        let sets = [self.descriptor_set];
        unsafe { self.renderer.device.free_descriptor_sets(self.renderer.descriptor_pool, &sets).unwrap(); }
    }
}