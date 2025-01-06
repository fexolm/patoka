use std::borrow::Cow;
use std::ffi;
use std::ffi::{c_char, c_void, CStr};

use ash::{Device, Entry, Instance, vk};
use ash::ext::debug_utils;
use ash::khr::{surface, swapchain};
use winit::error::OsError;
use winit::raw_window_handle::{HandleError, HasDisplayHandle, HasWindowHandle};
use winit::window::Window;

use crate::render::hal;
use crate::render::hal::{Error, RendererCreateInfo, Result};

pub struct VulkanRenderer<'w> {
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

    pub(crate) device: Device,

    window: &'w Window,
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

impl<'w> hal::Renderer<'w> for VulkanRenderer<'w> {
    fn new(window: &'w Window, info: &RendererCreateInfo) -> Result<Box<Self>> {
        unsafe {
            let entry = Entry::linked();

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

            let SelectedPhysicalDevice { physical_device, graphics_family_idx, present_family_idx } = select_physical_device(&instance, &surface_loader, surface)?;

            let device_extension_names_raw = [
                swapchain::NAME.as_ptr(),
            ];

            let features = vk::PhysicalDeviceFeatures {
                shader_clip_distance: 1,
                ..Default::default()
            };

            let priorities = [1.0];

            let queue_infos: Vec<_> = [graphics_family_idx, present_family_idx].iter().map(|&idx| vk::DeviceQueueCreateInfo::default()
                .queue_family_index(idx)
                .queue_priorities(&priorities)
            ).collect();

            let device_create_info = vk::DeviceCreateInfo::default()
                .queue_create_infos(&queue_infos)
                .enabled_extension_names(&device_extension_names_raw)
                .enabled_features(&features);

            let device = instance
                .create_device(physical_device, &device_create_info, None)
                .unwrap();

            let present_queue = device.get_device_queue(present_family_idx, 0);
            let graphics_queue = device.get_device_queue(graphics_family_idx, 0);

            let swapchain_loader = swapchain::Device::new(&instance, &device);

            let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
                .surface(surface)
                .min_image_count(3)
                .image_color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
                .image_format(vk::Format::B8G8R8A8_UNORM)
                .image_extent(vk::Extent2D {
                    width: window.inner_size().width,
                    height: window.inner_size().height,
                })
                .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
                .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                .present_mode(vk::PresentModeKHR::FIFO)
                .pre_transform(vk::SurfaceTransformFlagsKHR::IDENTITY)
                .clipped(true)
                .image_array_layers(1);

            let swapchain = swapchain_loader
                .create_swapchain(&swapchain_create_info, None)
                .unwrap();

            Ok(Box::new(Self {
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
            }))
        }
    }
}

impl<'w> Drop for VulkanRenderer<'w> {
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