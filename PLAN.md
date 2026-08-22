# xix planning document

This is a working document. It collects what we know about Xilem
from building a real application on it, and it records the decisions
we make about the fork. Edit it as the plan changes. Sections marked
OPEN are not decided yet.

Last updated: 2026-08-21.

## 1. Why a fork

We built a font editor, runebender-xilem, on Xilem. We then built the
same editor again on GPUI (runebender-gpui) to compare the two
frameworks on one application. The comparison gave us a list of
problems with Xilem that we hit in practice, not in theory. That list
is the starting point for xix.

The fork must keep what Xilem gets right:

- Pure Rust, no macros or DSL for the view layer.
- Vello for rendering, Parley for text, kurbo for geometry.
- A widget layer (Masonry) that is usable without the reactive layer.

OPEN: the thesis of the fork. See section 5.

## 2. Evidence from runebender-xilem

Source: `runebender-xilem/context/*.md`, the pinned upstream rev, and
the port history. runebender-xilem is about 34k lines of Rust and
pins Xilem to git rev `7819435e` (2026-08-07, Xilem 0.4.0 workspace).

### 2.1 No window-level keyboard shortcuts

Xilem has no way to register a shortcut like Cmd+S at the window
level. The workaround was a full-size `KeyboardShortcutsWidget` in a
`zstack` that requests focus on pointer move and handles keys when
focused. Both positions in the stack fail:

- At the bottom, it only gets focus when the pointer is over empty
  background, so shortcuts only work there.
- At the top, it intercepts all pointer events and breaks the widgets
  below it, even without `set_handled()`.

There is no "transparent" widget that receives keyboard events but
lets pointer events pass through. What we wanted: window-level
shortcut registration that fires regardless of focus.

Note: `runebender-xilem/context/keyboard-shortcuts-zstack.md`.

### 2.2 No gesture layer above ui-events

`ui-events` gives raw W3C-style pointer events. `ui-input-state` gives
frame-based polling. Neither gives click-versus-drag, drag thresholds,
a drag state machine, or delegate callbacks (`drag_began`,
`drag_changed`, `drag_ended`). Every tool needs these. We wrote
`mouse.rs` to provide them. A Linebender contributor agreed the
gesture layer belongs somewhere (possibly Understory) but it does not
exist yet.

Note: `runebender-xilem/context/mouse-ui-events.md`.

### 2.3 Text input is incomplete

`text_input` accepts `text_alignment()` but the placeholder ignores
it. Small, but typical: the bundled widgets cover demos, not dense
tool UIs with many aligned numeric fields.

Note: `runebender-xilem/context/text-input-placeholder-alignment.md`.

### 2.4 Version churn across Linebender crates

Linebender crates move at different speeds. `spline` (hyperbezier) was
on kurbo 0.9 while the app was on kurbo 0.12, so `kurbo::Point` was two
incompatible types and we converted at every call. Upstream is now
kurbo 0.13.1, vello 0.8, parley 0.8, peniko 0.6, wgpu 28, winit 0.30.
Apps pin to git revs because releases lag the fixes they need, and
each release breaks the app.

Note: `runebender-xilem/context/kurbo-version-mismatch.md`.

### 2.5 Rebuild-everything reactive loop

Every state change rebuilds the full view tree from `AppState`, then
Xilem diffs it against the old tree. For a font editor the tree is
large (glyph grid with hundreds of cells, editor canvas, sidebars,
panels). Runebender worked around it with `Arc` data and custom
widgets that bypass the view layer for the canvas.

OPEN: we have not measured this. Before we claim it is a problem we
need numbers. See section 6.

Note: `runebender-xilem/ARCHITECTURE.md` section 1.

### 2.6 Xilem versus Masonry split

Custom widgets need a Masonry `Widget` impl plus a Xilem `View` impl
plus a `ViewMarker`. The generics (`View<State, Action, Context>`,
`ViewSequence`, `OneOf`) are hard to read and hard to extend.
Runebender ended up with its own `widgets` module that does most of
the real work, and uses the Xilem layer mostly for layout chrome.

## 3. Evidence from runebender-gpui

