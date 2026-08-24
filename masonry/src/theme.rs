// Copyright 2019 the Xilem Authors and the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Default values used by various widgets in their paint methods.

#![allow(missing_docs, reason = "Names are self-explanatory.")]

use crate::core::{
    DefaultProperties, PropertySet, PropertyStack, Selector, StyleProperty, StyleSet,
};
use crate::kernel::{Radius, Space, Stroke};
use crate::layout::{AsUnit, Length};
use crate::palette::css::DIM_GRAY;
use crate::parley::{GenericFamily, LineHeight};
use crate::peniko::Color;

// We use glob imports here to avoid frequent merge conflicts.
use crate::properties::*;
use crate::widgets::*;

/// Default color for the app background.
///
/// If the app driver does some kind beginning-of-frame clearing,
/// it should clear with this color by default.
pub const BACKGROUND_COLOR: Color = Color::from_rgb8(0x1D, 0x1D, 0x1D);

pub const BORDER_WIDTH: Length = Length::const_px(1.);

// Zync color variations from https://tailwindcss.com/docs/colors
pub const ZYNC_900: Color = Color::from_rgb8(0x18, 0x18, 0x1b);
pub const ZYNC_800: Color = Color::from_rgb8(0x27, 0x27, 0x2a);
pub const ZYNC_700: Color = Color::from_rgb8(0x3f, 0x3f, 0x46);
pub const ZYNC_600: Color = Color::from_rgb8(0x52, 0x52, 0x5b);
pub const ZYNC_500: Color = Color::from_rgb8(0x71, 0x71, 0x7a);

pub const ACCENT_COLOR: Color = Color::from_rgb8(0x3b, 0x7e, 0xe4);
pub const TEXT_COLOR: Color = Color::from_rgb8(0xf2, 0xf2, 0xf2);
pub const DISABLED_TEXT_COLOR: Color = Color::from_rgb8(0xa0, 0xa0, 0x9a);
pub const PLACEHOLDER_COLOR: Color = Color::from_rgba8(0xFF, 0xFF, 0xFF, 0x8F);
pub const TEXT_BACKGROUND_COLOR: Color = Color::from_rgb8(0x16, 0x16, 0x16);
pub const FOCUS_COLOR: Color = Color::from_rgb8(0xff, 0xff, 0xff);

// TODO: The following constants are not being used in properties
pub const TEXT_SIZE_NORMAL: f32 = 15.0;
pub const BASIC_WIDGET_HEIGHT: Length = Length::const_px(18.0);
pub const BORDERED_WIDGET_HEIGHT: f64 = 24.0;
pub const SCROLLBAR_COLOR: Color = Color::from_rgb8(0xff, 0xff, 0xff);
pub const SCROLLBAR_BORDER_COLOR: Color = Color::from_rgb8(0x77, 0x77, 0x77);
pub const SCROLLBAR_WIDTH: f64 = 8.;
pub const SCROLLBAR_PAD: f64 = 2.;
pub const SCROLLBAR_MIN_SIZE: f64 = 45.;
pub const SCROLLBAR_RADIUS: f64 = 5.;
pub const SCROLLBAR_EDGE_WIDTH: f64 = 1.;
pub const DEFAULT_GAP: Length = Length::const_px(10.0);
pub const DEFAULT_SPACER_LEN: Length = Length::const_px(10.0);
pub const WIDGET_CONTROL_COMPONENT_PADDING: Length = Length::const_px(4.0);
pub const SELECTOR_MIN_WIDTH: f64 = 100.0;

