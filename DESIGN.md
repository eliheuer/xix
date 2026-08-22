# xix design: a UI framework for design engineers

Status: proposal, 2026-08-21. Companion to [PLAN.md](PLAN.md), which
holds the evidence. This document holds the architecture. Nothing
here is final; it is the thing we argue with.

## 1. Who it is for

The core user is a design engineer: a hybrid designer and programmer
who builds internal tools and SaaS products fast, and who does most of
the building through AI agents. They know what good looks like. They
do not want to spend the afternoon telling an agent that the padding
is wrong again.

The second user is the professional graphics application: font
editors, node-based editors, video editors, trading terminals. Same
framework, same design kernel, heavier canvas.

The test for every decision: does an agent, given only the framework
and a one-paragraph brief, produce a screen a designer would ship
without touching spacing, type, or color? If not, the framework is
missing a constraint, not the agent a skill.

## 2. Why agent-built UI looks bad, and what fixes it

Agent output drifts because the frameworks it targets accept any
value. `padding: 13px`, `#3b3f44`, `font-size: 15px`, a margin on a
child here and a gap on a parent there. Every value is a free number,
so every value is a decision, and a thousand small decisions made
independently never add up to a system. The current fixes (MCP design
registries, prompt skills that "ban generic fonts by name", review
rule checklists) all try to constrain the agent from outside. They
work a little. They do not work by construction.

xix moves the constraint into the type system. Design is a type.

- Spacing, radius, elevation, type, color, and motion are enums, not
  numbers. Widget APIs accept the enum. The compiler rejects off-scale
  values before a human sees them.
- Elements never carry outer margins. Containers own all spacing.
  The margin conversation is deleted, not won.
- Spacing is relational. A container resolves its gaps from its
  region, its density, and how deep its groups nest, so the
  "internal space is less than or equal to external space" rule holds
  automatically.
- Typography is metric-aware. Text sits on a rhythm, icons center on
  cap height, and nested corners compute their own radius.
- Color is by role, generated in OKLCH from a few seeds, contrast
  checked when the theme is built.
- The loop closes without a window. Headless render to PNG, a text
  dump of the tree with resolved tokens, a design lint, and snapshot
  tests. The agent can see what it built and check it.

Rust is the right medium for this because the checks are static and
free. A `Space::M` is one `match` away from a float. No runtime, no
stylesheet, no DSL. The agent already writes Rust; the compiler is
the reviewer that never gets tired.

## 3. Principles specific to this document

These extend the seven principles in PLAN.md.

8. **Defaults are the product.** The default theme, density, type
   scale, and layout rules must be good enough to ship. Escape
   hatches exist (`Space::raw(px)`) but they are visible in code and
   flagged by the lint.
9. **Compose at the level designers think.** Stacks, insets, groups,
   sections, forms, shells, panels, not divs and flex properties.
10. **One vocabulary, end to end.** The same token names appear in
    the theme file, the Rust API, the lint output, the tree dump, and
    the docs. An agent reads one file (`LLMS.md`) and knows all of it.
11. **No DSL, no macros for views.** Makepad bets on a live DSL for
    agents; Slint on markup. We bet on Rust types plus compiler
    diagnostics, because that is the loop agents already run, and
    because a DSL is a second language to keep consistent.

## 4. The design kernel

Everything visible is derived from the kernel. Widgets, layout, and
the canvas kit consume it; nothing bypasses it except the app's own
canvas drawing.

### 4.1 Tokens as types

```rust
pub enum Space   { None, Hairline, Xs, S, M, L, Xl, Xxl, Raw(f32) }
pub enum Radius  { None, S, M, L, Full, Raw(f32) }
pub enum Elev    { Flat, Raised, Overlay, Modal }
pub enum Text    { Display, Title, Heading, Body, Label, Caption, Code }
pub enum Role    { Surface, SurfaceAlt, Border, Text, TextMuted,
                   Accent, Success, Warning, Danger, Selection }
pub enum Emphasis{ Primary, Secondary, Tertiary }
pub enum Motion  { Instant, Quick, Standard, Slow }
```

