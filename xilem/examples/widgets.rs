// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Every control, on one rhythm, at three densities.
//!
//! A gallery is usually a catalog: here is a button, here is a checkbox.
//! This one is a test. All of the controls are on screen together, sized
//! from [`xilem::kernel`] and resolved by a theme, and the density switch
//! at the top restyles the whole set at once.
//!
//! That combination is the thing to look at. A catalog tells you a widget
//! exists. A gallery where everything is visible at the same time, on the
//! same scale, tells you whether the set is coherent: whether a button and
//! a text field are the same height, whether a switch and a checkbox sit
//! on the same baseline, whether a row of mixed controls lines up. Those
//! are the defects that make an application look homemade, and they are
//! invisible one widget at a time.
//!
//! It caught its own first defect. The application's accent is green, and
//! every built-in control rendered blue, because the widget defaults were
//! a hardcoded property set rather than the theme. `masonry::theme::Theme`
//! now exists for that reason, and this example builds its widget defaults
//! from the same colors its own views use, so the switch, the slider, the
//! progress bar, the checkbox tick, and the radio dot all follow.
//!
//! One half of that defect is still here, and it is worth seeing: the
//! built-in labels are a size larger than the body text beside them. Color
//! is a property the theme can set for every widget at once; text size is
//! decided by each view at construction, because text style does not
//! inherit down the tree. That is DESIGN.md D11, finding 3, and it is the
//! next thing to fix.
//!
//! It is also the fastest visual review surface in the repo. Render it
//! headlessly and look at the PNG:
//!
//! ```sh
//! XIX_SCREENSHOT=widgets.png cargo run -p xilem --features screenshot --example widgets
//! ```

use masonry::kernel::{ControlSize, Radius, Space, TextSize};
use masonry::layout::{Dim, Length};
use masonry::properties::Dimensions;
use masonry::properties::types::CrossAxisAlignment;
use masonry::theme::Theme as WidgetTheme;
use winit::dpi::LogicalSize;
use winit::error::EventLoopError;
use xilem::style::Style;
use xilem::view::{
    FlexExt as _, FlexSpacer, button, checkbox, flex_col, flex_row, label, progress_bar, prose,
    radio_button, radio_group, sized_box, slider, spinner, switch, text_input,
};
use xilem::{Color, EventLoop, EventLoopBuilder, WidgetView, WindowOptions, Xilem};

// ---------------------------------------------------------------- theme

#[derive(Clone, Copy)]
enum Role {
    App,
    Panel,
    Field,
    Text,
    Muted,
    Accent,
}

/// The three densities a desktop application actually ships.
#[derive(Clone, Copy, PartialEq)]
enum Density {
    Compact,
    Cozy,
    Comfortable,
}

impl Density {
    const ALL: [(Self, &'static str); 3] = [
        (Self::Compact, "Compact"),
        (Self::Cozy, "Cozy"),
        (Self::Comfortable, "Comfortable"),
    ];

    /// How far spacing stretches.
    fn space_scale(self) -> f64 {
        match self {
            Self::Compact => 0.75,
            Self::Cozy => 1.0,
            Self::Comfortable => 1.5,
        }
    }

    /// How far control heights stretch. Less than spacing, on purpose: a
    /// hit target that follows spacing all the way down stops being one.
    fn size_scale(self) -> f64 {
        match self {
            Self::Compact => 0.9,
            Self::Cozy => 1.0,
            Self::Comfortable => 1.2,
        }
    }
}

struct Theme {
    density: Density,
}

impl Theme {
    fn space(&self, space: Space) -> Length {
        Length::px(space.px() * self.density.space_scale())
    }

    fn size(&self, size: ControlSize) -> Length {
        let scaled = size.px() * self.density.size_scale();
        // The floor a text control needs. It belongs in the widget, not
        // here: see DESIGN.md D13.
        Length::px(scaled.max(f64::from(TextSize::Body.px()) * 2.3))
    }

    /// The defaults every built-in widget styles itself from, stated in
    /// this theme's own terms. Without this the app's accent and the
    /// controls' accent are different colors.
    fn widget_defaults(&self) -> masonry::core::DefaultProperties {
        WidgetTheme {
            app: self.color(Role::App),
            panel: self.color(Role::Panel),
            control: self.color(Role::Field),
            control_active: self.color(Role::Panel),
            field: self.color(Role::Field),
            accent: self.color(Role::Accent),
            text: self.color(Role::Text),
            text_muted: self.color(Role::Muted),
            ..WidgetTheme::dark()
        }
        .with_text_size(TextSize::Body.px())
        .property_set()
    }

