use std::borrow::Cow;
use std::cell::Cell;
use std::ffi;
use std::ffi::{c_char, c_void, CStr};
use std::sync::Arc;

use ash::{Device, Entry, Instance, vk};
use ash::ext::debug_utils;
use ash::khr::{surface, swapchain};
use vk_mem::{Allocator, AllocatorCreateInfo};
use winit::error::OsError;
use winit::raw_window_handle::{HandleError, HasDisplayHandle, HasWindowHandle};
use winit::window::Window;

use crate::render::hal::{Error, RendererCreateInfo, Result};
use crate::render::hal::vulkan::command_list::CommandList;
use crate::render::hal::vulkan::FRAME_OVERLAP;
use crate::render::hal::vulkan::sync::{Fence, Semaphore};

pub struct Renderer {
    pub(crate) entry: Entry,
    pub(crate) instance: Instance,
    pub(crate) debug_utils_loader: debug_utils::Instance,
    pub(crate) debug_callback: vk::DebugUtilsMessengerEXT,

    pub(crate) physical_device: vk::PhysicalDevice,

    pub(crate) present_family_idx: u32,
    pub(crate) graphics_family_idx: u32,
    pub(crate) present_queue: vk::Queue,
    pub(crate) graphics_queue: vk::Queue,

    pub(crate) surface_loader: surface::Instance,
    pub(crate) surface: vk::SurfaceKHR,

    pub(crate) swapchain_loader: swapchain::Device,
    pub(crate) swapchain: vk::SwapchainKHR,
    pub(crate) swapchain_images: Vec<vk::Image>,
    pub(crate) swapchain_imageviews: Vec<vk::ImageView>,

    pub(crate) device: Device,

    pub(crate) command_pool: vk::CommandPool,

    pub(crate) allocator: Allocator,

    pub(crate) descriptor_pool: vk::DescriptorPool,

    window: Arc<Window>,

    frame_number: Cell<usize>,
    swapchain_image_idx: Cell<u32>,
}
impl From<vk::Result> for Error {
    fn from(res: vk::Result) -> Self {
        Error::Backend(format!("Vulkan error: {}", res))
    }
}

impl From<OsError> for Error {
    fn from(err: OsError) -> Self {
        Error::Backend(format!("OS error: {}", err))
    }
}

impl From<HandleError> for Error {
    fn from(err: HandleError) -> Self {
        Error::Backend(format!("Invalid handle: {}", err))
    }
}

fn get_enabled_layers() -> Vec<*const c_char> {
    let enabled_layers = [c"VK_LAYER_KHRONOS_validation"];
    enabled_layers
        .iter()
        .map(|raw_name| raw_name.as_ptr())
        .collect()
}

fn get_enabled_extensions(window: &Window) -> Vec<*const c_char> {
    let mut res = ash_window::enumerate_required_extensions(window.display_handle()
        .expect("Failed to get winow handle").as_raw())
        .unwrap()
        .to_vec();

    res.push(debug_utils::NAME.as_ptr());
    res
}

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = *p_callback_data;
    let message_id_number = callback_data.message_id_number;

    let message_id_name = if callback_data.p_message_id_name.is_null() {
        Cow::from("")
    } else {
        ffi::CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
    };

    let message = if callback_data.p_message.is_null() {
        Cow::from("")
    } else {
        ffi::CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };

    println!(
        "{message_severity:?}:\n{message_type:?} [{message_id_name} ({message_id_number})] : {message}\n",
    );

    vk::FALSE
}

struct SelectedPhysicalDevice {
    physical_device: vk::PhysicalDevice,
    graphics_family_idx: u32,
    present_family_idx: u32,
}

fn get_required_device_extensions() -> [&'static CStr; 1] {
    [swapchain::NAME]
}

fn check_required_extensions(instance: &Instance, device: vk::PhysicalDevice) -> bool {
    let required_extentions = get_required_device_extensions();

    let extension_props = unsafe {
        instance
            .enumerate_device_extension_properties(device)
            .unwrap()
    };

    for required in required_extentions.iter() {
        let found = extension_props.iter().any(|ext| {
            let name = unsafe { CStr::from_ptr(ext.extension_name.as_ptr()) };
            required == &name
        });

        if !found {
            return false;
        }
    }

    true
}