Widget and layout APIs take these and nothing else:

```rust
stack_v((
    heading("Metrics"),
    form((
        field("Ascender", number(&m.ascender)),
        field("Descender", number(&m.descender)),
    )),
    button("Apply", Emphasis::Primary, on_apply),
))
.gap(Space::M)
.inset(Space::L)
```

Resolution is a `match` against the active theme's scale at layout
time, through the property cascade. Zero cost. `Raw` variants exist
for the 1 percent of cases that need them, and the lint lists every
one.

### 4.2 Scales and density

The theme holds scales, not values:

- Space scale on a 4 pt base: 0, 1, 4, 8, 12, 16, 24, 32, 48, 64.
  Steps are tokens; the theme may shift the whole scale by density.
- Density: `Compact` (pro tools), `Regular`, `Comfortable` (consumer
  SaaS). One property, inherited down the tree, changes every
  resolved space and control height at once.
- Type scale with line heights on a 4 pt rhythm. Sizes may be odd;
  line heights never are.
- Radius scale with a factor, so a theme can be square, soft, or
  round without touching widgets.
- Control heights derived from density and text size, so inputs,
  buttons, and selects in one row are the same height by construction.

### 4.3 Relational spacing

The rule that makes layouts "just come out right" is proximity:
things that belong together sit closer than things that do not.
Designers do this by hand. xix does it by structure.

- `group(...)` is a stack that marks a grouping level. A stack
  resolves its gap from its grouping depth: depth 0 uses step N,
  depth 1 uses step N-1, and so on. Outer gaps are always at least
  inner gaps.
- Regions set the base step: `Toolbar` and `StatusBar` are tight,
  `Panel` and `Inspector` are medium, `Content` and `Dialog` are
  generous. A stack inside a toolbar is tight without being told.
- Insets follow the same table. A panel's inset is one step above its
  internal gap.

An agent writes structure. Spacing is a consequence of structure.
The agent can still say `.gap(Space::L)` explicitly, and the lint
reports when an explicit value breaks the proximity rule.

### 4.4 Typography that knows its metrics

Parley exposes real font metrics, so xix can do what CSS cannot do
well:

- Cap-height alignment. An icon next to a label centers on the cap
  height of the text, not on the line box. A label inside a button is
  optically centered.
- Leading trim. A stack of two text blocks measures its gap from the
  ink of the first to the ink of the second, so the rhythm the
  designer sees is the rhythm the token says.
- Baseline grid. Body text snaps line boxes to the 4 pt rhythm.
- Numeric tabular figures by default in tables, inputs, and status
  bars.

### 4.5 Color by role, built in OKLCH

A theme is a few seeds (accent hue, neutral tint, contrast target,
light or dark) plus rules. The kernel generates the full role palette
in OKLCH: surfaces at stepped lightness, borders derived from
surfaces, text at verified contrast, hover, pressed, focused,
selected, and disabled states derived per role. runebender-core's
OKLCH theme JSON is a working prototype of this.

Contrast is checked when the theme is built (APCA or WCAG, selectable)
and the build fails on a failing pair. An agent never picks a hex
value; it picks `Role::TextMuted` on `Role::SurfaceAlt`, and that pair
is known to pass.

### 4.6 Details that come right by construction

- Nested corners: a child inside an inset container gets
  `outer_radius - inset` automatically, clamped at zero.
- Hit targets: any interactive widget below the minimum target size
  grows its hit area, not its visual, and the lint reports it.
- One primary per region: a second `Emphasis::Primary` button in the
  same region is a lint warning.
- Focus rings, selection, and disabled looks come from tokens and
  apply to every widget, including custom ones that use the kernel.
- Motion: property transitions use `Motion` tokens with one easing
  family; no widget invents its own duration.

## 5. Layout model

xix uses one layout model, taken from how designers already work in
Figma auto layout, instead of CSS flex plus grid plus positioning:

