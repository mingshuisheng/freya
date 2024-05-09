use dioxus_core::{Template, VirtualDom};
use freya_common::EventMessage;
use freya_core::prelude::*;
use freya_engine::prelude::*;
use freya_hooks::PlatformInformation;
use freya_native_core::NodeId;
use futures_task::Waker;
use futures_util::FutureExt;
use pin_utils::pin_mut;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tokio::{
    select,
    sync::{mpsc, watch, Notify},
};
use torin::geometry::{Area, Size2D};
use tracing::info;
use uuid::Uuid;
use winit::event_loop::EventLoopProxy;
use winit::{dpi::PhysicalSize, window::Window};

use crate::{accessibility::AccessKitManager, render::render_skia, winit_waker::winit_waker};
use crate::{EmbeddedFonts, HoveredNode};

/// Manages the Application lifecycle
pub struct Application {
    pub(crate) sdom: SafeDOM,
    pub(crate) vdom: VirtualDom,
    pub(crate) events: EventsQueue,
    pub(crate) vdom_waker: Waker,
    pub(crate) proxy: EventLoopProxy<EventMessage>,
    pub(crate) mutations_notifier: Option<Arc<Notify>>,
    pub(crate) event_emitter: EventEmitter,
    pub(crate) event_receiver: EventReceiver,
    pub(crate) nodes_state: NodesState,
    pub(crate) focus_sender: FocusSender,
    pub(crate) focus_receiver: FocusReceiver,
    pub(crate) accessibility: AccessKitManager,
    pub(crate) font_collection: FontCollection,
    pub(crate) font_mgr: FontMgr,
    pub(crate) ticker_sender: broadcast::Sender<()>,
    pub(crate) plugins: PluginsManager,
    pub(crate) navigator_state: NavigatorState,
    pub(crate) measure_layout_on_next_render: bool,
    pub(crate) platform_information: Arc<Mutex<PlatformInformation>>,
    pub(crate) default_fonts: Vec<String>,
}

