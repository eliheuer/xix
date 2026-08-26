// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Run a Xilem app inside an HTML `<canvas>`.
//!
//! This is a small browser driver for the Masonry widget tree. It does not use
//! winit or the GPU. The root view is built the same way the native driver builds
//! it, hosted in a [`RenderRoot`], fed pointer and keyboard events from the DOM,
//! rasterized with Vello CPU, and blitted into the canvas with `putImageData`.
//! Every current browser can run it; nothing is needed beyond the canvas.
//!
//! Use it from an example built as a `cdylib`:
//!
//! ```ignore
//! #[cfg(target_arch = "wasm32")]
//! #[xilem::wasm_bindgen::prelude::wasm_bindgen]
//! pub fn mount(canvas: xilem::web_sys::HtmlCanvasElement) {
//!     xilem::web::mount(canvas, || run(EventLoop::with_user_event()));
//! }
//! ```
//!
//! `run` is the same function the native build uses. Its call to
//! [`Xilem::run_in`](crate::Xilem::run_in) lands here instead of in winit.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use imaging_vello_cpu::VelloCpuRenderer;
use masonry::app::{
    RenderRoot, RenderRootOptions, RenderRootSignal, VisualLayerKind, WindowSizePolicy,
};
use masonry::core::keyboard::{Code, Key, KeyState};
use masonry::core::{
    ErasedAction, KeyboardEvent, Modifiers, PointerButton, PointerButtonEvent, PointerEvent,
    PointerId, PointerInfo, PointerScrollEvent, PointerState, PointerType, PointerUpdate,
    ScrollDelta, TextEvent, WidgetId, WindowEvent,
};
use masonry::dpi::{PhysicalPosition, PhysicalSize};
use masonry::imaging::record::{Scene, replay_transformed};
use masonry::imaging::{ImageRenderer as _, Painter};
use masonry::kurbo::{Affine, Rect};
use masonry::peniko::{Blob, Color};
use masonry::util::Duration;
use wasm_bindgen::prelude::*;
use wasm_bindgen::{Clamped, JsCast};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use crate::core::{
    DynMessage, MessageCtx, MessageResult, ProxyError, RawProxy, SendMessage, View, ViewId,
    ViewPathTracker as _,
};
use crate::window_view::WindowViewState;
use crate::{AppState, InitialRootWidget, ViewCtx, WindowView, Xilem};

/// A Latin fallback font, because the browser exposes no system fonts to wasm.
// TODO(xix): let the app register its own fonts and drop this.
const FALLBACK_FONT: &[u8] =
    include_bytes!("../../masonry_testing/resources/fonts/roboto/Roboto-Regular.ttf");

thread_local! {
    static MOUNT_CANVAS: RefCell<Option<HtmlCanvasElement>> = const { RefCell::new(None) };
}

/// Mount an app into `canvas`.
///
/// `run` must build the app and call [`Xilem::run_in`](crate::Xilem::run_in), exactly as
/// the native entry point does. The canvas is sized to its CSS box times the device
/// pixel ratio, receives focus on click, and follows the size of its box after that.
pub fn mount(
    canvas: HtmlCanvasElement,
    run: impl FnOnce() -> Result<(), winit::error::EventLoopError>,
) {
    console_error_panic_hook::set_once();
    MOUNT_CANVAS.with(|slot| *slot.borrow_mut() = Some(canvas));
    run().expect("xilem web: app failed to start");
}

/// Entry point used by `Xilem::run_in` on wasm32. Takes the canvas set by [`mount`].
pub(crate) fn run<State, Logic, WindowIter>(app: Xilem<State, Logic>)
where
    State: AppState + 'static,
    Logic: FnMut(&mut State) -> WindowIter + 'static,
    WindowIter: Iterator<Item = WindowView<State>>,
{
    let canvas = MOUNT_CANVAS
        .with(|slot| slot.borrow_mut().take())
        .expect("xilem web: call `xilem::web::mount` with a canvas first");
    WebApp::start(app, canvas);
}

/// Messages from views to the app, sent through a proxy and delivered on the next frame.
type MessageQueue = Arc<Mutex<VecDeque<(Arc<[ViewId]>, SendMessage)>>>;

#[derive(Debug)]
struct WebProxy(MessageQueue);

impl RawProxy for WebProxy {
    fn send_message(&self, path: Arc<[ViewId]>, message: SendMessage) -> Result<(), ProxyError> {
        self.0.lock().unwrap().push_back((path, message));
        Ok(())
    }
    fn dyn_debug(&self) -> &dyn std::fmt::Debug {
        self
    }
}

