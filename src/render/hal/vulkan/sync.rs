use std::sync::Arc;

use ash::vk;

use crate::render::hal::{FenceCreateInfo, SemaphoreCreateInfo};
use crate::render::hal::vulkan::FRAME_OVERLAP;
use crate::render::hal::vulkan::renderer::Renderer;

pub struct Semaphore {
    semaphores: [vk::Semaphore; FRAME_OVERLAP],
    renderer: Arc<Renderer>,
}

impl Semaphore {
    pub fn new(renderer: Arc<Renderer>, create_info: &SemaphoreCreateInfo) -> Self {
        let info = vk::SemaphoreCreateInfo::default();
        let semaphores = (0..FRAME_OVERLAP)
            .map(|_| unsafe { renderer.device.create_semaphore(&info, None).unwrap() })
            .collect::<Vec<vk::Semaphore>>()
            .try_into().unwrap();
        Self {
            semaphores,
            renderer,
        }
    }

    pub(crate) unsafe fn get_raw(&self) -> vk::Semaphore {
        self.semaphores[self.renderer.current_frame()]
    }
}

impl Drop for Semaphore {
    fn drop(&mut self) {
        for s in self.semaphores {
            unsafe { self.renderer.device.destroy_semaphore(s, None) }
        }
    }
}

pub struct Fence {
    fences: [vk::Fence; FRAME_OVERLAP],
    renderer: Arc<Renderer>,
}

impl Drop for Fence {
    fn drop(&mut self) {
        for f in self.fences {
            unsafe { self.renderer.device.destroy_fence(f, None) }
        }
    }
}

impl Fence {
    pub fn new(renderer: Arc<Renderer>, create_info: &FenceCreateInfo) -> Self {
        let info = vk::FenceCreateInfo::default()
            .flags(vk::FenceCreateFlags::SIGNALED);

        let fences = (0..FRAME_OVERLAP)
            .map(|_| unsafe { renderer.device.create_fence(&info, None).unwrap() })
            .collect::<Vec<vk::Fence>>()
            .try_into().unwrap();
        Self {
            fences,
            renderer,
        }
    }

    pub(crate) unsafe fn get_raw(&self) -> vk::Fence {
        self.fences[self.renderer.current_frame()]
    }

    pub fn wait(&self) {
        let frame = self.renderer.current_frame();
        unsafe { self.renderer.device.wait_for_fences(&self.fences[frame..frame + 1], true, 1000000000).unwrap(); }
    }

    pub fn reset(&self) {
        let frame = self.renderer.current_frame();
        unsafe { self.renderer.device.reset_fences(&self.fences[frame..frame + 1]).unwrap(); }
    }
}