    fn color(&self, role: Role) -> Color {
        use masonry::peniko::color::{AlphaColor, Oklch, Srgb};
        let oklch =
            |l: f32, c: f32, h: f32| AlphaColor::<Oklch>::new([l, c, h, 1.0]).convert::<Srgb>();
        match role {
            Role::App => oklch(0.17, 0.005, 260.0),
            Role::Panel => oklch(0.21, 0.006, 260.0),
            Role::Field => oklch(0.26, 0.008, 260.0),
            Role::Text => oklch(0.94, 0.004, 260.0),
            Role::Muted => oklch(0.68, 0.012, 260.0),
            Role::Accent => oklch(0.78, 0.150, 155.0),
        }
    }
}

// ---------------------------------------------------------------- state

struct Gallery {
    theme: Theme,
    text: String,
    checked: bool,
    switched: bool,
    choice: usize,
    amount: f64,
    progress: f64,
}

const CHOICES: [&str; 3] = ["Left", "Center", "Right"];

// ---------------------------------------------------------------- parts

/// A labeled slot in the gallery: the name on the left, the control on
/// the right, both centered on one row.
fn slot<V: WidgetView<Gallery> + 'static>(
    t: &Theme,
    name: &'static str,
    control: V,
) -> impl WidgetView<Gallery> + use<V> {
    flex_row((
        sized_box(
            label(name)
                .text_size(TextSize::Body)
                .color(t.color(Role::Muted)),
        )
        .dims(Dimensions::new(
            Dim::Fixed(Length::px(104.0)),
            Dim::Fixed(t.size(ControlSize::Row)),
        )),
        control,
    ))
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .gap(t.space(Space::Md))
}