/// The values every built-in widget styles itself from.
///
/// Before this existed, [`default_property_set`] was a few hundred lines of
/// hardcoded colors and pixel counts, so an application that themed its own
/// views got a screen where its accent was one color and every built-in
/// control was another. A theme is data, and this is the data.
///
/// The measurements come from the [`kernel`](crate::kernel) scale, so a
/// theme states intent (`Radius::Md`) rather than a number, and one change
/// moves every widget that named that step.
#[derive(Clone, Copy, Debug)]
pub struct Theme {
    /// The window behind everything.
    pub app: Color,
    /// A panel, sidebar, or bar that sits on the app surface.
    pub panel: Color,
    /// The face of a control: a button, a chip, a track.
    pub control: Color,
    /// The face of a control that is pressed or active.
    pub control_active: Color,
    /// The inside of a text field.
    pub field: Color,
    /// A border at rest.
    pub border: Color,
    /// A border under the pointer.
    pub border_hover: Color,
    /// A border that has to be seen: a scrollbar edge, a strong divider.
    pub border_strong: Color,
    /// The focus ring.
    pub focus: Color,
    /// The color that means "this one".
    pub accent: Color,
    /// Body text.
    pub text: Color,
    /// Secondary text.
    pub text_muted: Color,
    /// Text that cannot be used right now.
    pub text_disabled: Color,
    /// Placeholder text inside an empty field.
    pub placeholder: Color,
    /// The body text size, in logical pixels.
    pub text_size: f32,
    /// The corner every control rounds to.
    pub radius: Radius,
    /// The border width every control draws.
    pub stroke: Stroke,
    /// The inset inside a control, across the axis of its text.
    pub control_padding: Space,
    /// The gap between two adjacent widgets.
    pub gap: Space,
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

impl Theme {
    /// The dark theme the framework ships with.
    pub const fn dark() -> Self {
        Self {
            app: BACKGROUND_COLOR,
            panel: ZYNC_900,
            control: ZYNC_800,
            control_active: ZYNC_700,
            field: TEXT_BACKGROUND_COLOR,
            border: ZYNC_700,
            border_hover: ZYNC_500,
            border_strong: ZYNC_600,
            focus: FOCUS_COLOR,
            accent: ACCENT_COLOR,
            text: TEXT_COLOR,
            text_muted: DIM_GRAY,
            text_disabled: DISABLED_TEXT_COLOR,
            placeholder: PLACEHOLDER_COLOR,
            text_size: TEXT_SIZE_NORMAL,
            radius: Radius::Md,
            stroke: Stroke::Hairline,
            control_padding: Space::Md,
            gap: Space::Md,
        }
    }

    /// Returns a copy with a different accent color.
    pub const fn with_accent(mut self, accent: Color) -> Self {
        self.accent = accent;
        self
    }

