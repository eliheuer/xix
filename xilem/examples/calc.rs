// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! A calculator.
//!
//! This is the upstream Xilem calculator reworked to look designed.
//! Everything visible comes from the `tokens` module: spacing steps,
//! radii, text sizes, and colors by role. The view code below has no
//! raw numbers and no raw colors.
//!
//! xix note. Reworking this example forced these things, which the
//! framework should own instead of every example:
//! - Tokens as types: `Space`, `Radius`, `Text`, `Role` (here: consts).
//! - Button roles (digit, operator, function, primary) with hover and
//!   pressed colors derived from the role, not set per widget.
//! - Relational spacing: window inset > section gap > key gap.
//! - Text alignment to the end for a numeric display, and tabular
//!   figures (not available yet; the display jitters as digits change).
//! - A "display" part: a primary line and a muted secondary line.
//! - An API trap: `label(..).color(..)` wraps the view in a style `Prop`,
//!   after which `Label`'s own builders (`text_alignment`) are gone.
//!   Builder order should not matter.

use masonry::core::{PropertyStack, Selector};
use masonry::properties::Dimensions;
use masonry::properties::types::{CrossAxisAlignment, MainAxisAlignment};
use masonry::theme::default_property_set;
use winit::dpi::LogicalSize;
use winit::error::EventLoopError;
use xilem::style::{Background, BorderColor, Style};
use xilem::view::{
    FlexExt as _, GridSequence, GridTrackSize, button, flex_col, grid, label, repeat_tracks,
};
use xilem::{Color, EventLoop, EventLoopBuilder, TextAlign, WidgetView, WindowOptions, Xilem};

/// Design tokens. The only place in this file where a number or a color is written.
mod tokens {
    use masonry::layout::Length;
    use xilem::Color;

    /// Spacing scale, 4 pt base.
    pub(crate) mod space {
        use super::Length;
        pub(crate) const XS: Length = Length::const_px(4.);
        pub(crate) const S: Length = Length::const_px(8.);
        pub(crate) const M: Length = Length::const_px(12.);
        pub(crate) const L: Length = Length::const_px(20.);
    }

    pub(crate) mod radius {
        use super::Length;
        pub(crate) const M: Length = Length::const_px(12.);
    }

    /// Text sizes, in px.
    pub(crate) mod text {
        pub(crate) const DISPLAY: f32 = 44.;
        pub(crate) const SECONDARY: f32 = 18.;
        pub(crate) const KEY: f32 = 20.;
    }

    /// Colors by role.
    pub(crate) mod role {
        use super::Color;
        pub(crate) const SURFACE: Color = Color::from_rgb8(0x15, 0x15, 0x17);
        pub(crate) const TEXT: Color = Color::from_rgb8(0xf4, 0xf4, 0xf5);
        pub(crate) const TEXT_MUTED: Color = Color::from_rgb8(0x8f, 0x8f, 0x97);

        /// Digit keys.
        pub(crate) const KEY: Color = Color::from_rgb8(0x2a, 0x2a, 0x2e);
        /// Function keys: clear, delete, negate.
        pub(crate) const KEY_FUNCTION: Color = Color::from_rgb8(0x3a, 0x3a, 0x40);
        /// Operator keys.
        pub(crate) const KEY_OPERATOR: Color = Color::from_rgb8(0x3b, 0x6e, 0xc4);
        /// The primary action: equals.
        pub(crate) const KEY_PRIMARY: Color = Color::from_rgb8(0xe0, 0x8a, 0x3c);

        /// Hover and pressed states are the same overlay for every key role.
        pub(crate) const KEY_HOVER_BORDER: Color = Color::from_rgba8(0xff, 0xff, 0xff, 0x40);
        pub(crate) const KEY_PRESSED: Color = Color::from_rgba8(0xff, 0xff, 0xff, 0x24);
    }
}

use tokens::{radius, role, space, text};

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
            Self::Digit => role::KEY,
            Self::Function => role::KEY_FUNCTION,
            Self::Operator => role::KEY_OPERATOR,
            Self::Primary => role::KEY_PRIMARY,
        }
    }
}

/// One calculator key.
fn key(
    text: &'static str,
    key_role: KeyRole,
    callback: impl Fn(&mut Calculator) + Send + Sync + 'static,
) -> impl WidgetView<Calculator> {
    button(label(text).text_size(text::KEY).color(role::TEXT), callback)
        .dims(Dimensions::STRETCH)
        .background_color(key_role.color())
        .corner_radius(radius::M)
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
            .text_size(text::SECONDARY)
            .text_alignment(TextAlign::End)
            .color(role::TEXT_MUTED),
        label(data.entry_text())
            .text_size(text::DISPLAY)
            .text_alignment(TextAlign::End)
            .color(role::TEXT),
    ))
    .cross_axis_alignment(CrossAxisAlignment::End)
    .main_axis_alignment(MainAxisAlignment::End)
    .gap(space::XS)
    .padding(space::M)
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
    .gap(space::S)
}

fn app_logic(data: &mut Calculator) -> impl WidgetView<Calculator> + use<> {
    flex_col((display(data).flex(1.), keypad(data).flex(2.5)))
        .gap(space::M)
        .padding(space::L)
}

pub(crate) fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let data = Calculator {
        current_num_index: 0,
        clear_current_entry_on_input: false,
        numbers: ["".into(), "".into()],
        result: None,
        operation: None,
    };

    // Key states: no resting border, a light border on hover, a light overlay when pressed.
    // TODO(xix): this belongs to a button role in the theme, not to the app.
    let mut default_properties = default_property_set();
    let mut stack = PropertyStack::new();
    stack.push(Selector::new(), BorderColor::new(Color::TRANSPARENT));
    stack.push(
        Selector::new().with_hovered(true),
        BorderColor::new(role::KEY_HOVER_BORDER),
    );
    stack.push(
        Selector::new().with_active(true),
        Background::Color(role::KEY_PRESSED),
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
        .with_default_base_color(role::SURFACE);
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
