# xix

A fork of [Xilem](https://github.com/linebender/xilem), the Rust UI
framework from the Linebender ecosystem.

The code is an import of upstream Xilem at rev `ce7b04d2`
(2026-08-19) with full history, so upstream can be merged in later.
Crate names are unchanged for now. The architecture is still in a
research phase; nothing is decided. Upstream's own README is kept at
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
