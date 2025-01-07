use ash::vk;

pub trait Image {
    unsafe fn get_image_view(&self) -> vk::ImageView;
    unsafe fn get_image(&self) -> vk::Image;
}
pub struct Framebuffer {
    pub(super) image_view: vk::ImageView,
    pub(super) image: vk::Image,
    pub(super) idx: u32,
}
impl Image for Framebuffer {
    unsafe fn get_image_view(&self) -> vk::ImageView {
        self.image_view
    }
    unsafe fn get_image(&self) -> vk::Image {
        self.image
    }
}

