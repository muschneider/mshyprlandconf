# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.9.0] - 2026-06-07

### Added — live Hyprland, undo/redo, profiles & persistence

- **Core `hyprctl` module** (UI-free wrappers around the CLI, run off the UI
  thread inside `iced::Task`): `detect()` parses `hyprctl version` into a
  `HyprlandInfo { version, tag }` (`None` when `hyprctl` is missing or Hyprland
  isn't running — **degrades gracefully**); `apply_keyword(name, value)` and
  `reload()` return a typed `HyprctlError`; plus `parse_semver`/`is_newer` for
  version comparison. Re-exported as `hyprconf_core::{HyprctlError, HyprlandInfo}`.
- **Core `validate::unsupported_options(schema, config, running_version)`** flags
  set options whose schema `since` is newer than the running Hyprland (a soft
  warning). Added real `since` metadata to the schema (`decoration:rounding_power`
  → 0.45.0; `general:snap:*` → 0.42.0).
- **GUI live-apply & reload:** when a running Hyprland is detected, the status bar
  shows a version badge, a **live-apply** toggle and a **⟳ reload** button.
  With live-apply on, each committed, valid scalar edit is pushed via
  `hyprctl keyword`; reload triggers `hyprctl reload`. The status bar also warns
  when set options need a newer Hyprland than is running.
- **Undo/redo** (`edit.rs` `EditSnapshot` + `Loaded::snapshot`/`restore`): full
  edit-state snapshots on an undo/redo stack, with **coalescing** so a burst of
  continuous edits (typing in a field, dragging a slider) collapses into one
  step while discrete edits (toggles, enum picks, add/remove) are individual.
  Header **↶/↷** buttons and **Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y** shortcuts
  (via an `iced` event subscription); a fresh load clears the history.
- **Settings persistence (`settings.rs`)** under
  `$XDG_CONFIG_HOME/hyprconf/settings.toml` (serde + toml): theme, last output
  format, window size and recent files (deduped, newest-first, capped at 8).
  Restored on launch (theme, format, window size) and saved on change.
- **Profiles & recents (`profiles.rs` + a profiles panel):** save the current
  config as a named profile under `$XDG_DATA_HOME/hyprconf/profiles/<name>.{conf,lua}`,
  reopen saved profiles or recent files, and **import** a config from an
  arbitrary path — all reusing the existing load/serialize pipeline.
- **Responsive layout:** below 860 px the sidebar collapses to an icons-only
  rail with hover tooltips; it expands again when widened. `Ctrl+S` toggles the
  save panel.

### Tests

- Core (69, +3 `hyprctl` over 0.8.0's count plus 1 `validate`): version-output
  parsing, unrecognised output → `None`, semver parse/compare, and
  `unsupported_options` flags a too-new `since`.
- GUI (39): `EditSnapshot` round-trips a scalar **and** collection edit through
  undo/redo; coalescing groups typing but not discrete edits; `Settings`
  round-trips through TOML, dedupes/caps recents, and tolerates partial files;
  profile-name sanitisation. Live-apply/reload, the profiles panel, the Hyprland
  status bar and the collapsed sidebar verified visually with `grim`
  (Hyprland 0.55.2).

### Notes & trade-offs

- Live-apply is **off by default** and only available when Hyprland is detected;
  it pushes scalar `keyword`s only (collection edits are applied via Save +
  reload). Undo/redo does not re-push to Hyprland.
- Settings use a plain (non-fsync) write since they're non-critical — this keeps
  frequent saves (e.g. window-resize) cheap.
- The `since`-mismatch check only warns when an option is newer than the running
  Hyprland; all curated `since` values are ≤ 0.55.2.
- No native file dialog: import uses a path field (keeps the build dep-light).

## [0.8.0] - 2026-06-04

### Added — saving, format conversion, preview/diff (core + GUI)

- **Core `fs` module:** `atomic_write` (write to a temp file in the same dir →
  `fsync` → `rename`, with dir fsync and temp cleanup on failure),
  `backup_existing` (timestamped `*.<unixsecs>.bak` copy), and `save_atomically`
  (backup + atomic write → `SaveReport`). `CoreError::Fs` wraps `FsError`.
- **Core `validate` module:** `validate_config(&Schema, &Config) -> Vec<ConfigProblem>`
  with `Severity::{Error, Warning}` — out-of-range/invalid values are errors,
  unknown keys are warnings; plus `has_errors`.
- **Core `conf::config_to_conf`** (from 0.7.0) is the conf side of conversion;
  `LuaSerializer::serialize` is the Lua side.
- **GUI save flow (`save.rs`):**
  - `plan_save` chooses **Preserve** (same-format `conf`, scalar-only edits:
    edits the original document(s) in place via `set_option`, preserving
    comments/structure) or **Regenerate** (format conversion, collection edits,
    or Lua: fresh serialization).
  - **Multi-file aware:** preserve routes each edited scalar to the source file
    that defines it and writes back **only the files that changed**.
  - `review` aggregates scalar + structured-collection validation into jump-to-
    field problems; `blocked` gates the write (errors always block; warnings
    block unless overridden).
  - `perform_save` writes each changed file atomically with a backup.
- **GUI save panel:** output-format selector (conf ⇄ Lua), a **dynamic-Lua-loss
  warning** when regenerating a Lua-origin config, the validation list with
  jump-to-field buttons + a "save anyway" override, a per-file **before/after
  diff** (`diff.rs`, dependency-free LCS) with +/- counts, and a write button
  disabled while blocked. On success the GUI reloads from disk. A header `save…`
  button and a status-bar result line were added.

### Tests

- Core: 3 `fs` tests (create/replace, **backup preserves the prior file**,
  **the original survives a failed write**), 3 `validate` tests, plus the
  `config_to_conf` test.
- GUI: 6 `save` tests — **preserve edits only the changed scalar line**
  (comments kept); **convert conf→lua is semantically equivalent** (parse-back)
  and **preview == on-disk**; collection edits force regenerate;
  **lua→conf fires the dynamic-loss warning**; **multi-file preserve writes only
  the changed include**; preserved output re-parses. Plus 3 `diff` tests. Save
  panel verified visually with `grim`.

### Notes & trade-offs

- Preserve mode is implemented for `conf` (Prompt 3 gave it surgical
  `set_option`); Lua and all conversions **regenerate** a single fresh file
  (comments not preserved). Editing a `$variable`-valued option turns that one
  line into a literal; untouched lines keep their `$var`.
- Backups are suffixed with unix-seconds (`.bak`) to avoid a calendar-date dep.

## [0.7.0] - 2026-06-04

### Added — editors for the structured collections (`hyprconf-gui`)

- **`config_to_conf` in core** (`conf::config_to_conf` / `ConfSerializer::serialize_config`):
  fresh `.conf` generation from an in-memory `Config` (the counterpart to
  `LuaSerializer::serialize`), emitting scalar options, env, monitors,
  workspaces, window/layer rules, beziers, animations, exec, and keybinds
  (grouped into `submap = NAME … submap = reset` blocks). This is what lets the
  model serialize to **both** formats.
- **Collection editing engine (`edit.rs`):** `CollectionAction` for structural
  ops (`Add`/`Remove`/`Duplicate`/`Move` per `CollectionId`) and per-collection
  field edits; touched-collection tracking folded into the global unsaved count
  (`total_unsaved`); pure validation helpers (`keybind_issue`,
  `window_rule_issue`, `monitor_issue`, …) and parsing helpers for mods,
  window-rule matchers and monitor `extra` tokens.
- **Keybind editor:** a table with add/remove/duplicate/reorder; per row a
  **modifier multi-select** (SUPER/SHIFT/CTRL/ALT), key field, **dispatcher
  `pick_list`** (exec, killactive, workspace, movetoworkspace, togglefloating,
  fullscreen, movefocus, resizeactive, …), args field, **bind-flag chips**
  (m/e/r/l/n/t/i), and a submap field. Validates empty key/dispatcher and
  dispatchers that require arguments.
- **Window-rule & layer-rule editors:** `windowrulev2`↔legacy `v2` toggle,
  rule/effect field, and a **match-criteria builder** (`key : value` rows with
  add/remove) plus a raw-matcher escape hatch for v1 regexes / commas the
  builder can't model. Both map through the shared `WindowRule` model so they
  serialize to `hl.window_rule({name,match})` and `windowrule[v2] = …`.
- **Monitor editor:** connector, mode, position, scale, and transform/vrr/mirror
  (mapped to/from the model's `extra` tokens).
- **Submap, env, and exec editors** (exec with an `exec`/`exec-once`/
  `exec-shutdown` picker), all with ordering.
- **Pending-changes view** now lists edited collections alongside scalar diffs.

### Tests

- 6 new collection tests (23 GUI tests total): add/remove/duplicate/reorder;
  mods + flags; the window-rule match builder; monitor transform/vrr via `extra`;
  validation of obviously-invalid entries; and — the headline acceptance —
  **a GUI-built keybind, window rule and monitor round-trip through BOTH `.lua`
  and `.conf`** (serialize via the core serializers, parse back, assert value
  equality). Plus a core `config_to_conf` test. Editors verified visually with
  `grim` (keybind mods/flags/validation; window-rule v2 + match builder + raw).

### Notes & trade-offs

- The match builder splits on `,` then the first `:`; matcher values containing
  commas (rare regexes) are handled via the **raw** field. Monitor `extra`
  editing keeps the recognized transform/vrr/mirror tokens; other trailing
  tokens are dropped on edit (use a future raw field if needed).
- Workspaces / beziers / animations remain read-only this step.
- Collection dirty-tracking is a touched-set (per-collection), not item-level
  baseline diffing.

## [0.6.0] - 2026-06-04

### Added — scalar option editing (`hyprconf-gui`)

- **A UI-free editing engine (`edit.rs`)** applied to the in-memory `Config`:
  - `EditAction` covers every edit (toggle, enum pick, int/float slider, color
    channel, text edit, gradient add/remove stop, reset).
  - **Two-way binding:** edits commit typed `Value`s into the model; widgets
    read back from the model.
  - **Dirty tracking** against a load-time baseline: `dirty` set + `dirty_count`;
    editing a field back to its baseline clears it.
  - **Reset-to-default** sets the schema default, rebaselines and clears the
    field's dirty flag, drafts and errors.
  - **Non-blocking validation:** per-field draft text + error map; invalid /
    out-of-range input is rejected (error recorded) **without mutating the
    model** — the last valid value is preserved.
  - A **pending-diff** API (`pending_diff`) listing `(path, baseline, current)`.
- **Per-`ValueType` editors (`view.rs`):** `Bool`→toggler; `Int`/`Float`→
  bounded slider (when min+max known, honoring step) + validated numeric input;
  `String`→text input; `Enum`→`pick_list`; `Color`→swatch + `rgba()` hex field
  + R/G/B/A channel sliders; `Gradient`→multi-stop editor (per-stop swatch + hex
  + add/remove) with an angle field; `Vec2`→two numeric fields.
- **Per-option affordances:** the description as an `ⓘ` hover **tooltip**, a
  **modified** dot, and a **reset** control (disabled when already default,
  tooltip "Reset to default"), plus inline error text with a danger-bordered
  input.
- **Global dirty indicator:** a header pill (`● N unsaved` / `no changes`) that
  toggles a **Pending changes** view showing every edit as `baseline → current`
  with a per-row reset (the "debug pending diff" surface).

### Tests

- 9 new `edit` unit tests (17 GUI tests total) verifying: bool/enum/int/float/
  color/vec2/gradient edits round-trip into the model; range and garbage input
  are rejected without corrupting the model; reset restores the default and
  clears dirty/draft/error; and the pending-diff listing. Editors verified
  visually via `grim` screenshots (toggler, bounded sliders, gradient stops with
  live color swatches, reset/info controls).

### Notes & trade-offs

- A slider is shown only for numeric options with **both** bounds; min-only
  options (e.g. `gaps_in`) use the validated input alone (picking an arbitrary
  upper bound would be misleading). Step is honored by the slider; range by the
  input.
- Reset rebaselines the field to default, so a reset option reads as "clean"
  (no unsaved change) per the spec — acceptable while persistence is out of
  scope; revisit when saving lands.
- Structured collections (keybinds/rules/monitors/…) remain read-only this step.

## [0.5.1] - 2026-06-04

### Changed — GUI visual redesign (`hyprconf-gui`)

- **Theming.** Replaced the light/dark toggle with a **theme picker**
  (`pick_list` over all 22 built-in Iced themes); the default is now
  **Catppuccin Mocha** (fits the Hyprland aesthetic). All custom styling is
  derived from the active theme's *extended palette*, so it stays readable in
  every theme (verified in both Catppuccin Mocha and Catppuccin Latte).
- **Layout & hierarchy.** Restructured into a styled **header bar**
  (brand + search + theme picker), a tinted **sidebar panel**, a padded
  **content area**, and a **status bar** — each with theme-aware backgrounds so
  the regions read as distinct panels.
- **Icons.** Every section and collection now has an icon in the sidebar, the
  pane header and search results (emoji glyphs; degrade to a `•` bullet if a
  glyph is unavailable).
- **Option cards.** Options render as rounded, bordered cards: bold label,
  dimmed `section:path`, and the value **right-aligned in the accent color**;
  schema defaults are muted and tagged with a small `default` badge.
- **Sidebar polish.** Custom nav-button styling (accent fill when selected,
  subtle hover), uppercase group headers, and **live count badges** on each
  collection. Search results use a hover-highlight row style and keep icons.
- **Status bar.** A colored format badge (`conf`/`Lua`), the source path,
  included-file count, options-set count, and warnings (in the danger color).

### Notes

- Icons rely on a system emoji font (standard on a Hyprland desktop); they are
  purely decorative, so missing glyphs never hide information.
- `main.rs` now stores an `iced::Theme` directly (replacing the `Appearance`
  enum); `Message::ThemeSelected(Theme)` replaces `ThemeToggled`. No core or
  test changes; all 83 tests still pass.

## [0.5.0] - 2026-06-04

### Added — the core wired into the GUI (`hyprconf-gui`)

- **Non-blocking startup load.** On launch the GUI locates and parses the user's
  config inside an `iced::Task` (off the UI thread), showing a **Loading** state
  until it completes, then **Loaded** / **NotFound** / **Error** states.
  - Detection: `hyprland.lua` then `hyprland.conf` under `$XDG_CONFIG_HOME/hypr`
    (via the `directories` crate), falling back to `~/.config/hypr`.
  - `--config <path>` (or `--config=<path>`) overrides detection; format is
    inferred from the extension.
  - Parsing goes through `hyprconf-core` (`LuaParser`/`ConfParser` + the
    `bundle_to_config` mappers), following includes; out-of-schema keys are
    counted as warnings, never dropped.
- **Browse UI** (read-only this step):
  - a scrollable **left sidebar** listing all schema **sections** plus a
    **collections** group (Keybinds, Window Rules, Monitors, … with live counts);
  - a **main pane** showing the selected section's options as `label: value`
    (effective value from the file, or the schema default marked `(default)`),
    or a one-line summary per item for collections;
  - a **status bar** with detected format (Lua/conf), source path, included-file
    count, options-set count and warning count.
- **Live fuzzy search** (`fuzzy.rs`, dependency-free subsequence matcher with
  prefix/contiguity bonuses and label > path > description weighting). A
  non-empty query replaces the pane with ranked matches across all sections;
  clicking a result jumps to its section.
- **`--check`**: a headless flag that loads the config, prints a one-line
  summary and exits (no window) — handy for scripting/CI smoke tests.

### Tests

- 8 GUI unit tests: `load_config` for explicit `.conf`/`.lua` (typed values),
  missing-path error, extension→format mapping; and the fuzzy matcher
  (subsequence, empty query, prefix/contiguity ranking, field weighting).
- Verified end-to-end against a **real** `~/.config/hypr/hyprland.conf` via
  `--check` (auto-detected through a symlink: 51 options set, 59 warnings for
  keys outside the curated schema subset — preserved, not dropped) and via an
  explicit `--config` override that followed 2 includes. The window was launched
  briefly to confirm it renders without panicking.

### Dependencies

- `hyprconf-gui` now depends on `directories` (XDG config dir lookup).

### Design notes

- `iced::Task::perform` runs the synchronous load on the executor (not the UI
  thread), satisfying "non-blocking with a visible loading state" without
  pulling in a heavier async story; the message carries `Arc<LoadState>` so it
  stays cheaply `Clone`.
- The GUI stays a thin shell over the core: it owns no config knowledge beyond
  the schema, reusing `conf::value_to_conf` for value display so on-screen
  rendering matches what would be written back.

## [0.4.0] - 2026-06-03

### Added — the Lua format (`hyprconf-core::lua`)

- **Reading strategy: lossless static parse** of the declarative subset via the
  [`full_moon`] crate (concrete-syntax Lua parser). Chosen over sandboxed
  evaluation because it is the right fit for an *editor*: comments, ordering and
  formatting survive round-trips and untrusted config code is never executed.
  - `LuaDocument` owns the parsed `Ast`; `to_text()` reproduces the source
    byte-for-byte (full_moon is lossless).
  - `lua::document_to_config` / `bundle_to_config` interpret the recognised
    `hl.*` calls and top-level `require` into the format-agnostic `Config`.
  - **Dynamic Lua is never flattened:** anything outside the declarative subset
    (functions, loops, conditionals, `local x = require(...)`, method chains,
    `hl.on`/`hl.timer`/`hl.define_submap` closures, ...) is left untouched in
    the lossless document and reported as `LuaWarning::DynamicRegion` (read-only
    / externally managed).
  - The optional sandboxed `mlua` eval path is **not** implemented this step
    (it is optional per spec; would sit behind a `lua-eval` feature).
- **`LuaParser`**: `parse_str` (pure) and `parse_file` (follows
  `require("mod")` → `<dir>/mod.lua`, detecting cycles and missing files as the
  typed `LuaError`).
- **`LuaSerializer`**: emits fresh idiomatic Lua against the `hl` API confirmed
  in `meta/hl.meta.lua` — `hl.config({...})` (nested tables, dotted keys quoted),
  `hl.bind`, `hl.window_rule`/`hl.layer_rule`/`hl.monitor`/`hl.workspace_rule`,
  `hl.env`, `hl.exec_cmd`, `hl.animation`, `hl.curve`, and `$variables` as Lua
  `local`s. Also re-serializes a parsed document losslessly. Plus `value_to_lua`.
- **`CoreError::Lua`** wraps `LuaError` (`#[from]`, additive).

### Tests

- 6 new integration tests (`tests/lua_roundtrip.rs`) + fixtures
  (`tests/fixtures/lua/*.lua`, `tests/fixtures/conf/roundtrip.conf`):
  - **cross-format round-trip** `.conf → Config → .lua → parse → Config`,
    asserting semantic equality over binds, window rules, monitors, decoration
    options and animations;
  - **Lua `Config → .lua → Config`** round-trip (the reverse direction);
  - **lossless** byte-for-byte round-trip of Lua fixtures (comments/order);
  - **dynamic Lua preserved verbatim and flagged read-only**;
  - `require()` following across files; missing file as a typed error.
- 7 new unit tests across the lua modules (AST→intermediate lowering, callee/
  require extraction, escape round-trip, value/key rendering).

### Dependencies

- `hyprconf-core` now depends on `full_moon` (lossless Lua parser).

### Confirmed against the official stub / flagged uncertainties

- The `hl` API shape used for emission is taken from `meta/hl.meta.lua`
  (Hyprland 0.55.2): `HL.API` fields `config`, `bind`, `window_rule`,
  `layer_rule`, `monitor`, `workspace_rule`, `env`, `exec_cmd`, `animation`,
  `curve`; `HL.BindOptions` (`repeating`/`locked`/`release`/`non_consuming`/
  `transparent`/`ignore_mods`); `HL.MonitorSpec` (`output`/`mode`/`position`/
  `scale`); `HL.WindowRuleSpec` (`name`/`match`).
- **Could not verify** and chose round-trip-faithful encodings (hyprconf
  conventions, clearly documented): bind dispatcher passed as a single
  `"dispatcher args"` string (the stub types it `HL.Dispatcher|function`);
  `mouse`/`submap` carried as bind opts (Hyprland uses `bindm` and
  `define_submap` closures); window/layer `match` kept as a raw matcher string
  (rather than the official `match` key→value table) to preserve arbitrary
  matchers exactly; monitor trailing modifiers in an `extra` array; exec kind in
  a `{ when = ... }` table. These are symmetric (read↔write) so the Config
  round-trips; a later step can map them onto stricter `hl` shapes where safe.
- The conf-only `submap` marker list does not survive to Lua (Lua models
  submaps as closures); the per-bind `submap` association does. Tests compare
  accordingly.

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

[Unreleased]: https://github.com/hyprconf/hyprconf/compare/v0.9.0...HEAD
[0.9.0]: https://github.com/hyprconf/hyprconf/compare/v0.8.0...v0.9.0
[0.8.0]: https://github.com/hyprconf/hyprconf/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/hyprconf/hyprconf/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/hyprconf/hyprconf/compare/v0.5.1...v0.6.0
[0.5.1]: https://github.com/hyprconf/hyprconf/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/hyprconf/hyprconf/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/hyprconf/hyprconf/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/hyprconf/hyprconf/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/hyprconf/hyprconf/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/hyprconf/hyprconf/releases/tag/v0.1.0
