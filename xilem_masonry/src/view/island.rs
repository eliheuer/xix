// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::marker::PhantomData;
use std::sync::Arc;

use masonry::core::ArcStr;
use masonry::imaging::Painter;
use masonry::kurbo::Size;
use masonry::widgets::{self, IslandEvent, Viewport};

use crate::core::{MessageCtx, MessageResult, Mut, View, ViewMarker};
use crate::{Pod, ViewCtx};

/// A retained, pannable, zoomable drawing surface that reports gestures in
/// canvas coordinates.
///
/// This is the primitive under a glyph canvas, a node graph, a timeline,
/// and a map. The island owns the view (where the canvas is, how far it is
/// zoomed, which pointer gestures are navigation). The application owns the
/// document, and sees only the gestures that are about content, already
/// converted into its own coordinates.
///
/// `scene` draws the canvas. It runs on every repaint, including the
/// repaints a pan or a zoom causes, so it should draw from data it holds
/// rather than compute. Apply [`Viewport::affine`] to canvas geometry.
///
/// `on_event` receives content gestures as [`IslandEvent`].
///
/// # Example
///
/// ```
/// # use xilem_masonry as xilem;
/// use xilem::masonry::kurbo::{Circle, Point};
/// use xilem::masonry::widgets::IslandEvent;
/// use xilem::view::island;
/// # use xilem::WidgetView;
///
/// struct App {
///     dot: Point,
/// }
///
/// # fn dot_island() -> impl WidgetView<App> {
/// island(
///     // The scene closure takes a snapshot of the document, so a pan or
///     // a zoom repaints without touching application state.
///     |app: &App| {
///         let dot = app.dot;
///         move |painter: &mut xilem::masonry::imaging::Painter<'_>, viewport: &_, _size| {
///             let _ = (dot, viewport, painter);
///         }
///     },
///     // Content gestures arrive in canvas coordinates.
///     |app: &mut App, event| {
///         if let IslandEvent::Down { pos, .. } = event {
///             app.dot = pos;
///         }
///     },
/// )
/// # }
/// ```
pub fn island<State, Scene, Paint, OnEvent>(
    scene: Scene,
    on_event: OnEvent,
) -> Island<State, Scene, Paint, OnEvent>
where
    State: 'static,
    Scene: Fn(&State) -> Paint + Send + Sync + 'static,
    Paint: Fn(&mut Painter<'_>, &Viewport, Size) + Send + Sync + 'static,
    OnEvent: Fn(&mut State, IslandEvent) + Send + Sync + 'static,
{
    Island {
        scene: Arc::new(scene),
        on_event,
        viewport: None,
        flip_y: false,
        alt_text: None,
        phantom: PhantomData,
    }
}

/// The [`View`] created by [`island`].
#[must_use = "View values do nothing unless provided to Xilem."]
pub struct Island<State, Scene, Paint, OnEvent> {
    scene: Arc<Scene>,
    on_event: OnEvent,
    viewport: Option<Viewport>,
    flip_y: bool,
    alt_text: Option<ArcStr>,
    phantom: PhantomData<fn() -> (State, Paint)>,
}

impl<State, Scene, Paint, OnEvent> Island<State, Scene, Paint, OnEvent> {
    /// Sets the viewport, which otherwise the island owns.
    ///
    /// Use this when the application drives the view, for example from a
    /// zoom control or a "fit" command.
    pub fn viewport(mut self, viewport: Viewport) -> Self {
        self.viewport = Some(viewport);
        self
    }

    /// Makes canvas y increase upward, as design and chart spaces do.
    pub fn flipped_y(mut self, flip_y: bool) -> Self {
        self.flip_y = flip_y;
        self
    }

    /// Sets alt text for the island's contents.
    pub fn alt_text(mut self, alt_text: impl Into<ArcStr>) -> Self {
        self.alt_text = Some(alt_text.into());
        self
    }
}

impl<State, Scene, Paint, OnEvent> ViewMarker for Island<State, Scene, Paint, OnEvent> {}

impl<State, Action, Scene, Paint, OnEvent> View<State, Action, ViewCtx>
    for Island<State, Scene, Paint, OnEvent>
where
    State: 'static,
    Action: 'static,
    Scene: Fn(&State) -> Paint + Send + Sync + 'static,
    Paint: Fn(&mut Painter<'_>, &Viewport, Size) + Send + Sync + 'static,
    OnEvent: Fn(&mut State, IslandEvent) + Send + Sync + 'static,
{
    type Element = Pod<widgets::Island>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, state: &mut State) -> (Self::Element, Self::ViewState) {
        let paint = (self.scene)(state);
        (
            ctx.with_action_widget(|ctx| {
                let mut widget = widgets::Island::new(paint).with_flipped_y(self.flip_y);
                if let Some(viewport) = self.viewport {
                    widget = widget.with_viewport(viewport);
                }
                if let Some(alt_text) = &self.alt_text {
                    widget = widget.with_alt_text(alt_text.clone());
                }
                ctx.create_pod(widget)
            }),
            (),
        )
    }

    fn rebuild(
        &self,
        prev: &Self,
        (): &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        state: &mut State,
    ) {
        // The scene closure captures a snapshot of the document, so a
        // rebuild is how new content reaches the island.
        widgets::Island::set_paint(&mut element, (self.scene)(state));
        if let Some(viewport) = self.viewport
            && prev.viewport != Some(viewport)
        {
            widgets::Island::set_viewport(&mut element, viewport);
        }
        if self.alt_text != prev.alt_text {
            widgets::Island::set_alt_text(&mut element, self.alt_text.clone());
        }
    }

    fn teardown(&self, (): &mut Self::ViewState, _: &mut ViewCtx, _: Mut<'_, Self::Element>) {}

    fn message(
        &self,
        (): &mut Self::ViewState,
        message: &mut MessageCtx,
        _element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        debug_assert!(
            message.remaining_path().is_empty(),
            "id path should be empty in Island::message"
        );
        match message.take_message::<IslandEvent>() {
            Some(event) => {
                (self.on_event)(app_state, *event);
                MessageResult::RequestRebuild
            }
            None => {
                tracing::error!("Wrong message type in Island::message, got {message:?}.");
                MessageResult::Stale
            }
        }
    }
}
