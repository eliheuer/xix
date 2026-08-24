// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A whole desktop screen, with no numbers in it.
//!
//! This is the shortest argument for xix. It is a three-pane application
//! shell: a header with a save state, a sidebar of sections and rows, a
//! content area, an inspector of fields, and a status bar. It is the shape
//! of a font editor, a node editor, an asset browser, or an admin tool.
//!
//! Every spacing, control size, radius, border, and type value comes from
//! [`xilem::kernel`]. `Space::Md`, not 8. `ControlSize::Row`, not 22.
//! `TextSize::Body`, not 12. Three numbers remain in the file (two pane
//! widths and a thumbnail height), and they are the whole of the app's own
//! geometry. That is the point: a screen written this way cannot drift,
//! because there is nothing to drift to.
//! An author who wants a gap picks a step; there is no value between the
//! steps to pick by accident, so a hundred independent choices still land
//! on one rhythm.
//!
//! The color roles work the same way. A view asks for `Role::Accent`, not
//! for a hex value, so a theme is data and the screen restyles without an
//! edit. See `tokens.rs` for that idea driven live by sliders.
//!
//! Try it: change `Space::Md` to `Space::Lg` on line one of `sidebar` and
//! watch the whole column re-space consistently. Then try to make it look
//! wrong without leaving the vocabulary.

use masonry::kernel::{ControlSize, Radius, Space, Stroke, TextSize};
use masonry::layout::Dim;
use masonry::properties::Dimensions;
use masonry::properties::types::CrossAxisAlignment;
use masonry::theme::default_property_set;
use winit::dpi::LogicalSize;
use winit::error::EventLoopError;
use xilem::style::Style;
use xilem::view::{
    FlexExt as _, FlexSpacer, button, flex_col, flex_row, label, portal, sized_box, text_input,
};
use xilem::{Color, EventLoop, EventLoopBuilder, WidgetView, WindowOptions, Xilem};

// ---------------------------------------------------------------- theme

/// Color by role, never by value. One dark theme here; a real app loads
/// these from data and the screen below does not change.
#[derive(Clone, Copy)]
enum Role {
    App,
    Panel,
    Field,
    Line,
    Text,
    Muted,
    Accent,
    Warning,
}

fn color(role: Role) -> Color {
    use masonry::peniko::color::{AlphaColor, Oklch, Srgb};
    let oklch = |l: f32, c: f32, h: f32| AlphaColor::<Oklch>::new([l, c, h, 1.0]).convert::<Srgb>();
    match role {
        Role::App => oklch(0.17, 0.005, 260.0),
        Role::Panel => oklch(0.21, 0.006, 260.0),
        Role::Field => oklch(0.26, 0.008, 260.0),
        Role::Line => oklch(0.34, 0.010, 260.0),
        Role::Text => oklch(0.94, 0.004, 260.0),
        Role::Muted => oklch(0.68, 0.012, 260.0),
        Role::Accent => oklch(0.78, 0.150, 155.0),
        Role::Warning => oklch(0.83, 0.140, 85.0),
    }
}

// ---------------------------------------------------------------- state

struct App {
    selected: usize,
    search: String,
    name: String,
    width: String,
    dirty: bool,
}

/// The sidebar contents: a section title, then rows of (label, count).
const SECTIONS: [(&str, &[(&str, &str)]); 3] = [
    (
        "Library",
        &[
            ("All assets", "862"),
            ("Recent", "24"),
            ("Shared with me", "7"),
        ],
    ),
    (
        "Collections",
        &[("Marketing", "128"), ("Product", "310"), ("Archive", "424")],
    ),
    ("Filters", &[("Missing alt text", "12"), ("Oversized", "3")]),
];

// ---------------------------------------------------------------- parts

/// A section header: caption type, muted.
///
/// It would carry a disclosure caret, but the framework ships no icon set
/// and the default font stack has no coverage for the geometric shapes
/// block, so a typed `\u{25be}` renders as tofu. Icons are framework
/// furniture, and their absence is why apps reach for Unicode.
fn section_header(text: &'static str) -> impl WidgetView<App> + use<> {
    label(text)
        .text_size(TextSize::Caption)
        .color(color(Role::Muted))
}

