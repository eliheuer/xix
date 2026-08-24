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

`xilem::kernel` is the design kernel: `Space`, `ControlSize`, `Stroke`,
`Radius`, and `TextSize`, a closed vocabulary that the styling APIs
take in place of a number.

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
