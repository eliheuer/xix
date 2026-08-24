# xix design, version 2

Status: exploration, 2026-08-22. Companion to [PLAN.md](PLAN.md),
which holds the evidence. This document holds one candidate
architecture, written down so it can be argued with. No decision has
been made. Version 1 is in git history (commit `d963879`); section
16 lists what changed and why; section 18 is the decisive review.

## 1. Who it is for, stated precisely

There are three actors, and the framework serves all three.

- **The design engineer.** A hybrid designer and programmer who
  directs the work and judges the result, and who does not read the
  code. They see the running application, screenshots, and data
  files (theme, keymap, layout). They speak in design terms: tighten
  this panel, make the primary action stronger, this grid needs more
  air.
- **The agent.** Writes all the code. Needs a small stable
  vocabulary, a compiler that catches mistakes, a way to see and
  touch the running app without a human, and a fast fix loop.
- **The end user of the product.** A professional using a font
  editor, a node-based editor for local AI, a video editor, a game
  editor, or a code editor. Needs speed, reliability, keyboard-driven
  workflows, deep customization, and accessibility.

The first two actors are new as a pair. No existing framework is
designed for a human who never reads the code and an agent who
writes all of it. That pairing decides most of what follows: the
agent loop is a product surface, design vocabulary is the shared
language between human and agent, and "if it compiles, it works" is
a requirement rather than a slogan, because nobody is reading the
code to catch what the compiler did not.

## 2. Why this should exist

Shnatsel's question in the Xilem thread is the right one: with over
40 Rust GUI libraries and no production-ready one, a new one needs a
reason. The reason for xix, in one paragraph:

Nobody is building a production application layer (widgets,
platform, docking, menus, canvas, text) on the Linebender rendering
stack, for professional tools, with an agent as the primary
developer and a designer as the primary director. Upstream Xilem is
on the backburner; its maintainers say the gap is menus and
features, and its users say the gap is a widget set and platform
support, not reactivity. GPUI has the application layer but not the
stack, the openness, or the rendering for path-heavy scenes. Makepad
has an agent loop but a dynamically typed DSL that fails silently.
Iced and Slint have their own stacks and their own answers. xix is
the combination, and the combination is the reason.

The corollary: xix does not compete on reactive architecture. The
reactive layer is the smallest boring thing that works with the
grain of Rust, chosen last, with everything beneath it independent
of that choice.

## 3. Principles

The seven in PLAN.md, plus:

8. **If it compiles, it works.** Static checks over runtime gotchas.
   No silent failures. No value the type system could have rejected
   is accepted at runtime. The Makepad `splash.md` gotcha list is the
   anti-pattern. (Noah's objection, that constraining everything at
   compile time is a losing battle, is right about *everything*. xix
   types the few things that matter: the design vocabulary, the state
   flow, and the structure that is static anyway. Dynamic structure
   stays dynamic.)
9. **Local reasoning.** What the code says is what happens. No hidden
   closures, no calls whose effect depends on where they run, no
   magic. Markus Kohlhase's Leptos example (`count.get()` compiling
   in the wrong place and silently never updating) is the
   anti-pattern. Explicit state, explicit updates.
10. **The agent can see and touch the app without a human.**
    Headless render, tree dump, event injection, assertions, diffs.
    Deterministic frames. No probes that lag a frame.
11. **Defaults are the product.** The default theme, density, type
    scale, and layout rules ship as they are.
12. **Design is the shared language.** Token names are the words the
    design engineer uses to direct the agent, the names in the code,
    the names in the lint, the names in the inspector, and the names
    in the theme file. One vocabulary, end to end.
13. **No DSL, no macros for views.** The agent writes Rust; the
    compiler is the reviewer.

## 4. Priorities, reordered

Version 1 led with the reactive layer. The thread and the Runebender
ports say that is backwards. In order of effort:

1. **The widget set and platform layer.** Production parts, native
   menus and shortcuts, docking, multi-window, drag-and-drop, IME,
   accessibility, clipboard, file dialogs, high DPI. This is where
   every Rust framework is thin and where the pro app lives or dies.
2. **The canvas kit.** Viewport, gestures, tools, retained scene,
   hit testing, transform groups. The center of every target app.
3. **The agent loop and the inspector.** The development interface.
4. **The design kernel.** Tokens, scales, relational spacing,
   metric-aware text, OKLCH roles. Small in code, large in effect.
5. **The reactive layer.** Last, smallest, boring.

Everything in 1 through 4 is built on Masonry widgets and works with
any reactive layer, including none (an app can drive widgets
directly) or someone else's (a Leptos-on-Masonry, as dman-os asked
for). That is the hedge: if the choice in 5 is wrong, 1 through 4
survive.

## 5. The widget set and platform layer

Every part is two layers. A headless behavior (focus, keyboard,
pointer state, ARIA role) built on Understory's focus, responder,
overlay, and selection crates, and a styled layer that uses only
kernel tokens. A new look is a theme file. The split is the one
gpui-base and gpui-component, and Bevy's headless widgets and
Feathers, both reached on their own.

Parts, by the screens the target apps are made of:

- Frame: `Shell` with titlebar or toolbar, sidebar, content,
  inspector, status bar, and a dock. Layouts serialize. Panels drag,
  split, tab, float, and restore.
- Chrome: toolbar, tool button, segmented control, tabs, breadcrumbs,
  status items, menu bar (native through `muda` where the platform
  has one, in-window through Masonry layers elsewhere, one model
  for both), context menu, command palette, tooltip, popover,
  dialog, toast, notification.
- Data: virtualized table with tabular figures, sticky header,
  column resize, sort, selection; virtualized list; tree; outline;
  property grid for inspectors.
- Input: text, multiline, number with drag-to-scrub, slider, select,
  combo with search, checkbox, switch, radio, color (OKLCH), date,
  keybinding capture, file path.
- Content: image, SVG, markdown, code block, progress, spinner,
  badge, avatar.
- Pro-app specific: timeline ruler and tracks, curve editor, color
  ramp, transport controls, zoom slider, minimap, rulers and guides.