fn check_required_features(instance: &Instance, device: vk::PhysicalDevice) -> bool {
    let features = unsafe { instance.get_physical_device_features(device) };
    let mut features2 = vk::PhysicalDeviceFeatures2::default();
    let mut features12 = vk::PhysicalDeviceVulkan12Features::default();
    let mut features13 = vk::PhysicalDeviceVulkan13Features::default();
    features2.p_next = &mut features12 as *mut _ as *mut c_void;
    features12.p_next = &mut features13 as *mut _ as *mut c_void;

    unsafe { instance.get_physical_device_features2(device, &mut features2) };

    features.sampler_anisotropy == vk::TRUE
        && features12.buffer_device_address == vk::TRUE
        && features12.descriptor_indexing == vk::TRUE
        && features13.dynamic_rendering == vk::TRUE
        && features13.synchronization2 == vk::TRUE
}

unsafe fn find_queue_families(instance: &Instance, surface_loader: &surface::Instance, surface: vk::SurfaceKHR, device: vk::PhysicalDevice) -> Option<(u32, u32)> {
    unsafe {
        let props = instance.get_physical_device_queue_family_properties(device);

        let (mut graphics, mut present) = (None, None);

        for (idx, family) in props.iter().enumerate() {
            let idx = idx as u32;

            if graphics.is_none() && family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                graphics = Some(idx);
                continue;
            }

            let present_support = surface_loader.get_physical_device_surface_support(device, idx, surface).unwrap();

            if present.is_none() && present_support {
                present = Some(idx);
            }
        }

        if let (Some(g), Some(p)) = (graphics, present) {
            Some((g, p))
        } else {
            None
        }
    }
}
unsafe fn select_physical_device(instance: &Instance, surface_loader: &surface::Instance, surface: vk::SurfaceKHR) -> Result<SelectedPhysicalDevice> {
    let devices = instance
        .enumerate_physical_devices()?;

    Ok(devices
        .iter()
        .find_map(|&physical_device| {
            if !check_required_extensions(instance, physical_device) || !check_required_features(instance, physical_device) {
                return None;
            }

            if let Some((graphics_family_idx, present_family_idx)) = find_queue_families(instance, surface_loader, surface, physical_device) {
                Some(SelectedPhysicalDevice { physical_device, graphics_family_idx, present_family_idx })
            } else {
                None
            }
        }).expect("Couldn't find suitable device."))
}

fn create_swapchain_image_views(
    device: &Device,
    swapchain_images: &[vk::Image],
    format: vk::Format,
) -> Vec<vk::ImageView> {
    swapchain_images
        .iter()
        .map(|image| {
            create_image_view(
                device,
                *image,
                1,
                format,
                vk::ImageAspectFlags::COLOR,
            ).unwrap()
        })
        .collect::<Vec<_>>()
}
fn create_image_view(
    device: &Device,
    image: vk::Image,
    mip_levels: u32,
    format: vk::Format,
    aspect_mask: vk::ImageAspectFlags,
) -> Result<vk::ImageView> {
    let create_info = vk::ImageViewCreateInfo::default()
        .image(image)
        .view_type(vk::ImageViewType::TYPE_2D)
        .format(format)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask,
            base_mip_level: 0,
            level_count: mip_levels,
            base_array_layer: 0,
            layer_count: 1,
        });

    unsafe { Ok(device.create_image_view(&create_info, None)?) }
}