impl Application {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        sdom: SafeDOM,
        vdom: VirtualDom,
        proxy: &EventLoopProxy<EventMessage>,
        mutations_notifier: Option<Arc<Notify>>,
        window: &Window,
        fonts_config: &EmbeddedFonts,
        mut plugins: PluginsManager,
        default_fonts: Vec<String>,
    ) -> Self {
        let accessibility = AccessKitManager::new(window, proxy.clone());

        let mut font_collection = FontCollection::new();
        let def_mgr = FontMgr::default();

        let mut provider = TypefaceFontProvider::new();

        for (font_name, font_data) in fonts_config {
            let ft_type = def_mgr.new_from_data(font_data, None).unwrap();
            provider.register_typeface(ft_type, Some(*font_name));
        }

        let font_mgr: FontMgr = provider.into();
        font_collection.set_default_font_manager(def_mgr, "Fira Sans");
        font_collection.set_dynamic_font_manager(font_mgr.clone());

        let (event_emitter, event_receiver) = mpsc::unbounded_channel::<DomEvent>();
        let (focus_sender, focus_receiver) = watch::channel(ACCESSIBILITY_ROOT_ID);

        plugins.send(PluginEvent::WindowCreated(window));

        let platform_information = Arc::new(Mutex::new(PlatformInformation::from_winit(
            window.inner_size(),
        )));

        Self {
            sdom,
            vdom,
            events: EventsQueue::new(),
            vdom_waker: winit_waker(proxy),
            proxy: proxy.clone(),
            mutations_notifier,
            event_emitter,
            event_receiver,
            nodes_state: NodesState::default(),
            accessibility,
            focus_sender,
            focus_receiver,
            font_collection,
            font_mgr,
            ticker_sender: broadcast::channel(5).0,
            plugins,
            navigator_state: NavigatorState::new(NavigationMode::NotKeyboard),
            measure_layout_on_next_render: false,
            platform_information,
            default_fonts,
        }
    }

    /// Provide the launch state and few other utilities like the EventLoopProxy
    pub fn provide_vdom_contexts<State: 'static>(&mut self, app_state: Option<State>) {
        if let Some(state) = app_state {
            self.vdom.insert_any_root_context(Box::new(state));
        }
        self.vdom
            .insert_any_root_context(Box::new(self.proxy.clone()));
        self.vdom
            .insert_any_root_context(Box::new(self.focus_receiver.clone()));
        self.vdom
            .insert_any_root_context(Box::new(Arc::new(self.ticker_sender.subscribe())));
        self.vdom
            .insert_any_root_context(Box::new(self.navigator_state.clone()));
        self.vdom
            .insert_any_root_context(Box::new(self.platform_information.clone()));
    }

    /// Make the first build of the VirtualDOM and sync it with the RealDOM.
    pub fn init_doms<State: 'static>(&mut self, scale_factor: f32, app_state: Option<State>) {
        self.plugins.send(PluginEvent::StartedUpdatingDOM);

        self.provide_vdom_contexts(app_state);

        self.sdom.get_mut().init_dom(&mut self.vdom, scale_factor);
        self.plugins.send(PluginEvent::FinishedUpdatingDOM);
    }

    /// Update the DOM with the mutations from the VirtualDOM.
    pub fn apply_vdom_changes(&mut self, scale_factor: f32) -> (bool, bool) {
        self.plugins.send(PluginEvent::StartedUpdatingDOM);

        let (repaint, relayout) = self
            .sdom
            .get_mut()
            .render_mutations(&mut self.vdom, scale_factor);

        self.plugins.send(PluginEvent::FinishedUpdatingDOM);

        if repaint {
            if let Some(mutations_notifier) = &self.mutations_notifier {
                mutations_notifier.notify_one();
            }
        }

        (repaint, relayout)
    }

    /// Poll the VirtualDOM for any new change
    pub fn poll_vdom(&mut self, window: &Window) {
        let waker = &self.vdom_waker.clone();
        let mut cx = std::task::Context::from_waker(waker);

        loop {
            {
                let fut = async {
                    select! {
                        ev = self.event_receiver.recv() => {
                            if let Some(ev) = ev {
                                let data = ev.data.any();
                                self.vdom.handle_event(ev.name.into(), data, ev.element_id, ev.bubbles);

                                self.vdom.process_events();
                            }
                        },
                        _ = self.vdom.wait_for_work() => {},
                    }
                };
                pin_mut!(fut);

                match fut.poll_unpin(&mut cx) {
                    std::task::Poll::Ready(_) => {}
                    std::task::Poll::Pending => break,
                }
            }

            let (must_repaint, must_relayout) =
                self.apply_vdom_changes(window.scale_factor() as f32);

            if must_relayout {
                self.measure_layout_on_next_render = true;
            }

            if must_relayout || must_repaint {
                window.request_redraw();
            }
        }
    }

    /// Process the events queue
    pub fn process_events(&mut self, scale_factor: f32) {
        process_events(
            &self.sdom.get(),
            &mut self.events,
            &self.event_emitter,
            &mut self.nodes_state,
            scale_factor as f64,
        )
    }

    /// Create the Accessibility tree
    /// This will iterater the DOM ordered by layers (top to bottom)
    /// and add every element with an accessibility ID to the Accessibility Tree
    pub fn process_accessibility(&mut self) {
        let fdom = &self.sdom.get();
        let layout = fdom.layout();
        let rdom = fdom.rdom();

        process_accessibility(
            &layout,
            rdom,
            &mut self.accessibility.accessibility_manager().lock().unwrap(),
        );
    }

    /// Send an event
    pub fn send_event(&mut self, event: PlatformEvent, scale_factor: f32) {
        self.events.push(event);
        self.process_events(scale_factor);
    }

    /// Replace a VirtualDOM Template
    pub fn vdom_replace_template(&mut self, template: Template) {
        self.vdom.replace_template(template);
    }

    /// Render the App into the Window Canvas
    pub fn render(&mut self, hovered_node: &HoveredNode, canvas: &Canvas, window: &Window) {
        self.plugins.send(PluginEvent::BeforeRender {
            canvas,
            font_collection: &self.font_collection,
            freya_dom: &self.sdom.get(),
        });

        self.start_render(hovered_node, canvas, window.scale_factor() as f32);

        self.accessibility
            .render_accessibility(window.title().as_str());

        self.plugins.send(PluginEvent::AfterRender {
            canvas,
            font_collection: &self.font_collection,
            freya_dom: &self.sdom.get(),
        });
    }

    /// Resize the Window
    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.measure_layout_on_next_render = true;
        self.sdom.get().layout().reset();
        *self.platform_information.lock().unwrap() = PlatformInformation::from_winit(size);
    }

    /// Measure the a text group given it's ID.
    pub fn measure_text_group(&self, text_id: &Uuid, scale_factor: f32) {
        self.sdom.get().measure_paragraphs(text_id, scale_factor);
    }

    pub fn focus_next_node(&mut self, direction: AccessibilityFocusDirection, window: &Window) {
        self.accessibility
            .focus_next_node(direction, &self.focus_sender, window)
    }

    /// Notify components subscribed to event loop ticks.
    pub fn event_loop_tick(&self) {
        self.ticker_sender.send(()).ok();
    }

    /// Update the [NavigationMode].
    pub fn set_navigation_mode(&mut self, mode: NavigationMode) {
        self.navigator_state.set(mode);
    }

    /// Measure the layout
    pub fn process_layout(&mut self, inner_size: PhysicalSize<u32>, scale_factor: f32) {
        self.accessibility.clear_accessibility();

        {
            let fdom = self.sdom.get();

            self.plugins
                .send(PluginEvent::StartedLayout(&fdom.layout()));

            process_layout(
                &fdom,
                Area::from_size(Size2D::from((
                    inner_size.width as f32,
                    inner_size.height as f32,
                ))),
                &mut self.font_collection,
                scale_factor,
                &self.default_fonts,
            );

            self.plugins
                .send(PluginEvent::FinishedLayout(&fdom.layout()));
        }

        if let Some(mutations_notifier) = &self.mutations_notifier {
            mutations_notifier.notify_one();
        }

        self.process_accessibility();

        let fdom = self.sdom.get();
        info!(
            "Processed {} layers and {} group of paragraph elements",
            fdom.layers().len_layers(),
            fdom.paragraphs().len_paragraphs()
        );
    }

    /// Start rendering the RealDOM to Window
    pub fn start_render(&mut self, hovered_node: &HoveredNode, canvas: &Canvas, scale_factor: f32) {
        let fdom = self.sdom.get();

        let mut matrices: Vec<(Matrix, Vec<NodeId>)> = Vec::default();
        let mut opacities: Vec<(f32, Vec<NodeId>)> = Vec::default();

        process_render(
            &fdom,
            &mut self.font_collection,
            |fdom, node_id, area, font_collection, layout| {
                let render_wireframe = if let Some(hovered_node) = &hovered_node {
                    hovered_node
                        .lock()
                        .unwrap()
                        .map(|id| id == *node_id)
                        .unwrap_or_default()
                } else {
                    false
                };
                if let Some(dioxus_node) = fdom.rdom().get(*node_id) {
                    render_skia(
                        canvas,
                        area,
                        &dioxus_node,
                        font_collection,
                        &self.font_mgr,
                        render_wireframe,
                        &mut matrices,
                        &mut opacities,
                        &self.default_fonts,
                        layout,
                        scale_factor,
                    );
                }
            },
        );
    }
}