const PRIMARY_MOUSE: PointerInfo = PointerInfo {
    pointer_id: Some(PointerId::PRIMARY),
    persistent_device_id: None,
    pointer_type: PointerType::Mouse,
};

struct WebApp<State: 'static, Logic> {
    state: State,
    logic: Logic,
    window_view: WindowView<State>,
    view_ctx: ViewCtx,
    view_state: WindowViewState,
    render_root: RenderRoot,
    signals: Rc<RefCell<VecDeque<RenderRootSignal>>>,
    messages: MessageQueue,
    renderer: VelloCpuRenderer,
    canvas: HtmlCanvasElement,
    context: CanvasRenderingContext2d,
    background: Color,
    size: PhysicalSize<u32>,
    scale: f64,
    pointer: PointerState,
    needs_redraw: bool,
    last_frame_ms: f64,
}

impl<State, Logic, WindowIter> WebApp<State, Logic>
where
    State: AppState + 'static,
    Logic: FnMut(&mut State) -> WindowIter + 'static,
    WindowIter: Iterator<Item = WindowView<State>>,
{
    fn start(app: Xilem<State, Logic>, canvas: HtmlCanvasElement) {
        let (mut state, logic, runtime, default_properties, window_view, fonts) =
            app.into_root_parts();

        let messages: MessageQueue = Arc::default();
        let mut view_ctx = ViewCtx::new(Arc::new(WebProxy(messages.clone())), runtime);
        let (InitialRootWidget(pod), view_state) =
            window_view.masonry_root.build(&mut view_ctx, &mut state);

        let (size, scale) = canvas_size(&canvas);
        canvas.set_width(size.width);
        canvas.set_height(size.height);
        let context = canvas
            .get_context("2d")
            .ok()
            .flatten()
            .expect("xilem web: canvas has no 2d context")
            .dyn_into::<CanvasRenderingContext2d>()
            .expect("xilem web: not a 2d context");

        let signals: Rc<RefCell<VecDeque<RenderRootSignal>>> = Rc::default();
        let sink = signals.clone();
        let mut render_root = RenderRoot::new(
            pod.new_widget,
            move |signal| sink.borrow_mut().push_back(signal),
            RenderRootOptions {
                default_properties: Arc::new(default_properties),
                use_system_fonts: false,
                size_policy: WindowSizePolicy::User,
                size,
                scale_factor: scale,
                test_font: Some(Blob::new(Arc::new(FALLBACK_FONT))),
            },
        );
        for font in fonts {
            drop(render_root.register_fonts(font));
        }
        let fallback = render_root.get_layer_root(0).id();
        render_root.set_focus_fallback(Some(fallback));
        render_root.handle_text_event(TextEvent::WindowFocusChange(true));

        let background = window_view
            .base_color
            .unwrap_or(masonry::theme::BACKGROUND_COLOR);

        let app = Rc::new(RefCell::new(Self {
            state,
            logic,
            window_view,
            view_ctx,
            view_state,
            render_root,
            signals,
            messages,
            renderer: VelloCpuRenderer::new(1, 1),
            canvas: canvas.clone(),
            context,
            background,
            size,
            scale,
            pointer: PointerState::default(),
            needs_redraw: true,
            last_frame_ms: js_sys::Date::now(),
        }));

        install_listeners(&app, &canvas);
        start_frame_loop(app);
    }

    // ---- messages and rebuilds (mirrors MasonryDriver) ----

    fn dispatch_message(&mut self, id_path: Vec<ViewId>, message: DynMessage) -> MessageResult<()> {
        let mut message_context = MessageCtx::new(
            std::mem::take(self.view_ctx.environment()),
            id_path,
            message,
        );
        let res = self.window_view.masonry_root.message(
            &mut self.view_state,
            &mut message_context,
            &mut self.render_root,
            &mut self.state,
        );
        let (env, _id_path, _message) = message_context.finish();
        *self.view_ctx.environment() = env;
        res
    }

    fn handle_message_result(&mut self, result: MessageResult<()>) {
        match result {
            MessageResult::Action(()) => self.run_logic(),
            MessageResult::RequestRebuild => {
                self.window_view.masonry_root.rebuild(
                    &self.window_view.masonry_root,
                    &mut self.view_state,
                    &mut self.view_ctx,
                    &mut self.render_root,
                    &mut self.state,
                );
            }
            MessageResult::Nop => {}
            MessageResult::Stale => tracing::info!("Discarding message"),
        }
    }

    fn run_logic(&mut self) {
        let Some(next_view) = (self.logic)(&mut self.state).next() else {
            return;
        };
        next_view.masonry_root.rebuild(
            &self.window_view.masonry_root,
            &mut self.view_state,
            &mut self.view_ctx,
            &mut self.render_root,
            &mut self.state,
        );
        if let Some(color) = next_view.base_color
            && color != self.background
        {
            self.background = color;
            self.needs_redraw = true;
        }
        if let Some(props) = &next_view.default_properties
            && self
                .window_view
                .default_properties
                .as_ref()
                .is_none_or(|p| !Arc::ptr_eq(p, props))
        {
            self.render_root.set_default_properties(props.clone());
        }
        self.window_view = next_view;
    }

    fn on_action(&mut self, widget_id: WidgetId, action: ErasedAction) {
        let Some(path) = self.view_ctx.get_id_path(widget_id) else {
            tracing::error!("Got action {action:?} for unknown widget");
            return;
        };
        let id_path = path.clone();
        let result = self.dispatch_message(id_path, DynMessage(action));
        self.handle_message_result(result);
    }

    fn drain_messages(&mut self) {
        loop {
            let next = self.messages.lock().unwrap().pop_front();
            let Some((path, message)) = next else {
                break;
            };
            let result = self.dispatch_message(Vec::from(&*path), message.into());
            self.handle_message_result(result);
        }
    }

    fn process_signals(&mut self) {
        loop {
            let next = self.signals.borrow_mut().pop_front();
            let Some(signal) = next else {
                break;
            };
            match signal {
                RenderRootSignal::Action(action, widget_id) => self.on_action(widget_id, action),
                RenderRootSignal::RequestRedraw | RenderRootSignal::RequestAnimFrame => {
                    self.needs_redraw = true;
                }
                RenderRootSignal::SetCursor(icon) => {
                    let _ = self.canvas.style().set_property("cursor", icon.name());
                }
                RenderRootSignal::ClipboardStore(_text) => {
                    // TODO(xix): navigator.clipboard.writeText; needs the Clipboard web-sys feature.
                }
                RenderRootSignal::Exit => {
                    // Nothing to exit on the web; leave the last frame on the canvas.
                }
                _ => {}
            }
        }
    }

    // ---- frames ----

    fn frame(&mut self, now_ms: f64) {
        let (size, scale) = canvas_size(&self.canvas);
        if size != self.size || scale != self.scale {
            self.size = size;
            self.scale = scale;
            self.canvas.set_width(size.width);
            self.canvas.set_height(size.height);
            self.render_root
                .handle_window_event(WindowEvent::Rescale(scale));
            self.render_root
                .handle_window_event(WindowEvent::Resize(size));
            self.needs_redraw = true;
        }
        let dt = Duration::from_secs_f64(((now_ms - self.last_frame_ms) / 1000.0).max(0.0));
        self.last_frame_ms = now_ms;
        if self.render_root.needs_anim() {
            self.render_root
                .handle_window_event(WindowEvent::AnimFrame(dt));
        }
        self.drain_messages();
        self.process_signals();
        if self.render_root.needs_rewrite_passes() {
            self.needs_redraw = true;
        }
        if self.needs_redraw {
            self.needs_redraw = false;
            self.redraw();
            // Redraw may emit more signals (cursor, anim); pick them up next frame.
            self.process_signals();
        }
    }

    fn redraw(&mut self) {
        let (visual_layers, _tree_update) = self.render_root.redraw();
        let width = self.size.width.max(1);
        let height = self.size.height.max(1);

        let mut scene = Scene::new();
        {
            let mut painter = Painter::new(&mut scene);
            painter.fill_rect(
                Rect::new(0.0, 0.0, f64::from(width), f64::from(height)),
                self.background,
            );
            for layer in &visual_layers.layers {
                if let VisualLayerKind::Scene(layer_scene) = &layer.kind {
                    replay_transformed(layer_scene, &mut scene, Affine::IDENTITY * layer.transform);
                }
            }
        }
        let image = self
            .renderer
            .render_source(&mut scene, width, height)
            .expect("xilem web: render failed");
        let data = ImageDataExt::new(Clamped(&image.data), image.width, image.height)
            .expect("xilem web: ImageData");
        drop(self.context.put_image_data(&data, 0.0, 0.0));
    }

    // ---- input ----

    fn pointer_position(&self, event: &web_sys::MouseEvent) -> PhysicalPosition<f64> {
        let rect = self.canvas.get_bounding_client_rect();
        PhysicalPosition {
            x: (f64::from(event.client_x()) - rect.left()) * self.scale,
            y: (f64::from(event.client_y()) - rect.top()) * self.scale,
        }
    }

    fn update_pointer(&mut self, event: &web_sys::MouseEvent) {
        self.pointer.position = self.pointer_position(event);
        self.pointer.modifiers = modifiers_of(
            event.shift_key(),
            event.ctrl_key(),
            event.alt_key(),
            event.meta_key(),
        );
    }

    fn on_pointer_move(&mut self, event: &web_sys::PointerEvent) {
        self.update_pointer(event);
        self.render_root
            .handle_pointer_event(PointerEvent::Move(PointerUpdate {
                pointer: PRIMARY_MOUSE,
                current: self.pointer.clone(),
                coalesced: vec![],
                predicted: vec![],
            }));
    }

    fn on_pointer_down(&mut self, event: &web_sys::PointerEvent) {
        self.update_pointer(event);
        let button = button_of(event.button());
        if let Some(button) = button {
            self.pointer.buttons.insert(button);
        }
        self.render_root
            .handle_pointer_event(PointerEvent::Down(PointerButtonEvent {
                pointer: PRIMARY_MOUSE,
                button,
                state: self.pointer.clone(),
            }));
    }

    fn on_pointer_up(&mut self, event: &web_sys::PointerEvent) {
        self.update_pointer(event);
        let button = button_of(event.button());
        if let Some(button) = button {
            self.pointer.buttons.remove(button);
        }
        self.render_root
            .handle_pointer_event(PointerEvent::Up(PointerButtonEvent {
                pointer: PRIMARY_MOUSE,
                button,
                state: self.pointer.clone(),
            }));
    }

    fn on_pointer_leave(&mut self) {
        self.render_root
            .handle_pointer_event(PointerEvent::Leave(PRIMARY_MOUSE));
    }

    fn on_wheel(&mut self, event: &web_sys::WheelEvent) {
        self.update_pointer(event);
        // DOM_DELTA_LINE (1) and DOM_DELTA_PAGE (2) are scaled to pixels.
        let unit = match event.delta_mode() {
            1 => 20.0,
            2 => f64::from(self.size.height),
            _ => 1.0,
        };
        let delta = PhysicalPosition {
            x: -event.delta_x() * unit * self.scale,
            y: -event.delta_y() * unit * self.scale,
        };
        self.render_root
            .handle_pointer_event(PointerEvent::Scroll(PointerScrollEvent {
                pointer: PRIMARY_MOUSE,
                delta: ScrollDelta::PixelDelta(delta),
                state: self.pointer.clone(),
            }));
    }

    fn on_key(&mut self, event: &web_sys::KeyboardEvent, state: KeyState) {
        let key = match Key::from_str(&event.key()) {
            Ok(key) => key,
            Err(_) => Key::Character(event.key()),
        };
        let code = Code::from_str(&event.code()).unwrap_or(Code::Unidentified);
        let keyboard = KeyboardEvent {
            state,
            key,
            code,
            modifiers: modifiers_of(
                event.shift_key(),
                event.ctrl_key(),
                event.alt_key(),
                event.meta_key(),
            ),
            repeat: event.repeat(),
            ..KeyboardEvent::default()
        };
        self.render_root
            .handle_text_event(TextEvent::Keyboard(keyboard));
    }
}