impl Renderer {
    pub fn new(window: Arc<Window>, info: &RendererCreateInfo) -> Result<Arc<Self>> {
        unsafe {
            let entry = Entry::linked();

            let instance = {
                let app_info = vk::ApplicationInfo::default()
                    .engine_name(c"Patoka Engine")
                    .application_name(c"Patoka App")
                    .application_version(vk::make_api_version(0, 1, 0, 0))
                    .engine_version(vk::make_api_version(0, 1, 0, 0))
                    .api_version(vk::make_api_version(0, 1, 3, 0));

                let create_flags = vk::InstanceCreateFlags::default();

                let enabled_layers = get_enabled_layers();
                let enabled_extensions = get_enabled_extensions(&window);

                let create_info = vk::InstanceCreateInfo::default()
                    .application_info(&app_info)
                    .enabled_layer_names(&enabled_layers)
                    .enabled_extension_names(&enabled_extensions)
                    .flags(create_flags);

                entry.create_instance(&create_info, None)?
            };

            let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
                .message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                                      | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                                      | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
                )
                .message_type(vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE)
                .pfn_user_callback(Some(vulkan_debug_callback));

            let debug_utils_loader = debug_utils::Instance::new(&entry, &instance);
            let debug_callback = debug_utils_loader
                .create_debug_utils_messenger(&debug_info, None)?;

            let surface = ash_window::create_surface(
                &entry,
                &instance,
                window.display_handle()?.as_raw(),
                window.window_handle()?.as_raw(),
                None,
            )?;

            let surface_loader = surface::Instance::new(&entry, &instance);

            let SelectedPhysicalDevice { physical_device, graphics_family_idx, present_family_idx } = select_physical_device(&instance, &surface_loader, surface)?;

            let device = {
                let device_extension_names_raw = [
                    swapchain::NAME.as_ptr(),
                ];

                let features = vk::PhysicalDeviceFeatures {
                    shader_clip_distance: 1,
                    ..Default::default()
                };

                let mut features2 = vk::PhysicalDeviceFeatures2::default()
                    .features(features);
                let mut features12 = vk::PhysicalDeviceVulkan12Features::default()
                    .descriptor_indexing(true)
                    .buffer_device_address(true);
                let mut features13 = vk::PhysicalDeviceVulkan13Features::default()
                    .synchronization2(true)
                    .dynamic_rendering(true);
                features2.p_next = &mut features12 as *mut _ as *mut c_void;
                features12.p_next = &mut features13 as *mut _ as *mut c_void;

                let priorities = [1.0];

                let queue_infos: Vec<_> = [graphics_family_idx, present_family_idx].iter().map(|&idx| vk::DeviceQueueCreateInfo::default()
                    .queue_family_index(idx)
                    .queue_priorities(&priorities)
                ).collect();

                let create_info = vk::DeviceCreateInfo::default()
                    .queue_create_infos(&queue_infos)
                    .enabled_extension_names(&device_extension_names_raw)
                    .push_next(&mut features2);
                instance
                    .create_device(physical_device, &create_info, None)
                    .unwrap()
            };

            let present_queue = device.get_device_queue(present_family_idx, 0);
            let graphics_queue = device.get_device_queue(graphics_family_idx, 0);

            let swapchain_loader = swapchain::Device::new(&instance, &device);

            let swapchain = {
                let create_info = vk::SwapchainCreateInfoKHR::default()
                    .surface(surface)
                    .min_image_count(3)
                    .image_color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
                    .image_format(vk::Format::B8G8R8A8_UNORM)
                    .image_extent(vk::Extent2D {
                        width: 800,
                        height: 600,
                    })
                    .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST)
                    .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                    .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                    .present_mode(vk::PresentModeKHR::FIFO)
                    .pre_transform(vk::SurfaceTransformFlagsKHR::IDENTITY)
                    .clipped(true)
                    .image_array_layers(1);

                swapchain_loader
                    .create_swapchain(&create_info, None)
                    .unwrap()
            };

            let swapchain_images = swapchain_loader.get_swapchain_images(swapchain)?;
            let swapchain_imageviews = create_swapchain_image_views(&device, &swapchain_images, vk::Format::B8G8R8A8_UNORM);

            let command_pool = {
                let create_info = vk::CommandPoolCreateInfo::default()
                    .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                    .queue_family_index(graphics_family_idx);
                device.create_command_pool(&create_info, None)?
            };

            let allocator = Allocator::new(AllocatorCreateInfo::new(&instance, &device, physical_device)).unwrap();

