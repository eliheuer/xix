// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Design tokens, live.
//!
//! Three sliders (density, roundness, hue) restyle a working panel. The
//! panel's code names tokens and never numbers: `Space::Md`,
//! `ControlSize::Row`, `Radius::Md`, `TextSize::Caption`. The sliders
//! change the theme those tokens resolve against.
//!
//! This is the second half of the kernel idea, and `app_shell.rs` is the
//! first. There, the vocabulary is closed, so a screen cannot drift. Here,
//! the same vocabulary is a level of indirection: one number moves and the
//! whole interface re-spaces, still in proportion, because every gap in it
//! was a step on one scale rather than a pixel count someone chose.
//!
//! A compact density is not a smaller copy of a comfortable one. Text does
//! not shrink with spacing, and hit targets shrink less than gaps do, so
//! the theme resolves each scale differently. That is a thing a theme can
//! know and a stylesheet cannot.
//!
//! What is still missing, and what this example is really testing: the
//! theme is threaded through every function by hand as `&Theme`. It wants
//! to be a context the framework carries, so a view reads the resolved
//! token without being handed the theme. See DESIGN.md D9 and D12.

use masonry::kernel::{ControlSize, Radius, Space, Stroke, TextSize};
use masonry::layout::{Dim, Length};
use masonry::properties::Dimensions;
use masonry::properties::types::CrossAxisAlignment;
use masonry::theme::default_property_set;
use winit::dpi::LogicalSize;
use winit::error::EventLoopError;
use xilem::style::Style;
use xilem::view::{
    FlexExt as _, FlexSpacer, button, flex_col, flex_row, label, sized_box, slider, text_input,
};
use xilem::{Color, EventLoop, EventLoopBuilder, WidgetView, WindowOptions, Xilem};

// ---------------------------------------------------------------- theme

/// Color by role. A view asks for `Role::Accent` and never for a value.
#[derive(Clone, Copy)]
enum Role {
    App,
    Panel,
    Field,
    Text,
    Muted,
    Accent,
}

/// The theme: three numbers, and the rules for resolving the kernel scales
/// against them. This is the whole of what a theme is.
struct Theme {
    /// 0 is compact, 1 is comfortable.
    density: f64,
    /// 0 is square, 1 is round.
    roundness: f64,
    /// Accent hue, in degrees.
    hue: f64,
}

impl Theme {
    /// Spacing scales fully with density: this is what the setting is for.
    fn space(&self, space: Space) -> Length {
        Length::px(space.px() * (0.7 + 0.5 * self.density))
    }

    /// Control heights scale less. A row can tighten, but a hit target
    /// that follows spacing all the way down stops being hittable.
    fn size(&self, size: ControlSize) -> Length {
        let scaled = size.px() * (0.85 + 0.3 * self.density);
        // A control that holds text has a floor the theme does not get to
        // set. Masonry will happily give a text input a box smaller than
        // its own line height and clip the descenders, so the floor lives
        // here. It should live in the widget: see DESIGN.md D13.
        Length::px(scaled.max(f64::from(TextSize::Body.px()) * 2.3))
    }

    /// Corners are their own axis, so a compact theme is not forced to be
    /// a sharp one.
    fn radius(&self, radius: Radius) -> Length {
        Length::px(radius.px() * (0.25 + 1.5 * self.roundness))
    }

    /// Text does not scale with density at all. Legibility is not a
    /// function of how much room the layout has.
    fn text(&self, text: TextSize) -> f32 {
        text.px()
    }

    fn color(&self, role: Role) -> Color {
        use masonry::peniko::color::{AlphaColor, Oklch, Srgb};
        let oklch =
            |l: f32, c: f32, h: f32| AlphaColor::<Oklch>::new([l, c, h, 1.0]).convert::<Srgb>();
        let hue = self.hue as f32;
        match role {
            Role::App => oklch(0.17, 0.006, hue),
            Role::Panel => oklch(0.22, 0.008, hue),
            Role::Field => oklch(0.28, 0.012, hue),
            Role::Text => oklch(0.95, 0.005, hue),
            Role::Muted => oklch(0.70, 0.020, hue),
            Role::Accent => oklch(0.75, 0.150, hue),
        }
    }
}

// ---------------------------------------------------------------- state

struct App {
    theme: Theme,
    name: String,
    email: String,
    plan: usize,
}

const PLANS: [&str; 3] = ["Free", "Team", "Studio"];

// ---------------------------------------------------------------- parts

