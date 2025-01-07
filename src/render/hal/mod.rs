use std::fmt;
use std::fmt::{Debug, Display};

pub mod vulkan;

#[derive(Debug)]
pub enum Error {
    Backend(String)
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Backend(msg) => {
                write!(f, "{msg}")
            }
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

pub struct RendererCreateInfo {}

pub struct CommandListCreateInfo {}

pub struct SemaphoreCreateInfo {}

pub struct FenceCreateInfo {}


pub enum BindingType {
    UniformBuffer,
    StagingBuffer,
    Texture,
    Sampler,
}

bitflags::bitflags! {
    #[derive(Clone, Copy)]
    pub struct ShaderStages: u8 {
        const Vertex = 0x1;
        const Fragment = 0x2;
        const Compute = 0x4;
    }
}
pub struct DescriptorSetBinding {
    pub typ: BindingType,
    pub binding: u32,
    pub stage: ShaderStages,
}
pub struct DescriptorSetLayoutCreateInfo {
    pub bindings: Vec<DescriptorSetBinding>,
}
