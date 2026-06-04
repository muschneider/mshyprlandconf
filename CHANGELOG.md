# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2026-06-03

### Added — the `.conf` (legacy hyprlang) format (`hyprconf-core::conf`)

- **Lossless document model (`conf::document`).** `ConfDocument` keeps every
  physical line verbatim (`Line { raw, ending, kind }`, where `LineEnding` is
  `Lf`/`CrLf`/`None`), so `to_text()` reproduces an unedited file
  **byte-for-byte**. The structured `LineKind` (`Assignment`/`Directive`/
  `Source`/`SectionOpen`/`SectionClose`/`Comment`/`Blank`/`Unknown`) rides
  alongside the raw text; assignment/directive/source lines store
  `indent + key + sep + value + trailing` pieces so an edit rewrites only the
  value.
- **`ConfParser` (`conf::parser`).**
  - `parse_str` — pure, no I/O: splits lines (faithful inverse of join,
    handling `\r\n`/missing final newline), classifies each line, tracks
    `section { ... }` nesting to resolve canonical `:`-paths
    (`decoration:blur:size`), and honours hyprlang's `##` comment escape.
  - `parse_file` — follows `source =` includes, resolving relative/absolute/
    **glob** paths (via the new `glob` dep), building a `ConfBundle` of all
    loaded files, and **detecting cycles and missing files** as the typed
    `ConfError` (`IncludeCycle`/`NotFound`/`Io`/`Glob`) — never panicking.
- **`ConfSerializer` + `value_to_conf` (`conf::serializer`).** Thin
  byte-faithful serialization plus rendering of typed `Value`s back to `.conf`
  text. `ConfDocument::set_option`/`set_option_value` edit in place (returning
  `SetOutcome::Edited`) or insert into the right section block
  (`InsertedInSection`) / append a flat key (`Appended`).
- **Semantic mapper (`conf::mapper`).** `document_to_config` /
  `bundle_to_config` interpret a document/bundle into the format-agnostic
  `Config`: expanding `$variables` (`$name` and `${name}`, cross-file, env vars
  left intact), mapping scalar keys onto `OptionSpec`s and parsing them into
  typed `Value`s, and turning directives into the structured collections
  (binds with flag/submap tracking, window/layer rules, monitors, workspaces,
  env, exec, bezier, animation). **Nothing is dropped:** unknown keys and
  unparseable values are stored (unknown scalars as `Value::String`) and
  surfaced as `ConfWarning`s. Provenance (source/line/leading-comments/
  trailing-comment) is populated.
- **`CoreError::Conf`** wraps `ConfError` (`#[from]`, additive).

### Tests

- 11 new integration tests (`tests/conf_roundtrip.rs`) over 7 fixtures
  (`tests/fixtures/conf/`): a **byte-for-byte no-edit round-trip** over every
  fixture; **single-value edit changes exactly one line** (asserted via line
  diff, with indent + inline comment preserved); insertion into an existing
  section and flat append; a **multi-file `source =` setup** (round-trips each
  file, cross-file variable expansion, last-wins merge); semantic mapping of
  variables/submaps/windowrulev2/unknown-key warnings; and **cyclic/missing
  includes as typed errors**. Plus 11 new unit tests across the conf modules
  (line-split inverse, comment escaping, piece reconstruction, directive
  detection, var expansion, bind/bezier/animation parsing).

### Dependencies

- `hyprconf-core` now depends on `glob` (for `source =` glob includes).

### Design notes

- **Two-layer design:** the lossless `ConfDocument` is the source of truth for
  serialization/editing (guaranteeing fidelity), while the semantic `Config` is
  a derived projection. This cleanly separates "preserve the file exactly" from
  "understand the file", and is why a one-option edit can be surgical.
- **Classification never affects round-trip:** even a misclassified or unknown
  line is preserved verbatim via `raw`; `LineKind` only drives editing and
  semantic mapping.
- Source-path `$variable`/env expansion is intentionally minimal for now
  (literal/relative/abs/glob paths); richer expansion can follow without
  changing the model.

## [0.2.0] - 2026-06-03

### Added (all in `hyprconf-core`, GUI shell unchanged)

- **`value` module** — format-agnostic scalar values and their Hyprland text
  forms:
  - `Color` (RGBA) with `from_hyprland_str` accepting `rgba(RRGGBBAA)`,
    `rgb(RRGGBB)` and legacy `0xAARRGGBB`/`0xRRGGBB`, plus `to_rgba_string`.
  - `Gradient` (color stops + optional `Ndeg` angle) with parse/format and
    round-trip.
  - `Vec2` with `"x y"` / `"x, y"` parsing and `"x y"` formatting.
  - `Value` (scalar enum: Bool/Int/Float/Color/Gradient/String/Enum/Vec2) and a
    typed `ValueParseError`.
