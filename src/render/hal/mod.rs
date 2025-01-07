use std::fmt;
use std::fmt::{Debug, Display};

use winit::window::Window;

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
pub trait Renderer<'w> {
    fn new(window: &'w Window, info: &RendererCreateInfo) -> Result<Box<Self>>;
    fn create_command_list<'r>(&'r self, create_info: &CommandListCreateInfo) -> Box<dyn CommandList + 'r>;
}
pub trait CommandList {}