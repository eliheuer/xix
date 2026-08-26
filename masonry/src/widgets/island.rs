// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A scene island: a retained, pannable, zoomable drawing surface.

use accesskit::{Node, Role};

use crate::core::{
    AccessCtx, ArcStr, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, PaintCtx, PointerButton,
    PointerButtonEvent, PointerEvent, PointerScrollEvent, PointerUpdate, PropertiesMut,
    PropertiesRef, RegisterCtx, ScrollDelta, Widget, WidgetMut,
};
use crate::imaging::Painter;
use crate::kurbo::{Affine, Axis, Point, Rect, Size, Vec2};
use crate::layout::{LenReq, Length};

/// The preferred size of an island with nothing telling it otherwise.
const DEFAULT_LENGTH: Length = Length::const_px(320.);

/// How far the view moves per line of scroll.
const LINE_SCROLL: f64 = 20.0;

/// A view onto an unbounded canvas: where it is, and how big it is drawn.
///
/// Canvas space is the application's own coordinate system, in whatever
/// units the document uses. Screen space is logical pixels inside the
/// island. The island owns this, so panning and zooming cost the
/// application nothing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    /// Screen-space position of the canvas origin.
    pub offset: Vec2,
    /// Screen pixels per canvas unit.
    pub zoom: f64,
    /// Whether canvas y increases upward (design space) or downward (screen).
    pub flip_y: bool,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            offset: Vec2::ZERO,
            zoom: 1.0,
            flip_y: false,
        }
    }
}

impl Viewport {
    /// The smallest and largest zoom a gesture can reach.
    pub const MIN_ZOOM: f64 = 0.02;
    /// See [`Self::MIN_ZOOM`].
    pub const MAX_ZOOM: f64 = 256.0;

    /// The canvas-to-screen transform. Apply it to anything you draw.
    pub fn affine(&self) -> Affine {
        let y = if self.flip_y { -self.zoom } else { self.zoom };
        Affine::new([self.zoom, 0.0, 0.0, y, self.offset.x, self.offset.y])
    }

    /// Screen point to canvas point.
    pub fn to_canvas(&self, screen: Point) -> Point {
        self.affine().inverse() * screen
    }

    /// Canvas point to screen point.
    pub fn to_screen(&self, canvas: Point) -> Point {
        self.affine() * canvas
    }

    /// Move the view by a screen-space delta.
    pub fn pan(&mut self, delta: Vec2) {
        self.offset += delta;
    }

    /// Zoom by `factor` while holding `anchor` (a screen point) still.
    ///
    /// This is what a scroll wheel and a pinch both want: the thing under
    /// the pointer does not move.
    pub fn zoom_about(&mut self, anchor: Point, factor: f64) {
        let before = self.to_canvas(anchor);
        self.zoom = (self.zoom * factor).clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
        let after = self.to_canvas(anchor);
        let shift = after - before;
        self.offset += Vec2::new(
            shift.x * self.zoom,
            shift.y * if self.flip_y { -self.zoom } else { self.zoom },
        );
    }

    /// Frame `content` inside a `size` island, leaving `pad` screen pixels.
    pub fn fit(content: Rect, size: Size, pad: f64, flip_y: bool) -> Self {
        let mut view = Self {
            flip_y,
            ..Self::default()
        };
        let (w, h) = (content.width().max(1e-6), content.height().max(1e-6));
        let zoom = ((size.width - pad * 2.0) / w)
            .min((size.height - pad * 2.0) / h)
            .clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
        view.zoom = zoom;
        let center = content.center();
        view.offset = Vec2::new(
            size.width / 2.0 - center.x * zoom,
            size.height / 2.0 - center.y * if flip_y { -zoom } else { zoom },
        );
        view
    }
}

