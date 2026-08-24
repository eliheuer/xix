// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Containers that space themselves from what they are.

use masonry::kernel::Region;
use masonry::properties::types::CrossAxisAlignment;
use masonry::properties::{Gap, Padding};

use crate::style::Style;
use crate::view::{Flex, FlexSequence, Prop, flex_col, flex_row};

/// What [`column`] and [`row`] return: a flex container with its gap and
/// inset already set from its [`Region`].
///
/// The type is named rather than hidden behind `impl WidgetView` so that
/// the result stays styleable. A panel still needs a background and a card
/// still needs a corner, and a region decides spacing, not appearance.
pub type RegionStack<Seq, State, Action> =
    Prop<Padding, Prop<Gap, Flex<Seq, State, Action>, State, Action>, State, Action>;

/// A vertical container whose spacing comes from its [`Region`].
///
/// This is the form to reach for instead of `flex_col(..).gap(..)`. The
/// gap and the inset are decided by what the container is, so they are not
/// a judgment call on each screen and cannot drift between screens:
///
/// ```ignore
/// column(Region::Panel, (header, body, status))
/// column(Region::Form, (name_field, email_field))
/// column(Region::List, rows)
/// ```
///
/// A region that paints nothing (a form, a list, an inline run) has no
/// inset of its own, so nesting one inside a panel does not double the
/// panel's padding. That is the whole of the margin question, answered
/// once, in [`Region::inset`].
pub fn column<State, Action, Seq>(region: Region, children: Seq) -> RegionStack<Seq, State, Action>
where
    State: 'static,
    Action: 'static,
    Seq: FlexSequence<State, Action> + Send + Sync,
{
    stack(flex_col(children), region, CrossAxisAlignment::Stretch)
}

/// A horizontal container whose spacing comes from its [`Region`].
///
/// See [`column`]. A row centers its children across the axis and a column
/// stretches them, because those are the two defaults that are right
/// almost every time: a label beside a control wants a shared center line,
/// and a stack of rows wants a shared left edge and a full-width highlight.
/// Both were wrong in the first version of this file, and the example that
/// used it showed it immediately.
pub fn row<State, Action, Seq>(region: Region, children: Seq) -> RegionStack<Seq, State, Action>
where
    State: 'static,
    Action: 'static,
    Seq: FlexSequence<State, Action> + Send + Sync,
{
    stack(flex_row(children), region, CrossAxisAlignment::Center)
}

/// Shared body of [`column`] and [`row`].
fn stack<State, Action, Seq>(
    flex: Flex<Seq, State, Action>,
    region: Region,
    alignment: CrossAxisAlignment,
) -> RegionStack<Seq, State, Action>
where
    State: 'static,
    Action: 'static,
    Seq: FlexSequence<State, Action> + Send + Sync,
{
    flex.cross_axis_alignment(alignment)
        .gap(region.gap())
        .padding(region.inset())
}
