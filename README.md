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

## Build

```sh
cargo build --workspace --examples
cargo run -p xilem --example mason
```

The repo pins stable Rust in `rust-toolchain.toml` (upstream MSRV
is 1.96).

## License

Apache-2.0, the same as upstream Xilem.
