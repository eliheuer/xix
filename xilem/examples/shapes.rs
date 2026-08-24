// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A small graphics editor, to show what a canvas island is for.
//!
//! Draw rectangles, select them, drag them, edit them in the inspector.
//! Middle-drag or scroll to pan, command-scroll to zoom. The document is a
//! `Vec<Shape>` in application state, and it is the only state this file
//! manages.
//!
//! The point of the example is what is *not* here. There is no viewport
//! math, no screen-to-canvas conversion, no drag state machine, and no
//! decision about which pointer gestures mean navigation. [`island`] owns
//! all of that, and hands the application gestures that are already in
//! document coordinates. In the two ports that led to this fork, that code
//! was about 150 lines per canvas, rewritten per canvas, and it was the
//! largest single source of duplication in both.
//!
//! The same primitive is what a node graph, a timeline, and a map want. A
//! node editor is this file with a different `Shape` and a different paint.
//!
//! Everything else is [`xilem::kernel`] tokens, as in `app_shell.rs`.

use masonry::kernel::{ControlSize, Radius, Space, TextSize};
use masonry::layout::Dim;
use masonry::properties::Dimensions;
use masonry::properties::types::CrossAxisAlignment;
use masonry::theme::default_property_set;
use masonry::widgets::{IslandEvent, Viewport};
use winit::dpi::LogicalSize;
use winit::error::EventLoopError;
use xilem::kurbo::{Line, Point, Rect, Stroke};
use xilem::masonry::imaging::Painter;
use xilem::style::Style;
use xilem::view::{
    FlexExt as _, FlexSpacer, button, flex_col, flex_row, island, label, sized_box, text_input,
};
use xilem::{Color, EventLoop, EventLoopBuilder, WidgetView, WindowOptions, Xilem};

// ---------------------------------------------------------------- theme

fn oklch(l: f32, c: f32, h: f32) -> Color {
    use masonry::peniko::color::{AlphaColor, Oklch, Srgb};
    AlphaColor::<Oklch>::new([l, c, h, 1.0]).convert::<Srgb>()
}

fn app_bg() -> Color {
    oklch(0.17, 0.005, 260.0)
}
fn panel() -> Color {
    oklch(0.21, 0.006, 260.0)
}
fn field() -> Color {
    oklch(0.26, 0.008, 260.0)
}
fn grid_line() -> Color {
    oklch(0.24, 0.006, 260.0)
}
fn text() -> Color {
    oklch(0.94, 0.004, 260.0)
}
fn muted() -> Color {
    oklch(0.68, 0.012, 260.0)
}
fn accent() -> Color {
    oklch(0.78, 0.150, 155.0)
}

/// The colors a shape can be, as a small closed palette.
const SWATCHES: [(f32, f32); 5] = [
    (0.150, 155.0),
    (0.140, 85.0),
    (0.130, 25.0),
    (0.120, 265.0),
    (0.100, 320.0),
];

fn swatch(index: usize) -> Color {
    let (c, h) = SWATCHES[index % SWATCHES.len()];
    oklch(0.74, c, h)
}

// ------------------------------------------------------------- document

/// One rectangle, in canvas units.
#[derive(Clone, Copy)]
struct Shape {
    rect: Rect,
    color: usize,
}

/// What a drag is currently doing.
#[derive(Clone, Copy, PartialEq)]
enum Gesture {
    None,
    /// Drawing a new rectangle from this corner.
    Draw(Point),
    /// Moving the selection.
    Move,
}

#[derive(Clone, Copy, PartialEq)]
enum Tool {
    Select,
    Draw,
}

struct App {
    /// The document. Everything else on this struct is view state.
    shapes: Vec<Shape>,
    selected: Option<usize>,
    tool: Tool,
    gesture: Gesture,
    /// The live corner of a rectangle being drawn.
    preview: Option<Rect>,
    cursor: Point,
    x: String,
    y: String,
    w: String,
    h: String,
}

impl App {
    fn new() -> Self {
        let shapes = vec![
            Shape {
                rect: Rect::new(40.0, 40.0, 200.0, 140.0),
                color: 0,
            },
            Shape {
                rect: Rect::new(160.0, 120.0, 300.0, 260.0),
                color: 1,
            },
            Shape {
                rect: Rect::new(60.0, 220.0, 140.0, 320.0),
                color: 3,
            },
        ];
        let mut app = Self {
            shapes,
            selected: Some(0),
            tool: Tool::Select,
            gesture: Gesture::None,
            preview: None,
            cursor: Point::ZERO,
            x: String::new(),
            y: String::new(),
            w: String::new(),
            h: String::new(),
        };
        app.refresh_fields();
        app
    }

    fn selection(&self) -> Option<Shape> {
        self.selected.and_then(|i| self.shapes.get(i)).copied()
    }

    fn refresh_fields(&mut self) {
        match self.selection() {
            Some(shape) => {
                self.x = format!("{:.0}", shape.rect.x0);
                self.y = format!("{:.0}", shape.rect.y0);
                self.w = format!("{:.0}", shape.rect.width());
                self.h = format!("{:.0}", shape.rect.height());
            }
            None => {
                self.x.clear();
                self.y.clear();
                self.w.clear();
                self.h.clear();
            }
        }
    }

