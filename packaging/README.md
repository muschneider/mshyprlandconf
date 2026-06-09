<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# Packaging

Distribution recipes for hyprconf. The binary is named **`hyprconf`** (the
package/crate is `hyprconf-gui`).

| Target  | File                                              | Notes                                              |
| ------- | ------------------------------------------------- | -------------------------------------------------- |
| Arch    | [`aur/PKGBUILD`](aur/PKGBUILD)                    | `cd packaging/aur && makepkg -si`. Set the real `sha256sums` when tagging. |
| Nix     | [`../flake.nix`](../flake.nix)                    | `nix run .` / `nix build .` / `nix develop`.       |
| Flatpak | [`flatpak/io.github.hyprconf.hyprconf.yaml`](flatpak/io.github.hyprconf.hyprconf.yaml) | Optional; needs a generated `cargo-sources.json` (offline build). |
| Desktop | [`hyprconf.desktop`](hyprconf.desktop)            | Installed by the AUR/Nix/Flatpak recipes.          |

## Runtime dependencies (Linux)

hyprconf is an iced (winit + wgpu) app. At runtime it loads, via the dynamic
linker, Wayland, libxkbcommon, and a Vulkan/GL stack:

- `wayland`, `libxkbcommon`
- `vulkan-icd-loader` (+ your GPU's Vulkan ICD) and/or `libGL`

`hyprctl` (from Hyprland) is an **optional** runtime dependency — present only
for live-apply/reload; the app runs fine without it.

## Reproducible release builds

The workspace `[profile.release]` enables `lto = "fat"`, `codegen-units = 1`,
and `strip = true`. For byte-for-byte reproducibility, also export
`SOURCE_DATE_EPOCH` and `RUSTFLAGS="--remap-path-prefix=$PWD=."` before building.
The release GitHub Actions workflow builds with `--locked` to pin the dependency
graph from `Cargo.lock`.
