<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# hyprconf

A Linux-first desktop GUI for viewing and editing the **full** surface of
[Hyprland](https://hyprland.org) configuration — for both the modern **Lua**
format and the legacy **conf** (hyprlang) format, over one shared model.

![hyprconf editing the General section](assets/screenshot.png)

## Why

Hyprland has a large, fast-moving configuration surface. Editing it by hand
means memorising option names, value formats (`rgba(...)`, gradients, `Vec2`,
bind flags), and — since 0.55 — juggling **two** on-disk formats. hyprconf gives
you a typed, validated, searchable UI over that surface while treating your file
as the source of truth: it preserves comments and ordering where it can, shows
you an exact diff before writing, and never silently drops anything it doesn't
understand.

## Features

- **Both formats, first-class.** Reads `hyprland.lua` (the `hl` API, `require()`
  sourcing) **and** `hyprland.conf` (`key = value`, `{}` sections, `source=`,
  `$variables`, `windowrule[v2]`). You choose which format to write; convert
  between them in one click.
- **Typed editors for every option** — toggles, bounded sliders, enum pickers,
  `Vec2`, and a **visual color picker** (saturation/value square + hue strip,
  HEX + RGBA, live preview) for colors and gradient stops.
- **Structured collections** — full editors for keybinds (modifiers, dispatcher,
  flags, submaps), window/layer rules, monitors, env, exec, with
  add/remove/reorder.
- **Live fuzzy search** across the whole option surface.
- **Safe saving** — a validation pass (errors block, warnings are overridable),
  a per-file before/after **diff preview**, then **atomic writes with timestamped
  backups**. Comment-preserving in-place edits for same-format `.conf`.
- **Undo/redo** (with sensible coalescing) and `Ctrl+Z` / `Ctrl+Shift+Z`.
- **Live apply** — when a running Hyprland is detected (`hyprctl`), optionally
  push edits instantly with `hyprctl keyword`, and reload on demand.
- **Profiles & recents** — save the current config as a named profile, reopen
  recents, import any file.
- **Persistent settings** — theme (22 built-in themes), last format, window
  size and recent files survive restarts.
- **`--check`** — a headless "load and summarise" mode for scripts/CI.

## Supported Hyprland versions

hyprconf's option **schema** is vendored from the Hyprland **0.55.2** Lua stub
(`meta/`, see [`meta/README.md`](meta/README.md)) — that's the version it knows
the most about. It still reads and writes configs for older and newer Hyprland
releases; out-of-schema keys are **preserved, never dropped** (and surfaced as
warnings). When a running Hyprland is detected, options whose `since` version is
newer than what's running are flagged in the status bar.

## Lua vs conf, and the dynamic-Lua caveat

- The in-memory model is **format-agnostic**. Reading either format produces the
  same `Config`; you pick the output format independently.
- **conf → conf, scalar-only edits** are *preserved*: hyprconf edits the original
  document(s) in place, keeping comments, ordering and untouched lines
  byte-for-byte, and (for multi-file setups) rewrites only the files that changed.
- **Format conversions, collection edits, and Lua output** *regenerate* a fresh
  file from the model (no original formatting to preserve).
- **Dynamic Lua is never executed or flattened.** Hyprland's Lua config can
  contain loops, functions, conditionals, and `hl.on`/timer closures. hyprconf
  parses Lua *losslessly* (it does not run it) and only interprets the
  *declarative* subset. Anything dynamic is left untouched and reported as a
  read-only region — and **converting a Lua config that contains dynamic regions
  to conf will drop them**, which hyprconf warns about before you save.

## Install

### Arch Linux (AUR)

A `PKGBUILD` lives in [`packaging/aur/`](packaging/aur/PKGBUILD). With an AUR
helper once published, or directly from the repo:

```sh
cd packaging/aur && makepkg -si
```

### Nix (flake)

```sh
nix run github:hyprconf/hyprconf       # run without installing
nix profile install github:hyprconf/hyprconf   # install into your profile
```

A local checkout works too: `nix run .` / `nix build .`.

### cargo install

```sh
cargo install --git https://github.com/hyprconf/hyprconf hyprconf-gui
```

On Linux you need the usual iced/winit/wgpu system libraries at build time
(e.g. `libxkbcommon`, `wayland`); see [`packaging/`](packaging/) and the CI
workflow for the exact package lists.

### From source

```sh
git clone https://github.com/hyprconf/hyprconf
cd hyprconf
cargo run -p hyprconf-gui --release       # or: just run
```

## Usage

```sh
hyprconf                               # auto-detect ~/.config/hypr/hyprland.{lua,conf}
hyprconf --config /path/to/hyprland.lua  # load a specific file
hyprconf --check                       # headless: load, print a summary, exit
```

- **Search**: type in the header box to fuzzy-find any option.
- **Edit**: change values with the typed editors; the 🎨 icon on any color field
  opens the visual picker.
- **Undo/redo**: `Ctrl+Z` / `Ctrl+Shift+Z` (or `Ctrl+Y`). `Ctrl+S` opens the
  save panel.
- **Convert & save**: open *save…*, pick the output format, review the diff,
  write. A backup of each overwritten file is kept.
- **Live apply**: if the status bar shows a detected Hyprland, toggle *live* to
  push scalar edits immediately, or hit *reload*.

Logging honours `RUST_LOG` (e.g. `RUST_LOG=debug hyprconf`).

## Development

Requires a Rust toolchain (`mise install` honours [`mise.toml`](mise.toml)) and
[`just`](https://github.com/casey/just).

```sh
just            # list recipes
just run        # run the GUI
just test       # run the test suite (incl. headless UI tests)
just lint       # clippy -D warnings
just fmt        # format
just ci         # fmt-check + lint + test (the full local gate)
```

See [ARCHITECTURE.md](ARCHITECTURE.md) for the core/gui split and the model, and
[CONTRIBUTING.md](CONTRIBUTING.md) to get started.

## License

Dual-licensed under either of [MIT](LICENSE-MIT) or
[Apache-2.0](LICENSE-APACHE), at your option. See [NOTICE](NOTICE) for
third-party attributions (including the vendored Hyprland metadata, which is
BSD-3-Clause). hyprconf is an independent project, not affiliated with Hyprland.
