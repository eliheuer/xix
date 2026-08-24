// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Regions: spacing that comes from what a container *is*.
//!
//! The [scale](super) removes the values between the steps. It does not
//! remove the choice: an author still picks `Space::Md` here and
//! `Space::Lg` there, and a hundred independent choices still drift, just
//! more coarsely. The way to stop that is to stop asking.
//!
//! A region says what a container is for. A toolbar is tight, a panel is
//! roomy, a list is dense, a dialog is generous, and none of that is a
//! judgment the author of a particular screen has to make, because it does
//! not vary by screen. It varies by kind of container, which the author
//! already knows and already has to state.
//!
//! ```ignore
//! column(Region::Panel, (header, body, footer))   // not .gap(Space::Md)
//! row(Region::Toolbar, (save, undo, redo))        // not .gap(Space::Sm)
//! ```
//!
//! The rule that keeps it coherent, and that [`Region::gap`] is checked
//! against in tests: an outer region's gap is never smaller than the gap of
//! a region nested inside it. Grouping reads as grouping, at every depth,
//! because the numbers cannot invert.

use super::Space;

/// What a container is for. This decides its inset and the gap between its
/// children, so neither is a number anyone types.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Region {
    /// The window's own content area, around everything else.
    Window,
    /// A dialog or a sheet: the most generous thing on screen.
    Dialog,
    /// A panel, sidebar, or inspector. The default for a pane of content.
    #[default]
    Panel,
    /// A titled group of related controls inside a panel.
    Section,
    /// A card in a collection.
    Card,
    /// A form: labeled fields down the page.
    Form,
    /// A toolbar, header, or status bar.
    Toolbar,
    /// A run of controls that belong to one another, like a button pair.
    Inline,
    /// A dense list, tree, or table: rows, not paragraphs.
    List,
}

impl Region {
    /// The space between children of this region.
    pub const fn gap(self) -> Space {
        match self {
            Self::Window => Space::Xl,
            Self::Dialog => Space::Lg,
            Self::Panel => Space::Lg,
            Self::Section => Space::Md,
            Self::Card => Space::Sm,
            Self::Form => Space::Md,
            Self::Toolbar => Space::Md,
            Self::Inline => Space::Sm,
            Self::List => Space::Xs,
        }
    }

    /// The space between this region's edge and its children.
    ///
    /// A region that groups without drawing anything (a form, a list, an
    /// inline run) has no inset: its parent already provided one, and a
    /// second one doubles the margin. This is the rule that removes the
    /// margin conversation.
    pub const fn inset(self) -> Space {
        match self {
            Self::Window => Space::Xl,
            Self::Dialog => Space::Xl,
            Self::Panel => Space::Lg,
            Self::Section => Space::None,
            Self::Card => Space::Md,
            Self::Toolbar => Space::Md,
            Self::Form | Self::Inline | Self::List => Space::None,
        }
    }

    /// Whether this region paints a surface of its own.
    ///
    /// A painted region owns an inset and a corner. An unpainted one is
    /// pure grouping.
    pub const fn is_surface(self) -> bool {
        matches!(
            self,
            Self::Window | Self::Dialog | Self::Panel | Self::Card | Self::Toolbar
        )
    }

    /// How loose this region is, from the window inward.
    ///
    /// The order is not a taste ranking: it is which regions can contain
    /// which, and the test below holds it to the gaps. Getting this wrong
    /// is what the invariant catches. The first version of this table put
    /// a toolbar inside a card, and the test said so.
    pub const fn rank(self) -> u8 {
        match self {
            Self::Window => 0,
            Self::Dialog => 1,
            Self::Panel => 2,
            Self::Toolbar => 3,
            Self::Section => 4,
            Self::Form => 5,
            Self::Card => 6,
            Self::Inline => 7,
            Self::List => 8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Region; 9] = [
        Region::Window,
        Region::Dialog,
        Region::Panel,
        Region::Section,
        Region::Form,
        Region::Card,
        Region::Toolbar,
        Region::Inline,
        Region::List,
    ];

    /// The invariant that makes nesting read correctly: whatever contains
    /// something else is not tighter than the thing it contains.
    #[test]
    fn outer_gaps_are_never_smaller_than_inner_gaps() {
        for outer in ALL {
            for inner in ALL {
                if outer.rank() < inner.rank() {
                    assert!(
                        outer.gap().px() >= inner.gap().px(),
                        "{outer:?} ({}) is tighter than {inner:?} ({}), which it can contain",
                        outer.gap().px(),
                        inner.gap().px()
                    );
                }
            }
        }
    }

    /// A region that draws nothing has no inset, so a group inside a panel
    /// does not double the panel's own padding.
    #[test]
    fn only_painted_regions_have_an_inset() {
        for region in ALL {
            if !region.is_surface() {
                assert_eq!(
                    region.inset().px(),
                    0.0,
                    "{region:?} paints nothing but insets its children"
                );
            }
        }
    }

    /// Every region is on the scale, by construction: there is no way to
    /// express a gap that is not a step.
    #[test]
    fn every_region_is_on_the_scale() {
        for region in ALL {
            let gap = region.gap().px();
            assert!(
                [0.0, 2.0, 4.0, 8.0, 12.0, 16.0, 24.0].contains(&gap),
                "{region:?} has an off-scale gap of {gap}"
            );
        }
    }
}