One implementation per widget. A Masonry widget is the unit; the
reactive layer hosts any widget through one adapter, so a custom
widget does not need a second `View` implementation. This matters
for agents: half the boilerplate, half the ways to be wrong.

Platform features that are not widgets but are part of the layer:
window management, native dialogs, drag-and-drop in and out, IME,
AccessKit trees from every part by default, clipboard with rich
types, system fonts, high DPI and scale changes, power-aware frame
pacing, and a web driver as a second target.

## 6. The canvas kit

A retained island. Its session, tools, and gesture machine live in
the widget; it emits typed messages outward; the reactive layer sets
it up and receives results and is never in the per-pointer-move
path. Both Runebender ports converged on this by force; xix makes it
the designed pattern.

- `Viewport2D`: offset, zoom, optional Y-up, device scale, visible
  rect, anchored zoom, fit. From runebender-core's `ViewPort` and
  `understory_view2d`.
- Gestures: click, double click, drag with threshold, modifiers,
  wheel, pinch, with a `ToolDelegate` (`drag_began`, `drag_changed`,
  `drag_ended`, `hover`, `cancel`). Written once, on `ui-events` and
  `understory_event_state`.
- Tools: a tool is a state machine over the delegate with a cursor,
  an overlay painter, and a keymap scope.
- Scene: retained presentation with damage regions; a point drag
  repaints its dirty rect. Paths batch by style. Vello draws.
- Hit testing: `understory_index` broad phase, `understory_precise_
  hit` narrow phase, zoom-scaled tolerances.
- Transform groups: ordinary widget subtrees under an affine with
  pointer events mapped through. The capability GPUI lacks and
  ComfyUI needed. Must be a tested first-class path.
- Snapping, guides, selection, marquee, handles, rulers, grid, HUD
  labels with collision avoidance, drawn with kernel text styles.
- A raw GPU island: a `wgpu` texture the app renders itself (a 3D
  viewport for a game editor, a video frame), composited under the
  UI on the same device Vello uses.

## 7. The agent loop and the inspector

This is the development interface. It has two halves.

**For the agent**, a protocol, in-process or over JSON lines,
modeled on what Makepad Studio already ships and what Masonry's
`TestHarness` already does:

- `render`: any view or the whole app to PNG through Vello CPU, no
  window, under a second.
- `dump`: the widget tree as text with resolved tokens, sizes,
  roles, and the accessibility tree.
- `query`: find widgets by id, role, text, or token.
- `click`, `drag`, `type`, `key`, `scroll`: inject events into the
  headless app and step frames deterministically.
- `assert` and `diff`: golden PNGs and golden dumps, with image diffs
  the agent can read as numbers.
- `lint`: raw values, proximity violations, contrast failures,
  undersized targets, duplicate primaries, text styles out of
  hierarchy.
- `LLMS.md`: the whole vocabulary in one file that fits in a context
  window, plus tested recipes (inspector panel, settings page, table
  page, node body, tool palette, login).

Every claim an agent makes about the UI ("the panel is aligned now")
can be backed by a render and a dump. The design engineer reviews
evidence, not code.

**For the design engineer**, an inspector overlay in the running app:
hover shows the part, its tokens, its region, its density, and its
resolved values, in the same names the agent uses. Nudging a token
writes a patch to the theme file, not to code. Layouts, keymaps, and
themes are files the designer edits or the inspector edits for them.
Over time this is where a Figma-like surface could sit, but the
first version is an inspector, not an editor.

## 8. The design kernel

Unchanged in substance from version 1. Tokens as enums (`Space`,
`Radius`, `Elev`, `Text`, `Role`, `Emphasis`, `Motion`) taken by
every part and layout API; scales with density; relational spacing
by region and grouping depth so inner space never exceeds outer
space; metric-aware typography from Parley (cap-height alignment,
leading trim, 4 pt rhythm); color by role generated in OKLCH from
seeds with contrast checked at theme build; nested corners, hit
targets, one primary per region by construction.

New in version 2: the kernel's cascade has to pick a substrate.
Masonry has a typed `Property` system with inheritance, a stack, and
a cache. Understory has `property`, `style`, `presentation`, and
`transition`. A prototype (PLAN.md section 9) resolves one token
through each and decides. The kernel's API does not change either
way.

## 9. Layout

One model, from Figma auto layout, on Masonry's measure and layout
passes: `stack_h`, `stack_v` with gap, inset, alignment, wrap;
per-child hug, fill, or fixed on each axis; `grid`; `overlay`;
`adaptive` by width class; `spacer`; `divider`. Taffy stays
pluggable for hosts that already use it.

## 10. The reactive layer, smallest and last

Requirements it must meet: explicit state, local reasoning, static
checks, no thread-locals, host-owned loop, no macros, retained
islands, memoization by key, and no lensing tower.

The candidate, in one paragraph: one state type per application,
fixed at the root. Views are `View<S>` with exactly that one generic
and no `Action` or `Context` parameters. A component is a plain
function `fn(&S, ...) -> impl View<S>` that reads what it needs and
whose handlers mutate `&mut S`. Children that need to address part
of the state take ids or paths as arguments (`glyph_cell(&s, id)`
and `move |s: &mut S| s.select(id)`), not lenses. Library parts that
need local state keep it in the retained widget, not in `S`.
`ViewSequence` is folded into `View` with static element counts;
fixed children are tuples, dynamic children are `Vec`. `memo(key,
|| view)` prunes subtrees. Side effects happen in handlers and in an
explicit request and response channel, not during rebuild. Windows
open and close imperatively.

This is Xilem's original DX (the part users in the thread say is the
least painful) with the parts its maintainers say failed removed:
the `State`-changing lenses, the `Action` and `Context` generics,
`OneOf`, the side-effecting views, the web generalization. It is not
type-erased (Faure) and it is not signals (Floem), so it keeps
principles 8 and 9 at the cost of some convenience for apps that
want many independent sub-states. Pro apps have one document and
one UI state; that cost is small for them.

What would change the candidate: if the compile-time measurement in
PLAN.md section 9 shows the one-generic view layer is still slow to
build, the next candidate is Faure's metadata-token erasure at
component boundaries only. If the apps turn out to need many
independent sub-states, `Edit<T>`/`Read<T>` (upstream, merged
2025-11) is the fallback. Either is a local change because sections
5 through 9 do not depend on this one.