/// `ImageData` construction is spelled differently across web-sys versions; keep it in one place.
struct ImageDataExt;
impl ImageDataExt {
    fn new(data: Clamped<&[u8]>, width: u32, height: u32) -> Result<web_sys::ImageData, JsValue> {
        web_sys::ImageData::new_with_u8_clamped_array_and_sh(data, width, height)
    }
}

fn canvas_size(canvas: &HtmlCanvasElement) -> (PhysicalSize<u32>, f64) {
    let scale = web_sys::window()
        .map(|w| w.device_pixel_ratio())
        .unwrap_or(1.0)
        .max(0.5);
    let rect = canvas.get_bounding_client_rect();
    let width = (rect.width() * scale).round().max(1.0) as u32;
    let height = (rect.height() * scale).round().max(1.0) as u32;
    (PhysicalSize::new(width, height), scale)
}

fn modifiers_of(shift: bool, ctrl: bool, alt: bool, meta: bool) -> Modifiers {
    let mut m = Modifiers::empty();
    m.set(Modifiers::SHIFT, shift);
    m.set(Modifiers::CONTROL, ctrl);
    m.set(Modifiers::ALT, alt);
    m.set(Modifiers::META, meta);
    m
}

fn button_of(button: i16) -> Option<PointerButton> {
    match button {
        0 => Some(PointerButton::Primary),
        1 => Some(PointerButton::Auxiliary),
        2 => Some(PointerButton::Secondary),
        3 => Some(PointerButton::X1),
        4 => Some(PointerButton::X2),
        _ => None,
    }
}

