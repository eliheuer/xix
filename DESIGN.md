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
this is what building a real font editor on xix (28 slices, see
`runebender-xix/PORT.md`) settled. Each decision states what the port
proved and what the fork will do. These are firm unless a later
prototype overturns them.

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
  clobbering typing, because there is no focus-aware bound field).

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
