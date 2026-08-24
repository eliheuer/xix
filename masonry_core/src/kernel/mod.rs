// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! The design kernel: the closed vocabulary an application styles with.
//!
//! Every sizing API in a UI framework that takes a bare number invites
//! drift. Given a free `f64`, an author (or an agent) picks 6 here and 5
//! there, and after a few hundred lines the interface has twenty spacing
//! values and no rhythm. Nothing in the code looks wrong; the screen just
//! looks unfinished.
//!
//! The fix is a vocabulary you cannot land between. [`Space`], [`ControlSize`],
//! [`Radius`], and [`TextSize`] are the whole scale. They convert into
//! the property types, so they work anywhere a [`Length`] does:
//!
//! ```ignore
//! flex_row(children).gap(Space::Md).padding(Space::Lg)
//! sized_box(child).height(ControlSize::Row).corner_radius(Radius::Sm)
//! label("Name").text_size(TextSize::Body)
//! ```
//!
//! The steps are a 4 px grid, which is where the type metrics land and
//! what every desktop design system converges on. `Space::raw` and the
//! other escape hatches exist for the case the scale genuinely cannot
//! express, and are meant to be rare enough to grep for.

mod region;

pub use region::Region;

use crate::layout::{Dim, Length};
use crate::properties::Padding;

/// The spacing scale: gaps between widgets, and insets inside them.
///
/// Steps are 0, 2, 4, 8, 12, 16, and 24 px.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Space {
    /// No space.
    None,
    /// 2 px. Inside a control, between an icon and its label.
    Xs,
    /// 4 px. Between rows of one list.
    Sm,
    /// 8 px. Between controls in a group. The default.
    #[default]
    Md,
    /// 12 px. Panel inset, and between groups.
    Lg,
    /// 16 px. Between sections.
    Xl,
    /// 24 px. Between regions of a window.
    Xxl,
}

impl Space {
    /// The step, in logical pixels.
    pub const fn px(self) -> f64 {
        match self {
            Self::None => 0.0,
            Self::Xs => 2.0,
            Self::Sm => 4.0,
            Self::Md => 8.0,
            Self::Lg => 12.0,
            Self::Xl => 16.0,
            Self::Xxl => 24.0,
        }
    }

    /// The step as a [`Length`].
    pub const fn length(self) -> Length {
        match Length::try_px(self.px()) {
            Some(length) => length,
            None => Length::ZERO,
        }
    }
}

/// The control-height scale: how tall an interactive thing is.
///
/// Named `ControlSize`, not `Size`, because [`kurbo::Size`](crate::kurbo::Size)
/// is everywhere in this stack and a clash there is worse than a longer
/// name here.
///
/// Heights are what make a panel read as tidy or ragged, and they are the
/// first thing an author invents a number for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum ControlSize {
    /// 10 px. A dot in a picker, a status light.
    Dot,
    /// 16 px. A color swatch or a chip.
    Swatch,
    /// 20 px. An icon, a small thumbnail.
    Icon,
    /// 22 px. A row in a dense list, sidebar, or tree.
    Row,
    /// 28 px. A text field, a button, a toggle. The default.
    #[default]
    Control,
    /// 36 px. A primary control, or a touch target.
    Large,
    /// 44 px. The minimum comfortable touch target.
    Touch,
}

impl ControlSize {
    /// The height, in logical pixels.
    pub const fn px(self) -> f64 {
        match self {
            Self::Dot => 10.0,
            Self::Swatch => 16.0,
            Self::Icon => 20.0,
            Self::Row => 22.0,
            Self::Control => 28.0,
            Self::Large => 36.0,
            Self::Touch => 44.0,
        }
    }

    /// The height as a [`Length`].
    pub const fn length(self) -> Length {
        match Length::try_px(self.px()) {
            Some(length) => length,
            None => Length::ZERO,
        }
    }
}

/// The stroke scale: border and outline widths.
///
/// A border is either there or emphasized. Three values is already
/// generous, and it keeps 1.5 px hairlines from appearing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stroke {
    /// No border.
    None,
    /// 1 px. The default border.
    #[default]
    Hairline,
    /// 2 px. An emphasized or focused border.
    Thick,
}