/// What the island reports to the application, in canvas coordinates.
///
/// The island handles the view (panning, zooming, cursor tracking). The
/// application handles the document, and only sees gestures that are
/// about content.
#[derive(Clone, Copy, Debug)]
pub enum IslandEvent {
    /// A button went down at this canvas position.
    Down {
        /// Where, in canvas space.
        pos: Point,
        /// Which button, if the platform said.
        button: Option<PointerButton>,
        /// Whether shift was held.
        shift: bool,
        /// Whether the platform's command modifier was held.
        command: bool,
        /// Whether alt was held.
        alt: bool,
    },
    /// The pointer moved with no button down.
    Hover {
        /// Where, in canvas space.
        pos: Point,
    },
    /// The pointer moved with the primary button down.
    Drag {
        /// Where, in canvas space.
        pos: Point,
        /// How far it moved since the last report, in canvas units.
        delta: Vec2,
    },
    /// The primary button came up.
    Up {
        /// Where, in canvas space.
        pos: Point,
    },
    /// The island panned or zoomed itself.
    ViewportChanged,
}

/// The paint callback. It gets the transform to apply, and the island size.
type PaintFn = Box<dyn Fn(&mut Painter<'_>, &Viewport, Size) + Send + Sync>;

/// A retained, pannable, zoomable drawing surface that reports gestures in
/// canvas coordinates.
///
/// An island is the primitive under a font editor's glyph canvas, a node
/// editor's graph, a video editor's timeline, and a map. Each of those
/// otherwise reimplements the same three things: a viewport, a screen-to-
/// content coordinate conversion, and a gesture stream that has already
/// had panning and zooming taken out of it.
///
/// The island owns the view state. The application owns the document. A
/// pan or a zoom repaints without the application being told anything it
/// has to act on, and every event the application does see is in its own
/// coordinates.
///
/// Built-in gestures:
///
/// - Middle-drag pans.
/// - Scroll pans; scroll with the command modifier zooms about the pointer.
///
/// Everything else is reported through [`IslandEvent`].
pub struct Island {
    paint: PaintFn,
    viewport: Viewport,
    size: Size,
    alt_text: Option<ArcStr>,
    /// Last screen position seen, for drag deltas.
    last: Option<Point>,
    /// True while the middle button is panning the view.
    panning: bool,
    /// True while the primary button is down on content.
    dragging: bool,
}

// --- MARK: BUILDERS
impl Island {
    /// Creates an island that draws with `paint`.
    ///
    /// The callback runs on every repaint, including the ones a pan or a
    /// zoom causes, so it should draw from data it already holds rather
    /// than compute. Apply [`Viewport::affine`] to canvas-space geometry.
    pub fn new(paint: impl Fn(&mut Painter<'_>, &Viewport, Size) + Send + Sync + 'static) -> Self {
        Self {
            paint: Box::new(paint),
            viewport: Viewport::default(),
            size: Size::ZERO,
            alt_text: None,
            last: None,
            panning: false,
            dragging: false,
        }
    }

    /// Sets the starting viewport.
    pub fn with_viewport(mut self, viewport: Viewport) -> Self {
        self.viewport = viewport;
        self
    }

    /// Makes canvas y increase upward, as design and chart spaces do.
    pub fn with_flipped_y(mut self, flip_y: bool) -> Self {
        self.viewport.flip_y = flip_y;
        self
    }

    /// Sets the text that describes the island to screen readers.
    pub fn with_alt_text(mut self, alt_text: impl Into<ArcStr>) -> Self {
        self.alt_text = Some(alt_text.into());
        self
    }
}

// --- MARK: METHODS
impl Island {
    /// The current viewport.
    pub fn viewport(&self) -> Viewport {
        self.viewport
    }

    /// The island's size, in logical pixels.
    pub fn size(&self) -> Size {
        self.size
    }
}

// --- MARK: WIDGETMUT
impl Island {
    /// Replaces the paint callback, and repaints.
    pub fn set_paint(
        this: &mut WidgetMut<'_, Self>,
        paint: impl Fn(&mut Painter<'_>, &Viewport, Size) + Send + Sync + 'static,
    ) {
        this.widget.paint = Box::new(paint);
        this.ctx.request_render();
    }

    /// Sets the viewport, and repaints.
    pub fn set_viewport(this: &mut WidgetMut<'_, Self>, viewport: Viewport) {
        if this.widget.viewport != viewport {
            this.widget.viewport = viewport;
            this.ctx.request_render();
        }
    }

    /// Frames `content` in the island.
    pub fn fit(this: &mut WidgetMut<'_, Self>, content: Rect, pad: f64) {
        let flip_y = this.widget.viewport.flip_y;
        let viewport = Viewport::fit(content, this.widget.size, pad, flip_y);
        Self::set_viewport(this, viewport);
    }

    /// Sets the alt text.
    pub fn set_alt_text(this: &mut WidgetMut<'_, Self>, alt_text: Option<impl Into<ArcStr>>) {
        this.widget.alt_text = alt_text.map(Into::into);
        this.ctx.request_accessibility_update();
    }
}

// --- MARK: IMPL WIDGET
impl Widget for Island {
    type Action = IslandEvent;

    fn accepts_focus(&self) -> bool {
        true
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(
        &mut self,
        _ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        _axis: Axis,
        len_req: LenReq,
        _cross_length: Option<Length>,
    ) -> Length {
        match len_req {
            LenReq::FitContent(space) => space,
            _ => DEFAULT_LENGTH,
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        self.size = size;
        ctx.set_clip_path(size.to_rect());
    }

    fn paint(
        &mut self,
        _ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        painter: &mut Painter<'_>,
    ) {
        (self.paint)(painter, &self.viewport, self.size);
    }

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        match event {
            PointerEvent::Down(PointerButtonEvent { button, state, .. }) => {
                let at = ctx.local_position(state.position);
                self.last = Some(at);
                ctx.request_focus();
                ctx.capture_pointer();
                if *button == Some(PointerButton::Auxiliary) {
                    self.panning = true;
                } else {
                    self.dragging = *button == Some(PointerButton::Primary);
                    ctx.submit_action::<Self::Action>(IslandEvent::Down {
                        pos: self.viewport.to_canvas(at),
                        button: *button,
                        shift: state.modifiers.shift(),
                        command: state.modifiers.meta() || state.modifiers.ctrl(),
                        alt: state.modifiers.alt(),
                    });
                }
                ctx.set_handled();
            }
            PointerEvent::Move(PointerUpdate { current, .. }) => {
                let at = ctx.local_position(current.position);
                let previous = self.last.unwrap_or(at);
                self.last = Some(at);
                if self.panning {
                    self.viewport.pan(at - previous);
                    ctx.submit_action::<Self::Action>(IslandEvent::ViewportChanged);
                    ctx.request_render();
                } else if self.dragging {
                    let delta = self.viewport.to_canvas(at) - self.viewport.to_canvas(previous);
                    ctx.submit_action::<Self::Action>(IslandEvent::Drag {
                        pos: self.viewport.to_canvas(at),
                        delta,
                    });
                } else {
                    ctx.submit_action::<Self::Action>(IslandEvent::Hover {
                        pos: self.viewport.to_canvas(at),
                    });
                }
                ctx.set_handled();
            }
            PointerEvent::Up(PointerButtonEvent { state, .. }) => {
                let at = ctx.local_position(state.position);
                self.last = Some(at);
                if self.panning {
                    self.panning = false;
                } else if self.dragging {
                    self.dragging = false;
                    ctx.submit_action::<Self::Action>(IslandEvent::Up {
                        pos: self.viewport.to_canvas(at),
                    });
                }
                ctx.set_handled();
            }
            PointerEvent::Scroll(PointerScrollEvent { delta, state, .. }) => {
                let at = ctx.local_position(state.position);
                let (dx, dy) = match delta {
                    ScrollDelta::PixelDelta(p) => (p.x, p.y),
                    ScrollDelta::LineDelta(x, y) => {
                        (f64::from(*x) * LINE_SCROLL, f64::from(*y) * LINE_SCROLL)
                    }
                    _ => (0.0, 0.0),
                };
                if state.modifiers.meta() || state.modifiers.ctrl() {
                    self.viewport.zoom_about(at, (dy / 300.0).exp());
                } else {
                    self.viewport.pan(Vec2::new(dx, dy));
                }
                ctx.submit_action::<Self::Action>(IslandEvent::ViewportChanged);
                ctx.request_render();
                ctx.set_handled();
            }
            _ => {}
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::Canvas
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        if let Some(alt_text) = &self.alt_text {
            node.set_description(&**alt_text);
        }
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TestHarness;
    use crate::theme::test_property_set;

    #[test]
    fn zoom_holds_the_anchor_still() {
        let mut viewport = Viewport::default();
        let anchor = Point::new(120.0, 80.0);
        let before = viewport.to_canvas(anchor);
        viewport.zoom_about(anchor, 2.5);
        let after = viewport.to_canvas(anchor);
        assert!((before.x - after.x).abs() < 1e-9, "{before:?} {after:?}");
        assert!((before.y - after.y).abs() < 1e-9, "{before:?} {after:?}");
        assert!((viewport.zoom - 2.5).abs() < 1e-9);
    }

    #[test]
    fn zoom_holds_the_anchor_still_when_y_is_flipped() {
        let mut viewport = Viewport {
            flip_y: true,
            ..Viewport::default()
        };
        let anchor = Point::new(40.0, 200.0);
        let before = viewport.to_canvas(anchor);
        viewport.zoom_about(anchor, 0.4);
        let after = viewport.to_canvas(anchor);
        assert!((before.x - after.x).abs() < 1e-9, "{before:?} {after:?}");
        assert!((before.y - after.y).abs() < 1e-9, "{before:?} {after:?}");
    }

    #[test]
    fn zoom_stays_inside_its_limits() {
        let mut viewport = Viewport::default();
        for _ in 0..100 {
            viewport.zoom_about(Point::ZERO, 4.0);
        }
        assert_eq!(viewport.zoom, Viewport::MAX_ZOOM);
        for _ in 0..100 {
            viewport.zoom_about(Point::ZERO, 0.25);
        }
        assert_eq!(viewport.zoom, Viewport::MIN_ZOOM);
    }

    #[test]
    fn fit_centers_the_content() {
        let content = Rect::new(0.0, 0.0, 100.0, 50.0);
        let size = Size::new(400.0, 300.0);
        let viewport = Viewport::fit(content, size, 20.0, false);
        let center = viewport.to_screen(content.center());
        assert!((center.x - size.width / 2.0).abs() < 1e-9);
        assert!((center.y - size.height / 2.0).abs() < 1e-9);
        // The content fits, with the padding respected on the tight axis.
        let drawn = viewport.zoom * content.width();
        assert!(drawn <= size.width - 40.0 + 1e-9);
    }

    #[test]
    fn a_point_survives_a_round_trip() {
        let viewport = Viewport {
            offset: Vec2::new(31.0, -12.5),
            zoom: 3.25,
            flip_y: true,
        };
        let canvas = Point::new(7.5, -3.25);
        let back = viewport.to_canvas(viewport.to_screen(canvas));
        assert!((canvas.x - back.x).abs() < 1e-9);
        assert!((canvas.y - back.y).abs() < 1e-9);
    }

    /// The application should see its own coordinates, not screen ones,
    /// and it should never see the navigation gestures at all.
    #[test]
    fn events_arrive_in_canvas_coordinates() {
        let island = Island::new(|_, _, _| {})
            .with_viewport(Viewport {
                offset: Vec2::new(100.0, 50.0),
                zoom: 2.0,
                flip_y: false,
            })
            .prepare();
        let mut harness = TestHarness::create_with_size(test_property_set(), island, (400, 300));

        harness.mouse_move(Point::new(140.0, 90.0));
        let hover = harness.pop_action::<IslandEvent>().expect("a hover");
        match hover.0 {
            IslandEvent::Hover { pos } => {
                assert!((pos.x - 20.0).abs() < 1e-9, "{pos:?}");
                assert!((pos.y - 20.0).abs() < 1e-9, "{pos:?}");
            }
            other => panic!("expected a hover, got {other:?}"),
        }

        harness.mouse_button_press(None);
        let down = harness.pop_action::<IslandEvent>().expect("a down");
        match down.0 {
            IslandEvent::Down { pos, .. } => {
                assert!((pos.x - 20.0).abs() < 1e-9, "{pos:?}");
                assert!((pos.y - 20.0).abs() < 1e-9, "{pos:?}");
            }
            other => panic!("expected a down, got {other:?}"),
        }
    }
}
