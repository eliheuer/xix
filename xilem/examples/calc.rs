// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A calculator, and the shortest before-and-after in the repo.
//!
//! An earlier version of this file carried its own `tokens` module: a
//! spacing scale, a radius, three text sizes, and eight colors, written
//! out as consts because the framework had none. Everything in it was
//! generic. Nothing in it was about arithmetic. That module is gone now,
//! and this file names framework vocabulary instead:
//!
//! - Measurements are [`xilem::kernel`] steps. `Radius::Lg`, not 12.
//!   `TextSize::Display`, not 44.
//! - Containers state what they are. `column(Region::Window, ..)` sets
//!   the window inset and the gap between the display and the keypad;
//!   the keypad's own gap comes from `Region::Card`. Neither is a number
//!   this file chose, and the relationship between them (outer looser
//!   than inner) is a framework invariant rather than this author's
//!   judgment.
//! - Colors are roles resolved by a [`Theme`], which also styles every
//!   built-in widget, so the key hover and pressed states are not set per
//!   button here.
//!
//! What is left in the file is a calculator: the arithmetic, the four key
//! kinds, and which key sits where.
//!
//! One trap remains, and it is worth knowing about: `label(..).color(..)`
//! wraps the view in a style `Prop`, after which `Label`'s own builders
//! (`text_alignment`) are no longer reachable. Builder order should not
//! matter. See DESIGN.md D11.

use masonry::core::{PropertyStack, Selector};
use masonry::kernel::{Radius, Region, TextSize};
use masonry::properties::Dimensions;
use masonry::properties::types::{CrossAxisAlignment, MainAxisAlignment};
use masonry::theme::Theme;
use winit::dpi::LogicalSize;
use winit::error::EventLoopError;
use xilem::style::{Background, BorderColor, Style};
use xilem::view::{
    FlexExt as _, GridSequence, GridTrackSize, button, column, flex_col, grid, label, repeat_tracks,
};
use xilem::{Color, EventLoop, EventLoopBuilder, TextAlign, WidgetView, WindowOptions, Xilem};

/// The palette, as roles. A theme built from these also styles the
/// built-in widgets, so the two cannot disagree.
mod role {
    use xilem::Color;

    fn oklch(l: f32, c: f32, h: f32) -> Color {
        use masonry::peniko::color::{AlphaColor, Oklch, Srgb};
        AlphaColor::<Oklch>::new([l, c, h, 1.0]).convert::<Srgb>()
    }

    pub(crate) fn surface() -> Color {
        oklch(0.17, 0.005, 260.0)
    }
    pub(crate) fn text() -> Color {
        oklch(0.96, 0.004, 260.0)
    }
    pub(crate) fn text_muted() -> Color {
        oklch(0.70, 0.012, 260.0)
    }
    /// Digit keys.
    pub(crate) fn key() -> Color {
        oklch(0.28, 0.008, 260.0)
    }
    /// Function keys: clear, delete, negate.
    pub(crate) fn key_function() -> Color {
        oklch(0.36, 0.012, 260.0)
    }
    /// Operator keys.
    pub(crate) fn key_operator() -> Color {
        oklch(0.55, 0.130, 255.0)
    }
    /// The primary action: equals.
    pub(crate) fn key_primary() -> Color {
        oklch(0.75, 0.140, 65.0)
    }
    pub(crate) fn key_hover_border() -> Color {
        text().with_alpha(0.25)
    }
    pub(crate) fn key_pressed() -> Color {
        text().with_alpha(0.14)
    }
}

#[derive(Copy, Clone)]
enum MathOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
}

impl MathOperator {
    fn as_str(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Subtract => "\u{2212}",
            Self::Multiply => "×",
            Self::Divide => "÷",
        }
    }

    fn perform_op(self, num1: f64, num2: f64) -> f64 {
        match self {
            Self::Add => num1 + num2,
            Self::Subtract => num1 - num2,
            Self::Multiply => num1 * num2,
            Self::Divide => num1 / num2,
        }
    }
}

struct Calculator {
    current_num_index: usize,
    clear_current_entry_on_input: bool, // For instances of negation used on a result.
    numbers: [String; 2],
    result: Option<String>,
    operation: Option<MathOperator>,
}

impl Calculator {
    fn get_current_number(&self) -> String {
        self.current_number().to_string()
    }

    fn current_number(&self) -> &str {
        &self.numbers[self.current_num_index]
    }

    fn set_current_number(&mut self, new_num: String) {
        self.numbers[self.current_num_index] = new_num;
    }

    fn clear_all(&mut self) {
        self.current_num_index = 0;
        self.result = None;
        self.operation = None;
        for num in self.numbers.iter_mut() {
            *num = "".into();
        }
    }

    fn clear_entry(&mut self) {
        self.clear_current_entry_on_input = false;
        if self.result.is_some() {
            self.clear_all();
            return;
        }
        self.set_current_number("".into());
    }

