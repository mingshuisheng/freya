use accesskit::NodeId;
use accesskit_winit::ActionRequestEvent;
use dioxus_core::Template;
use torin::geometry::{Point2D, Size2D};
use uuid::Uuid;
use winit::window::{CursorIcon, ResizeDirection};
/// Custom EventLoop messages
#[derive(Debug)]
pub enum EventMessage {
    /// Update the given template
    UpdateTemplate(Template),
    /// Pull the VirtualDOM
    PollVDOM,
    /// Request a rerender
    RequestRerender,
    /// Remeasure a text elements group
    RemeasureTextGroup(Uuid),
    /// Change the cursor icon
    SetCursorIcon(CursorIcon),
    /// Accessibility action request event
    ActionRequestEvent(ActionRequestEvent),
    /// Focus the given accessibility NodeID
    FocusAccessibilityNode(NodeId),
    /// Focus the next accessibility Node
    FocusNextAccessibilityNode,
    /// Focus the previous accessibility Node
    FocusPrevAccessibilityNode,
    /// Trigger window dragging
    DragWindow,
    /// Trigger window resize dragging
    DragResizeWindow(ResizeDirection),
    /// Set the window size
    SetWindowSize(Size2D),
    /// Set the window position
    SetWindowPosition(Point2D),
    /// Set the window size and postion
    SetWindowSizeAndPosition(Size2D, Point2D),
    /// Close the whole app
    ExitApp,
}

impl From<ActionRequestEvent> for EventMessage {
    fn from(value: ActionRequestEvent) -> Self {
        Self::ActionRequestEvent(value)
    }
}
