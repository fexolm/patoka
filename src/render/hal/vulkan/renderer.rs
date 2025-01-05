use std::borrow::Cow;
use std::ffi;
use std::ffi::{c_char, CString};

use ash::{Device, Entry, Instance, vk};
use ash::ext::debug_utils;
use ash::khr::{surface, swapchain};
use winit::error::OsError;
use winit::event_loop::EventLoop;
use winit::raw_window_handle::{HandleError, HasDisplayHandle, HasWindowHandle};
use winit::window::{Window, WindowBuilder};

use crate::render::hal;
use crate::render::hal::{Error, RendererCreateInfo, Result};

pub struct VulkanRenderer {
    pub(crate) event_loop: EventLoop<()>,
    pub(crate) window: Window,

    pub(crate) entry: Entry,
    pub(crate) instance: Instance,
    pub(crate) device: Device,
    pub(crate) surface_loader: surface::Instance,
    pub(crate) swapchain_loader: swapchain::Device,
    pub(crate) debug_utils_loader: debug_utils::Instance,
    pub(crate) debug_callback: vk::DebugUtilsMessengerEXT,

    pub(crate) physical_device: vk::PhysicalDevice,
    pub(crate) queue_family_index: u32,
    pub(crate) present_queue: vk::Queue,

    pub(crate) surface: vk::SurfaceKHR,
    pub(crate) surface_format: vk::SurfaceFormatKHR,
    pub(crate) surface_resolution: vk::Extent2D,

    pub(crate) swapchain: vk::SwapchainKHR,
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

impl hal::Renderer for VulkanRenderer {
    fn new(info: &RendererCreateInfo) -> Result<Box<Self>> {
        unsafe {
            let event_loop = EventLoop::new().unwrap();
            let window = WindowBuilder::new()
                .with_title(&info.title)
                .with_inner_size(winit::dpi::LogicalSize::new(f64::from(info.window_size.0), f64::from(info.window_size.1)))
                .build(&event_loop)?;

            let entry = Entry::linked();

            let application_name = CString::new(info.title.as_str()).unwrap();

            let app_info = vk::ApplicationInfo::default()
                .engine_name(c"Patoka Engine")
                .application_name(application_name.as_c_str())
                .application_version(vk::make_api_version(0, 1, 0, 0))
                .engine_version(vk::make_api_version(0, 1, 0, 0))
                .api_version(vk::make_api_version(0, 1, 3, 0));

            let create_flags = vk::InstanceCreateFlags::default();

            let enabled_layers = get_enabled_layers();

            let enabled_extentions = get_enabled_extensions(&window);

            let create_info = vk::InstanceCreateInfo::default()
                .application_info(&app_info)
                .enabled_layer_names(&enabled_layers)
                .enabled_extension_names(&enabled_extentions)
                .flags(create_flags);


            let instance = entry.create_instance(&create_info, None)?;

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

            let pdevices = instance
                .enumerate_physical_devices()?;

            let (physical_device, queue_family_index) = pdevices
                .iter()
                .find_map(|pdevice| {
                    instance
                        .get_physical_device_queue_family_properties(*pdevice)
                        .iter()
                        .enumerate()
                        .find_map(|(index, info)| {
                            let supports_graphic_and_surface =
                                info.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                                    && surface_loader
                                    .get_physical_device_surface_support(
                                        *pdevice,
                                        index as u32,
                                        surface,
                                    )
                                    .unwrap();
                            if supports_graphic_and_surface {
                                Some((*pdevice, index))
                            } else {
                                None
                            }
                        })
                }).expect("Couldn't find suitable device.");

            let queue_family_index = queue_family_index as u32;

            let device_extension_names_raw = [
                swapchain::NAME.as_ptr(),
            ];

            let features = vk::PhysicalDeviceFeatures {
                shader_clip_distance: 1,
                ..Default::default()
            };

            let priorities = [1.0];

            let queue_info = vk::DeviceQueueCreateInfo::default()
                .queue_family_index(queue_family_index)
                .queue_priorities(&priorities);

            let device_create_info = vk::DeviceCreateInfo::default()
                .queue_create_infos(std::slice::from_ref(&queue_info))
                .enabled_extension_names(&device_extension_names_raw)
                .enabled_features(&features);

            let device = instance
                .create_device(physical_device, &device_create_info, None)
                .unwrap();

            let present_queue = device.get_device_queue(queue_family_index, 0);

            let surface_format = surface_loader
                .get_physical_device_surface_formats(physical_device, surface)
                .unwrap()[0];

            let surface_capabilities = surface_loader
                .get_physical_device_surface_capabilities(physical_device, surface)
                .unwrap();
            let mut desired_image_count = surface_capabilities.min_image_count + 1;
            if surface_capabilities.max_image_count > 0
                && desired_image_count > surface_capabilities.max_image_count
            {
                desired_image_count = surface_capabilities.max_image_count;
            }

            let surface_resolution = match surface_capabilities.current_extent.width {
                u32::MAX => vk::Extent2D {
                    width: info.window_size.0,
                    height: info.window_size.1,
                },
                _ => surface_capabilities.current_extent,
            };

            let pre_transform = if surface_capabilities
                .supported_transforms
                .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
            {
                vk::SurfaceTransformFlagsKHR::IDENTITY
            } else {
                surface_capabilities.current_transform
            };
            let present_modes = surface_loader
                .get_physical_device_surface_present_modes(physical_device, surface)
                .unwrap();
            let present_mode = present_modes
                .iter()
                .cloned()
                .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
                .unwrap_or(vk::PresentModeKHR::FIFO);
            let swapchain_loader = swapchain::Device::new(&instance, &device);

            let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
                .surface(surface)
                .min_image_count(desired_image_count)
                .image_color_space(surface_format.color_space)
                .image_format(surface_format.format)
                .image_extent(surface_resolution)
                .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
                .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                .pre_transform(pre_transform)
                .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                .present_mode(present_mode)
                .clipped(true)
                .image_array_layers(1);

            let swapchain = swapchain_loader
                .create_swapchain(&swapchain_create_info, None)
                .unwrap();

            Ok(Box::new(Self {
                event_loop,
                window,
                entry,
                instance,
                device,
                surface_loader,
                swapchain_loader,
                debug_utils_loader,
                debug_callback,
                physical_device,
                queue_family_index,
                present_queue,
                surface,
                surface_format,
                surface_resolution,
                swapchain,
            }))
        }
    }
}

impl Drop for VulkanRenderer {
    fn drop(&mut self) {
        unsafe {
            self.swapchain_loader.destroy_swapchain(self.swapchain, None);
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.debug_utils_loader.destroy_debug_utils_messenger(self.debug_callback, None);
            self.instance.destroy_instance(None);
        }
    }
}