## 11. The proving applications, and what each one tests

- **Font editor (Runebender).** Tests the canvas kit, the inspector
  forms, the glyph grid (virtualized table), menus, undo. Bar: under
  5,000 lines at `PARITY.md`.
- **Node-based editor for local AI (ComfyUI class).** Tests transform
  groups with real widgets inside, `understory_node_graph`, wires
  and batching, level of detail, execution state by ticks, node
  definitions as data, ComfyUI workflow import, a long-running local
  backend over a socket. Bar: 1,000 nodes at 60 fps during pan,
  zoom, and a slider drag inside a node.
- **Agentic programming tool.** A shell around agents: conversation,
  diffs, file tree, terminal, task list. Tests the SaaS-style parts
  (markdown, code blocks, streaming text, tables), docking, and the
  case where the design engineer directs the whole thing without
  reading code. This is the dogfooding app: xix builds the tool the
  xix design engineer uses. Bar: the zero-tuning test (section 13).
- **Video editor.** Tests the timeline parts, a raw GPU island for
  frames, heavy background decode, and drag-to-scrub inputs. Bar:
  scrubbing at display rate with the UI responsive.
- **Game editor.** Tests the raw GPU island on a shared `wgpu`
  device, the property grid, asset browser, and multi-window.
- **Code editor (Zed class).** Tests a `TextView` island on Parley
  with a rope buffer, virtualized lines, optional tree-sitter
  highlighting, and 120 Hz input. Honest note: Zed's editor is most
  of Zed; this is a late target and a stretch, but the island model
  is the right shape for it, and it is the sharpest test of text
  rendering and input latency the stack will get.

## 12. Rust-native mechanics

- Tokens are enums with `const` resolution through the cascade.
  Zero cost.
- Static structure is static: tuples with compile-time counts;
  `Vec<Any>` for dynamic lists only.
- One app state type; handlers take `&mut S`; no lenses.
- Change detection by ticks on properties and widget state, so
  repaint and relayout are bounded by what changed.
- The host owns the loop: a winit driver, a bare `step(event)` API
  for embedding, a web driver later. No thread-locals.
- Stable Rust. Every dependency public and pinned. Fewer crates:
  `xix_core` (Masonry fork), `xix` (kernel, layout, parts, shell,
  app layer, reactive layer as modules), `xix_canvas`, `xix_tools`.

## 13. Acceptance tests

1. **Zero-tuning test.** An agent with `LLMS.md` and a one-paragraph
   brief builds an inspector panel, a settings page, and a table
   page. A designer counts spacing, type, and color corrections.
   Target zero. Every release.
2. **Never-read-the-code test.** A design engineer directs the
   agentic programming tool (section 11) through one week of
   changes using only the running app, renders, and the inspector.
   Count the times they had to open a source file. Target zero.
3. **Theme swap.** Change the theme file; every part and every
   proving app restyles; contrast passes in all themes.
4. Runebender under 5,000 lines at `PARITY.md`.
5. 1,000-node graph at 60 fps during pan, zoom, and an in-node drag.
6. A point drag repaints only its damage rect and rebuilds no views
   outside the canvas.
7. Headless render under one second; dump and lint under 100 ms.
8. Incremental rebuild after a one-line view change under five
   seconds; clean build of an app's view layer under a budget set
   after the prototype.
9. Every part has an AccessKit node and passes a screen reader
   smoke test on macOS and Windows.

## 14. Phases

0. Prototypes (PLAN.md section 9): token cascade on Masonry versus
   Understory; headless render of three text styles with correct
   rhythm; one-generic static view with compile-time tree shape,
   measured; Masonry driven from a caller-owned loop; `Transformed`
   hit testing under pan and zoom; a context menu on Masonry layers
   plus `understory_overlay`.
1. Engine fork on stable. Kernel. Layout. Twelve parts. Shell.
   Headless loop and `LLMS.md`. Acceptance test 1 on a settings page.
2. Canvas kit. Application layer (actions, menus, palette, docking,
   undo, background work). Runebender to `PARITY.md`.
3. Node graph kit. ComfyUI-class editor with workflow import. The
   agentic programming tool as the dogfooding app.
4. Inspector overlay. Web driver. Timeline parts. Raw GPU island.
5. Text island. Game editor and code editor experiments.

## 15. What we are deliberately not doing

- Not a DSL, not markup, not macros for views.
- Not CSS. No arbitrary numbers in the styled layer.
- Not signal-driven. Ticks and explicit keys.
- Not a new reactive architecture. The reactive layer is last and
  boring.
- Not a framework that owns your loop or your threads.
- Not a web framework. `xilem_web` is dropped; the web is a driver
  for the same Masonry tree, not a DOM backend.
- Not general-purpose. Dense, designed, professional tools.

## 17. What the Runebender port is teaching us (the canvas-island problem)

This section is written from porting Runebender to xix slice by slice
(see `runebender-xix/PORT.md`). The frictions cluster into one central
finding that should shape the fork.

### The pattern every graphics app needs, and the boilerplate it costs

A font editor is a shell of widgets around a few **retained canvas
islands**: the glyph grid, the glyph editor, later a kerning view and a
node graph. Each island owns its own session and gesture state, paints a
scene, and talks to the rest of the app. In xix today, each island costs
about 150 lines of identical scaffolding before any real content:

- a Masonry `Widget` impl (measure, layout, paint, `on_pointer_event`,
  `on_text_event`, accessibility, `children_ids`),
- a Xilem `View` impl (build, rebuild, teardown, message),
- an event enum and `submit_action` plumbing,
- an `Arc<Session>` that is cloned into the widget on build and
  reconciled on rebuild (keep the viewport, replace the session).

The grid, the editor, and even the icon button each re-implement this.
That is the clearest signal in the whole port: **the retained canvas
island must be a first-class framework primitive, not hand-rolled per
app.** Proposed shape for the xix canvas kit (DESIGN.md 6):