fn section<V: WidgetView<Gallery> + 'static>(
    t: &Theme,
    title: &'static str,
    body: V,
) -> impl WidgetView<Gallery> + use<V> {
    flex_col((
        label(title)
            .text_size(TextSize::Caption)
            .color(t.color(Role::Muted)),
        body,
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(t.space(Space::Md))
}

/// A push button at control height, primary or not.
fn push(t: &Theme, text: &'static str, primary: bool) -> impl WidgetView<Gallery> + use<> {
    let (bg, fg) = if primary {
        (t.color(Role::Accent), t.color(Role::App))
    } else {
        (t.color(Role::Field), t.color(Role::Text))
    };
    sized_box(
        button(
            label(text).text_size(TextSize::Body).color(fg),
            |_: &mut Gallery| {},
        )
        .background_color(bg)
        .corner_radius(Radius::Md),
    )
    .dims(Dimensions::new(
        Dim::Auto,
        Dim::Fixed(t.size(ControlSize::Control)),
    ))
}

/// The density switch: three chips, the current one filled.
fn density_switch(t: &Theme) -> impl WidgetView<Gallery> + use<> {
    let chips: Vec<_> = Density::ALL
        .iter()
        .map(|(density, name)| {
            let density = *density;
            let active = density == t.density;
            let (bg, fg) = if active {
                (t.color(Role::Accent), t.color(Role::App))
            } else {
                (t.color(Role::Field), t.color(Role::Text))
            };
            sized_box(
                button(
                    label(*name).text_size(TextSize::Body).color(fg),
                    move |gallery: &mut Gallery| gallery.theme.density = density,
                )
                .background_color(bg)
                .corner_radius(Radius::Md),
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

// -------------------------------------------------------------- gallery

fn text_section(gallery: &Gallery) -> impl WidgetView<Gallery> + use<> {
    let t = &gallery.theme;
    section(
        t,
        "Text",
        flex_col((
            slot(
                t,
                "Title",
                label("Widget gallery")
                    .text_size(TextSize::Title)
                    .color(t.color(Role::Text)),
            ),
            slot(
                t,
                "Body",
                label("The quick brown fox")
                    .text_size(TextSize::Body)
                    .color(t.color(Role::Text)),
            ),
            slot(
                t,
                "Caption",
                label("12 items, 3 selected")
                    .text_size(TextSize::Caption)
                    .color(t.color(Role::Muted)),
            ),
            slot(
                t,
                "Prose",
                prose("Prose wraps at the width it is given, and is selectable.")
                    .text_size(TextSize::Body)
                    .text_color(t.color(Role::Text)),
            ),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(t.space(Space::Sm)),
    )
}

fn input_section(gallery: &Gallery) -> impl WidgetView<Gallery> + use<> {
    let t = &gallery.theme;
    section(
        t,
        "Input",
        flex_col((
            slot(
                t,
                "Text field",
                sized_box(
                    text_input(gallery.text.clone(), |gallery: &mut Gallery, v| {
                        gallery.text = v;
                    })
                    .background_color(t.color(Role::Field))
                    .corner_radius(Radius::Sm),
                )
                .dims(Dimensions::new(
                    Dim::Fixed(Length::px(220.0)),
                    Dim::Fixed(t.size(ControlSize::Control)),
                )),
            ),
            slot(
                t,
                "Slider",
                slider(0.0, 1.0, gallery.amount, |gallery: &mut Gallery, v| {
                    gallery.amount = v;
                })
                .width(Length::px(220.0)),
            ),
            slot(
                t,
                "Buttons",
                flex_row((push(t, "Cancel", false), push(t, "Save", true)))
                    .cross_axis_alignment(CrossAxisAlignment::Center)
                    .gap(t.space(Space::Sm)),
            ),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(t.space(Space::Sm)),
    )
}

fn choice_section(gallery: &Gallery) -> impl WidgetView<Gallery> + use<> {
    let t = &gallery.theme;
    let radios: Vec<_> = CHOICES
        .iter()
        .enumerate()
        .map(|(i, name)| {
            radio_button(*name, gallery.choice == i, move |gallery: &mut Gallery| {
                gallery.choice = i;
            })
        })
        .collect();
    section(
        t,
        "Choice",
        flex_col((
            slot(
                t,
                "Checkbox",
                checkbox("Show hidden items", gallery.checked, |gallery: &mut Gallery, on| {
                    gallery.checked = on;
                }),
            ),
            slot(
                t,
                "Switch",
                switch(gallery.switched, |gallery: &mut Gallery, on| {
                    gallery.switched = on;
                }),
            ),
            slot(
                t,
                "Radio",
                radio_group(
                    flex_row(radios)
                        .cross_axis_alignment(CrossAxisAlignment::Center)
                        .gap(t.space(Space::Md)),
                ),
            ),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(t.space(Space::Sm)),
    )
}

fn feedback_section(gallery: &Gallery) -> impl WidgetView<Gallery> + use<> {
    let t = &gallery.theme;
    section(
        t,
        "Feedback",
        flex_col((
            slot(
                t,
                "Progress",
                sized_box(progress_bar(Some(gallery.progress)))
                    .dims(Dimensions::new(Dim::Fixed(Length::px(220.0)), Dim::Auto)),
            ),
            slot(
                t,
                "Indeterminate",
                sized_box(progress_bar(None))
                    .dims(Dimensions::new(Dim::Fixed(Length::px(220.0)), Dim::Auto)),
            ),
            slot(t, "Spinner", spinner()),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(t.space(Space::Sm)),
    )
}

fn app_logic(gallery: &mut Gallery) -> impl WidgetView<Gallery> + use<> {
    let t = &gallery.theme;
    let header = flex_row((
        label("Widgets")
            .text_size(TextSize::Title)
            .color(t.color(Role::Text)),
        FlexSpacer::Flex(1.0),
        density_switch(t),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .gap(t.space(Space::Md))
    .padding(t.space(Space::Md))
    .background_color(t.color(Role::Panel));

    let body = flex_row((
        flex_col((text_section(gallery), input_section(gallery)))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(t.space(Space::Xl))
            .flex(1.0),
        flex_col((choice_section(gallery), feedback_section(gallery)))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(t.space(Space::Xl))
            .flex(1.0),
    ))
    .gap(t.space(Space::Xl))
    .padding(t.space(Space::Lg));

    flex_col((header, body))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(Space::None)
        .background_color(t.color(Role::App))
}

fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let gallery = Gallery {
        theme: Theme {
            density: Density::Cozy,
        },
        text: "Untitled document".into(),
        checked: true,
        switched: true,
        choice: 1,
        amount: 0.62,
        progress: 0.4,
    };
    let background = gallery.theme.color(Role::App);
    let widget_defaults = gallery.theme.widget_defaults();
    let window_options = WindowOptions::new("Widgets")
        .with_min_inner_size(LogicalSize::new(560., 420.))
        .with_initial_inner_size(LogicalSize::new(880., 620.));
    Xilem::new_simple(gallery, app_logic, window_options)
        .with_default_properties(widget_defaults)
        .with_default_base_color(background)
        .run_in(event_loop)
}

fn main() -> Result<(), EventLoopError> {
    run(EventLoop::with_user_event())
}

// Boilerplate code: Identical across all applications which support Android

#[cfg(target_os = "android")]
// Safety: We are following `android_activity`'s docs here
#[expect(
    unsafe_code,
    reason = "We believe that there are no other declarations using this name in the compiled objects here"
)]
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;

    let mut event_loop = EventLoop::with_user_event();
    event_loop.with_android_app(app);

    run(event_loop).expect("Can create app");
}