    fn on_entered_digit(&mut self, digit: &str) {
        if self.result.is_some() {
            self.clear_all();
        } else if self.clear_current_entry_on_input {
            self.clear_entry();
        }
        let mut num = self.get_current_number();
        // Special case: Don't allow more than one decimal.
        if digit == "." {
            if num.contains('.') {
                // invalid action
                return;
            }
            // Make it so you don't end up with just a decimal point
            if num.is_empty() {
                num = "0".into();
            }
            num += ".";
        } else if num == "0" || num.is_empty() {
            num = digit.to_string();
        } else {
            num += digit;
        }
        self.set_current_number(num);
    }

    fn on_entered_operator(&mut self, operator: MathOperator) {
        self.clear_current_entry_on_input = false;
        if self.operation.is_some() && !self.numbers[1].is_empty() {
            if self.result.is_none() {
                // All info is there to create a result, so calculate it.
                self.on_equals();
            }
            // There is a result present, so put that on the left.
            self.move_result_to_left();
            self.current_num_index = 1;
        } else if self.current_num_index == 0 {
            if self.numbers[0].is_empty() {
                // Not ready yet. Left number needed.
                // invalid action
                return;
            } else {
                self.current_num_index = 1;
            }
        }
        self.operation = Some(operator);
    }

    /// For instances when you continue working with the prior result.
    fn move_result_to_left(&mut self) {
        self.clear_current_entry_on_input = true;
        self.numbers[0] = self.result.clone().expect("expected result");
        self.numbers[1] = "".into();
        self.operation = None;
        self.current_num_index = 0; // Moved to left
        self.result = None; // It's moved, so remove the result.
    }

    fn on_equals(&mut self) {
        // Requires both numbers be present
        if self.numbers[0].is_empty() || self.numbers[1].is_empty() {
            // invalid action
            return; // Just abort.
        } else if self.result.is_some() {
            // Repeat the operation using the prior result on the left.
            self.numbers[0] = self.result.clone().unwrap();
        }
        self.current_num_index = 0;
        let num1 = self.numbers[0].parse::<f64>();
        let num2 = self.numbers[1].parse::<f64>();
        // Display format error or display the result of the operation.
        self.result = Some(match (num1, num2) {
            (Ok(num1), Ok(num2)) => self.operation.unwrap().perform_op(num1, num2).to_string(),
            (Err(err), _) => err.to_string(),
            (_, Err(err)) => err.to_string(),
        });
    }

    fn on_delete(&mut self) {
        if self.result.is_some() {
            // Delete does not do anything with the result. Invalid action.
            return;
        }
        let mut num = self.get_current_number();
        if !num.is_empty() {
            num.remove(num.len() - 1);
            self.set_current_number(num);
        } // else, invalid action
    }

    fn negate(&mut self) {
        // If there is a result, negate that after clearing and moving it to the first number
        if self.result.is_some() {
            self.move_result_to_left();
        }
        let mut num = self.get_current_number();
        if num.is_empty() {
            // invalid action
            return;
        }
        if num.starts_with('-') {
            num.remove(0);
        } else {
            num = format!("-{num}");
        }
        self.set_current_number(num);
    }

    /// The secondary display line: the pending expression.
    fn expression_text(&self) -> String {
        let mut parts: Vec<&str> = Vec::new();
        if self.operation.is_some() || self.result.is_some() {
            parts.push(&self.numbers[0]);
        }
        if let Some(op) = self.operation {
            parts.push(op.as_str());
        }
        if self.result.is_some() {
            parts.push(&self.numbers[1]);
            parts.push("=");
        }
        parts.join(" ")
    }

    /// The primary display line: the result, or the number being entered.
    fn entry_text(&self) -> String {
        if let Some(result) = &self.result {
            return result.clone();
        }
        let current = self.current_number();
        if current.is_empty() {
            "0".into()
        } else {
            current.to_string()
        }
    }
}

/// Which kind of key a button is. Decides its color.
#[derive(Copy, Clone)]
enum KeyRole {
    Digit,
    Function,
    Operator,
    Primary,
}

impl KeyRole {
    fn color(self) -> Color {
        match self {
            Self::Digit => role::key(),
            Self::Function => role::key_function(),
            Self::Operator => role::key_operator(),
            Self::Primary => role::key_primary(),
        }
    }
}

/// One calculator key.
fn key(
    text: &'static str,
    key_role: KeyRole,
    callback: impl Fn(&mut Calculator) + Send + Sync + 'static,
) -> impl WidgetView<Calculator> {
    button(
        label(text).text_size(TextSize::Heading).color(role::text()),
        callback,
    )
    .dims(Dimensions::STRETCH)
    .background_color(key_role.color())
    .corner_radius(Radius::Lg)
}

