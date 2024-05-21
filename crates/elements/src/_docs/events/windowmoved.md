The `wheel` event fires when the user scrolls the mouse wheel while hovering over the element.

Event Data: [`WindowMovedData`](crate::events::WindowMovedData)

### Example

```rust, no_run
# use freya::prelude::*;
fn app() -> Element {
    rsx!(
        rect {
            width: "100",
            height: "100",
            background: "red",
            onwindowmoved: |_| println!("Scrolling with the wheel!")
        }
    )
}
```