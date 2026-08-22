// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Design tokens, live.
//!
//! Three controls (density, radius, accent hue) restyle a small form.
//! The form's code names tokens, never numbers; the sliders change the
//! theme the tokens resolve against. This is the shortest demonstration
//! of the xix idea: design is a type, and a theme is data.

use masonry::layout::Length;
use masonry::properties::Dimensions;
use masonry::properties::types::CrossAxisAlignment;
use masonry::theme::default_property_set;
use winit::dpi::LogicalSize;
use winit::error::EventLoopError;
use xilem::style::Style;
use xilem::view::{FlexExt as _, button, flex_col, flex_row, label, slider, text_input};
use xilem::{Color, EventLoop, EventLoopBuilder, WidgetView, WindowOptions, Xilem};

/// The theme: three numbers the user moves. Everything visible derives from it.
struct Theme {
    density: f64, // 0 compact .. 1 comfortable
    radius: f64,  // 0 square .. 1 round
    hue: f64,     // degrees
}

/// Tokens resolve against the theme. A view asks for `Space::M`; the theme says how many pixels.
#[derive(Clone, Copy)]
enum Space {
    S,
    M,
    L,
}

#[derive(Clone, Copy)]
enum Role {
    Surface,
    Field,
    Text,
    TextMuted,
    Accent,
}

impl Theme {
    fn space(&self, space: Space) -> Length {
        let step = 4.0 + 4.0 * self.density; // 4 pt at compact, 8 pt at comfortable
        let steps = match space {
            Space::S => 1.0,
            Space::M => 2.0,
            Space::L => 3.0,
        };
        Length::px(step * steps)
    }

    fn radius(&self) -> Length {
        Length::px(2.0 + 14.0 * self.radius)
    }

    fn color(&self, role: Role) -> Color {
        use masonry::peniko::color::{AlphaColor, Oklch, Srgb};
        let oklch = |l: f32, c: f32, h: f32| -> Color {
            AlphaColor::<Oklch>::new([l, c, h, 1.0]).convert::<Srgb>()
        };
        let hue = self.hue as f32;
        match role {
            Role::Surface => oklch(0.22, 0.01, hue),
            Role::Field => oklch(0.30, 0.015, hue),
            Role::Text => oklch(0.96, 0.01, hue),
            Role::TextMuted => oklch(0.70, 0.02, hue),
            Role::Accent => oklch(0.68, 0.16, hue),
        }
    }
}

struct App {
    theme: Theme,
    name: String,
    email: String,
}

fn control<F: Fn(&mut App, f64) + Send + Sync + 'static>(
    title: &'static str,
    value: f64,
    theme: &Theme,
    set: F,
) -> impl WidgetView<App> + use<F> {
    flex_row((
        label(title)
            .color(theme.color(Role::TextMuted))
            .dims(Dimensions::width(Length::px(64.0))),
        slider(0.0, 1.0, value, set).flex(1.0),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .gap(theme.space(Space::S))
}

fn field<F: Fn(&mut App, String) + Send + Sync + 'static>(
    title: &'static str,
    text: &str,
    theme: &Theme,
    set: F,
) -> impl WidgetView<App> + use<F> {
    flex_col((
        label(title).color(theme.color(Role::TextMuted)),
        text_input(text.to_string(), set)
            .background_color(theme.color(Role::Field))
            .corner_radius(theme.radius())
            .padding(theme.space(Space::S)),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(theme.space(Space::S))
}

fn card(app: &App) -> impl WidgetView<App> + use<> {
    let t = &app.theme;
    flex_col((
        label("New account").text_size(20.0).color(t.color(Role::Text)),
        field("Name", &app.name, t, |app: &mut App, v| app.name = v),
        field("Email", &app.email, t, |app: &mut App, v| app.email = v),
        flex_row((
            button(label("Cancel").color(t.color(Role::Text)), |_: &mut App| {})
                .background_color(t.color(Role::Field))
                .corner_radius(t.radius())
                .padding(t.space(Space::M)),
            button(label("Create").color(t.color(Role::Text)), |_: &mut App| {})
                .background_color(t.color(Role::Accent))
                .corner_radius(t.radius())
                .padding(t.space(Space::M)),
        ))
        .gap(t.space(Space::S)),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(t.space(Space::M))
    .padding(t.space(Space::L))
    .background_color(t.color(Role::Surface))
    .corner_radius(t.radius())
}

fn app_logic(app: &mut App) -> impl WidgetView<App> + use<> {
    let t = &app.theme;
    flex_col((
        control("Density", t.density, t, |app, v| app.theme.density = v),
        control("Radius", t.radius, t, |app, v| app.theme.radius = v),
        control("Hue", t.hue / 360.0, t, |app, v| app.theme.hue = v * 360.0),
        card(app),
    ))
    .gap(t.space(Space::M))
    .padding(t.space(Space::L))
}

pub(crate) fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let app = App {
        theme: Theme {
            density: 0.5,
            radius: 0.5,
            hue: 250.0,
        },
        name: String::new(),
        email: String::new(),
    };
    let window_options = WindowOptions::new("Design tokens")
        .with_initial_inner_size(LogicalSize::new(360., 420.));
    Xilem::new_simple(app, app_logic, window_options)
        .with_default_properties(default_property_set())
        .with_default_base_color(Color::from_rgb8(0x15, 0x15, 0x17))
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
