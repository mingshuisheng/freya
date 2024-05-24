use crate::definitions::PlatformEventData;

/// Data of a Wheel event.
#[derive(Debug, Clone, PartialEq)]
pub struct WindowMovedData {
    #[allow(dead_code)]
    x: i32,
    y: i32,
}

impl WindowMovedData {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

impl WindowMovedData {
    /// Get the X.
    pub fn get_x(&self) -> i32 {
        self.x
    }

    /// Get the Y.
    pub fn get_y(&self) -> i32 {
        self.y
    }
}

impl From<&PlatformEventData> for WindowMovedData {
    fn from(val: &PlatformEventData) -> Self {
        val.downcast::<WindowMovedData>().cloned().unwrap()
    }
}