/// A labeled field: caption over a control-height input.
fn field<F>(t: &Theme, name: &'static str, value: &str, set: F) -> impl WidgetView<App> + use<F>
where
    F: Fn(&mut App, String) + Send + Sync + 'static,
{
    flex_col((
        label(name)
            .text_size(t.text(TextSize::Caption))
            .color(t.color(Role::Muted)),
        sized_box(
            text_input(value.to_string(), set)
                .background_color(t.color(Role::Field))
                .corner_radius(t.radius(Radius::Sm)),
        )
        .dims(Dimensions::new(
            Dim::Stretch,
            Dim::Fixed(t.size(ControlSize::Control)),
        )),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(t.space(Space::Xs))
}

/// A read-only row, at list-row height.
fn kv(t: &Theme, name: &'static str, value: &'static str) -> impl WidgetView<App> + use<> {
    sized_box(
        flex_row((
            label(name)
                .text_size(t.text(TextSize::Body))
                .color(t.color(Role::Muted)),
            FlexSpacer::Flex(1.0),
            label(value)
                .text_size(t.text(TextSize::Body))
                .color(t.color(Role::Text)),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Center),
    )
    .dims(Dimensions::new(
        Dim::Stretch,
        Dim::Fixed(t.size(ControlSize::Row)),
    ))
}

/// A segmented control, one chip per plan.
fn chips(t: &Theme, current: usize) -> impl WidgetView<App> + use<> {
    let chips: Vec<_> = PLANS
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let active = i == current;
            let (bg, fg) = if active {
                (t.color(Role::Accent), t.color(Role::App))
            } else {
                (t.color(Role::Field), t.color(Role::Text))
            };
            sized_box(
                button(
                    label(*name).text_size(t.text(TextSize::Body)).color(fg),
                    move |app: &mut App| app.plan = i,
                )
                .background_color(bg)
                .corner_radius(t.radius(Radius::Md)),
            )
            .dims(Dimensions::new(
                Dim::Auto,
                Dim::Fixed(t.size(ControlSize::Control)),
            ))
        })
        .collect();
    flex_row(chips)
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .gap(t.space(Space::Sm))
}

/// A slider row for one theme number.
fn control<F: Fn(&mut App, f64) + Send + Sync + 'static>(
    t: &Theme,
    title: &'static str,
    value: f64,
    set: F,
) -> impl WidgetView<App> + use<F> {
    flex_row((
        sized_box(
            label(title)
                .text_size(t.text(TextSize::Body))
                .color(t.color(Role::Muted)),
        )
        .dims(Dimensions::new(
            Dim::Fixed(Length::px(72.0)),
            Dim::Fixed(t.size(ControlSize::Row)),
        )),
        slider(0.0, 1.0, value, set).flex(1.0),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .gap(t.space(Space::Md))
}

// ----------------------------------------------------------------- app

/// The panel under test. Not one number in it: every measurement is a
/// token, and the theme above decides what the token is worth.
fn panel(app: &App) -> impl WidgetView<App> + use<> {
    let t = &app.theme;
    flex_col((
        label("Account")
            .text_size(t.text(TextSize::Title))
            .color(t.color(Role::Text)),
        field(t, "Name", &app.name, |app: &mut App, v| app.name = v),
        field(t, "Email", &app.email, |app: &mut App, v| app.email = v),
        label("Plan")
            .text_size(t.text(TextSize::Caption))
            .color(t.color(Role::Muted)),
        chips(t, app.plan),
        kv(t, "Seats", "12"),
        kv(t, "Renews", "1 March"),
        flex_row((
            sized_box(
                button(
                    label("Cancel")
                        .text_size(t.text(TextSize::Body))
                        .color(t.color(Role::Text)),
                    |_: &mut App| {},
                )
                .background_color(t.color(Role::Field))
                .border_color(t.color(Role::Field))
                .border_width(Stroke::Hairline)
                .corner_radius(t.radius(Radius::Md)),
            )
            .dims(Dimensions::new(
                Dim::Auto,
                Dim::Fixed(t.size(ControlSize::Control)),
            )),
            sized_box(
                button(
                    label("Save")
                        .text_size(t.text(TextSize::Body))
                        .color(t.color(Role::App)),
                    |_: &mut App| {},
                )
                .background_color(t.color(Role::Accent))
                .corner_radius(t.radius(Radius::Md)),
            )
            .dims(Dimensions::new(
                Dim::Auto,
                Dim::Fixed(t.size(ControlSize::Control)),
            )),
        ))
        .gap(t.space(Space::Sm)),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(t.space(Space::Md))
    .padding(t.space(Space::Lg))
    .background_color(t.color(Role::Panel))
    .corner_radius(t.radius(Radius::Lg))
}

fn app_logic(app: &mut App) -> impl WidgetView<App> + use<> {
    let t = &app.theme;
    flex_col((
        control(t, "Density", t.density, |app, v| app.theme.density = v),
        control(t, "Roundness", t.roundness, |app, v| {
            app.theme.roundness = v;
        }),
        control(t, "Hue", t.hue / 360.0, |app, v| app.theme.hue = v * 360.0),
        panel(app),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(t.space(Space::Lg))
    .padding(t.space(Space::Lg))
    .background_color(t.color(Role::App))
}

pub(crate) fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let app = App {
        theme: Theme {
            density: 0.5,
            roundness: 0.5,
            hue: 250.0,
        },
        name: "Ada Lovelace".into(),
        email: "ada@example.com".into(),
        plan: 1,
    };
    let background = app.theme.color(Role::App);
    let window_options =
        WindowOptions::new("Design tokens").with_initial_inner_size(LogicalSize::new(420., 560.));
    Xilem::new_simple(app, app_logic, window_options)
        .with_default_properties(default_property_set())
        .with_default_base_color(background)
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
