use std::sync::Arc;

use ash::vk;

use crate::render::hal::{ComputePipelineCreateInfo, PipelineLayoutCreateInfo};
use crate::render::hal::vulkan::descriptor_set::DescriptorSetLayout;
use crate::render::hal::vulkan::renderer::Renderer;
use crate::render::hal::vulkan::shader::Shader;

pub struct PipelineLayout {
    pub(crate) layout: vk::PipelineLayout,

    renderer: Arc<Renderer>,
    descriptor_sets: Vec<Arc<DescriptorSetLayout>>,
}

impl PipelineLayout {
    pub fn new(renderer: Arc<Renderer>, create_info: PipelineLayoutCreateInfo) -> Arc<Self> {
        let sets = create_info.sets.iter().map(|s| s.layout)
            .collect::<Vec<_>>();
        let info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&sets);

        let layout = unsafe { renderer.device.create_pipeline_layout(&info, None).unwrap() };

        Arc::new(PipelineLayout { layout, renderer, descriptor_sets: create_info.sets })
    }
}

impl Drop for PipelineLayout {
    fn drop(&mut self) {
        unsafe { self.renderer.device.destroy_pipeline_layout(self.layout, None) };
    }
}

pub struct ComputePipeline {
    pub(crate) pipeline: vk::Pipeline,

    renderer: Arc<Renderer>,
    _layout: Arc<PipelineLayout>,
    _shader: Arc<Shader>,
}

impl ComputePipeline {
    pub fn new(renderer: Arc<Renderer>, create_info: ComputePipelineCreateInfo) -> Arc<Self> {
        let shader_stage = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::COMPUTE)
            .module(create_info.shader.shader)
            .name(create_info.entrypoint);

        let pipeline_infos = [vk::ComputePipelineCreateInfo::default()
            .layout(create_info.pipeline_layout.layout)
            .stage(shader_stage)];

        let pipeline = unsafe { renderer.device.create_compute_pipelines(vk::PipelineCache::null(), &pipeline_infos, None).unwrap()[0] };

        Arc::new(ComputePipeline { pipeline, renderer, _layout: create_info.pipeline_layout, _shader: create_info.shader })
    }
}

impl Drop for ComputePipeline {
    fn drop(&mut self) {
        unsafe { self.renderer.device.destroy_pipeline(self.pipeline, None) };
    }
}