    /// The topmost shape under a canvas point.
    fn hit(&self, at: Point) -> Option<usize> {
        self.shapes
            .iter()
            .enumerate()
            .rev()
            .find(|(_, shape)| shape.rect.contains(at))
            .map(|(i, _)| i)
    }

    /// Everything the island reports, in canvas coordinates already.
    fn on_canvas(&mut self, event: IslandEvent) {
        match event {
            IslandEvent::Down { pos, .. } => match self.tool {
                Tool::Select => {
                    self.selected = self.hit(pos);
                    self.gesture = if self.selected.is_some() {
                        Gesture::Move
                    } else {
                        Gesture::None
                    };
                    self.refresh_fields();
                }
                Tool::Draw => {
                    self.gesture = Gesture::Draw(pos);
                    self.preview = Some(Rect::from_points(pos, pos));
                }
            },
            IslandEvent::Drag { pos, delta } => match self.gesture {
                Gesture::Draw(origin) => self.preview = Some(Rect::from_points(origin, pos)),
                Gesture::Move => {
                    if let Some(shape) = self.selected.and_then(|i| self.shapes.get_mut(i)) {
                        shape.rect = shape.rect + delta;
                    }
                    self.refresh_fields();
                }
                Gesture::None => {}
            },
            IslandEvent::Up { .. } => {
                if let (Gesture::Draw(_), Some(rect)) = (self.gesture, self.preview)
                    && rect.width() > 2.0
                    && rect.height() > 2.0
                {
                    let color = self.shapes.len();
                    self.shapes.push(Shape { rect, color });
                    self.selected = Some(self.shapes.len() - 1);
                    self.tool = Tool::Select;
                    self.refresh_fields();
                }
                self.gesture = Gesture::None;
                self.preview = None;
            }
            IslandEvent::Hover { pos } => self.cursor = pos,
            IslandEvent::ViewportChanged => {}
        }
    }

    fn set_field(&mut self, which: usize, value: String) {
        match which {
            0 => self.x = value,
            1 => self.y = value,
            2 => self.w = value,
            _ => self.h = value,
        }
        let (Ok(x), Ok(y), Ok(w), Ok(h)) = (
            self.x.trim().parse::<f64>(),
            self.y.trim().parse::<f64>(),
            self.w.trim().parse::<f64>(),
            self.h.trim().parse::<f64>(),
        ) else {
            return;
        };
        if let Some(shape) = self.selected.and_then(|i| self.shapes.get_mut(i)) {
            shape.rect = Rect::new(x, y, x + w.max(1.0), y + h.max(1.0));
        }
    }

    fn delete_selection(&mut self) {
        if let Some(i) = self.selected {
            self.shapes.remove(i);
            self.selected = None;
            self.refresh_fields();
        }
    }
}

// ---------------------------------------------------------------- parts

fn tool_button(text_: &'static str, tool: Tool, active: bool) -> impl WidgetView<App> + use<> {
    let (bg, fg) = if active {
        (accent(), app_bg())
    } else {
        (field(), text())
    };
    sized_box(
        button(label(text_).text_size(TextSize::Body).color(fg), move |app: &mut App| {
            app.tool = tool;
        })
        .background_color(bg)
        .corner_radius(Radius::Md),
    )
    .dims(Dimensions::new(Dim::Auto, Dim::from(ControlSize::Control)))
}

