pub mod file;
pub mod keyboard;
pub mod mouse;
pub mod pointer;
pub mod scale_factor;
pub mod touch;
pub mod wheel;
pub mod window_moved;

use dioxus_core::Event;
pub use file::*;
pub use keyboard::*;
pub use mouse::*;
pub use pointer::*;
pub use scale_factor::*;
pub use touch::*;
pub use wheel::*;
pub use window_moved::*;

pub type KeyboardEvent = Event<KeyboardData>;
pub type MouseEvent = Event<MouseData>;
pub type WheelEvent = Event<WheelData>;
pub type TouchEvent = Event<TouchData>;
pub type PointerEvent = Event<PointerData>;
pub type WindowMovedEvent = Event<WindowMovedData>;
pub type ScaleFactorEvent = Event<ScaleFactorData>;
