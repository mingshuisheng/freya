use crate::definitions::PlatformEventData;

/// Data of a Wheel event.
#[derive(Debug, Clone, PartialEq)]
pub struct ScaleFactorData {
    #[allow(dead_code)]
    scale_factor: f32,
}

impl ScaleFactorData {
    pub fn new(scale_factor: f32) -> Self {
        Self { scale_factor }
    }
}

impl ScaleFactorData {
    /// Get the scale_factor.
    pub fn get_scale_factor(&self) -> f32 {
        self.scale_factor
    }
}

impl From<&PlatformEventData> for ScaleFactorData {
    fn from(val: &PlatformEventData) -> Self {
        val.downcast::<ScaleFactorData>().cloned().unwrap()
    }
}