- `stack_h`, `stack_v`: direction, gap, inset, alignment, wrap.
- Per-child sizing on each axis: `Hug` (fit content), `Fill` (take
  remaining space, with weight), `Fixed(Space)`.
- `grid`: columns and rows with gap; cells are stacks.
- `overlay`: layers in one box for badges, handles, and canvas HUDs.
- `adaptive`: alternatives chosen by available width class
  (`Compact`, `Medium`, `Expanded`), the SaaS responsive story
  without media queries.
- `spacer`, `divider`.

A Figma frame maps to a stack one to one, which matters for the
design engineer and for a future importer. The model is implemented
on Masonry's measure/layout pass. Taffy stays pluggable behind the
same API for hosts that already use it (principle 7).

## 6. Compose at the level designers think

Above the layout model sits a vocabulary of design-level parts. An
agent reaches for these first.

- `Shell`: the application frame with slots: title or toolbar,
  sidebar, content, inspector, status bar, and a dock for panels.
  Region spacing and dividers are the shell's job. Layouts persist
  as data.
- `Panel`, `Section`, `Field`, `Form`: labels align, help text and
  validation have a place, required marks and errors use roles.
- `Toolbar`, `ToolButton`, `SegmentedControl`, `Menu`, `ContextMenu`,
  `CommandPalette`, `Tooltip`, `Popover`, `Dialog`, `Toast`.
- `Table` and `List`, virtualized, with tabular figures, sticky
  header, column resize, selection.
- `Tabs`, `Tree`, `Breadcrumbs`, `Nav` for SaaS pages, `Page` with
  header and actions.
- Inputs: text, number with drag-to-scrub, slider, select, combo,
  checkbox, switch, radio, color (OKLCH), date.

Every part is two layers: a headless behavior (focus, keyboard,
ARIA roles, pointer state) built on Understory's focus, responder,
overlay, and selection crates, and a styled layer that uses only
kernel tokens. A new design system is a new theme file, not new
widgets. This is the split gpui-base and gpui-component, and Bevy's
headless widgets and Feathers, both arrived at independently.

## 7. The canvas kit

The canvas is a retained island (PLAN.md 8.2) with its own kit:

- `Viewport2D`: offset, zoom, optional Y-up, device scale, visible
  rect, `to_screen` and `to_design`, anchored zoom, fit. From
  runebender-core's `ViewPort` and `understory_view2d`.
- Gesture recognizer: click, double click, drag with threshold,
  modifiers, wheel and pinch, with a `ToolDelegate` trait
  (`drag_began`, `drag_changed`, `drag_ended`, `hover`, `cancel`).
  From runebender's `mouse.rs`, over `ui-events`. Written once.
- Tools: a tool is a state machine over the delegate with a cursor,
  an overlay painter, and a keymap scope. Tool switching is an
  action.
- Scene: a retained presentation tree (`understory_presentation`)
  with damage regions, so a point drag repaints the dirty rect, not
  the canvas. Paths batch by style. Vello draws it.
- Hit testing: spatial index (`understory_index`, R-tree or BVH)
  for broad phase, `understory_precise_hit` for narrow phase, with
  zoom-scaled tolerances.