```rust
// You implement only this; the kit provides the Widget + View wiring,
// the viewport, the gesture recognizer, and the message channel.
trait Island {
    type Msg;                 // typed messages out
    type Cmd;                 // typed commands in
    fn paint(&self, scene: &mut Scene, view: &Viewport, theme: &Theme);
    fn gesture(&mut self, g: Gesture, view: &Viewport) -> Vec<Self::Msg>;
    fn command(&mut self, cmd: Self::Cmd) -> Vec<Self::Msg>;
}
```

### The island/state-ownership dilemma

The single hardest question the port raised: **who owns the editable
document, the island or the app?**

- If the **island owns it** (what xix does now), painting and gestures
  are trivial and fast, but the app cannot read edits without pulling
  the session back out, and the app cannot *drive* edits (a panel field
  that changes a point) without a command channel in. We hit the
  concrete bug: the island edited its own clone, emitted only events,
  and the app saved a stale glyph. The fix pulls the session back
  through the element on every message, which round-trips the whole
  session widget -> app -> widget per edit.
- If the **app owns it** (what GPUI does: one `Entity`, panel and
  canvas share it), panel-to-canvas editing is free, but every
  keystroke re-runs the whole render and the framework must make that
  cheap (GPUI does, at the cost of the runtime-borrow-checked entity
  model we rejected in principle 1).

This is the core tension between our static, message-passing model and
GPUI's shared-mutable-entity model, made concrete. The resolution xix
should adopt:

1. The document (the norad font, the edit session) lives in **app
   state**, not in the island. The island borrows it to paint and
   holds only *ephemeral* state: the viewport, the in-progress gesture,
   the active pen buffer, hover.
2. Edits are **typed commands** the island issues against the document
   (nudge, drag-to, cut, add-contour), applied by the app, so both the
   canvas and the panels act on one source of truth.
3. Repaint is driven by **tick-based change detection** (DESIGN.md 12),
   not by re-running the view: when the document or the island's
   ephemeral state changes, only the island's scene is rebuilt, and
   only its damage rect repaints. This gives GPUI's shared-state
   ergonomics without GPUI's full re-render or its runtime borrow
   checks.

