# hyprconf

A Linux-first desktop GUI for viewing and editing the **full** surface of
[Hyprland](https://hyprland.org) configuration.

hyprconf treats both on-disk formats as first-class citizens:

- **Lua** — Hyprland's default since 0.55 (`~/.config/hypr/hyprland.lua`, the
  `hl` global table, `require()` sourcing, helpers).
- **conf** — legacy hyprlang (`~/.config/hypr/hyprland.conf`, `key = value`,
  `{}` sections, `source=`, `$variables`, `windowrulev2`, ...). Deprecated but
  still supported.

The app can read either format, and you choose which one to write. A
format-agnostic in-memory model is the core abstraction.

> Status: **work in progress.** The core reads and writes both formats
> (round-trip preserving), and the GUI loads and lets you browse your real
> config. Editing/saving from the UI and `hyprctl` integration are still to come.

## Workspace layout

| Crate           | Kind | Responsibility                                                            |
| --------------- | ---- | ------------------------------------------------------------------------- |
| `hyprconf-core` | lib  | Format-agnostic model, schema, parsers, serializers, validation. No GUI.  |
| `hyprconf-gui`  | bin  | Iced 0.14 desktop front-end (The Elm Architecture).                       |

Keeping `hyprconf-core` free of any UI dependency is what makes the interesting
logic testable in isolation.

## Requirements

- A Rust toolchain (managed via [`mise`](https://mise.jdx.dev): `mise install`).
- [`just`](https://github.com/casey/just) as the task runner (also pinned in
  `mise.toml`).
- On Linux, the usual iced/winit/wgpu system libraries (e.g. `libxkbcommon`,
  `libwayland`); see the CI workflow for the exact package list.

## Common tasks

```sh
just            # list all recipes
just run        # run the GUI (auto-detects ~/.config/hypr/hyprland.{lua,conf})
just test       # run the test suite
just lint       # clippy with -D warnings
just fmt        # format the workspace
just ci         # fmt-check + lint + test (the full local gate)
```

The GUI accepts:

```sh
just run -- --config /path/to/hyprland.lua   # load a specific file
just run -- --check                          # headless: load + print summary, no window
```

Logging honours `RUST_LOG`, e.g. `RUST_LOG=debug just run`.

## License

Dual-licensed under either of MIT or Apache-2.0, at your option.
