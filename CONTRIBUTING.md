<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# Contributing to hyprconf

Thanks for your interest! hyprconf is a Rust workspace; contributions of all
sizes are welcome — bug reports, docs, schema fixes, and features.

## Getting set up

```sh
git clone https://github.com/hyprconf/hyprconf
cd hyprconf
mise install            # installs the pinned Rust toolchain + just (or use rustup)
just run                # build & run the GUI
```

On Linux you need iced's system libraries (`libxkbcommon`, `wayland`); the exact
apt list is in `.github/workflows/ci.yml`.

## The local gate (run before every push)

CI runs exactly these three, and they must pass on stable Linux:

```sh
just ci      # == cargo fmt --all -- --check
             #  + cargo clippy --workspace --all-targets -- -D warnings
             #  + cargo test --workspace
```

`clippy` is **`-D warnings`** — no warnings allowed. `cargo test` includes the
headless UI tests in `crates/hyprconf-gui/src/ui_tests.rs`.

## Conventions

- **Keep `hyprconf-core` free of `iced`.** Anything that can be tested without a
  window belongs in the core (and should come with tests).
- **Add tests in the same change.** New parsing/serialization → a round-trip or
  snapshot test; new editor logic → an `edit.rs` unit test; new flows → a
  headless UI test.
- **SPDX headers.** Every source file starts with
  `// SPDX-License-Identifier: MIT OR Apache-2.0`.
- **Formatting/lints** are enforced by `rustfmt` and `clippy`; don't hand-format
  around them. Use `#[rustfmt::skip]` only where it genuinely aids readability
  (as in `schema/data.rs`).
- **Commits / PRs**: small, focused, with a clear message. Update `CHANGELOG.md`
  (Keep a Changelog format) and bump the workspace version when shipping a
  user-visible change.

## Touching the schema

The option **key set** is checked against the vendored Hyprland stub in `meta/`.
If you add an `OptionSpec`, its path (with `:`→`.`) must exist in
`meta/hyprland-config-keys.txt`, or the schema test fails. After a Hyprland
upgrade, regenerate the vendored files as described in
[`meta/README.md`](meta/README.md). Defaults/types/descriptions come from the
Hyprland wiki — cite where non-obvious.

## Architecture

Read [ARCHITECTURE.md](ARCHITECTURE.md) first — the core/gui split and the
format-agnostic model explain where new code should go.

## Licensing of contributions

By contributing you agree that your work is dual-licensed under **MIT OR
Apache-2.0**, matching the project (see [NOTICE](NOTICE)). Don't paste code or
assets under incompatible terms.

## Reporting bugs / requesting features

Use the issue templates under `.github/ISSUE_TEMPLATE/`. For bugs, include your
Hyprland version (`hyprctl version`), the format you're editing (lua/conf), and
a minimal config snippet that reproduces the problem.
