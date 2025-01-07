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