impl Stroke {
    /// The width, in logical pixels.
    pub const fn px(self) -> f64 {
        match self {
            Self::None => 0.0,
            Self::Hairline => 1.0,
            Self::Thick => 2.0,
        }
    }

    /// The width as a [`Length`].
    pub const fn length(self) -> Length {
        match Length::try_px(self.px()) {
            Some(length) => length,
            None => Length::ZERO,
        }
    }
}

impl From<Stroke> for Length {
    fn from(stroke: Stroke) -> Self {
        stroke.length()
    }
}

/// The corner scale.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Radius {
    /// Square corners.
    None,
    /// 4 px. Rows, fields, small controls. The default.
    #[default]
    Sm,
    /// 6 px. Buttons and chips.
    Md,
    /// 10 px. Cards and grid cells.
    Lg,
    /// 16 px. Panels and dialogs.
    Xl,
    /// A pill: half of the shorter side, as far as a length can say it.
    Full,
}

impl Radius {
    /// The radius, in logical pixels.
    pub const fn px(self) -> f64 {
        match self {
            Self::None => 0.0,
            Self::Sm => 4.0,
            Self::Md => 6.0,
            Self::Lg => 10.0,
            Self::Xl => 16.0,
            Self::Full => 9999.0,
        }
    }

    /// The radius as a [`Length`].
    pub const fn length(self) -> Length {
        match Length::try_px(self.px()) {
            Some(length) => length,
            None => Length::ZERO,
        }
    }
}

/// The type scale. Two sizes carry most of an interface.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub enum TextSize {
    /// 11 px. Section headers, counts, hints.
    Caption,
    /// 12 px. Rows, labels, fields. The default.
    #[default]
    Body,
    /// 14 px. A window or panel title.
    Title,
    /// 18 px. The one heading on a screen.
    Heading,
    /// 24 px. A display number or an empty-state line.
    Display,
}

impl TextSize {
    /// The size, in logical pixels.
    pub const fn px(self) -> f32 {
        match self {
            Self::Caption => 11.0,
            Self::Body => 12.0,
            Self::Title => 14.0,
            Self::Heading => 18.0,
            Self::Display => 24.0,
        }
    }
}

impl From<Space> for Length {
    fn from(space: Space) -> Self {
        space.length()
    }
}

impl From<Space> for Padding {
    fn from(space: Space) -> Self {
        Self::from(space.length())
    }
}

impl From<ControlSize> for Length {
    fn from(size: ControlSize) -> Self {
        size.length()
    }
}

impl From<ControlSize> for Dim {
    fn from(size: ControlSize) -> Self {
        Self::from(size.length())
    }
}

impl From<Radius> for Length {
    fn from(radius: Radius) -> Self {
        radius.length()
    }
}

impl From<TextSize> for f32 {
    fn from(size: TextSize) -> Self {
        size.px()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_is_on_a_four_pixel_grid_above_the_hairline_steps() {
        for space in [Space::Md, Space::Lg, Space::Xl, Space::Xxl] {
            assert_eq!(space.px() % 4.0, 0.0, "{space:?} is off the grid");
        }
        for size in [
            ControlSize::Dot,
            ControlSize::Swatch,
            ControlSize::Icon,
            ControlSize::Row,
            ControlSize::Control,
            ControlSize::Large,
            ControlSize::Touch,
        ] {
            assert_eq!(size.px() % 2.0, 0.0, "{size:?} is off the grid");
        }
    }

    #[test]
    fn steps_increase() {
        assert!(Space::Xs < Space::Sm && Space::Sm < Space::Md);
        assert!(Space::Xs.px() < Space::Sm.px());
        assert!(ControlSize::Dot.px() < ControlSize::Icon.px());
        assert!(ControlSize::Icon.px() < ControlSize::Control.px());
        assert!(Stroke::Hairline.px() < Stroke::Thick.px());
        assert!(TextSize::Caption.px() < TextSize::Body.px());
    }
}
