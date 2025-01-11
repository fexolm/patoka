use std::sync::Arc;

use ash::vk;

use crate::render::hal::ShaderCreateInfo;
use crate::render::hal::vulkan::renderer::Renderer;

pub struct Shader {
    pub(crate) shader: vk::ShaderModule,

    renderer: Arc<Renderer>,
}

impl Shader {
    pub fn new(renderer: Arc<Renderer>, create_info: ShaderCreateInfo) -> Arc<Self> {
        let info = vk::ShaderModuleCreateInfo::default()
            .code(create_info.code);

        let shader = unsafe { renderer.device.create_shader_module(&info, None).unwrap() };

        Arc::new(Shader { shader, renderer })
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe { self.renderer.device.destroy_shader_module(self.shader, None); }
    }
}