Source: `runebender-gpui/README.md`, `runebender-gpui/PARITY.md`.

The GPUI port reached near feature parity with the web editor in about
14.5k lines of app code (it shares `runebender-core` with the other
ports, so the line counts are not directly comparable to
runebender-xilem). Things GPUI provided that Xilem did not:

- Native menus and keyboard shortcuts as actions, dispatched by the
  framework.
- Context menus.
- A complete widget set through gpui-component (inputs, tabs, panels,
  scroll, tree views).
- An element model where a custom canvas is a first-class element,
  not a special case.

Things GPUI got wrong, which xix must not repeat:

- It does not compile on nightly Rust (`pathfinder_simd`), so the
  repo pins stable.
- The browser build needs `-Z build-std`, wasm atomics, and COOP/COEP
  headers. Text inputs and menus do not work in the browser.
- GPUI is a Zed internal that happens to be published. It tracks Zed
  main, gpui-component tracks it, and the app tracks both from git.
- Not Linebender: Vello, Parley, and kurbo are not the rendering
  stack, so our font tooling does not share types with the UI.

## 4. Upstream snapshot (2026-08-07, rev 7819435e)

Workspace crates: `xilem_core`, `xilem_masonry`, `xilem` (winit
wrapper), `xilem_web`, `masonry_core`, `masonry`, `masonry_winit`,
`masonry_testing`, `masonry_imaging`, `tree_arena`, `placehero`
(demo app). Version 0.4.0. `rust-version = "1.92"`. License
Apache-2.0.

Masonry widgets (38): align, badge, badged, button, canvas, checkbox,
collapse_panel, disclosure_button, divider, flex, grid, image,
indexed_stack, label, pagination, passthrough, portal, progress_bar,
prose, radio_button, radio_group, resize_observer, scroll_bar,
selector, selector_item, sized_box, slider, spinner, split,
step_input, svg, switch, text_area, text_input, variable_label,
virtual_scroll, zstack.

Missing from that list: menu, menu bar, context menu, tabs, tree,
tooltip, popover, modal, combo box, table, any window-level shortcut
registry. Note `passthrough` is new since Runebender's zstack
workaround; check whether it solves 2.1.

Upstream is active (commits within the last two weeks). A fork that
diverges hard will fall behind quickly. A fork that stays close can
merge upstream regularly.

## 5. OPEN: the thesis

Three candidate directions. They are not exclusive, but one has to be
the headline.

A. **Keep the model, finish the app layer.** Same reactive design.
   Add what a real app needs: window-level shortcuts, menus, context
   menus, gesture recognition, complete text input, tabs, tooltips,
   popovers. Stay mergeable with upstream.

B. **Replace the view layer, keep Masonry and Vello.** Drop the
   `View`/`ViewSequence` generics for a simpler element model on top
   of Masonry widgets. Bigger bet, bigger divergence.

C. **App-first fork.** Fork so Runebender can ship. Land fixes on our
   schedule, upstream what Linebender accepts, keep the rest. No
   architecture claim up front; let the thesis emerge from the fixes.

Questions to answer before choosing:

- Do we want to stay mergeable with upstream? If yes, B is out.
- Is Runebender the only target app, or is xix for others too?
- What is the browser story? `xilem_web` is a DOM backend, not
  Masonry-on-wasm. runebender-web already runs Vello in the browser.
- Who maintains the fork after the first push?

## 6. Things to measure before we decide

- Frame time of runebender-xilem on a large glyph grid and during a
  point drag, with the view-tree rebuild cost separated from paint.
- Whether Masonry's new `passthrough` widget solves the shortcut
  problem without a fork.
- How much of the upstream widget set Runebender actually uses, and
  how much lives in its own `widgets` module.
- Number of upstream breaking changes between the rev we pin and
  current main, and how long a rebase takes.

## 7. Next steps

1. Agree on the thesis (section 5).
2. Do the measurements (section 6) and record results here.
3. Write `docs/problems.md` as the polished, public version of
   section 2, and `docs/design.md` for the chosen direction.
4. Import the upstream workspace at a pinned rev and rename crates.
5. Port runebender-xilem to xix as the first real test.
