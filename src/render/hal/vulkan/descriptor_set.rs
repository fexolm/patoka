use std::ptr;
use std::sync::Arc;

use ash::vk;

use crate::render::hal::{BindingType, DescriptorSetLayoutCreateInfo, ShaderStages};
use crate::render::hal::vulkan::FRAME_OVERLAP;
use crate::render::hal::vulkan::image::Texture;
use crate::render::hal::vulkan::renderer::Renderer;

pub struct DescriptorSetLayout {
    pub(crate) layout: vk::DescriptorSetLayout,

    renderer: Arc<Renderer>,
}

fn convert_binding_type(binding: BindingType) -> vk::DescriptorType {
    match binding {
        BindingType::UniformBuffer => vk::DescriptorType::UNIFORM_BUFFER,
        BindingType::StorageBuffer => vk::DescriptorType::STORAGE_BUFFER,
        BindingType::Texture => vk::DescriptorType::STORAGE_IMAGE,
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
    pub fn new(renderer: Arc<Renderer>, create_info: DescriptorSetLayoutCreateInfo) -> Arc<Self> {
        let bindings = create_info.bindings.iter().map(|b| {
            vk::DescriptorSetLayoutBinding {
                binding: b.binding,
                descriptor_type: convert_binding_type(b.typ),
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
    descriptor_sets: Vec<vk::DescriptorSet>,

    renderer: Arc<Renderer>,
    layout: Arc<DescriptorSetLayout>,
}

impl DescriptorSet {
    pub fn new(renderer: Arc<Renderer>, layout: Arc<DescriptorSetLayout>) -> Arc<Self> {
        let layouts = (0..FRAME_OVERLAP)
            .map(|_| layout.layout)
            .collect::<Vec<_>>();
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(renderer.descriptor_pool)
            .set_layouts(&layouts);
        let descriptor_sets = unsafe { renderer.device.allocate_descriptor_sets(&alloc_info).unwrap() };

        Arc::new(DescriptorSet { descriptor_sets, renderer, layout })
    }

    pub(crate) fn get_current(&self) -> vk::DescriptorSet {
        self.descriptor_sets[self.renderer.current_frame()]
    }

    pub fn write_texture(&self, binding: u32, texture: &Texture) {
        let img_infos = [vk::DescriptorImageInfo::default()
            .image_view(texture.image_view)
            .image_layout(vk::ImageLayout::GENERAL)];

        let writes = [vk::WriteDescriptorSet::default()
            .dst_binding(binding)
            .dst_set(self.get_current())
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .image_info(&img_infos)];

        unsafe { self.renderer.device.update_descriptor_sets(&writes, &[]); }
    }
}

impl Drop for DescriptorSet {
    fn drop(&mut self) {
        unsafe { self.renderer.device.free_descriptor_sets(self.renderer.descriptor_pool, &self.descriptor_sets).unwrap(); }
    }
}