    /// Returns a copy with a different body text size.
    pub const fn with_text_size(mut self, text_size: f32) -> Self {
        self.text_size = text_size;
        self
    }
}

/// The property set for the framework's dark theme.
///
/// Equivalent to `Theme::dark().property_set()`.
pub fn default_property_set() -> DefaultProperties {
    Theme::dark().property_set()
}

impl Theme {
    /// Builds the default properties of every built-in widget from this
    /// theme.
    pub fn property_set(&self) -> DefaultProperties {
        let mut properties = DefaultProperties::new();

        // Badge
        properties.insert::<Badge, _>(Padding::from_vh(3.px(), 5.px()));
        properties.insert::<Badge, _>(CornerRadius { radius: 999.px() });
        properties.insert::<Badge, _>(BorderWidth { width: 0.px() });
        properties.insert::<Badge, _>(Background::Color(self.accent));
        properties.insert::<Badge, _>(BorderColor {
            color: self.control_active,
        });
        {
            let mut stack = PropertyStack::new();
            stack.push(
                Selector::new().with_disabled(true),
                Background::Color(self.control),
            );
            properties.insert_stack::<Badge>(stack);
        }

        // Button
        properties.insert::<Button, _>(Padding::from_vh(6.px(), 16.px()));
        properties.insert::<Button, _>(CornerRadius { radius: 6.px() });
        properties.insert::<Button, _>(BorderWidth {
            width: self.stroke.length(),
        });
        properties.insert::<Button, _>(Background::Color(self.control));
        properties.insert::<Button, _>(BorderColor {
            color: self.control_active,
        });
        {
            let mut stack = PropertyStack::new();
            stack.push(
                Selector::new().with_hovered(true),
                BorderColor {
                    color: self.border_hover,
                },
            );
            stack.push(
                Selector::new().with_focused(true),
                BorderColor { color: self.focus },
            );
            stack.push(
                Selector::new().with_active(true),
                Background::Color(self.control_active),
            );
            stack.push(
                Selector::new().with_disabled(true),
                Background::Color(Color::BLACK),
            );
            properties.insert_stack::<Button>(stack);
        }

        // Checkbox
        properties.insert::<Checkbox, _>(CornerRadius { radius: 4.px() });
        properties.insert::<Checkbox, _>(BorderWidth {
            width: self.stroke.length(),
        });
        properties.insert::<Checkbox, _>(Background::Color(self.control));
        properties.insert::<Checkbox, _>(BorderColor {
            color: self.control_active,
        });
        properties.insert::<Checkbox, _>(CheckmarkStrokeWidth { width: 2.0 });
        properties.insert::<Checkbox, _>(CheckmarkColor { color: self.text });
        {
            let mut stack = PropertyStack::new();
            stack.push(
                Selector::new().with_hovered(true),
                BorderColor {
                    color: self.border_hover,
                },
            );
            stack.push(
                Selector::new().with_focused(true),
                BorderColor { color: self.focus },
            );
            stack.push(
                Selector::new().with_active(true),
                Background::Color(self.control_active),
            );
            stack.push(
                Selector::new().with_disabled(true),
                PropertySet::new()
                    .with(Background::Color(Color::BLACK))
                    .with(CheckmarkColor {
                        color: self.text_disabled,
                    }),
            );
            properties.insert_stack::<Checkbox>(stack);
        }

        // DisclosureButton
        properties.insert::<DisclosureButton, _>(ContentColor::new(DIM_GRAY));
        properties.insert::<DisclosureButton, _>(Dimensions::fixed(
            Length::const_px(16.),
            Length::const_px(16.),
        ));
        properties.insert::<DisclosureButton, _>(Padding::all(4.px()));

        // Divider
        properties.insert::<Divider, _>(ContentColor::new(self.border_hover));

        // Switch
        properties.insert::<Switch, _>(CornerRadius { radius: 10.px() }); // Full pill shape
        properties.insert::<Switch, _>(BorderWidth {
            width: self.stroke.length(),
        });
        properties.insert::<Switch, _>(Background::Color(self.control_active));
        properties.insert::<Switch, _>(BorderColor {
            color: self.control_active,
        });
        properties.insert::<Switch, _>(ThumbColor(Color::WHITE));
        properties.insert::<Switch, _>(ThumbRadius(8.px()));
        properties.insert::<Switch, _>(TrackThickness(20.px()));
        {
            let mut stack = PropertyStack::new();
            stack.push(
                Selector::new().with_hovered(true),
                BorderColor {
                    color: self.border_hover,
                },
            );
            stack.push(
                Selector::new().with_focused(true),
                BorderColor { color: self.focus },
            );
            stack.push(
                Selector::classes(&["#toggled"]),
                Background::Color(self.accent),
            );
            stack.push(
                Selector::new().with_active(true),
                Background::Color(self.border_strong),
            );
            stack.push(
                Selector::new().with_disabled(true),
                Background::Color(Color::BLACK),
            );
            properties.insert_stack::<Switch>(stack);
        }

        // FIXME
        use crate::widgets::Selector as SelectorButton;

        // Selector
        properties.insert::<SelectorButton, _>(Padding::from_vh(6.px(), 16.px()));
        properties.insert::<SelectorButton, _>(CornerRadius { radius: 2.px() });
        properties.insert::<SelectorButton, _>(BorderWidth {
            width: self.stroke.length(),
        });

        properties.insert::<SelectorButton, _>(Background::Color(self.control));
        properties.insert::<SelectorButton, _>(BorderColor {
            color: self.control_active,
        });
        {
            let mut stack = PropertyStack::new();
            stack.push(
                Selector::new().with_active(true),
                Background::Color(self.control_active),
            );
            stack.push(
                Selector::new().with_disabled(true),
                Background::Color(Color::BLACK),
            );
            stack.push(
                Selector::new().with_hovered(true),
                BorderColor {
                    color: self.border_hover,
                },
            );
            stack.push(
                Selector::new().with_focused(true),
                BorderColor { color: self.focus },
            );
            properties.insert_stack::<SelectorButton>(stack);
        }

        // SelectorItem
        properties.insert::<SelectorItem, _>(Padding::from_vh(6.px(), 16.px()));
        properties.insert::<SelectorItem, _>(Background::Color(self.panel));
        {
            let mut stack = PropertyStack::new();
            stack.push(
                Selector::new().with_active(true),
                Background::Color(self.control),
            );
            stack.push(
                Selector::new().with_disabled(true),
                Background::Color(Color::BLACK),
            );
            properties.insert_stack::<SelectorItem>(stack);
        }

        // Flex
        properties.insert::<Flex, _>(Gap::new(DEFAULT_GAP));

        // Grid
        properties.insert::<Grid, _>(Gap::ZERO);

        // TextInput
        properties.insert::<TextInput, _>(Padding::from_vh(6.px(), 12.px()));
        properties.insert::<TextInput, _>(CornerRadius { radius: 4.px() });
        properties.insert::<TextInput, _>(BorderWidth {
            width: self.stroke.length(),
        });
        properties.insert::<TextInput, _>(BorderColor {
            color: self.border_strong,
        });
        properties.insert::<TextInput, _>(PlaceholderColor::new(self.placeholder));
        properties.insert::<TextInput, _>(CaretColor { color: self.text });
        properties.insert::<TextInput, _>(SelectionColor { color: self.accent });
        properties.insert::<TextInput, _>(Background::Color(self.field));
        {
            let mut stack = PropertyStack::new();
            stack.push(
                Selector::classes(&["#unfocused"]),
                SelectionColor {
                    color: self.text_disabled,
                },
            );
            stack.push(
                Selector::new().with_focused(true),
                BorderColor { color: self.focus },
            );
            stack.push(
                Selector::new().with_disabled(true),
                Background::Color(self.field),
            );
            properties.insert_stack::<TextInput>(stack);
        }

        // TextArea
        properties.insert::<TextArea<false>, _>(ContentColor::new(self.text));
        properties.insert::<TextArea<false>, _>(CaretColor { color: self.text });
        properties.insert::<TextArea<false>, _>(SelectionColor { color: self.accent });
        {
            let mut stack = PropertyStack::new();
            stack.push(
                Selector::new().with_disabled(true),
                ContentColor::new(self.text_disabled),
            );
            properties.insert_stack::<TextArea<false>>(stack);
        }
        properties.insert::<TextArea<true>, _>(ContentColor::new(self.text));
        properties.insert::<TextArea<true>, _>(CaretColor { color: self.text });
        properties.insert::<TextArea<true>, _>(SelectionColor { color: self.accent });
        {
            let mut stack = PropertyStack::new();
            stack.push(
                Selector::new().with_disabled(true),
                ContentColor::new(self.text_disabled),
            );
            properties.insert_stack::<TextArea<true>>(stack);
        }

        // Label
        properties.insert::<Label, _>(ContentColor::new(self.text));
        {
            let mut stack = PropertyStack::new();
            stack.push(
                Selector::new().with_disabled(true),
                ContentColor::new(self.text_disabled),
            );
            properties.insert_stack::<Label>(stack);
        }

        // ProgressBar
        properties.insert::<ProgressBar, _>(CornerRadius { radius: 2.px() });
        properties.insert::<ProgressBar, _>(BorderWidth {
            width: self.stroke.length(),
        });

        properties.insert::<ProgressBar, _>(Background::Color(self.panel));
        properties.insert::<ProgressBar, _>(BorderColor {
            color: self.control,
        });
        properties.insert::<ProgressBar, _>(BarColor(self.accent));

        // RadioButton
        properties.insert::<RadioButton, _>(BorderWidth {
            width: self.stroke.length(),
        });

        properties.insert::<RadioButton, _>(Background::Color(self.control));
        properties.insert::<RadioButton, _>(BorderColor {
            color: self.control_active,
        });
        properties.insert::<RadioButton, _>(CheckmarkColor { color: self.text });
        {
            let mut stack = PropertyStack::new();
            stack.push(
                Selector::new().with_active(true),
                Background::Color(self.control_active),
            );
            stack.push(
                Selector::new().with_hovered(true),
                BorderColor {
                    color: self.border_hover,
                },
            );
            stack.push(
                Selector::new().with_focused(true),
                BorderColor { color: self.focus },
            );
            stack.push(
                Selector::new().with_disabled(true),
                (
                    CheckmarkColor::new(self.text_disabled),
                    Background::Color(Color::BLACK),
                ),
            );
            properties.insert_stack::<RadioButton>(stack);
        }

        // Slider
        properties.insert::<Slider, _>(TrackThickness(4.px()));
        properties.insert::<Slider, _>(TrackColor {
            active: self.accent,
            inactive: self.control,
        });
        properties.insert::<Slider, _>(ThumbColor(self.text));
        properties.insert::<Slider, _>(ThumbRadius(7.px()));
        {
            let mut stack = PropertyStack::new();
            stack.push(
                Selector::new().with_hovered(true),
                BorderColor {
                    color: self.border_hover,
                },
            );
            stack.push(
                Selector::new().with_focused(true),
                BorderColor { color: self.focus },
            );
            properties.insert_stack::<Slider>(stack);
        }

        // Spinner
        properties.insert::<Spinner, _>(TrackColor {
            active: self.text,
            inactive: Color::TRANSPARENT,
        });
        properties.insert::<Spinner, _>(TrackThickness(8.px()));
        properties.insert::<Spinner, _>(AnimationDuration { seconds: 1.2 });

        // StepInput
        default_step_input_style::<u8>(self, &mut properties);
        default_step_input_style::<i8>(self, &mut properties);
        default_step_input_style::<u16>(self, &mut properties);
        default_step_input_style::<i16>(self, &mut properties);
        default_step_input_style::<u32>(self, &mut properties);
        default_step_input_style::<i32>(self, &mut properties);
        default_step_input_style::<u64>(self, &mut properties);
        default_step_input_style::<i64>(self, &mut properties);
        default_step_input_style::<usize>(self, &mut properties);
        default_step_input_style::<isize>(self, &mut properties);
        default_step_input_style::<f32>(self, &mut properties);
        default_step_input_style::<f64>(self, &mut properties);

        properties
    }
}

/// Applies the default text styles for Masonry into `styles`.
pub fn default_text_styles(styles: &mut StyleSet) {
    styles.insert(StyleProperty::LineHeight(LineHeight::FontSizeRelative(1.2)));
    styles.insert(GenericFamily::SystemUi.into());
}

fn default_step_input_style<T: Steppable>(theme: &Theme, properties: &mut DefaultProperties) {
    properties.insert::<StepInput<T>, _>(Padding::from_vh(6.px(), 0.px()));
    properties.insert::<StepInput<T>, _>(CornerRadius { radius: 6.px() });
    properties.insert::<StepInput<T>, _>(BorderWidth {
        width: theme.stroke.length(),
    });

    properties.insert::<StepInput<T>, _>(ContentColor::new(theme.text));
    properties.insert::<StepInput<T>, _>(Background::Color(theme.control));
    properties.insert::<StepInput<T>, _>(BorderColor {
        color: theme.control_active,
    });

    {
        let mut stack = PropertyStack::new();
        stack.push(
            Selector::new().with_disabled(true),
            (
                ContentColor::new(theme.text_disabled),
                Background::Color(Color::BLACK),
            ),
        );
        stack.push(
            Selector::new().with_hovered(true),
            BorderColor {
                color: theme.border_hover,
            },
        );
        stack.push(
            Selector::new().with_focused(true),
            BorderColor { color: theme.focus },
        );

        properties.insert_stack::<StepInput<T>>(stack);
    }
}

/// Set of default properties used in unit tests.
///
/// This lets us change default properties without having to reset all screenshots every time.
/// This should still be kept relatively close to `default_property_set()` so that screenshots look like end user apps.
#[cfg(test)]
pub(crate) fn test_property_set() -> DefaultProperties {
    #[allow(unused_mut, reason = "Sometimes we don't have anything to change")]
    let mut properties = default_property_set();

    properties
}