In other words, the canvas kit should split **document (app-owned,
command-edited) from view state (island-owned, ephemeral)**, which is
neither "island owns everything" (today's xix, awkward for panels) nor
"app re-renders everything" (GPUI, needs a cheap full render). This is
the most important architectural decision the port surfaced, and it is
the thing to prototype next.

### Smaller primitives the port proved are missing

Each of these was hand-written in the port and belongs in the kernel:

- **Cross-axis Fill** for flex (full-height panel columns need
  `sized_box` + `Dim::Stretch` today, with the background on the box).
- **Icon button** that paints a vector (kurbo) path, not only an SVG
  string.
- **Text into a scene**: shaping a label into a canvas scene
  (`text_label.rs`) is needed by every island (grid labels, measure
  labels, HUDs) and has no one-liner.
- **A gesture recognizer** (click vs drag, thresholds, modifiers,
  drag-from-origin) rewritten in every island's `on_pointer_event`.
- **Window-level actions/keymaps/menus**: editing keys only reach the
  focused island, so cross-cutting shortcuts (save, tool switch,
  ops) have no home yet. This is the standing gap from PLAN.md.

The token kernel and layout model (DESIGN.md 4, 5) held up well; the
gap is entirely the canvas-island substrate and the document/view-state
split above.

## 18. Decisions after the Runebender port

This section is the review. Sections 1 through 17 are the exploration;
this is what building a real font editor on xix (61 slices and counting,
see `runebender-xix/PORT.md`) settled. Each decision states what the port
proved and what the fork will do. These are firm unless a later
prototype overturns them. D1 through D8 came out of the first pass;
D9 records what the second pass (real interpolation and a push to match
the GPUI editor's visual quality) added, because chasing that quality is
where the design-engineer DX gaps showed up most clearly.

### D1. The core primitive is the Scene Island, not the View

The port built a grid, an editor, an icon button, and a preview strip.
Every interactive one hand-rolled the same ~150 lines: a Masonry
`Widget` (measure/layout/paint/pointer/text/accessibility/children), a
Xilem `View` (build/rebuild/teardown/message), an event enum, and
`Arc<Session>` reconciliation. That repetition is the loudest signal in
the whole project. xix will provide a first-class `Island` where the
app implements only:

```rust
trait Island {
    type Msg;   // typed, out to the app
    type Cmd;   // typed, in from the app
    fn paint(&self, scene: &mut Scene, view: &Viewport, theme: &Theme);
    fn gesture(&mut self, g: Gesture, view: &Viewport) -> Vec<Self::Msg>;
    fn command(&mut self, cmd: Self::Cmd) -> Vec<Self::Msg>;
}
```

The framework provides the Widget and View wiring, the viewport, the
gesture recognizer, damage-based repaint, and the message/command
channels. Display-only islands (the preview strip) can already use the
built-in `canvas` view with no boilerplate; the `Island` trait extends
that ease to interactive ones.

### D2. Split the document from view state

The hardest bug of the port: the editor island owned the edit session,
edited its own clone, and emitted only events, so the app saved a stale
glyph and panels could not edit points. The resolution is now a
decision. The document (the font, the edit session) lives in app state;
the island holds only ephemeral view state (viewport, active gesture,
pen buffer, hover). Edits are typed commands the island issues against
the document, applied by the app, so the canvas and the panels act on
one source of truth. This is neither "island owns everything" (awkward
for panels, the bug we hit) nor "app re-renders everything on every
keystroke" (GPUI, which needs a runtime-borrow entity graph we rejected
in principle 1).

### D3. Repaint by ticks, not by re-running views

D2 only works if reacting to a document change is cheap. Repaint is
driven by tick-based change detection (the Bevy mechanism, DESIGN.md
12): a document or view-state change bumps a tick, the affected island
rebuilds its scene, and only its damage rect repaints. This gives
GPUI's shared-state ergonomics without GPUI's full re-render or its
runtime borrows. This is the mechanism that makes the static model
competitive; it matters far more than the choice of reactive layer.

### D4. The reactive layer is thin, static, and last

Confirmed by the whole port: the reactive layer was nearly invisible.
The work was widgets, painting, gestures, and document plumbing. xix
keeps the reactive layer minimal and static (one `View<S>` generic,
explicit messages, no lens tower, `ViewSequence` folded into `View`,
per DESIGN.md 10) and treats it as replaceable. It is not the fork's
identity and it is chosen last. The identity is D1 through D3.

### D5. A window-level action / keymap / menu layer is required framework

The most-hit gap across 28 slices. Masonry routes keys only to the
focused widget, so the port put flip/rotate/boolean/decompose on
unmodified keys and needed a Save button, because there is nowhere to
hang a real Cmd+S or a menu. xix will build a window-level action
layer: actions as plain structs, a keymap as data, dispatch down the
focus chain, one action list feeding both the keymap and the menu bar
(native via `muda` where the platform has one, in-window via Masonry
layers plus `understory_overlay` elsewhere). This is DESIGN.md 9,
promoted from plan to decision.

### D6. Ship the small primitives the port hand-wrote

Each of these was written by hand in `runebender-xix` and belongs in
the kernel or canvas kit:

- Cross-axis Fill for flex (full-height columns needed `sized_box` +
  `Dim::Stretch` with the background on the box, not the content).
- Icon / vector-path button (Masonry's `svg` view takes an SVG string,
  not a kurbo path).
- Text into a scene (`text_label.rs`: shape a label into a canvas
  scene; every island needs it).
- A gesture recognizer (click vs drag, thresholds, modifiers,
  drag-from-origin), rewritten in every island.
- A number field bound to a value with drag-to-scrub and
  commit-on-Enter (the port hand-managed input buffers to avoid
  clobbering typing, because there is no focus-aware bound field). The
  second pass reinforced this: the Width/LSB/RSB row needed three
  hand-managed buffers with cross-field refresh, since editing one
  sidebearing changes the others.
- A collection / grid view. The glyph grid (`grid.rs`) is a whole
  hand-written widget: span packing, scroll math, per-cell hit routing,
  and cell sizing. A framework grid that takes items plus a per-item
  paint closure plus a selection model would delete it, and it is the
  same primitive a node canvas, an asset browser, and a timeline all
  need. This is the highest-value missing primitive for graphics apps.
- A flat, selectable list-row (label left, value right, selected
  highlight). The sidebar's Categories/Languages/Filters rows are
  hand-assembled from `button(child)` plus `FlexSpacer` inside a
  `sized_box(Dim::Stretch)`, because there is no styled list-row.
- A color swatch / chip. Marks render as rounded squares because there
  is no rounded or circular swatch view; the bottom bar and the Mark
  panel both want one.
- A collapsible section header (a disclosure caret plus a label),
  hand-made as `label("▾ …")` in both the sidebar and the right panel.

### D7. The agent loop is a product surface, and headless must rebuild

The entire port was driven and verified through headless screenshots;
that workflow is the design engineer's and the agent's primary lens, so
it is a product surface, not a test utility (principle 10, confirmed).
One concrete fix already landed: the headless renderer must run a
`rebuild` pass, or views that populate their scene in `rebuild` (the
`canvas` view) render blank. `TestHarness::render_root_mut` plus a
rebuild in the screenshot path fixed it.

### D8. What held up unchanged

The token/design kernel (tokens as enums, OKLCH roles, relational
spacing) and the Figma-style layout model (stacks, inset, hug/fill/
fixed) caused no friction wherever used. They stay as specified in
DESIGN.md 4 and 5. All the trouble was in the canvas-island substrate
and the document/view-state split, which is exactly where the fork's
real work is.

### D9. What matching the GPUI editor's quality taught (second pass)

The second pass had two goals: real variable-font interpolation, and
closing the visual-quality gap with the GPUI version of Runebender so
that a design engineer would reach for xix on look alone. Pushing on
quality, not features, is what surfaced the DX gaps below. The rule that
paid off: treat every "the GPUI version just looks better here" as a
missing primitive, not an app problem.

- Theme lives in app state, not in a framework context. The palette is
  an `Arc<Palette>` the app threads into every view by hand, and a theme
  switch has to rebuild the baked cell colors itself. The framework
  should own a theme context that widgets read, so swapping one token id
  reskins the tree with no manual plumbing. The token/OKLCH kernel (D8)
  is right; what is missing is the delivery path from token to widget.

- Derived view-state has no home, so it is hand-wired. The
  interpolation preview is a value computed from the document (the axis
  location) that drives a read-only overlay. The port recomputes it on
  every rebuild and threads it into the island by hand. The Island plus
  document/view-state model (D1, D2) should own this "document changes,
  a derived value is recomputed, the island repaints" loop, the same way
  a background layer, a reference glyph, or a measurement readout would
  want it.

- Read-only is a real view mode the framework should name. Off a master
  the editor shows the interpolated instance read-only: edits are
  swallowed and the editing affordances (points, handles, anchors) are
  hidden. The port gates this by hand in both `paint` and every event
  handler. A first-class read-only document view would carry that once.

- Visual defaults decide whether it looks professional. Two one-line
  changes closed most of the perceived gap: filling grid glyphs with
  their mark color instead of flat gray, and softening the cell corner
  radius. The lesson for the design kernel is that the roles must ship
  with defaults good enough that the untouched app already looks like
  the GPUI version, because the design engineer will judge on the
  default, not on what is tunable.

- Headless screenshots carried the whole second pass. Every visual
  decision here was made by rendering a PNG and looking, never by
  running the GUI. This is more evidence for D7: the headless renderer
  is the design engineer's and the agent's primary lens and a first
  product surface, not a test utility.

### D10. What the inspector panel taught (third pass)

The third pass moved the master switcher and the axis sliders out of a
strip across the top of the canvas and into the inspector, where the
GPUI version keeps them, then rebuilt the sidebar rows, the search
toggles, and the coordinate picker to match. It is panel work, not
canvas work, and it hit a different set of gaps.

- The view tuple has a hard arity limit. A flex container takes a tuple
  of children and the trait is implemented up to sixteen. Adding a
  seventeenth section to the inspector is a type error with no useful
  message, and the fix is to nest a container purely to satisfy the
  trait. Panels in a real application have dozens of children. The
  framework needs a children-collection that does not run out, and the
  error, when there is one, must say so.

- Every small control fights its own padding. A 10 px dot in the
  9-point reference picker is a button with the padding zeroed and a
  fixed-size box wrapped around it, or it stretches into a pill. A
  22 px thumbnail is the same. The parts list needs a control size
  scale (dot, icon, compact, default) instead of one button that
  assumes a text label.

- Style props are order-sensitive in a way nothing announces. Calling
  `.color()` before `.text_size()` fails to compile, because the first
  property wrapper hides the inherent methods of the widget under it.
  The rule (inherent methods first, style props last) is learnable, but
  it is invisible, and an agent hits it every time.

- Nesting a scroller inside a scroller fails silently. Wrapping the
  self-scrolling glyph grid in a portal produced a rail that painted
  three rows and then a dead region, with no warning. Scrollable is a
  property of a widget that the framework knows about, so a container
  that scrolls a child that already scrolls is a detectable mistake and
  should be a debug-time lint.

- Text does not clip to its box. A master named "Bricolage Grotesque
  96pt ExtraBold" runs straight out of a 250 px panel and over the
  canvas. The port works around it by computing short names. Overflow
  behavior (clip, ellipsis, wrap) belongs on the text widget, and the
  default must be to stay inside the box.

- Reference layers are the general case of the ghost overlay. Showing
  every other master at once made the canvas unreadable, so the port
  now draws only the masters the user turns on in the Layers list. That
  is the same shape as a background image, a reference glyph, or an
  onion-skin frame in a timeline: a set of read-only scenes composited
  under the editable one. The Scene Island (D1) should take a list of
  reference scenes, not just the one it edits.

### D11. Why the GPUI port looks better with less effort

This is the most important finding in the document, because it is about
the thing xix claims to be for. Both ports are written by the same
person with the same agents against the same design. The GPUI one looks
finished and the xix one looks rough, and the reason is not taste or
time. It is five properties of the API that the GPUI version has and
xix does not.

**1. Style and interactivity belong to a generic container, not to
specific widgets.** In GPUI every node is `div()`, and `div()` takes
background, border, radius, padding, flex, text style, an id, and
`on_click`. A row, a chip, a dot, a tab, a swatch, and a button are all
the same call with different arguments. In xix, `button` is the only
thing that clicks and it arrives with a padding, a minimum size, and a
background from the property set, while `sized_box` is the only thing
that carries a box style and it cannot click. Consequences, all of them
observed: a 10 px dot is a button with its padding zeroed inside a
fixed-size box; a layer row that needs two independent hit targets
becomes two sibling buttons in a flex row instead of one row with two
handlers; a list row is four nested views where GPUI needs one.
Decision: xix ships a styled, hit-testable, focusable container as the
base of the parts list, and every part is that container plus paint.

**2. A closed scale beats a free f64.** GPUI spacing is `px_1` to
`px_4` and its type is `text_xs` and `text_sm`, so an agent cannot land
between steps. The xix port, written the same way, drifted to twenty
distinct spacing values and four text sizes, and the drift is what
reads as rough: fields at three different heights, gaps at 4, 5, 6, 8,
10 and 12 px in one panel. This is the exact failure section 3 says the
framework exists to prevent, and the port hit it because the design
kernel (D8) is still a plan. The port now carries `Space`, `Type`, and
`Radius` enums in its own `ui.rs`, which fixed the rhythm in one pass.
That file is evidence, not a solution: every app on xix would write it
again, worse. Decision: the kernel moves into the framework, and widget
APIs take the enum and nothing else.

**3. Text style must cascade.** `text_sm()` on a GPUI container sets
the type for everything under it, so an agent states it once per panel.
In xix every label carries its own size and color, so an agent has to
repeat it at every leaf and will miss some. That is literally how the
inspector ended up with 16 px rows next to 12 px rows. Decision: text
style is an inherited property on the widget tree, set on a container
and read by the text widgets under it.

**4. Layout intent needs one-word names.** `flex_1()`, `items_center()`,
`justify_between()`, `min_h(0)`, `overflow_y_scroll()`. The xix
equivalents are `sized_box(..).dims(Dimensions::new(Dim::Stretch,
Dim::Fixed(Length::px(22.0))))`, a `FlexSpacer::Flex(1.0)` child, and a
`portal`. More nodes, more concepts, and more ways to be wrong: wrapping
the self-scrolling grid in a portal produced a rail that painted three
rows and then a dead region, silently. Decision: fill, hug, fixed,
center, between, and scroll are one-word modifiers on every view, and
scroll inside scroll is a debug-time lint.

**5. The framework must be abstractable by an ordinary programmer.**
In GPUI a helper that returns a styled row is a function returning
`Div`. The same helper in xix needs this signature:

```rust
pub fn row<S>(children: S, gap: Space) -> impl WidgetView<App> + use<S>
where
    S: FlexSequence<App, ()> + 'static,
    Flex<S, App, ()>: WidgetView<App, ()>,
    <Flex<S, App, ()> as WidgetView<App, ()>>::Widget: UsesProperty<Gap>,
```

Three where-clauses, one of them naming an associated type's property
bound, to say "a row with a gap". An agent will not write that. It will
inline the flex call at every site instead, and then the values drift,
which is finding 2 again. The generics problem the maintainer's review
describes is usually stated as an error-message problem. It is worse
than that: it blocks the app from building its own vocabulary, which is
the one thing that keeps a large UI consistent. Decision: children take
a boxed collection (which also removes the sixteen-child cap), and the
style methods stay available on the erased view type, so a helper's
return type is nameable.

A counterweight, measured rather than assumed. The obvious objection is
that GPUI wins because `gpui_component` exists, a third-party styled
parts library (Longbridge, not Zed). So I counted what the port
actually imports from it: four things. Text input, slider, one
resizable split, and the native menu bar. Everything else in that
13,809-line file, every row, cell, chip, tab, panel, toolbar, and
sidebar, is hand-composed from `div()`, 175 times.

That reverses the conclusion. The parts library is not the advantage.
The container plus the scale is the advantage, and the library covers
exactly the four parts that are genuinely hard to write: text editing
with IME and selection, a drag control, a resizable split, and a
platform menu. Xilem already has the first three. The fourth is D6's
menu problem, already decided.

So xix does not need a forty-component library before it is usable, and
building one first would be building the cheap half. The order is:
the styled container and the kernel scale (findings 1 and 2), then the
four hard parts, then a thin recipe layer that names the compositions
an application repeats (list row, field, section, toolbar, panel) and
that an app can bypass with the container whenever it wants something
else. Recipes graduate from an app into the framework when the second
proving application needs the same one, not before. Runebender's
`ui.rs` is the first candidate set.

The test from section 2 should be read against this: an agent given
only the framework and a one-paragraph brief produces a screen a
designer would ship. Today, given xix, it produces the rough version,
and given GPUI it produces the finished one. Closing that gap is the
project.

### D12. What shipped, and what the first framework pass found

D11 is a set of decisions. The first of them is now in the fork, so this
records what landed and what building it turned up.

Landed: `masonry_core::kernel`, re-exported as `xilem::kernel`. `Space`,
`ControlSize`, `Stroke`, `Radius`, and `TextSize`, converting into
`Length`, `Dim`, `Padding`, `Gap`, and `f32`, so they pass straight into
the existing styling APIs (`border_width`, `corner_radius`,
`Dimensions::fixed`, and `Label::text_size` now take `impl Into<_>`).
The Runebender port was rebuilt on it: every raw pixel is gone from its
spacing, type, radius, border, and control sizes, and the five that
remain are pane geometry. `xilem/examples/app_shell.rs` is the same
thing from scratch, a full three-pane screen with three numbers in it.

What the pass found:

- Naming collides with the graphics stack. The obvious name for the
  control-height scale is `Size`, and `kurbo::Size` is everywhere in
  this codebase, so it is `ControlSize`. Any kernel type has to be
  checked against kurbo, peniko, and parley before it is named.

- The escape hatch has to be explicit, not implicit. `Dimensions::fixed`
  became generic over `Into<Length>`, which cost the `const` form; it is
  now `fixed_px`. That is the right split: the token path is the easy
  one, the raw path is the one you have to type differently and can
  grep for. Every kernel-adjacent API should follow it.

- Pane geometry is not a token, and pretending otherwise would be
  dishonest. A sidebar is 232 px because that is how wide the content
  is, not because a scale says so. GPUI does the same. The kernel covers
  spacing, control size, stroke, radius, and type, and stops there.

- There is no icon set, and the default font has no symbols. A section
  header that types a disclosure caret renders tofu, in the example and
  in the port. Icons are framework furniture, the same class of thing as
  menus: boring, skipped, and disqualifying when missing. The parts list
  needs a small vector icon set with the kernel's sizes.

- The kernel is static, and a theme is data. `tokens.rs` drives density,
  radius, and hue from sliders, which the static enums cannot express.
  Both are wanted: the enums fix the vocabulary, and a theme should
  resolve them, so a compact density can move `Space::Md` from 8 to 6
  everywhere at once. That resolution path (D9's theme-context finding)
  is the next piece.

### D13. What the examples found

The examples are the test bed for these decisions, so what they resist is
evidence. Three things came out of writing them.

- A theme resolves the kernel, and the two are different jobs. The enums
  fix the vocabulary; a theme decides what a step is worth. `tokens.rs`
  drives density, roundness, and hue from sliders, and the panel under
  them re-spaces without a line changing. It also shows why one global
  multiplier is wrong: spacing scales fully with density, control heights
  scale less (a hit target that follows spacing down stops being
  hittable), and text does not scale at all. A theme knows that. A
  stylesheet with one scale factor cannot.

- Controls that hold text have a floor, and nothing enforces it. Give a
  text input a fixed height under its line height and it clips the
  descenders silently, with no warning at any layer. `tokens.rs` works
  around it by clamping in the theme, which is the wrong place: the
  widget knows its text metrics, so the minimum belongs to the widget and
  the fixed size should be negotiated against it, not obeyed.

- The island is the shape D1 predicted. `shapes.rs` is a working editor
  (draw, select, drag, inspector fields, pan, zoom) with no viewport math
  in it, because the widget owns the view and reports gestures already in
  document coordinates. The parts the application still writes are hit
  testing and a small gesture enum, and both are genuinely its own. The
  next thing the island needs is reference scenes (D10) and a tool
  delegate, so the gesture enum stops being per-app boilerplate.

### D14. A theme is data, and the widget defaults are the proof

The gallery example put every built-in control on one screen next to
application views that used the kernel, and the first render answered a
question the port had never asked: the application's accent was green and
every built-in control was blue. Not a bug in either place. The defaults
of all thirty-odd widgets were a few hundred lines of hardcoded colors
and pixel counts in `theme.rs`, reachable only by rewriting the function.

So `masonry::theme::Theme` now exists: a struct of role colors plus the
kernel steps a widget styles itself from, with `Theme::dark()` as the
shipped default and `Theme::property_set()` generating what
`default_property_set()` used to hardcode. An application states its
theme once and every built-in control follows it. The gallery does that
in nine lines and its controls turn green.

Two things this settles, and one it does not:

- Settled: a theme is data, not code. It was already the plan (D8); it is
  now true for the widget set, and the shipped dark theme is one value of
  the type rather than the only possible interface.

- Settled: the kernel reaches the widgets. `Theme` holds `Radius::Md`,
  `Space::Md`, and `Stroke::Hairline` rather than 6, 8, and 1, so a
  density change moves every built-in control with it.

- Not settled: text size. Color is a property, so one theme sets it for
  every widget at once. Text size is chosen by each view when it is
  built, so the gallery still shows built-in labels a size larger than the
  body text beside them. This is D11's third finding, and the fix is an
  inherited text style rather than a wider `Theme`.

### D15. How agent-written screens come out good, by construction

This is the question the whole fork answers, so it gets a mechanism, not
a principle. The mechanism is one sentence: **remove the decision, do not
document it.** If a value can be derived from context, the API must not
accept it. Every layer below is an application of that sentence, and the
test for each is the same: after this layer exists, what is an agent
still free to get wrong?

**Layer 1, the scale. Landed.** `Space`, `ControlSize`, `Stroke`,
`Radius`, `TextSize`. Removes the values between the steps. What an agent
can still get wrong: which step. Twenty spacing values became seven.

**Layer 2, regions. Landed.** A container states what it is, not what it
measures: `column(Region::Panel, ..)`, `row(Region::Toolbar, ..)`,
`column(Region::List, rows)`. The gap and the inset come from the kind.
This removes the remaining choice, because "how much space between a
toolbar's buttons" does not vary per screen, and an author who is asked
anyway will answer differently each time. Rebuilding `app_shell.rs` on
regions took its spacing decisions from 21 to 2, and the two that remain
are `Space::None` on the seams between panes, which is a statement that
there is no gap rather than a guess about how big one is.

Two rules make regions compose without a margin conversation:

- A region that paints nothing has no inset. A form inside a panel does
  not double the panel's padding. This is the whole of the margin
  question, answered once in `Region::inset` rather than argued per
  element. Elements have no outer margin at all; there is no such
  property to set.
- An outer region's gap is never smaller than an inner region's gap, so
  grouping reads as grouping at every depth. This is a unit test, and it
  caught the first version of the table putting a toolbar inside a card.

**Layer 3, the theme. Landed.** `masonry::theme::Theme` holds the role
colors and the kernel steps every built-in widget styles itself from, so
one value restyles the whole widget set. A view asks for a role and never
for a color. What is still missing here is inherited text style, which is
why built-in labels are still a size larger than the body text beside
them.

**Layer 4, the parts. Next.** The recipes an application repeats: list
row, field, section header, toolbar, panel, card, dialog, and the four
hard ones (text input, slider, resizable split, menu). Every part is
built from the layers above, so a part cannot be off the scale, and an
application that uses parts is drift-proof without trying. The rule from
D11 stands: a part graduates from an application into the framework when
a second application needs the same one.

**Layer 5, enforcement. Next.** Three tools, all of which exist to make a
mistake loud instead of invisible:

- `xix lint`: raw pixel outside a pane geometry, second primary action in
  one region, hit target under the minimum, contrast pair that fails,
  region nested inside a tighter one, text clipped by a fixed height.
- `xix dump`: the widget tree with resolved tokens, so an agent can read
  back what it actually built instead of guessing from its own source.
- Headless screenshots (landed): the agent renders a PNG and looks at it.
  Every visual decision in this fork so far was made that way.

**The files a design engineer owns.** The point of the layers is that the
application's code does not carry appearance, so appearance can live in
data the agent never edits:

```
xix.toml         # density, font stack, motion, platform behavior
theme.toml       # role seeds in OKLCH, contrast targets, per-role overrides
```

A theme file states seeds and rules, not a value per widget: an accent
hue, a surface lightness ramp, a contrast floor. The build resolves those
into a `Theme` and fails on a pair that cannot meet the floor, which is
the same discipline the kernel applies to spacing. Live reload of both is
data reload, not code reload, so it stays inside D2's "live editing is for
data" line.

**What the agent is told.** One `LLMS.md` in the repository, holding the
whole vocabulary and nothing else: the five scales, the region list, the
role list, the parts list, and the three commands. The instruction to an
agent is not "use 8px gaps"; it is "containers are regions, colors are
roles, sizes are steps, and `xix lint` has to pass". That fits in a
prompt, which is the actual constraint.

**The honest remainder.** After all five layers, an agent can still choose
the wrong region for a container, the wrong role for a color, and a bad
information hierarchy. Those are design judgments, and no framework
removes them. What the layers remove is the class of defect that makes
agent output look unfinished even when the judgment was fine: the 13 px
padding, the three button heights, the doubled margin, the accent that
does not match the accent two panels over.

### D16. Smaller findings from the region pass

- The generics complaint in D11 was harsher than the truth. Writing a
  reusable container helper fails on a missing bound, but the bound is
  `Seq: Send + Sync`, not the associated-type incantation the app ended up
  with. It is discoverable, but not from the error, which lists unrelated
  types that implement the trait. The fix is a better diagnostic, not a
  redesign.

- A helper that returns `impl WidgetView` is unstyleable by its caller.
  `column(Region::Panel, ..)` still has to accept a background and a
  corner, so the return type is named (`RegionStack`) rather than hidden.
  Any part the framework ships has to make the same choice, and hiding the
  type is the wrong half of it.

- Stretch has no per-child opt-out. A column stretches its children, which
  is right for rows and wrong for a button, and there is no way to say
  "hug" on one child: `Dim::Auto` means "let the container decide", and the
  container decided. `app_shell.rs` works around it with a spacer. A `Hug`
  that resists a stretching parent is missing from `Dim`.

### The one-sentence conclusion

xix is not a reactive-architecture bet; it is a Scene Island substrate
with a document/view-state discipline and a window-level action layer,
on the Linebender stack, and the reactive layer is a thin static
convenience on top. The port turned that from a hypothesis into a
decision.

## 16. What changed from version 1, and why

- **Three actors instead of one.** Version 1 described the design
  engineer as a coder who uses agents. The brief is stronger: they
  never read the code. So the agent loop moved from a tooling
  section to a primary interface, and principles 8 through 10 became
  requirements.
- **Priorities reordered.** The thread (dman-os, Shnatsel, Faure's
  own scope cuts) says the gap is widgets and platform, not
  reactivity. Version 2 puts the widget set first and the reactive
  layer last, and makes everything below the reactive layer
  independent of it.
- **A reason to exist, stated.** Shnatsel's question gets a direct
  answer in section 2.
- **Explicit state.** Markus Kohlhase's examples are the spec for
  the reactive layer's DX. The candidate is Xilem's original
  callback model with one state type and no lenses, instead of
  version 1's one-way typed messages.
- **One implementation per widget.** Custom widgets need a Masonry
  impl only; the reactive layer adapts any widget.
- **More proving apps, honestly graded.** Agentic programming tool
  (the dogfooding app), video editor, game editor, code editor,
  each with what it tests and a note on how far off it is.
- **Raw GPU island** added for video frames and 3D viewports.
- **Accessibility** promoted to an acceptance test, because every
  surveyed framework fails it and SaaS products cannot.

## Sources

PLAN.md sections 4 through 12 and the Zulip thread filed in section
11, plus the sources listed there.
