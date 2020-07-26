#[cfg(feature = "gilrs")]
pub use crate::gilrs::GilRsControllerSystem;
pub use crate::input::{Input, InputManager, InputResult};
pub use crate::system::InputManagementSystem;

// TODO : Remove this
pub use amethyst::input::StringBindings;

#[cfg(feature = "gilrs")]
pub mod gilrs;
mod input;
mod system;
