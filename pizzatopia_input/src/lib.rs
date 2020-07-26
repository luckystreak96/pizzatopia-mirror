pub use crate::input::{Input, InputManager, InputResult};
pub use crate::system::InputManagementSystem;
#[cfg(feature = "gilrs")]
pub use gilrs::GilRsControllerSystem;

// TODO : Remove this
pub use amethyst::input::StringBindings;

#[cfg(feature = "gilrs")]
mod gilrs;
mod input;
mod system;
