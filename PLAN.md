# xix planning document

xix is a fork of [Xilem](https://github.com/linebender/xilem) for
professional graphics software: font editors, node-based editors,
video editors, trading and chart GUIs. Applications where quality and
customizability matter more than time to first demo.

This is a working document. It compiles what we know, records the
decisions we make, and lists what is still open. The architecture
proposal built on this evidence is [DESIGN.md](DESIGN.md). Sections marked
OPEN are not decided. Edit it as the plan changes.

Last updated: 2026-08-21.

## 1. Principles

These come first because every later choice is tested against them.

1. **Maximally adapted to Rust.** The promise of Xilem was an
   architecture that fits the language: idiomatic, expressive, zero
   cost abstractions, and no surrender of the safety guarantees.
   xix keeps that promise. We do not trade static checks for
   runtime checks to gain convenience unless the trade is local,
   explicit, and measured. No `unsafe` in the framework outside
   `tree_arena`. No API whose normal failure mode is a panic on a
   reentrant borrow.
2. **Built for real applications.** The fork exists because three
   Runebender ports taught us what a graphics app needs. Every
   feature in xix must be pulled by an application in section 3,
   not pushed by a pitch. ("We are building THIS application with
   THESE needs", as one Lobsters commenter put it.)
3. **Canvas first.** The center of every target app is a large,
   app-owned, GPU-drawn surface with its own coordinate system,
   tools, gestures, and 60 fps drags. Widgets surround it. The
   framework must be designed from that surface outward, not from a
   counter example outward.
4. **Keep the Linebender stack.** Vello, Parley, kurbo, peniko,
   color, ui-events, and Understory stay as dependencies. The fork
   covers the framework layers only (Masonry and the reactive
   layer). Geometry types are shared with the application's engine
   (runebender-core already speaks kurbo). Rendering goes through
   the `imaging` abstraction so the framework stays renderer
   agnostic (Vello GPU, Vello Hybrid, Vello CPU, or another
   backend), and the project is run so that outside contributors
   can land changes, not as one application's internal toolkit.
5. **Customizable by the user of the app.** Themes, keymaps, menus,
   and panel layouts are data, loadable at runtime, not constants
   compiled into the binary.
6. **Builds on another machine.** Public, pinned, reproducible.
   Every dependency from crates.io or a public git rev with a
   committed lockfile. Stable Rust for the native build.
7. **A library, not a framework.** The host owns the event loop and
   the threads. No blocking `run()` as the only entry point; a
   caller must be able to drive the framework from its own loop
   (winit, baseview in an audio plugin, a browser frame callback).
   No thread-local state anywhere in the framework. Layout is a
   replaceable module, not a hard dependency.

## 2. Why a fork, in one paragraph

Xilem's own maintainers say its current architecture "isn't remotely
close to working" for non-trivial apps and needs "large, sweeping
changes, even if that means throwing out a lot of code" (section 5).
Masonry underneath it is sound, well tested, and close to what we
need. GPUI shows what an application layer for this class of app
looks like, but GPUI is a Zed internal, tracks Zed main, is not
built on the Linebender stack, and has its own rendering limits for
path-heavy scenes (section 6). Nobody is building the combination we
need: Masonry plus Vello plus a real application layer plus a
reactive layer that stays idiomatic Rust. That combination is xix.

## 3. Target applications and what they demand

| Need | Font editor | Node editor | Video editor | Trading / charts |
|---|---|---|---|---|
| Pan/zoom canvas with app-owned coordinates | yes | yes | timeline | yes |
| Thousands of paths per frame, 60 fps | outlines | wires, nodes | waveforms | candles, lines |
| Widgets *inside* a transformed canvas | no | yes (node bodies) | clips | annotations |
| Gesture recognizer (click vs drag, thresholds, modifiers) | yes | yes | yes | yes |
| Tool state machines | 8 tools | few | many | few |
| Window-level shortcuts, menus, context menus | yes | yes | yes | yes |
| Docking, tabs, resizable panels, saved layouts | yes | yes | yes | yes |
| Virtualized lists and tables (100k rows) | glyph grid | no | media bin | yes |
| Text inputs for numbers, dense forms | yes | yes | yes | yes |
| High quality text (shaping, bidi, fallback) | yes | labels | titles | labels |
| Runtime theming, user themes | yes | yes | yes | yes |
| Undo/redo with coalescing | yes | yes | yes | no |
| Background work (file watch, shaping, decode, sockets) | yes | yes | heavy | heavy, streaming |
| Multi-window | yes | yes | yes | yes |
| Browser build | yes | nice | no | nice |
| Images and video frames on the GPU | trace, preview | thumbnails | core | no |

The first application is Runebender. `runebender-core` (16.6k lines)
is UI-agnostic and already consumed by two shells, so it is the ideal
test: the engine exists, the feature list exists (`PARITY.md` in
runebender-gpui), and a third shell on xix is directly comparable.

## 4. Evidence from the Runebender ports

Three ports of the same editor, two on the frameworks under study.

### 4.1 runebender-xilem (Xilem 0.4 at git rev `7819435e`, 2026-08-07)

About 28k lines. The split tells the story: `src/components/` (custom
Masonry widgets) is 13,257 lines, `src/views/` (Xilem view
composition) is 2,316 lines. There are 17 custom `impl Widget` and
14 custom `View` impls. Stock Xilem views are used for layout
scaffolding and text inputs. Every toolbar, panel, grid cell, scroll
container, and the editor canvas is hand-written below the view
layer.

What was fought, with the repo's own words:

- **Shortcuts and focus.** "There's no straightforward way to
  implement global keyboard shortcuts like Cmd+S." The zstack
  workaround fails at the bottom (shortcuts only over empty
  background) and at the top ("intercepts all pointer events first,
  breaking interaction with widgets below, even when not calling
  `set_handled()`"). The editor widget now grabs focus on every
  pointer event, and the toolbar swallows every click so the editor
  does not see it. (`context/keyboard-shortcuts-zstack.md`,
  `editor_canvas/mod.rs:402`, `edit_mode_toolbar.rs:193`)
- **Rebuild cost on drags.** "During drags we emit `SessionUpdate`
  on each mouse move. That forces a full Xilem view rebuild and tanks
  performance." Fix: `DRAG_UPDATE_THROTTLE = 3`, so the rest of the
  UI sees every third pointer move. (`settings.rs:78-87`)
- **The canvas owns its state and cannot be driven by the view.**
  "During an active pointer gesture the widget owns live editing
  state. Overwriting the session here would reset the tool to Ready
  and revert path edits, breaking the drag. Skip the overwrite."
  Undo lives in the widget, not in `AppState`. (`view.rs:70-76`)
- **Scrolling.** `portal()` was replaced by a custom
  `GridScrollWidget` because "every rebuild would construct views
  for hundreds of off-screen glyphs" and Portal "doesn't accept
  keyboard focus and has no keyboard event handling".
  (`grid_scroll_handler.rs:7-30`)
- **Layout differs headless versus live.** "The app renders BROKEN
  in the real window but renders PIXEL-PERFECT headless." A probe
  showed the canvas laid out at 10000 x 8784 because the live
  measure path "takes these huge preferred sizes at face value".
  `.width()` and `.height()` each reset the other axis.
  (`.agents/UI-PARITY.md`, `lib.rs:168-177`)
- **Multi-window messaging.** `MessageResult::RequestRebuild` only
  rebuilds the current window, so every custom view returns
  `Action(())` to force a full app rebuild. (`LLMS.md`)
- **Missing platform features.** No menus, no context menus, no
  drag-and-drop (`masonry_winit` discards `DroppedFile`), no wasm
  target for `masonry_winit` at all. The browser editor had to be a
  separate Vue plus wasm rewrite.
- **Upgrade cost.** Moving between two Masonry revs produced 306
  compile errors, all in the UI layer, zero in the model. Every
  custom widget needed a new `measure` method; `masonry::vello` was
  replaced by `imaging::Painter`; `Length`/`.px()` changes alone
  caused about 99 errors. (`.agents/MASONRY-UPGRADE.md`)
- **Version splits.** `spline` on kurbo 0.9 versus the app on kurbo
  0.12, solved with an aliased dependency and conversion at every
  call. (`context/kurbo-version-mismatch.md`)
- **Small widget gaps.** `text_input` placeholder ignores
  `text_alignment()`.
- **Theming.** `theme.rs` is 602 lines of compile-time constants.
  No runtime switching. Masonry's property system is untouched.

No upstream code is vendored or patched. Everything is functional
replacement below the view layer.

### 4.2 runebender-gpui (gpui 0.2.x from Zed git, plus gpui-component)

14,591 lines, of which `main.rs` is 13,809: one `Entity`, one
`Render` impl, no component tree. Near feature parity with the web
editor, which the Xilem port never reached.

What GPUI gave:

- `cx.listener(|this, ev, _, cx| ...)` hands event handlers
  `&mut self`. An event handler is a method call. No message enum,
  no lens, no reducer. This is the single biggest reason the port
  went fast.
- `actions!` plus `bind_keys` plus `on_action`: 55 actions in one
  list feed both the keymap and the native menu bar. Menus and
  shortcuts were near-free.
- `div()` with Tailwind-style flexbox maps 1:1 from the web editor's
  CSS.
- `canvas(measure, paint)` as the escape hatch, and `paint_path`
  taking a path built from `kurbo::BezPath` in about 20 lines.
- `FocusHandle`, `cx.spawn`, `background_executor().timer`,
  `cx.subscribe_in`: all straightforward.

What GPUI cost:

- **Rendering architecture for path-heavy scenes.** "gpui's renderer
  ends its render pass at every run of paths, rasterizes them to an
  intermediate texture, and starts a new pass. This is the shape of
  the problem behind the 3-6 fps report." Fixed by batching one path
  per color. Then "gpui indexes vertices with a u16": a screen of
  outlines exceeds 65,535 vertices and `PathBuilder::build` fails
  silently, so batches are flushed every 12 subpaths and halved on
  failure. (commits `e22cfd5`, `41477b5`, `8bfa913`)
- **No incremental anything.** Every pointer move re-runs the whole
  13k-line render and repaints the canvas. It reaches 60 fps only
  after caching sorted glyph orders, ink boxes, and `Arc<BezPath>`
  outlines per glyph.
- **No layout read-back.** Bounds escape the paint closure through
  `Arc<Mutex<Bounds>>`; the grid uses a probe canvas that "lags the
  layout by a frame".
- **No transforms for element trees.** Zed issue #53303 (2026-04-07,
  open, no maintainer reply) asks for transform groups so that real
  elements can live inside a pan/zoom canvas: "Standard GPUI elements
  like div, text, h_stack, v_stack cannot be used inside a canvas
  paint callback." Node editors on GPUI draw everything with
  primitives.
- **Missing drawing features.** No blur for paths (a CPU rasterizer
  module was written), no stroked text (painted eight times), masks
  are rectangles only, no scroll snapping, text line boxes too tall.
- **gpui-component barely used.** Of 60+ components the app uses
  `Input`, `Slider` (rebuilt flat from `gpui-base`), `resizable`,
  `AppMenuBar`, `Root`, `Theme`. Dock, tabs, lists, popovers,
  tooltips, tables, context menus: all hand-rolled from `div()`.
  `Root`'s tab traversal and the focus ring had to be disabled.
- **Theming bridge is lossy.** Colors round-trip through `#rrggbb`
  (alpha dropped); widgets branch on light/dark mode, not only on
  tokens.
- **Dependency surface.** 924 crates in `Cargo.lock`. 23 crates from
  zed-industries/zed at one pinned rev, 4 from gpui-component, plus
  Zed forks of `font-kit`, `reqwest`, `scap` (screen capture, for a
  font editor). Release binary 25.8 MB, wasm 25.4 MB, `target/` 16 GB.
- **Toolchains.** Native needs stable (`pathfinder_simd` breaks on
  nightly); the wasm build needs nightly (`-Z build-std` for
  atomics). Two mutually exclusive toolchains for one repo.
- **Browser.** Text inputs cannot take focus, in-window menus do not
  activate, all action dispatch panics (the profiler feature calls
  `Instant::now()`), clipboard read is a stub.

### 4.3 runebender-core: what an engine asks of a framework

Core is 16.6k lines, UI-free: norad types, kurbo types, free
functions over glyphs, a text engine with HarfRust shaping, an
`UndoState<T>` snapshot stack, a `ViewPort { offset, zoom }` with
Y-flip. It defines no pointer, gesture, or keyboard types. It
produces no display list, only `BezPath`s with semantic tags
(`ColoredStroke { path, popcount, wide }`) and theme tokens in OKLCH
JSON.

So the gesture state machine (click versus drag, 3 px threshold,
`MouseDelegate` with `left_drag_began/changed/ended`), hit testing
with zoom-scaled tolerances, tool dispatch, and per-path render
caching are written again in every shell. That is why the GPUI shell
is one 13.8k-line file. These belong in the framework's application
layer, not in core and not in each app.

## 5. Upstream Xilem in August 2026

Workspace at rev `7819435e`: `xilem_core`, `xilem_masonry`, `xilem`,
`xilem_web`, `masonry_core`, `masonry`, `masonry_winit`,
`masonry_testing`, `masonry_imaging`, `tree_arena`, `placehero`.
Version 0.4.0, `rust-version = "1.92"`, Apache-2.0. Deps: vello 0.8,
kurbo 0.13.1, parley 0.8, peniko 0.6.1, wgpu 28, winit 0.30.13,
ui-events 0.3, understory_virtual_list 0.1.2.

Masonry now has: a new layout system (measure/layout), the `imaging`
abstraction so it can render through Vello Classic, Vello Hybrid, or
Vello CPU, a typed `Property` system with a property stack and cache,
a basic layer system (`LayerType`, `Layer` trait, `SelectorMenu` and
`Tooltip` layers), a `Canvas` widget that records an `imaging` scene,
a `Transformed` view (affine on a subtree), `Passthrough`,
`VirtualScroll` moving onto `understory_virtual_list`, drag-window
signals, IME, and a serious `TestHarness`. 38 widgets. Still missing:
menu bar, context menu, popover, modal, tabs, tree, table, docking,
window-level shortcut registry, drag-and-drop, wasm driver.

Tracking issue #1343 (layers and menus, 2025-08-15) has no items
checked. The Q1 2026 Linebender post says two core team members
moved to full-time jobs at Canva. There is no Q2 post. Last upstream
commit 2026-08-07. Organizational risk is real.

### The maintainer's own review (2026-08)

"A critical review of Xilem in 2026" (HackMD, by a core maintainer;
discussed on Lobsters and r/rust) says, in its own words:

- "The current architecture isn't remotely close to working. Making
  non-trivial apps with Xilem is painful, people aren't using it,
  other frameworks are much more convenient."
- "Xilem still hasn't found a scalable way to compose components and
  manage complex state."
- "Premature optimization: the Xilem architecture has been driven by
  efforts to maximize performance. Given that we still don't
  benchmark Xilem, these efforts are mostly based on faith."
- "About 30 traits (187 if we include `xilem_web`), the `View` trait
  has about 70 implementations."
- "Two-way bindings don't work in Rust. At least half its problems
  come from baking two-way bindings into its API." Every view carries
  a `State` generic it rarely needs.
- Higher-order components (`Provides`, `WithContext`, `Fork`, `Lens`,
  `MapMessage`, `MapState`, `Memoize`, `Frozen`, `RunOnce`) have
  signatures that are "not an acceptable public API".
- "Views shouldn't have side-effects" (`fork`, `task`, `worker`,
  `run_once`, and the `WindowView` model).
- Proposed fix: a non-generic, type-erased `Component` with local
  state through `CompCtx`, a "metadata token" to keep callbacks
  typed while erasing the view types, side effects only in
  callbacks, a request/response model for background work, windows
  opened imperatively, and aggressive scope cuts (remove tokio,
  `Environment`, `OneOfCtx`, virtual scroll from the MVP, variable
  labels, most examples).
- "If Xilem is to survive, it will need large, sweeping changes, even
  if that means throwing out a lot of code and starting over."

From the Linebender Zulip thread around the review (August 2026,
author Olivier Faure): reverse-dependency search "turned up very
little"; the only high-profile Xilem user is tinymist (a Typst
renderer); Configura used Masonry for an internal renderer at some
point. "At the last RustWeek, we basically agreed to put it on the
backburner for the foreseeable future." The architecture problems
are "fixable", and he hopes for "a widely advertised Xilem release
before the end of the year" if he can work on it semi-regularly.
Planned: a Xilem testing framework wrapping `masonry_testing::
TestHarness`, and keeping Placehero (the Mastodon "hero app") in the
repo as continuous dogfooding with fake local data behind a feature
flag. His read on low adoption: "a lack of some very specific
features people want (e.g. menus) and a lack of advertising of the
features we do have."

Two things follow. First, the fork is not a rebellion; the upstream
diagnosis matches ours. Second, the proposed cure moves type checks
to runtime ("some type checking will move from compile time to
runtime; I believe this is the right trade-off"). That is the point
where xix must decide whether to follow (section 8).

Lobsters reaction (no Linebender maintainers replied): "brutal" to
build anything moderately complex; praise for Qt signals/slots and
Slint's separation of UI declaration from logic; "build THIS
application with THESE needs". The r/rust thread could not be fetched
(Reddit blocks automated access); paste any comments that matter.

## 6. The GPUI ecosystem, surveyed

- **GPUI** (gpui 0.2.2 on crates.io, 2025-10; live development on
  Zed main). "A hybrid immediate and retained mode, GPU accelerated
  UI framework." Three registers: `Entity<T>` state on one foreground
  thread with runtime-checked access through `App`/`Context`;
  `Render` views returning element trees laid out by Taffy with a
  Tailwind-style `div`; the `Element` trait (`request_layout`,
  `prepaint` with hitboxes, `paint`) for custom surfaces. Actions are
  user structs bound through keymaps and dispatched through a focus
  tree. Windows, menus, async executors, and a test context are
  built in. Web platform merged 2026-02-26 (WebGPU, needs wasm
  atomics and shared memory).
- **gpui-component** (longbridge, 13k stars): 60+ components,
  semantic theme tokens, dock layout with draggable tabs, nested
  splits, edge docks and serializable tiles, virtual table for
  hundreds of thousands of rows, charts, a 200k-line code editor.
  Split into `gpui-base` (unstyled behavior) and `gpui-component`
  (styled). Ships Longbridge Pro, a commercial trading terminal.
- **awesome-gpui** lists roughly 100 apps and libraries. Relevant to
  us: ferrum-flow and gpui-flow (node editors), opencut (video
  editor), Futureboard Studio (DAW), Monocurl (animation), gpui-d3rs,
  gpui-px and plotters-gpui (charts), gpui-whiteboard (pan/zoom
  canvas), Base GPUI (headless component port), gpui-tea (Elm-style
  runtime), Glass-HQ/gpui (a fork adding iOS and web), React Native
  GPUI. The category we target is exactly where GPUI is growing.
- **Why not just use GPUI.** Section 4.2, plus: it is not on the
  Linebender stack (no Vello, no Parley, no kurbo in the API), it
  tracks one company's editor, forks are already appearing to cover
  platforms Zed does not care about, and its entity model is a
  runtime-borrow-checked object graph, which is the opposite of
  principle 1.
- **What to take from GPUI.** Actions as plain structs bound by data
  (keymap JSON), one action list driving keymap and menus, a focus
  tree that dispatches actions upward, handlers that receive
  `&mut self`, the three-phase element protocol as the custom
  drawing contract, a component library with a headless base and a
  styled layer, and a dock system that serializes.

### 6.1 Makepad, the closest project in spirit

Makepad 2.0 (MIT, 6.5k stars, macOS/Windows/Linux/web/iOS/Android)
calls itself "an AI-accelerated application development environment
for Rust". Its answer to agent-generated UI is Splash, "a live
readable UI DSL" so "models can generate UI in small, reviewable
steps". The August 2026 `counter` example puts application state in
a script VM (`script_mod! { let state = { counter: 0 } }`) and the
Rust side mutates it with `script_eval!`. Layout values in the DSL
are free numbers (`spacing: 12`). Rendering is Makepad's own shader
language and renderer, not Vello or Parley. Makepad Studio offers
live editing without recompiling and Figma-like visual tooling.
Apps: Robrix (Matrix client), Moly.

Further facts (2026-08-22). Text: Makepad vendors `rustybuzz` and
`unicode-bidi`, so complex-script shaping exists. Accessibility: one
reference to AccessKit in the whole repo; a 2025 survey found
"Narrator cannot detect window chrome or content". Docs: the same
survey found "no documentation I can find" for the DSL, and crates.io
releases with no matching git tags. Team: the top two contributors
on `dev` are from Robius (Robrix) and one other account; the founder
commits as "Admin". The framework went from `live_design!` (1.0, May
2025) to the Splash script VM (2.0, 2026) in about a year.

Splash's own agent guide (`splash.md` in the repo) is the strongest
evidence for the xix thesis. It is a list of runtime gotchas:
"Every container MUST have `height: Fit`, most common failure mode";
"`as int` type casting produces NaN"; "Default text color is white";
"`new_batch: true` required or text renders behind the background
(invisible)"; "TabBar renders zero-size"; widgets that "silently
fail"; checkbox state "lost, internal post-processing discards the
`on_click` scope"; `#x` prefix needed for hex colors containing `e`.
Every one of these is a value or state the type system could have
rejected. The design knowledge lives in a markdown file the agent
must remember, not in the API.

What Makepad has that xix only plans: a working agent loop today.
`AGENTS.md` documents a Studio remote protocol over JSON lines with
`WidgetTreeDump`, `WidgetQuery`, `Screenshot`, `Click`, `TypeText`,
and per-build ids. That is the spec xix's `xix render`, `xix dump`,
and interaction tooling should meet or exceed.

What it gets right: a tight loop, a designer-facing studio, one
vocabulary for screens, widgets, layout, and state. What it gives
up, against our principles: the compiler no longer checks the UI
(principle 1), the stack is not renderer-agnostic (principle 4), and
there is a second language with its own runtime to keep consistent.
xix keeps live editing for data (themes, keymaps, layouts) and puts
the UI vocabulary in Rust types so diagnostics are the feedback loop.
See DESIGN.md section 2 and the blog post draft on elih.net.

## 7. Linebender pieces we can use instead of writing

- **Understory** (forest-rs, Apache-2.0/MIT, `no_std` + alloc,
  kurbo-native): "foundational spatial and scene data structures for
  user interfaces and graphics editors." Crates for overlays and
  popups (`understory_overlay`, `understory_anchor`), focus
  navigation, a responder chain event router, selection, typed
  properties with cascade and transitions, a retained presentation
  tree, virtual lists, outlines (tree views), a 2D viewport
  (`understory_view2d`), spatial indexes (R-tree, BVH) with precise
  hit testing, and `understory_node_graph` for node editors. Masonry
  already depends on `understory_virtual_list`. This is the missing
  application-layer substrate, headless, waiting for a host.
  How xix would use it (dependency, not fork; checked 2026-08-22):

  | Understory crate | xix subsystem | Notes |
  |---|---|---|
  | `overlay`, `anchor` | menus, context menus, popovers, tooltips, dialogs | Published 0.1.0 (2026-08-08). Host in Masonry layers. |
  | `focus`, `responder`, `event_state` | focus chain, action dispatch, hover/click/drag machines | Replaces the per-app gesture layer (`mouse.rs`). |
  | `index`, `box_tree`, `precise_hit` | canvas hit testing, visibility, transform groups | Box tree holds world AABBs under affines. |
  | `view2d` | `Viewport2D` | Published 0.1.0. Compare with runebender-core `ViewPort`. |
  | `selection` | canvas and list selection model | |
  | `virtual_list`, `outline` | tables, lists, tree views | Masonry already uses `virtual_list`. |
  | `node_graph` | node editor kit | Git only. |
  | `property`, `style`, `presentation`, `box_decoration`, `transition` | design kernel cascade, retained presentation | Overlaps Masonry's `Property` system; pick one or bridge. |
  | `timing`, `animation`, `motion` | motion tokens, timers | `timing` published 0.1.2. |

  Only five crates are on crates.io (`anchor`, `overlay`, `timing`,
  `view2d`, `virtual_list`); the rest are git dependencies at a
  pinned rev, which principle 6 allows with a lockfile. Risks: nine
  months old, two main authors, fine-grained crate boundaries that
  may merge or split, and the property/style overlap with Masonry.
  Plan: depend on the published crates first, vendor nothing, and
  send fixes upstream; the prototype in section 9 decides the
  cascade question.

- **Vello** 0.8, Vello Hybrid (beta), Vello CPU, through the
  `imaging` abstraction. Compute-based rasterization handles
  path-heavy scenes without the per-path render-pass breaks GPUI has.
  `AnyRender` (ergonomic) versus `imaging` (performance, complete).
- **Parley** 0.8 with system font enumeration, fallback, AccessKit
  text. Adopted by Bevy. **Glifo** for glyph rendering and atlases.
- **ui-events** for W3C-style input, already used by Masonry for IME.
- **muda** (Tauri) for native menu bars and context menus on winit,
  if we want native rather than in-window menus.
- **Subduction** for native compositor and AppKit widget embedding,
  if a native menu bar or native text field is ever required.

## 8. OPEN: the thesis

The evidence points at a layered answer. This is a proposal for
discussion, not a decision.

### 8.1 What to keep, fork, and build

| Layer | Source | xix stance |
|---|---|---|
| Geometry, color, text, render (kurbo, peniko, color, parley, vello, imaging) | Linebender | dependency, track releases |
| Spatial, overlay, focus, properties, viewport, node graph | Understory | dependency, contribute |
| Widget engine (`masonry_core`, `masonry`, `masonry_winit`, `masonry_testing`, `tree_arena`) | Xilem repo | **fork**, stay close, upstream what they accept |
| Reactive layer (`xilem_core`, `xilem_masonry`, `xilem`) | Xilem repo | **fork and simplify**, see 8.2 |
| `xilem_web`, `placehero`, most examples | Xilem repo | drop (the web generalization in `xilem_core` is the main source of accidental complexity, see 8.2) |
| Application layer: actions, keymaps, menus, docking, tabs, command palette, theme tokens, gesture recognizer, viewport and tool framework, canvas with retained scene, virtual table, charts | new | **build**, this is the product |
| Web driver for Masonry (winit on wasm or a direct WebGPU driver) | new | build, runebender-web proves Vello in the browser |

The application layer is where GPUI wins today and where Xilem has
nothing. It is also where the three Runebender shells duplicated the
most code. It does not depend on which reactive model we pick, as
long as the reactive layer can host retained, widget-owned islands.

### 8.2 The reactive layer: three options against principle 1

**A. Upstream's direction: type-erased `Component`.** Non-generic
components, local state from a context, metadata token to keep
callbacks typed, side effects only in callbacks. Solves the
generics explosion and the HOC signatures. Cost: `Box<dyn>` at every
component boundary, string keys for identity, some checks at runtime.
The author says downcast errors are impossible "unless you try really
hard". Against principle 1: partly. Not unsafe, no reentrant-borrow
panics, but no longer zero cost and no longer statically checked end
to end. Advantage: if upstream goes this way, xix stays mergeable.

**B. One-way data flow with typed messages.** Views borrow state
immutably to build (`&State`), callbacks return a typed `Action`,
the app owns state and applies actions. Removes the `State` generic
from views, which the review says causes half the problems. Fully
static, zero cost, idiomatic ownership. Cost: Iced-style message
boilerplate, and the "handler gets `&mut self`" ergonomics that made
the GPUI port fast are gone unless components own local state.

**C. Keep Xilem's `View<State, Action>` and fix around it.** Adopt
`Edit<T>`/`Read<T>` (non-contiguous state, merged 2025-11), add the
application layer, and accept the compile-time and error-message
cost. Lowest effort, highest merge compatibility, and the one
option the maintainer says is a dead end.

Whichever we choose, two rules hold for graphics apps:

- **Retained islands.** The canvas, the timeline, the chart, the node
  graph are retained widgets that own their session state, run their
  own gesture machine, and emit typed messages outward. The reactive
  layer sets them up and receives results; it is never in the
  per-pointer-move path. Both Runebender ports converged on this
  independently; xix makes it the documented pattern with a typed
  message channel instead of the `Arc::ptr_eq` and throttle hacks.
- **Nothing is rebuilt that did not change.** Whatever the view model,
  a drag in the canvas must not rebuild the sidebar. Measure it
  (section 9).

**D. Fine-grained signals (Floem).** Floem (lapce/floem, MIT,
used by the Lapce editor) builds the view tree once and updates it
through `RwSignal`s and effects in the style of `leptos_reactive`.
Only the views that read a changed signal update; nothing is
re-generated per frame. It uses Taffy for layout and renders through
Vello, vger, Skia, or tiny-skia, so it already proves a
renderer-agnostic Linebender-adjacent stack is possible. Against
principle 1: signals are runtime dependency tracking with interior
mutability (`Rc`-like handles, effects scheduled by a runtime), so
they are not zero cost and not statically checked, but they do not
panic on reentrant borrows the way GPUI's entity model can. Against
principle 3: signals fit the "nothing is rebuilt that did not
change" rule naturally, which neither Xilem's diffing nor GPUI's
re-render does. Worth taking: the stable view tree, fine-grained
invalidation, and the ergonomics of reading state where it is used.
Open question: whether the same invalidation precision can be had
with static types, for example by deriving dependency sets at compile
time from what a view reads, instead of a runtime tracker.

Verdict after the comments in section 11: signals are not the
driver. The benchmark case for signals is "very small application
state produces a large DOM tree"; graphics apps are the opposite
(large state, moderate tree), where a signal per datum is the cost,
not the saving. Signal libraries also keep their runtime in
thread-local storage, which breaks principle 7. If xix offers
signals at all, it is as an opt-in tool for async-heavy state, on
top of a static model. Tick-based change detection (section 12) is
the preferred mechanism for "nothing is rebuilt that did not
change".

**The static direction, made concrete.** Several contributors point
the same way: keep the SwiftUI-like mix of immediate and declarative
with no hidden stateful components; treat the complexity as
accidental, caused by generalizing `xilem_core` over both Masonry and
the DOM under Rust's orphan rules (`OneOfCtx`, `OrphanView`); put the
reactive layer in one crate specialized for Masonry; merge
`ViewSequence` into `View` with static element counts
(`ViewSequence::ELEMENTS_COUNT` was already planned); add static
type sugar while keeping trait bounds in check; stay macro-free. The
ambition beyond that: "very detailed composable components in the
style of the diesel query builder which allow skipping entire steps
of render/tree building, maybe even at compile time". In other words,
use the type system to know the shape of the tree before runtime, and
skip work because of it. That is the zero-cost reading of principle
1, and it is the opposite of type erasure.

Recommendation to discuss (revised): a static view layer in one
crate, specialized for Masonry, with `ViewSequence` folded into
`View`, compile-time tree shape where the structure is static, and
option B's one-way flow for app state. Tick-based change detection
for invalidation. Signals, if ever, opt-in. Original text follows.

Recommendation to discuss: B for the framework core, with a small,
explicit, typed escape for local component state (so a text field
can own its buffer without threading it through the app), and the
`Component` ideas from A adopted only where they are zero cost
(closure-based component functions, keys for identity, side effects
in callbacks, imperative window creation). Keep the door open to A if
the compile-time cost of B's generics turns out to be the real
problem rather than the `State` parameter.

### 8.3 Questions to settle before code

- Mergeability with upstream Masonry: hard requirement or nice to
  have? (Recommendation: hard requirement for `masonry_core`, soft
  for the rest.)
- Native menus (muda) or in-window menus (Masonry layers plus
  Understory overlay), or both behind one `Menu` model?
- Browser: winit's wasm backend on Masonry, or a direct WebGPU
  driver like runebender-web? Which apps need it in year one?
- Name and crate layout: `xix_core`, `xix_masonry`, `xix_app`? Or
  keep Masonry's crate names to ease merging and name only the new
  layers?
- Who maintains it after the first push, and on what release
  cadence? (Recommendation: tagged releases in lockstep with Vello
  and Parley, never "pin a git rev" for downstream apps.)

## 9. Measure before deciding

- runebender-xilem: frame time and allocation on a point drag, split
  into view rebuild, Masonry passes, and Vello paint. Same with
  `DRAG_UPDATE_THROTTLE = 1`. This tells us whether the rebuild cost
  is architecture or implementation.
- Compile time of runebender-xilem's view layer alone, and the
  incremental rebuild time after a one-line view change. The review
  claims generics "can blow up compile times"; get numbers.
- Whether Masonry's `Transformed` keeps pointer hit testing correct
  under pan/zoom (the node editor requirement GPUI cannot meet).
- Whether the new layer system plus `understory_overlay` can host a
  context menu and a menu bar without a child window.
- Vello versus GPUI on a screen of 800 glyph outlines: frame time,
  and behavior at the vertex counts that broke GPUI.
- How much of upstream Masonry changed between `7819435e` and main
  today, and how long one rebase takes.
- Prototype: resolve one token (`Space::M` under a `Density` and a
  region) through Masonry's `Property` cascade and through
  `understory_style`, and compare API, cost, and how each handles
  inheritance and animation slots. Decides whether the design kernel
  sits on Masonry properties, Understory properties, or a bridge.
- Prototype: a static `View` with `ViewSequence` folded in and
  compile-time element counts, plus tick-based change detection on
  Masonry properties. Compare compile time, error messages, and
  per-drag work against Xilem 0.4 on the Runebender glyph grid.
- Prototype: drive Masonry from a caller-owned loop (baseview or a
  bare winit loop) with no framework-owned thread-locals, to prove
  principle 7 before the API hardens.

## 10. Next steps

1. Discuss and decide section 8. Record decisions here.
2. Run the measurements in section 9 and record results here.
3. Write `docs/problems.md` (public, polished version of sections 4
   and 5) and `docs/design.md` (the chosen architecture, with the
   application-layer API sketched against Runebender's needs).
4. Import the upstream workspace at a pinned rev with history,
   remove what section 8.1 drops, rename, and get it building on
   stable.
5. Port runebender-xilem to xix as the first real test, tracking
   `PARITY.md` from runebender-gpui as the bar.

## 11. Collected comments

Comments from the public discussion of the 2026 review and related
threads, kept here because they should guide the design. Add to this
list as they come in. Paraphrased where the source is not quoted.

- "The promise of Xilem to me was an architecture that wants to be
  maximally adapted to Rust. Idiomatic, expressive, zero cost
  abstractions, doesn't abandon the safety guarantees of the
  language. I'd hope, if it has a future, it doesn't stray too far
  from this vision." (Now principle 1.)
- "Agree, these are the reason why I prefer Xilem's approach over
  others. The closest one right now is probably GPUI but it's heavily
  geared toward Zed and is not very contributors/renderer-agnostic
  friendly so it's also not an option for me." (Two requirements:
  renderer-agnostic through `imaging`, and a contribution process
  that is not one company's editor. See principle 4 and 8.3.)
- "I also really like the signal architecture that Floem uses, so
  that maybe another source of inspiration for the new Xilem
  redesign." (Now option D in 8.2.)
- Shnatsel: "what is Xilem's raison d'être? Why does it exist? And
  does it still have a reason to exist, as opposed to improving Iced
  or Slint instead?" Over 40 Rust GUI libraries and "not a single
  production-ready one"; one more needs "a really good reason". (xix
  must answer this in one paragraph; see DESIGN.md section 2.)
- dman-os: "the biggest gap today is a good, production ready native
  widget set and good platform support", not the reactive layer;
  Linebender should "prioritize having good, cross-platform native
  widget lib over innovation on reactive layer"; would like a Masonry
  backend for Leptos considered; Makepad "appears even more
  maintenance mode than Xilem"; GPUI "the most production ready and
  the one with the most mindshare" but "built around a product with
  minimal abstractions". Signals: "the least magical runtime and
  programming model"; Jetpack Compose the most elegant facade, "but
  they had to build a whole language around it".
- Noah: "a growing trend is to move more towards fine grained
  reactivity + elements checked at runtime (signals/rc refcell).
  Constraining things to compile time in Rust for GUIs feels like a
  losing battle." (The counter-position to principle 1; answered in
  DESIGN.md section 3.)
- Markus Kohlhase: uses `xilem_web` because of explicit state. The
  Leptos trap: `let v = count.get(); view! { <p>{v}</p> }` compiles
  and silently does not update; "`.get()` has a side effect and
  behaves differently depending on where it was called." In xilem_web
  "`html::p(state.clicks)`" and "`.on_click(|state: &mut AppState, _|
  state.clicks += 1)`": "no magic", "with xilem, it's a bit like Rust
  itself: if it compiles, it does what it's supposed to." Prefers
  "explicit state rather than local, implicit, hidden state."
- Muhammad Ragib Hasin: Iced's "full on Elm architecture is not my
  cookie"; Leptos's "macro heavy architecture bounced me right off";
  Xilem's API "however cumbersome, is the least painful"; "maybe we
  could approach a leptos-like DX without the macros."
- Mohamed: passed on Xilem, writing a reactive framework on
  masonry/parley/taffy; "disciplined MVC against a diffable retained
  widget tree can be given an MVU, MVP, MVVM API without much
  trouble."
- voedipus, Austin M. Reppert, Horu: GPUI "the most usable GUI lib
  in Rust right now" and also "slow", "really badly optimized"
  (matches section 4.2).
- Austin M. Reppert: imgui uses string ids the same way as the
  proposed components; virtual lists should be low priority.

- Wojciech Niedźwiedź (three years on a GPU-driven 2D renderer with
  a separate UI; toolkit path react, dioxus, leptos, tachys): signal
  benchmarks assume "very small application state produces large DOM
  tree"; real applications are the opposite. "Having signals as an
  additional optimization tool might be fine, going signal-first is
  a dead end, xilem will never be able to achieve good performance
  for real-time apps." The biggest win: "very detailed composable
  components in style of diesel query builder which allow to skip
  entire steps of render/tree building, maybe even at compile time."
  (Now in 8.2, the static direction.)
- A `xilem_web` user: uses it "precisely because it doesn't use any
  signals"; implemented real projects with Leptos and is "extremely
  dissatisfied"; "reliability is more important than performance."
- Philipp Mildenberger (Xilem core, returning after a hiatus): wants Xilem
  to stay "this mix of immediate and declarative GUI inspired by
  SwiftUI, i.e. not hidden stateful components a la React." The
  complexity is accidental, from generalizing through Xilem Core
  across web and Masonry under Rust's orphan rules (`OneOfCtx`,
  `OrphanView`). "I'm not really fond of removing a lot of the
  statics though, making it more dynamic and thus more runtime-error-
  prone." Strict types "help put the LLMs feet down via the
  compiler." App state in async-heavy apps needs redesign; signals
  "can still be worthwhile and a nice addition, especially when
  having a lot of async stuff going on, but probably shouldn't be
  the main driver of state." In hindsight: "most of it in one crate
  with more specialization", accept that web and Masonry diverge,
  merge `ViewSequence` with `View` statically
  (`ViewSequence::ELEMENTS_COUNT`), more static sugar, no macros.
  (Now in 8.2 and 8.1.)
- Bill Johnson (audio plugin developer): came to Xilem for "the minimal core"
  that let them (1) integrate their own layout system (taffy) and
  (2) "retain control over the event loop. Other libraries have you
  call a function like run() that blocks for the duration of the
  application." Signals "store info in thread-local variables which
  is tricky with audio plugins because the UI is usually a side
  thread and the host application controls threading." (Now
  principle 7.)
- Lobsters, objectif_lune: building anything moderately complex on
  Xilem and Masonry directly was "brutal"; praise for Qt's
  signals/slots and for Slint separating UI declaration from logic.
- Lobsters, krig: build for concrete applications with concrete
  needs, or the framework grows complexity nobody asked for. (Now
  principle 2.)
- Lobsters, bitshift and replies: users accept GC-language overhead
  when the API is simple and compiles fast; the real complaint about
  Electron and GTK4 apps is memory, not speed.

## 12. Lessons from Bevy

Bevy is the one large Rust project that solved the object-graph
problem at scale, and it did it the Rust-native way. What transfers:

- **ECS is the with-the-grain answer to the object graph.** Entities
  are ids, components live in tables, systems declare what they read
  and write in their types, and the scheduler checks the access sets
  once when the schedule is built, then runs disjoint systems in
  parallel with no per-frame borrow checks. No `Rc<RefCell>`, no
  runtime borrow panics in the normal path. Masonry already moved
  this way in 2024 ("Widgets own keys" into `tree_arena`, post-mortem
  section 5) and planned the "Component" half (typed values attached
  to widgets); that became the `Property` system.
- **Change detection by ticks, not observers.** `Changed<T>` queries
  compare a write tick per component to the system's last-run tick.
  Fine-grained invalidation with no dependency tracker and no
  signals: writes are recorded where they happen, reads filter on
  them. This is the static alternative to Floem-style signals we
  asked for in option D, and it is cheap.
- **Relationships as data.** `ChildOf` and `Children` are components
  with the inverse maintained by the engine, so the tree is data you
  can query, not pointers you must own. Masonry's `children_ids` and
  arena are the same idea.
- **Observers and events for callbacks.** Bevy 0.19 BSN: "BSN
  entities can easily observe events, making it easy to embed
  callback-style behaviors in your scenes." Callbacks are systems
  with declared access, attached to an entity. That is how a button
  handler gets `&mut` to exactly the state it names and nothing else,
  checked at schedule-build time. This is closer to principle 1 than
  either GPUI listeners or Xilem's `&mut State` closures.
- **Scenes as patches.** BSN builds UI as layered patches over
  defaults: "You only need to set the fields you care about, and the
  rest will have their default values." Widgets are parameterized by
  `SceneComponent` props. This is a theming and composition model:
  a theme is a patch, a user override is a patch. Compare Masonry's
  property stack and Understory's style cascade.
- **Headless widgets under a styled set.** `bevy_ui_widgets`
  (behavior, no look) under Feathers (the editor's look). Same
  split as `gpui-base` under `gpui-component`. xix should ship both
  layers from day one.
- **Parley.** Bevy migrated to Parley in the 0.19 cycle because it
  "had meaningfully better documentation and was somewhat nicer to
  use" than cosmic-text. The Linebender text stack is now validated
  by the largest Rust engine.
- **Process.** Bevy prototyped BSN in 0.17, shipped it in 0.19, and
  says reactivity will be "an ecosystem of competing reactivity
  implementations" on top of the scene layer rather than one blessed
  model. Xilem put the reactive model first and the widget engine
  second; Bevy did the reverse and is further along.

What does not transfer: a full ECS scheduler and world is overkill
for a UI thread, Bevy's layout (Taffy) and rendering are built for
games, and a game loop re-runs every system every frame, which a
UI must not do. The idea to take is not "use Bevy" but "ids plus
typed components plus tick-based change detection plus declared
access" as the shape of the widget tree and its invalidation.

Sources: https://bevy.org/news/bevy-0-19/,
https://bevy.org/news/bevy-0-17/, https://bevy.org/news/bevy-0-18/

## Sources

- runebender-xilem: `context/*.md`, `ARCHITECTURE.md`, `LLMS.md`,
  `.agents/UI-PARITY.md`, `.agents/MASONRY-UPGRADE.md`
- runebender-gpui: `README.md`, `PARITY.md`, `Cargo.toml`, commits
  `e22cfd5`, `41477b5`, `8bfa913`, `119e6f2`, `50a39f6`
- runebender-core: `README.md`, `AGENTS.md`, `src/`
- Xilem upstream checkout at `7819435e`: `ARCHITECTURE.md`,
  `masonry/ARCHITECTURE.md`, `masonry_core/src/core/layer.rs`,
  `masonry/src/widgets/{canvas,passthrough}.rs`,
  `xilem_masonry/src/view/transform.rs`
- A critical review of Xilem in 2026:
  https://hackmd.io/@s_haMSbyTAOWfoXc1aYNUg/Hka74gCwZg
  (Lobsters: https://lobste.rs/s/bccwjx/critical_review_xilem_2026,
  r/rust: https://www.reddit.com/r/rust/comments/1vmle4d/)
- Post-mortem for work on Xilem in 2024:
  https://poignardazur.github.io/2025/03/24/plan-for-linebender-post-mortem/
- Linebender in 2026 Q1: https://linebender.org/blog/tmil-25/
- Xilem #1343 layers and menus:
  https://github.com/linebender/xilem/issues/1343
- Xilem #1444 non-contiguous app state:
  https://github.com/linebender/xilem/pull/1444
- Understory: https://github.com/forest-rs/understory
- GPUI: https://github.com/zed-industries/zed/tree/main/crates/gpui,
  https://docs.rs/gpui/latest/gpui/trait.Element.html,
  web platform PR https://github.com/zed-industries/zed/pull/50228,
  transform groups issue
  https://github.com/zed-industries/zed/issues/53303
- gpui-component: https://github.com/longbridge/gpui-component
- awesome-gpui: https://github.com/zed-industries/awesome-gpui
- Glass-HQ/gpui: https://github.com/Glass-HQ/gpui
- Floem: https://github.com/lapce/floem
- Makepad: https://makepad.nl/, https://github.com/makepad/makepad,
  https://makepad.rs/guide/start/makepad-framework-architecture,
  splash.md and AGENTS.md in the makepad repo (dev branch)
- 2025 survey of Rust GUI libraries (boringcactus):
  https://www.boringcactus.com/2025/04/13/2025-survey-of-rust-gui-libraries.html
- Raph Levien, Xilem: an architecture for UI in Rust (2022):
  https://raphlinus.github.io/rust/gui/2022/05/07/ui-architecture.html