/// A list row: label left, count right, accent outline when it is current.
fn list_row(index: usize, text: &'static str, count: &'static str, active: bool) -> impl WidgetView<App> + use<> {
    let (fg, border) = if active {
        (color(Role::Accent), color(Role::Accent))
    } else {
        (color(Role::Text), Color::TRANSPARENT)
    };
    sized_box(
        button(
            flex_row((
                label(text).text_size(TextSize::Body).color(fg),
                FlexSpacer::Flex(1.0),
                label(count).text_size(TextSize::Body).color(color(Role::Muted)),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Center),
            move |app: &mut App| app.selected = index,
        )
        .background_color(color(Role::Panel))
        .border_color(border)
        .border_width(Stroke::Hairline)
        .corner_radius(Radius::Sm),
    )
    .dims(Dimensions::new(Dim::Stretch, Dim::from(ControlSize::Row)))
}

/// A labeled field: caption over a control-height input.
fn field<F>(name: &'static str, value: String, set: F) -> impl WidgetView<App> + use<F>
where
    F: Fn(&mut App, String) + Send + Sync + 'static,
{
    flex_col((
        label(name).text_size(TextSize::Caption).color(color(Role::Muted)),
        sized_box(
            text_input(value, set)
                .background_color(color(Role::Field))
                .corner_radius(Radius::Sm),
        )
        .dims(Dimensions::new(Dim::Stretch, Dim::from(ControlSize::Control))),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(Space::Xs)
}

/// A read-only row: name left, value right.
fn kv(name: &'static str, value: &'static str) -> impl WidgetView<App> + use<> {
    sized_box(
        flex_row((
            label(name).text_size(TextSize::Body).color(color(Role::Muted)),
            FlexSpacer::Flex(1.0),
            label(value).text_size(TextSize::Body).color(color(Role::Text)),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Center),
    )
    .dims(Dimensions::new(Dim::Stretch, Dim::from(ControlSize::Row)))
}

/// A push button at control height.
fn action(text: &'static str, primary: bool) -> impl WidgetView<App> + use<> {
    let (bg, fg) = if primary {
        (color(Role::Accent), color(Role::App))
    } else {
        (color(Role::Field), color(Role::Text))
    };
    sized_box(
        button(
            label(text).text_size(TextSize::Body).color(fg),
            move |app: &mut App| app.dirty = !primary,
        )
        .background_color(bg)
        .corner_radius(Radius::Md),
    )
    .dims(Dimensions::new(Dim::Auto, Dim::from(ControlSize::Control)))
}

/// A card in the content grid.
fn card(title: &'static str, meta: &'static str, accent: Role) -> impl WidgetView<App> + use<> {
    sized_box(
        flex_col((
            sized_box(label(""))
                .dims(Dimensions::new(Dim::Stretch, Dim::Fixed(masonry::layout::Length::px(72.0))))
                .background_color(color(accent))
                .corner_radius(Radius::Sm),
            label(title).text_size(TextSize::Body).color(color(Role::Text)),
            label(meta).text_size(TextSize::Caption).color(color(Role::Muted)),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(Space::Xs)
        .padding(Space::Md)
        .background_color(color(Role::Panel))
        .corner_radius(Radius::Lg),
    )
    .dims(Dimensions::new(Dim::Fixed(masonry::layout::Length::px(168.0)), Dim::Auto))
}

// ---------------------------------------------------------------- panes

fn header(app: &App) -> impl WidgetView<App> + use<> {
    let (state, state_color) = if app.dirty {
        ("Not saved", color(Role::Warning))
    } else {
        ("Saved", color(Role::Muted))
    };
    flex_row((
        label("Asset library").text_size(TextSize::Title).color(color(Role::Text)),
        FlexSpacer::Flex(1.0),
        label(state).text_size(TextSize::Body).color(state_color),
        action("Import", false),
        action("Publish", true),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .gap(Space::Md)
    .padding(Space::Md)
    .background_color(color(Role::Panel))
}

fn sidebar(app: &App) -> impl WidgetView<App> + use<> {
    let mut index = 0;
    let sections: Vec<_> = SECTIONS
        .iter()
        .map(|(title, rows)| {
            let rows: Vec<_> = rows
                .iter()
                .map(|(text, count)| {
                    let row = list_row(index, text, count, index == app.selected);
                    index += 1;
                    row
                })
                .collect();
            flex_col((
                section_header(title),
                flex_col(rows)
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .gap(Space::Xs),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(Space::Sm)
        })
        .collect();
    flex_col((
        sized_box(
            text_input(app.search.clone(), |app: &mut App, v| app.search = v)
                .placeholder("Search")
                .background_color(color(Role::Field))
                .corner_radius(Radius::Sm),
        )
        .dims(Dimensions::new(Dim::Stretch, Dim::from(ControlSize::Control))),
        portal(
            flex_col(sections)
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(Space::Lg),
        )
        .flex(1.0),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(Space::Md)
    .padding(Space::Md)
    .background_color(color(Role::Panel))
}

fn content() -> impl WidgetView<App> + use<> {
    let row_one = flex_row((
        card("Hero banner", "3200 x 1400", Role::Accent),
        card("Product shot", "2048 x 2048", Role::Warning),
        card("Team photo", "1600 x 900", Role::Line),
    ))
    .gap(Space::Md);
    let row_two = flex_row((
        card("Icon set", "48 files", Role::Line),
        card("Typography", "12 files", Role::Accent),
        card("Brand marks", "9 files", Role::Warning),
    ))
    .gap(Space::Md);
    portal(
        flex_col((row_one, row_two))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(Space::Md)
            .padding(Space::Lg),
    )
}

fn inspector(app: &App) -> impl WidgetView<App> + use<> {
    flex_col((
        section_header("Asset"),
        field("Name", app.name.clone(), |app: &mut App, v| {
            app.name = v;
            app.dirty = true;
        }),
        field("Width", app.width.clone(), |app: &mut App, v| {
            app.width = v;
            app.dirty = true;
        }),
        kv("Format", "PNG"),
        kv("Size", "4.2 MB"),
        kv("Modified", "Today"),
        section_header("Publishing"),
        kv("Channel", "Web"),
        kv("Status", "Draft"),
        action("Preview", false),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(Space::Md)
    .padding(Space::Lg)
    .background_color(color(Role::Panel))
}

fn status() -> impl WidgetView<App> + use<> {
    flex_row((
        label("862 assets").text_size(TextSize::Caption).color(color(Role::Muted)),
        FlexSpacer::Flex(1.0),
        label("Connected").text_size(TextSize::Caption).color(color(Role::Muted)),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .padding(Space::Md)
    .background_color(color(Role::Panel))
}

fn app_logic(app: &mut App) -> impl WidgetView<App> + use<> {
    let center = flex_col((header(app), content().flex(1.0), status()))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(Space::None)
        .background_color(color(Role::App));
    flex_row((
        sized_box(sidebar(app))
            .dims(Dimensions::new(
                Dim::Fixed(masonry::layout::Length::px(232.0)),
                Dim::Stretch,
            ))
            .background_color(color(Role::Panel)),
        sized_box(center)
            .dims(Dimensions::new(Dim::Stretch, Dim::Stretch))
            .flex(1.0),
        sized_box(inspector(app))
            .dims(Dimensions::new(
                Dim::Fixed(masonry::layout::Length::px(264.0)),
                Dim::Stretch,
            ))
            .background_color(color(Role::Panel)),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(Space::None)
    .background_color(color(Role::App))
}

pub(crate) fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let app = App {
        selected: 0,
        search: String::new(),
        name: "hero-banner".into(),
        width: "3200".into(),
        dirty: false,
    };
    let window_options = WindowOptions::new("Asset library")
        .with_initial_inner_size(LogicalSize::new(1080., 680.));
    Xilem::new_simple(app, app_logic, window_options)
        .with_default_properties(default_property_set())
        .with_default_base_color(color(Role::App))
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