fn digit(digit: &'static str) -> impl WidgetView<Calculator> {
    key(digit, KeyRole::Digit, |data: &mut Calculator| {
        data.on_entered_digit(digit);
    })
}

fn operator(math_operator: MathOperator) -> impl WidgetView<Calculator> {
    key(
        math_operator.as_str(),
        KeyRole::Operator,
        move |data: &mut Calculator| {
            data.on_entered_operator(math_operator);
        },
    )
}

fn digit_row(nums: [&'static str; 3]) -> impl GridSequence<Calculator> {
    nums.map(digit)
}

/// The display: a muted expression line above the entry line, both aligned to the end.
fn display(data: &Calculator) -> impl WidgetView<Calculator> + use<> {
    flex_col((
        label(data.expression_text())
            .text_size(TextSize::Heading)
            .text_alignment(TextAlign::End)
            .color(role::text_muted()),
        label(data.entry_text())
            .text_size(TextSize::Display)
            .text_alignment(TextAlign::End)
            .color(role::text()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::End)
    .main_axis_alignment(MainAxisAlignment::End)
    .gap(Region::List.gap())
    .padding(Region::Card.inset())
}

fn keypad(data: &Calculator) -> impl WidgetView<Calculator> + use<> {
    let can_clear_entry = !data.get_current_number().is_empty() || data.result.is_some();
    grid((
        key(
            "CE",
            if can_clear_entry {
                KeyRole::Function
            } else {
                KeyRole::Digit
            },
            Calculator::clear_entry,
        ),
        key("C", KeyRole::Function, Calculator::clear_all),
        key("DEL", KeyRole::Function, Calculator::on_delete),
        operator(MathOperator::Divide),
        digit_row(["7", "8", "9"]),
        operator(MathOperator::Multiply),
        digit_row(["4", "5", "6"]),
        operator(MathOperator::Subtract),
        digit_row(["1", "2", "3"]),
        operator(MathOperator::Add),
        key("±", KeyRole::Function, Calculator::negate),
        digit("0"),
        key(".", KeyRole::Digit, |data: &mut Calculator| {
            data.on_entered_digit(".");
        }),
        key("=", KeyRole::Primary, Calculator::on_equals),
    ))
    .columns(repeat_tracks(4, GridTrackSize::FRACTION))
    .rows(repeat_tracks(5, GridTrackSize::FRACTION))
    .gap(Region::Card.gap())
}

fn app_logic(data: &mut Calculator) -> impl WidgetView<Calculator> + use<> {
    column(
        Region::Window,
        (display(data).flex(1.), keypad(data).flex(2.5)),
    )
}

pub(crate) fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let data = Calculator {
        current_num_index: 0,
        clear_current_entry_on_input: false,
        numbers: ["".into(), "".into()],
        result: None,
        operation: None,
    };

    // The theme styles every built-in widget, so the calculator's keys and
    // any control it grows later cannot disagree about color.
    let theme = Theme {
        app: role::surface(),
        panel: role::surface(),
        control: role::key(),
        control_active: role::key_function(),
        accent: role::key_operator(),
        text: role::text(),
        text_muted: role::text_muted(),
        ..Theme::dark()
    };
    // Key states: no resting border, a light border on hover, a light
    // overlay when pressed. Still per-app: a button role in the theme
    // would carry it. See DESIGN.md D15, layer 4.
    let mut default_properties = theme.property_set();
    let mut stack = PropertyStack::new();
    stack.push(Selector::new(), BorderColor::new(Color::TRANSPARENT));
    stack.push(
        Selector::new().with_hovered(true),
        BorderColor::new(role::key_hover_border()),
    );
    stack.push(
        Selector::new().with_active(true),
        Background::Color(role::key_pressed()),
    );
    default_properties.insert_stack::<masonry::widgets::Button>(stack);

    let min_window_size = LogicalSize::new(240., 360.);
    let window_size = LogicalSize::new(320., 480.);
    let window_options = WindowOptions::new("Calculator").with_min_inner_size(min_window_size);
    // On iOS, winit has unsensible handling of `inner_size`
    // See https://github.com/rust-windowing/winit/issues/2308 for more details
    #[cfg(not(target_os = "ios"))]
    let window_options = window_options.with_initial_inner_size(window_size);
    let app = Xilem::new_simple(data, app_logic, window_options)
        .with_default_properties(default_properties)
        .with_default_base_color(role::surface());
    app.run_in(event_loop)?;
    Ok(())
}

// Boilerplate code: Identical across all applications which support Android

fn main() -> Result<(), EventLoopError> {
    run(EventLoop::with_user_event())
}

// Boilerplate code for the web: identical across all applications which support the browser.

#[cfg(target_arch = "wasm32")]
#[xilem::wasm_bindgen::prelude::wasm_bindgen]
pub fn mount(canvas: xilem::web_sys::HtmlCanvasElement) {
    xilem::web::mount(canvas, || run(EventLoop::with_user_event()));
}
