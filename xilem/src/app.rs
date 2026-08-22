// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::iter::Once;
use std::sync::Arc;

use masonry::core::DefaultProperties;
use masonry::peniko::{Blob, Color};
use masonry::theme::{BACKGROUND_COLOR, default_property_set};
#[cfg(not(target_arch = "wasm32"))]
use masonry_winit::app::{MasonryState, MasonryUserEvent, NewWindow};
use tokio::runtime::Runtime as TokioRuntime;
use winit::error::EventLoopError;

use crate::core::map_state;
use crate::window_options::WindowCallbacks;
#[cfg(not(target_arch = "wasm32"))]
use crate::MasonryDriver;
use crate::{EventLoopBuilder, WidgetView, WindowId, WindowOptions, WindowView};

/// The tokio runtime the app hands to views. Multi-threaded natively, current-thread on the web.
fn new_runtime() -> TokioRuntime {
    #[cfg(not(target_arch = "wasm32"))]
    {
        TokioRuntime::new().unwrap()
    }
    #[cfg(target_arch = "wasm32")]
    {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
    }
}

// TODO - Rename to `AppLauncher` or something.

/// Runtime builder.
#[must_use = "A Xilem app does nothing unless ran."]
pub struct Xilem<State, Logic> {
    state: State,
    logic: Logic,
    runtime: Arc<TokioRuntime>,
    default_properties: Option<DefaultProperties>,
    default_base_color: Color,
    // Font data to include in loading.
    fonts: Vec<Blob<u8>>,
    // Callback invoked once on startup, after windows creation.
    #[cfg(not(target_arch = "wasm32"))]
    on_start: Option<Box<dyn FnOnce(&mut MasonryState<'_>)>>,
}

/// State type used by [`Xilem::new_simple`].
pub struct ExitOnClose<S> {
    state: S,
    running: bool,
}

impl<S> AppState for ExitOnClose<S> {
    fn keep_running(&self) -> bool {
        self.running
    }
}

impl<State>
    Xilem<
        ExitOnClose<State>,
        Box<dyn FnMut(&mut ExitOnClose<State>) -> Once<WindowView<ExitOnClose<State>>>>,
    >
{
    /// Create an app builder for a single window app with fixed window attributes
    /// that exits once the window is closed.
    ///
    /// If you want to have multiple windows or change e.g. the window title depending
    /// on the state you should instead use [`Xilem::new`] (which this function wraps).
    pub fn new_simple<View>(
        state: State,
        logic: impl FnMut(&mut State) -> View + 'static,
        window_options: WindowOptions<State>,
    ) -> Self
    where
        View: WidgetView<State>,
    {
        Self::new_simple_with_tokio(
            state,
            logic,
            window_options,
            Arc::new(new_runtime()),
        )
    }

    /// Same as [`Self::new_simple`] but allows passing an existing tokio runtime.
    pub fn new_simple_with_tokio<View>(
        state: State,
        mut logic: impl FnMut(&mut State) -> View + 'static,
        window_options: WindowOptions<State>,
        tokio_rt: Arc<TokioRuntime>,
    ) -> Self
    where
        View: WidgetView<State>,
    {
        let window_id = WindowId::next();
        let callbacks = Arc::new(window_options.callbacks);
        Xilem::new_inner(
            ExitOnClose {
                state,
                running: true,
            },
            Box::new(move |ExitOnClose { state, .. }| {
                let callbacks = callbacks.clone();
                let on_close = move |wrapper: &mut ExitOnClose<_>| {
                    wrapper.running = false;
                    if let Some(on_close) = &callbacks.on_close {
                        on_close(&mut wrapper.state);
                    }
                };
                std::iter::once(
                    crate::window(
                        window_id,
                        String::new(),
                        map_state(logic(state), |wrapper: &mut ExitOnClose<_>| {
                            &mut wrapper.state
                        }),
                    )
                    .with_options(|_| WindowOptions {
                        reactive: window_options.reactive.clone(),
                        initial: window_options.initial.clone(),
                        callbacks: WindowCallbacks {
                            on_close: Some(Box::new(on_close)),
                        },
                    }),
                )
            }),
            tokio_rt,
        )
    }
}

/// The trait [`Xilem::new`] expects to be implemented for the state.
///
/// [`Xilem::new_simple`] does not use this trait implementation.
pub trait AppState {
    /// Returns whether the application should keep running or exit.
    ///
    /// Checked after every state mutation (actions, close requests, etc.).
    fn keep_running(&self) -> bool;
}

impl<State, Logic, WindowIter> Xilem<State, Logic>
where
    State: AppState + 'static,
    Logic: FnMut(&mut State) -> WindowIter + 'static,
    WindowIter: Iterator<Item = WindowView<State>>,
{
    /// Initialize the builder state for your app with an app logic function that returns a window iterator.
    pub fn new(state: State, logic: Logic) -> Self
    where
        State: AppState,
    {
        Self::new_inner(state, logic, Arc::new(new_runtime()))
    }

    fn new_inner(state: State, logic: Logic, runtime: Arc<TokioRuntime>) -> Self {
        Self {
            state,
            logic,
            runtime,
            default_properties: None,
            default_base_color: BACKGROUND_COLOR,
            fonts: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            on_start: None,
        }
    }

    /// Load a font when this `Xilem` is run.
    ///
    /// This is an interim API whilst font lifecycles are determined.
    pub fn with_font(mut self, data: impl Into<Blob<u8>>) -> Self {
        self.fonts.push(data.into());
        self
    }

    // TODO: Find better ways to customize default property set.
    /// Sets default properties of widget tree.
    pub fn with_default_properties(mut self, default_properties: DefaultProperties) -> Self {
        self.default_properties = Some(default_properties);
        self
    }

    // TODO: Find better ways to customize default base color.
    /// Sets default base color of windows.
    pub fn with_default_base_color(mut self, default_base_color: Color) -> Self {
        self.default_base_color = default_base_color;
        self
    }

    /// Registers a callback to be called once the application has started
    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_on_start(mut self, callback: impl FnOnce(&mut MasonryState<'_>) + 'static) -> Self {
        self.on_start = Some(Box::new(callback));
        self
    }

    /// Run app with custom window attributes.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn run_in(mut self, mut event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
        #[cfg(feature = "screenshot")]
        if let Ok(path) = std::env::var("XIX_SCREENSHOT") {
            self.screenshot(std::path::Path::new(&path))
                .expect("XIX_SCREENSHOT: headless render failed");
            return Ok(());
        }
        let event_loop = event_loop.build()?;
        let proxy = event_loop.create_proxy();
        let default_properties = self
            .default_properties
            .take()
            .unwrap_or_else(default_property_set);
        let (driver, windows) =
            self.into_driver_and_windows(move |event| proxy.send_event(event).map_err(|err| err.0));
        masonry_winit::app::run_with(event_loop, windows, driver, default_properties)
    }

    /// On the web there is no event loop to run in. The app is mounted into the canvas
    /// registered by [`crate::web::mount`]; see that function.
    #[cfg(target_arch = "wasm32")]
    pub fn run_in(self, _event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
        #[cfg(feature = "web")]
        {
            crate::web::run(self);
            Ok(())
        }
        #[cfg(not(feature = "web"))]
        {
            panic!("xilem on wasm32 needs the `web` feature")
        }
    }

    /// Build the first window's root view. Used by the headless and web runners.
    #[cfg(any(feature = "screenshot", all(target_arch = "wasm32", feature = "web")))]
    pub(crate) fn into_root_parts(
        mut self,
    ) -> (
        State,
        Logic,
        Arc<TokioRuntime>,
        DefaultProperties,
        WindowView<State>,
        Vec<Blob<u8>>,
    ) {
        let default_properties = self
            .default_properties
            .take()
            .unwrap_or_else(default_property_set);
        let mut windows = (self.logic)(&mut self.state);
        let mut window_view = windows.next().expect("the app produced no windows");
        drop(windows);
        window_view
            .base_color
            .get_or_insert(self.default_base_color);
        (
            self.state,
            self.logic,
            self.runtime,
            default_properties,
            window_view,
            self.fonts,
        )
    }

    /// Render the app's first window once, headless, and write it to `path` as a PNG.
    ///
    /// This is the `XIX_SCREENSHOT` path used by `run_in`. It builds the view tree
    /// exactly as the driver would, hosts the root widget in a
    /// [`masonry_testing::TestHarness`], rasterizes one frame with Vello CPU, and
    /// exits without creating a window or an event loop. Size and background come
    /// from the window's options, falling back to 400 x 400 and the default base color.
    #[cfg(feature = "screenshot")]
    pub fn screenshot(mut self, path: &std::path::Path) -> Result<(), image::ImageError> {
        use masonry_testing::{TestHarness, TestHarnessParams};
        use winit::dpi::PhysicalSize;
        use xilem_core::View as _;
        use xilem_masonry::{InitialRootWidget, ViewCtx};

        #[derive(Debug)]
        struct NopProxy;
        impl xilem_core::RawProxy for NopProxy {
            fn send_message(
                &self,
                _path: Arc<[xilem_core::ViewId]>,
                _message: xilem_core::SendMessage,
            ) -> Result<(), xilem_core::ProxyError> {
                Ok(())
            }
            fn dyn_debug(&self) -> &dyn std::fmt::Debug {
                self
            }
        }

        let default_properties = self
            .default_properties
            .take()
            .unwrap_or_else(default_property_set);
        let mut windows = (self.logic)(&mut self.state);
        let window_view = windows
            .next()
            .expect("XIX_SCREENSHOT: the app produced no windows");
        let size = window_view
            .options
            .initial
            .inner_size
            .map(|size| size.to_physical::<u32>(1.0))
            .unwrap_or(PhysicalSize::new(400, 400));
        let background = window_view.base_color.unwrap_or(self.default_base_color);

        let mut ctx = ViewCtx::new(Arc::new(NopProxy), self.runtime.clone());
        let (InitialRootWidget(pod), _view_state) =
            window_view.masonry_root.build(&mut ctx, &mut self.state);

        let mut params = TestHarnessParams::default().with_size(size);
        params.background_color = background;
        let mut harness = TestHarness::create_with(default_properties, pod.new_widget, params);
        let image = harness.render();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        image.save(path)
    }

    /// Builds the [`MasonryDriver`] and the initial windows.
    ///
    /// The given event sink function sends the given event to the event loop
    /// and returns the given event as an error in case the event loop is stopped.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn into_driver_and_windows(
        self,
        proxy: impl Fn(MasonryUserEvent) -> Result<(), MasonryUserEvent> + Send + Sync + 'static,
    ) -> (MasonryDriver<State, Logic>, Vec<NewWindow>) {
        MasonryDriver::new(
            self.state,
            self.logic,
            proxy,
            self.runtime,
            self.default_base_color,
            self.fonts,
            self.on_start,
        )
    }
}