fn field_view(name: &'static str, value: String, which: usize) -> impl WidgetView<App> + use<> {
    flex_col((
        label(name).text_size(TextSize::Caption).color(muted()),
        sized_box(
            text_input(value, move |app: &mut App, v| app.set_field(which, v))
                .background_color(field())
                .corner_radius(Radius::Sm),
        )
        .dims(Dimensions::new(Dim::Stretch, Dim::from(ControlSize::Control))),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(Space::Xs)
}

fn color_row(current: usize) -> impl WidgetView<App> + use<> {
    let swatches: Vec<_> = (0..SWATCHES.len())
        .map(|i| {
            sized_box(
                button(label(""), move |app: &mut App| {
                    if let Some(shape) = app.selected.and_then(|s| app.shapes.get_mut(s)) {
                        shape.color = i;
                    }
                })
                .padding(Space::None)
                .background_color(swatch(i))
                .border_color(if i == current { text() } else { swatch(i) })
                .border_width(masonry::kernel::Stroke::Hairline)
                .corner_radius(Radius::Sm),
            )
            .dims(Dimensions::fixed(ControlSize::Swatch, ControlSize::Swatch))
        })
        .collect();
    flex_row(swatches)
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .gap(Space::Sm)
}

// --------------------------------------------------------------- panes

fn canvas() -> impl WidgetView<App> + use<> {
    island(
        // A snapshot of the document. The island repaints from this on
        // every pan and zoom without asking the application for anything.
        |app: &App| {
            let shapes = app.shapes.clone();
            let selected = app.selected;
            let preview = app.preview;
            move |painter: &mut Painter<'_>, viewport: &Viewport, size| {
                let to_screen = viewport.affine();
                painter.fill_rect(size.to_rect(), app_bg());
                // A canvas-space grid, so panning and zooming are visible.
                let step = 40.0;
                let top_left = viewport.to_canvas(Point::ZERO);
                let bottom_right = viewport.to_canvas(Point::new(size.width, size.height));
                let first_x = (top_left.x / step).floor() * step;
                let first_y = (top_left.y / step).floor() * step;
                let mut x = first_x;
                while x < bottom_right.x {
                    let line = Line::new((x, top_left.y), (x, bottom_right.y));
                    painter
                        .stroke(&(to_screen * line), &Stroke::new(1.0), grid_line())
                        .draw();
                    x += step;
                }
                let mut y = first_y;
                while y < bottom_right.y {
                    let line = Line::new((top_left.x, y), (bottom_right.x, y));
                    painter
                        .stroke(&(to_screen * line), &Stroke::new(1.0), grid_line())
                        .draw();
                    y += step;
                }
                for (i, shape) in shapes.iter().enumerate() {
                    let rect = to_screen.transform_rect_bbox(shape.rect);
                    painter.fill(rect.to_rounded_rect(4.0), swatch(shape.color)).draw();
                    if Some(i) == selected {
                        painter
                            .stroke(&rect.to_rounded_rect(4.0), &Stroke::new(2.0), text())
                            .draw();
                    }
                }
                if let Some(rect) = preview {
                    let rect = to_screen.transform_rect_bbox(rect);
                    painter.stroke(&rect, &Stroke::new(1.0), accent()).draw();
                }
            }
        },
        App::on_canvas,
    )
    .alt_text("Shape canvas")
}

fn header(app: &App) -> impl WidgetView<App> + use<> {
    flex_row((
        tool_button("Select", Tool::Select, app.tool == Tool::Select),
        tool_button("Rectangle", Tool::Draw, app.tool == Tool::Draw),
        FlexSpacer::Flex(1.0),
        label(format!("{} shapes", app.shapes.len()))
            .text_size(TextSize::Body)
            .color(muted()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .gap(Space::Md)
    .padding(Space::Md)
    .background_color(panel())
}

fn inspector(app: &App) -> impl WidgetView<App> + use<> {
    let has_selection = app.selection().is_some();
    flex_col((
        label("Shape").text_size(TextSize::Caption).color(muted()),
        flex_row((
            field_view("X", app.x.clone(), 0).flex(1.0),
            field_view("Y", app.y.clone(), 1).flex(1.0),
        ))
        .gap(Space::Md),
        flex_row((
            field_view("W", app.w.clone(), 2).flex(1.0),
            field_view("H", app.h.clone(), 3).flex(1.0),
        ))
        .gap(Space::Md),
        label("Color").text_size(TextSize::Caption).color(muted()),
        color_row(app.selection().map(|s| s.color).unwrap_or(usize::MAX)),
        has_selection.then(|| {
            sized_box(
                button(
                    label("Delete").text_size(TextSize::Body).color(text()),
                    |app: &mut App| app.delete_selection(),
                )
                .background_color(field())
                .corner_radius(Radius::Md),
            )
            .dims(Dimensions::new(Dim::Auto, Dim::from(ControlSize::Control)))
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(Space::Md)
    .padding(Space::Lg)
    .background_color(panel())
}

fn status(app: &App) -> impl WidgetView<App> + use<> {
    flex_row((
        label("Middle-drag or scroll to pan, command-scroll to zoom")
            .text_size(TextSize::Caption)
            .color(muted()),
        FlexSpacer::Flex(1.0),
        label(format!("{:.0}, {:.0}", app.cursor.x, app.cursor.y))
            .text_size(TextSize::Caption)
            .color(muted()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .padding(Space::Md)
    .background_color(panel())
}

fn app_logic(app: &mut App) -> impl WidgetView<App> + use<> {
    let center = flex_col((header(app), canvas().flex(1.0), status(app)))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(Space::None)
        .background_color(app_bg());
    flex_row((
        sized_box(center)
            .dims(Dimensions::new(Dim::Stretch, Dim::Stretch))
            .flex(1.0),
        sized_box(inspector(app))
            .dims(Dimensions::new(
                Dim::Fixed(masonry::layout::Length::px(240.0)),
                Dim::Stretch,
            ))
            .background_color(panel()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(Space::None)
    .background_color(app_bg())
}

pub(crate) fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let window_options =
        WindowOptions::new("Shapes").with_initial_inner_size(LogicalSize::new(960., 640.));
    Xilem::new_simple(App::new(), app_logic, window_options)
        .with_default_properties(default_property_set())
        .with_default_base_color(app_bg())
        .run_in(event_loop)
}

fn main() -> Result<(), EventLoopError> {
    run(EventLoop::with_user_event())
}

// Boilerplate code for the web: identical across all applications which support the browser.

#[cfg(target_arch = "wasm32")]
#[xilem::wasm_bindgen::prelude::wasm_bindgen]
pub fn mount(canvas: xilem::web_sys::HtmlCanvasElement) {
    xilem::web::mount(canvas, || run(EventLoop::with_user_event()));
}