fn install_listeners<State, Logic, WindowIter>(
    app: &Rc<RefCell<WebApp<State, Logic>>>,
    canvas: &HtmlCanvasElement,
) where
    State: AppState + 'static,
    Logic: FnMut(&mut State) -> WindowIter + 'static,
    WindowIter: Iterator<Item = WindowView<State>>,
{
    // The canvas must be focusable to receive keyboard events.
    if canvas.get_attribute("tabindex").is_none() {
        drop(canvas.set_attribute("tabindex", "0"));
    }
    drop(canvas.style().set_property("touch-action", "none"));

    macro_rules! listen {
        ($event:literal, $ty:ty, |$app:ident, $ev:ident| $body:block) => {{
            let $app = app.clone();
            let closure = Closure::<dyn FnMut($ty)>::new(move |$ev: $ty| $body);
            canvas
                .add_event_listener_with_callback($event, closure.as_ref().unchecked_ref())
                .expect("xilem web: add listener");
            closure.forget();
        }};
    }

    listen!("pointermove", web_sys::PointerEvent, |app, ev| {
        app.borrow_mut().on_pointer_move(&ev);
    });
    listen!("pointerdown", web_sys::PointerEvent, |app, ev| {
        ev.prevent_default();
        if let Some(target) = ev
            .target()
            .and_then(|t| t.dyn_into::<HtmlCanvasElement>().ok())
        {
            drop(target.set_pointer_capture(ev.pointer_id()));
            drop(target.focus());
        }
        app.borrow_mut().on_pointer_down(&ev);
    });
    listen!("pointerup", web_sys::PointerEvent, |app, ev| {
        if let Some(target) = ev
            .target()
            .and_then(|t| t.dyn_into::<HtmlCanvasElement>().ok())
        {
            drop(target.release_pointer_capture(ev.pointer_id()));
        }
        app.borrow_mut().on_pointer_up(&ev);
    });
    listen!("pointerleave", web_sys::PointerEvent, |app, _ev| {
        app.borrow_mut().on_pointer_leave();
    });
    listen!("wheel", web_sys::WheelEvent, |app, ev| {
        ev.prevent_default();
        app.borrow_mut().on_wheel(&ev);
    });
    listen!("keydown", web_sys::KeyboardEvent, |app, ev| {
        if ev.key() != "F5" && ev.key() != "F12" {
            ev.prevent_default();
        }
        app.borrow_mut().on_key(&ev, KeyState::Down);
    });
    listen!("keyup", web_sys::KeyboardEvent, |app, ev| {
        app.borrow_mut().on_key(&ev, KeyState::Up);
    });
    listen!("contextmenu", web_sys::Event, |_app, ev| {
        ev.prevent_default();
    });
}

fn start_frame_loop<State, Logic, WindowIter>(app: Rc<RefCell<WebApp<State, Logic>>>)
where
    State: AppState + 'static,
    Logic: FnMut(&mut State) -> WindowIter + 'static,
    WindowIter: Iterator<Item = WindowView<State>>,
{
    let window = web_sys::window().expect("xilem web: no window");
    let callback: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let callback_handle = callback.clone();
    let window_handle = window.clone();
    *callback.borrow_mut() = Some(Closure::new(move |now_ms: f64| {
        app.borrow_mut().frame(now_ms);
        if let Some(cb) = callback_handle.borrow().as_ref() {
            drop(window_handle.request_animation_frame(cb.as_ref().unchecked_ref()));
        }
    }));
    if let Some(cb) = callback.borrow().as_ref() {
        drop(window.request_animation_frame(cb.as_ref().unchecked_ref()));
    }
}