            let descriptor_pool = {
                let pool_sizes = [
                    vk::DescriptorPoolSize { ty: vk::DescriptorType::UNIFORM_BUFFER, descriptor_count: 4096 },
                    vk::DescriptorPoolSize { ty: vk::DescriptorType::STORAGE_BUFFER, descriptor_count: 4096 },
                    vk::DescriptorPoolSize { ty: vk::DescriptorType::SAMPLED_IMAGE, descriptor_count: 4096 },
                    vk::DescriptorPoolSize { ty: vk::DescriptorType::SAMPLER, descriptor_count: 4096 },
                ];

                let create_info = vk::DescriptorPoolCreateInfo::default()
                    .pool_sizes(&pool_sizes)
                    .max_sets(1000)
                    .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET);

                device.create_descriptor_pool(&create_info, None).unwrap()
            };

            Ok(Arc::new(Self {
                entry,
                instance,
                device,
                surface_loader,
                swapchain_loader,
                debug_utils_loader,
                debug_callback,
                physical_device,
                present_family_idx,
                graphics_family_idx,
                present_queue,
                graphics_queue,
                surface,
                swapchain,
                window,
                swapchain_images,
                swapchain_imageviews,
                command_pool,
                frame_number: Cell::new(0),
                swapchain_image_idx: Cell::new(0),
                allocator,
                descriptor_pool,
            }))
        }
    }

    pub(crate) fn current_frame(&self) -> usize {
        self.frame_number.get()
    }

    pub fn start_frame(&self, signal_semaphore: &Semaphore) {
        unsafe {
            let (idx, _) = self.swapchain_loader.acquire_next_image(self.swapchain, 1000000000, signal_semaphore.get_raw(), vk::Fence::null()).unwrap();
            self.swapchain_image_idx.replace(idx);
        }
    }

    fn semaphore_submit_info(semaphore: &Semaphore) -> vk::SemaphoreSubmitInfo {
        unsafe {
            vk::SemaphoreSubmitInfo::default()
                .semaphore(semaphore.get_raw())
                .stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                .device_index(0)
                .value(1)
        }
    }

    pub(crate) fn get_current_swapchain_img(&self) -> vk::Image {
        self.swapchain_images[self.swapchain_image_idx.get() as usize]
    }

    pub fn submit(&self, command_list: &CommandList, wait_semaphores: &[&Semaphore], signal_semaphores: &[&Semaphore], signal_fence: &Fence) {
        let cl_submit_infos = [vk::CommandBufferSubmitInfo::default()
            .command_buffer(command_list.get_raw())
            .device_mask(0)];

        let wait_semaphore_infos = wait_semaphores.iter().map(|s| Self::semaphore_submit_info(s)).collect::<Vec<_>>();
        let signal_semaphore_infos = signal_semaphores.iter().map(|s| Self::semaphore_submit_info(s)).collect::<Vec<_>>();

        let submit_infos = [vk::SubmitInfo2::default()
            .wait_semaphore_infos(&wait_semaphore_infos)
            .signal_semaphore_infos(&signal_semaphore_infos)
            .command_buffer_infos(&cl_submit_infos)];

        unsafe { self.device.queue_submit2(self.graphics_queue, &submit_infos, signal_fence.get_raw()).unwrap() }
    }

    pub fn present(&self, wait_semaphore: &Semaphore) {
        unsafe {
            let swapchains = [self.swapchain];
            let wait_semaphores = [wait_semaphore.get_raw()];
            let image_indices = [self.swapchain_image_idx.get()];
            let present_info = vk::PresentInfoKHR::default()
                .swapchains(&swapchains)
                .wait_semaphores(&wait_semaphores)
                .image_indices(&image_indices);
            self.swapchain_loader.queue_present(self.graphics_queue, &present_info).unwrap();
            self.frame_number.replace((self.current_frame() + 1) % FRAME_OVERLAP);
        }
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();
            self.device.destroy_descriptor_pool(self.descriptor_pool, None);
            self.device.destroy_command_pool(self.command_pool, None);
            for &v in &self.swapchain_imageviews {
                self.device.destroy_image_view(v, None);
            }

            self.swapchain_loader.destroy_swapchain(self.swapchain, None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.device.destroy_device(None);
            self.debug_utils_loader.destroy_debug_utils_messenger(self.debug_callback, None);
            self.instance.destroy_instance(None);
        }
    }
}