# xix

A fork of [Xilem](https://github.com/linebender/xilem), the Rust UI
framework from the Linebender ecosystem.

## Why this fork exists

Every Rust GUI toolkit lets you write an interface. None of them stop
you writing an inconsistent one. They take a number wherever a
measurement is needed, which means every gap, inset, corner, and text
size is a decision, made separately, by whoever or whatever is typing.
A person makes those decisions consistently for about a hundred lines.
An agent does not make them consistently at all. That is why UI written
by an LLM reads as unfinished even when the logic is correct: 13 px
here and 12 px there, three button heights, a margin on a child that
fights a gap on its parent. The usual answers put the constraint
outside the framework, in a token file, an MCP server, or a review
checklist, and they help, but they cannot work by construction while
the framework underneath still accepts any number.

Xix moves the constraint into the type system. Measurements are steps
on a closed scale, so there is no value between them to land on by
accident. Containers state what they are (`column(Region::Panel, ..)`,
`row(Region::Toolbar, ..)`) and their spacing follows from that, so the
question "how much space goes here" is not asked at every call site.
Colors are roles resolved by a theme, which also styles every built-in
widget, so an application's accent and its controls cannot disagree.
The effect is measurable rather than aesthetic: porting a font editor
to plain Xilem produced twenty distinct spacing values, and rebuilding
its panels on regions took the number of places that state a gap or an
inset from 54 to 4. The calculator example used to open with its own
private design system of scales and colors; that module is now deleted,
and there is not one raw pixel value left in the file.

The second thing the fork is for is software with a canvas at the
center: font editors, node graphs, timelines, chart terminals, CAD.
Toolkits in this space hand you a paint callback and leave the rest,
so every such application rewrites the same viewport, the same
screen-to-document coordinate conversion, and the same drag state
machine. Two ports of the same font editor did it twice, about 150
lines each time, and it was the largest single piece of duplication in
both. Xix has that as a widget: an island owns the viewport, decides
which gestures are navigation, and reports the rest to the application
already converted into document coordinates. The examples include a
small graphics editor built on it with no viewport math in the file.

Who this is for: a design engineer who knows what good looks like and
builds through agents, and anyone shipping a professional graphics
application in Rust who does not want to spend the first month on
plumbing. Who it is not for yet: anyone who needs it finished. It is
early, there is no menu bar or shortcut system yet, the styled parts
list is thin, and the architecture is still research. Everything here
is meant to be upstreamable, and the best outcome is that Xilem takes
the ideas and this fork stops being necessary.

The code is an import of upstream Xilem at rev `ce7b04d2`
(2026-08-19) with full history, so upstream can be merged in later.
Crate names are unchanged for now. Upstream's own README is kept at
`docs/UPSTREAM_README.md`. Two documents hold the research.
[PLAN.md](PLAN.md) holds the evidence: what went wrong with Xilem for
real applications, what GPUI, Floem, and Bevy do, and the questions
still open. [DESIGN.md](DESIGN.md) holds the architecture proposal:
a UI framework for design engineers and professional graphics
software, where design is a type and good layout comes by
construction.

## What is different so far

**A design kernel.** `xilem::kernel` is a closed vocabulary the styling
APIs take in place of a number: `Space`, `ControlSize`, `Stroke`,
`Radius`, and `TextSize`.

```rust
flex_row(children).gap(Space::Md).padding(Space::Lg)
sized_box(child).height(ControlSize::Row).corner_radius(Radius::Sm)
label(name).text_size(TextSize::Body)
```

The reason is measured, not aesthetic. Porting a font editor to xilem
produced twenty distinct spacing values and four text sizes, because
every sizing API takes a free `f64`. The same editor ported to GPUI did
not drift, because GPUI's vocabulary is a closed scale you cannot land
between. See DESIGN.md D11.

**A theme is data.** `masonry::theme::Theme` holds the role colors and
the kernel steps that every built-in widget styles itself from, so an
application states its theme once and the whole widget set follows.
`default_property_set()` is now `Theme::dark().property_set()`.

**A canvas island.** `island` is a retained, pannable, zoomable surface
that owns the viewport and reports gestures already converted into
document coordinates. It is the primitive under a glyph canvas, a node
graph, a timeline, and a map, and it is the code both Runebender ports
rewrote per canvas.

## Build

```sh
cargo build --workspace --examples
cargo run -p xilem --example app_shell
```

Three examples are worth reading first:

| Example | What it shows |
|---|---|
| `app_shell` | A whole desktop screen (sidebar, content, inspector, status bar) with the kernel and no loose numbers |
| `shapes` | A small graphics editor on `island`: draw, select, drag, pan, zoom, with no viewport math in the file |
| `tokens` | Density, roundness, and hue as sliders: the same tokens, resolved by a theme, restyle a working panel |
| `widgets` | Every control on one screen at three densities: a coherence test, not a catalog |
| `canvas` | A plain retained drawing surface, without the island's gestures |

Add `XIX_SCREENSHOT=out.png` and the `screenshot` feature to render any
example headlessly, with no window:

```sh
XIX_SCREENSHOT=shell.png cargo run -p xilem --features screenshot --example app_shell
```

The repo pins stable Rust in `rust-toolchain.toml` (upstream MSRV
is 1.96).

## License

Apache-2.0, the same as upstream Xilem.