- **`structured` module** — the ordered, repeatable constructs: `Keybind` +
  `KeybindFlags` (`bind*` keyword derivation), `WindowRule`, `LayerRule`,
  `MonitorRule`, `WorkspaceRule`, `EnvVar`, `Exec`/`ExecKind`, `Bezier`,
  `Animation`, `Submap`, `Variable`, and a uniform `StructuredValue`.
- **`schema` module** — the data-driven option surface:
  - `ValueType` (scalar + structured kinds, incl. `Enum(variants)`), `EnumVariant`,
    `NumericRange`, `OptionSpec` (path/label/description/type/default/range/`since`),
    `Section`, `CollectionId`, `CollectionSpec`, and `Schema`.
  - `Schema::load()` (infallible, `const`-built) and a cached `Schema::shared()`.
  - `OptionSpec::validate`/`validate_default` and `Schema::validate`
    (duplicate-path, scalar-type, valid-default and non-empty-enum checks).
  - ~160 curated options across all 14 required sections plus all 11 structured
    collections (monitors, workspaces, window/layer rules, keybinds, submaps,
    env, exec, variables, beziers, animations).
- **`model` module** — `Config` (format-agnostic): scalar options in an
  insertion-ordered `IndexMap<String, Tracked<Value>>` plus ordered `Vec`s for
  every structured collection. `Tracked<T>` carries `Provenance`
  (source/span/line/leading-comments/trailing-comment) for future
  comment-preserving round-trips. `Config::default_from_schema(&Schema)` and
  `ConfigFormat`/`Span`.
- **`CoreError`** gained `Schema`, `Validation { path, reason }`, and a
  `#[from] ValueParseError` variant (additive; `#[non_exhaustive]`).
- **`meta/`** — vendored upstream data: `hl.meta.lua` (the official Hyprland
  0.55.2 Lua stub), `hyprland-config-keys.txt` (its 341-key `HL.ConfigKey`
  list), and a `README.md` documenting provenance, the `:`↔`.` mapping, and the
  regeneration procedure.

### Tests

- 40 core tests, including: Color/Gradient/Vec2 parse + format + round-trip and
  error cases; schema section/collection presence; no duplicate paths; every
  default validates against its type/range; every enum has ≥1 variant;
  `Config::default_from_schema` population and provenance; and a
  **data-driven cross-check** (`every_option_path_exists_in_vendored_stub`) that
  asserts every schema path maps to a real key in the vendored stub.

### Dependencies

- `hyprconf-core` now depends on `indexmap` (insertion-ordered option map).

### Design notes

- The schema is shipped as compile-time `const` builders (in
  `schema/data.rs`, marked `#[rustfmt::skip]` to read as a data table) rather
  than an embedded RON/TOML blob: every default is type-checked by the compiler,
  so `Schema::load()` is infallible. Option *keys* remain externally verifiable
  against `meta/`.
- `Value` is scalar-only; structured constructs use their own typed structs.
- Defaults/types come from the Hyprland wiki (Configuring/Variables) and may
  drift between releases; the vendored stub only authoritatively provides the
  key *set*. `since` hints are deliberately `None` pending a wiki scrape.

## [0.1.0] - 2026-06-03

### Added

- Cargo workspace (`resolver = "2"`) with two members:
  - `hyprconf-core` — UI-free library crate. Exposes `version()`, a typed
    `CoreError` (`thiserror`, `#[non_exhaustive]`), and a `Result` alias. No
    `iced` dependency, so the interesting logic stays unit-testable.
  - `hyprconf-gui` — Iced 0.14 binary. Boots via the functional
    `iced::application(boot, update, view)` builder with `.title()` / `.theme()`,
    renders a top bar (title + light/dark theme toggle) over an empty content
    area, and switches the built-in `Theme::Light` / `Theme::Dark` through
    application state.
- `tracing` + `tracing-subscriber` initialised in the GUI with an `EnvFilter`
  that respects `RUST_LOG` (defaults to `info`).
- Workspace-wide lints (`unsafe_code = "deny"`, `clippy::all = "warn"`), opted
  into per crate via `[lints] workspace = true`.
- Tooling:
  - `mise.toml` pinning `rust` and `just`.
  - `rustfmt.toml` (stable-only options).
  - `justfile` with `build`, `run`, `test`, `lint`, `fmt`, `fmt-check`, `ci`.
  - GitHub Actions workflow `ci.yml` (fmt + clippy `-D warnings` + test on
    stable Linux, with the system deps iced needs).
- `CHANGELOG.md` (this file) and a `README.md` stub.

### Notes

- Tests added in this step: `hyprconf-core` unit tests for `version()` and
  `CoreError` display/conversion (4 tests, all passing).
- Verified locally against `iced = 0.14.0` / `iced_widget = 0.14.2`: the 0.14
  `widget::horizontal_space()` helper was removed; we use
  `widget::Space::new().width(Length::Fill)` instead.

[Unreleased]: https://github.com/hyprconf/hyprconf/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/hyprconf/hyprconf/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/hyprconf/hyprconf/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/hyprconf/hyprconf/releases/tag/v0.1.0