- Transform groups: a subtree of ordinary widgets under an affine,
  with pointer events mapped through it. Masonry has `Transformed`
  and a compose pass; xix makes this a supported, tested path,
  because it is the thing GPUI cannot do (Zed issue #53303) and the
  thing ComfyUI needed.
- Snapping and guides (`understory_guides`), selection model
  (`understory_selection`), marquee, handles, rulers, grid.
- HUD overlays (measurements, labels with collision avoidance) drawn
  with kernel text styles so they match the rest of the app.

## 8. The two proving applications

### 8.1 Font editor (Runebender)

Shell: tool toolbar, sidebar with a virtualized glyph grid and
categories, content with the editor canvas, inspector with selection,
metrics, anchors, and transformations as forms, status bar.
`runebender-core` supplies every edit. The canvas kit supplies
viewport, gestures, hit testing, scene, and tools. The app supplies
the tool logic that calls core, the inspector forms, and the theme.

Most of runebender-gpui's 13.8k lines is gesture, hit, cache, and
theme plumbing. On xix that is framework. The app should be under
5k lines and reach `PARITY.md`. That number is the acceptance test.

### 8.2 Node-based editor (ComfyUI class)

ComfyUI's 2025 and 2026 lesson: canvas-drawn node widgets became "a
development bottleneck" where "even small UI changes could take days",
so Nodes 2.0 moved node bodies to Vue components over the canvas,
and now runs two rendering systems with the zoom, hit testing, and
z-order seams between them.

xix avoids the seam. The graph is a transform group inside the
canvas. Node bodies are ordinary widget subtrees: forms, sliders,
combos, image previews, laid out by the same stack and inset model,
so nodes have correct spacing for free. Ports, wires, selection, and
groups live in the retained scene under and over the nodes.

- Document: `understory_node_graph` (nodes, ports, edges, groups,
  projection, session, computed state). Serialize to JSON; offer a
  ComfyUI workflow import so existing graphs load.
- Wires: cubic beziers from kurbo, batched into one path per state
  (idle, selected, executing, error), hit tested with tolerance.
- Level of detail: below a zoom threshold, nodes mount a card
  instead of a widget subtree. Only nodes intersecting the viewport
  mount at all. A thousand-node graph stays interactive.
- Execution state arrives as data; ticks repaint only the nodes and
  wires whose state changed.
- Search-to-add is the command palette. The minimap is a second
  `Viewport2D` on the same scene. Reroutes, groups, and notes are
  node kinds.
- Node definitions are data (inputs, outputs, widgets, defaults), so
  an agent or a server can add node types without UI code.

## 9. The application layer

Shared by every app and by the SaaS tools:

- Actions as plain structs; keymaps as data, with scopes (global,
  region, tool); one action list feeds keymap, menu bar, context
  menus, and the command palette. Dispatch walks the focus chain
  (`understory_responder`).
- Menus through Masonry layers plus `understory_overlay`; native
  menu bar through `muda` where the platform has one. One `Menu`
  model renders both.
- Undo as a typed snapshot or command stack with coalescing by edit
  kind, from runebender-core's `UndoState`.
- Background work as request and response keyed by identity,
  canceled on teardown; no framework executor, the host brings one
  (principle 7).
- Windows opened and closed imperatively.
- Persistence for layouts, themes, keymaps, and recent files, as
  files the user can edit.

## 10. The agent loop

This is a product surface, not a test utility.

- `xix render`: headless render of any view or the whole app to PNG
  through Vello CPU, in under a second, with no window. On a machine
  where the user is working, this is the only way an agent should
  look at its work.
- `xix dump`: the widget tree as text, with resolved tokens, sizes,
  roles, and the accessibility tree. An agent reads this instead of
  squinting at pixels.
- `xix lint`: raw values, proximity violations, contrast failures,
  undersized hit targets, duplicate primaries, text styles used out
  of hierarchy, orphan margins (impossible by API, checked anyway
  for custom widgets).
- Snapshot tests: golden PNGs and golden dumps, per widget and per
  screen, run in CI. Masonry's `TestHarness` already does most of
  this.
- `LLMS.md`: one file that holds the entire vocabulary: tokens,
  layout, parts, shell, canvas, actions, and the ten rules of good
  layout the kernel enforces. It fits in one context window.
- Recipes: inspector panel, settings page, data table page, login,
  node body, tool palette. Short, exact, and tested.
- An in-app inspector overlay (like browser devtools) that shows the
  tree, resolved tokens, and lets the designer nudge a token live and
  write it back to the theme file. Nudge in the app, not in the code.

## 11. Rust-native mechanics

- Tokens are enums with `const` resolution. Resolution happens once
  per layout through Masonry's property cascade with inheritance
  (density, region, group depth are inherited properties).
- Static structure is static: tuples for fixed children with
  compile-time counts; `Vec<Any>` only for dynamic lists. Views carry
  no `State` parameter (PLAN.md 8.2, option B); handlers produce
  typed messages; islands own their state.
- Themes are plain structs, loaded from TOML, validated on load,
  with a `const` default compiled in. No thread-locals; the theme is
  a property on the root.
- Change detection by ticks on properties and widget state, so
  "nothing is rebuilt that did not change" is a property of the
  engine, not of the app author's discipline.
- The host owns the loop. `xix` offers a winit driver, a bare
  `step(event) -> Vec<Signal>` API for embedding, and later a web
  driver. Nothing blocks.
- Stable Rust. Every dependency public and pinned.

## 12. Crate layout

Fewer crates, more specialization (PLAN.md 8.2). Proposal:

- `xix_core`: the Masonry fork (engine, widgets base, winit driver,
  testing, imaging). Stays mergeable with upstream.
- `xix`: design kernel, layout model, parts, shell, application
  layer, reactive layer, prelude. Modules, not crates, until a
  boundary earns itself.
- `xix_canvas`: viewport, gestures, tools, scene, hit testing,
  transform groups, and the node graph kit as a feature.
- `xix_tools`: the `xix` CLI, headless render, dump, lint,
  snapshots.

## 13. Acceptance tests

"Best framework for this" must be measurable.

1. Zero-tuning test. An agent with only `LLMS.md` and a one-paragraph
   brief builds an inspector panel, a settings page, and a data
   table page. A designer reviews. Count corrections to spacing,
   type, or color. Target: zero. Run it every release.
2. Theme swap. Change the theme file; every widget and both proving
   apps restyle with no code change. Contrast passes in all themes.
3. Runebender on xix under 5k lines at `PARITY.md`.
4. A 1,000-node graph at 60 fps during pan, zoom, and a slider drag
   inside a node.
5. A point drag in the font editor repaints only its damage rect and
   rebuilds no views outside the canvas.
6. Headless render of the full app under one second; dump and lint
   under 100 ms.
7. Clean build of the view layer of either app under a budget we set
   after the prototype; incremental rebuild after a one-line view
   change under five seconds.

## 14. Phases

0. Decisions and prototypes (PLAN.md sections 8 and 9), plus two
   more spikes: the token cascade on Masonry properties, and a
   headless PNG render of a stack of three text styles with correct
   rhythm and cap-height alignment.
1. Engine fork building on stable. Design kernel. Layout model.
   Twelve parts. Shell. Headless loop and `LLMS.md`. Acceptance test
   1 with a settings page.
2. Application layer. Canvas kit. Runebender port to `PARITY.md`.
3. Node graph kit. A ComfyUI-class editor demo with workflow import.
   Web driver.
4. SaaS parts (pages, nav, tables, auth screens). Inspector overlay.
   Figma variable and auto layout mapping.

## 15. What we are deliberately not doing

- Not a DSL, not markup, not macros for views.
- Not CSS. No arbitrary numbers in the styled layer.
- Not signal-driven. Ticks and explicit keys.
- Not a framework that owns your loop or your threads.
- Not a general GUI toolkit for every app. Tools and products with
  dense, designed interfaces. If it is also good for a calculator,
  fine, but that is not the test.

## Sources

- PLAN.md sections 4 through 12 and their sources.
- Understory crates: https://github.com/forest-rs/understory
- ComfyUI Nodes 2.0: https://blog.comfy.org/p/comfyui-node-2-0,
  https://docs.comfy.org/interface/nodes-2
- Makepad 2.0 and Splash: https://makepad.nl/
- Radix Themes spacing and scaling:
  https://www.radix-ui.com/themes/docs/theme/spacing
- Spacing heuristics (8 pt grid, internal versus external):
  https://cieden.com/book/sub-atomic/spacing/spacing-best-practices
- Better Design (design context for agents, as the outside-in
  approach): https://github.com/marvkr/better-design
- Zed issue #53303 (transform groups):
  https://github.com/zed-industries/zed/issues/53303
