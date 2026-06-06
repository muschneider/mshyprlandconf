//! The embedded Hyprland option data, expressed as compact `const` builders.
//!
//! # Provenance
//!
//! - **Option *keys*** are taken verbatim from the vendored upstream stub
//!   `meta/hyprland-config-keys.txt` (Hyprland 0.55.2 `HL.ConfigKey`). The
//!   `schema::tests::every_option_path_exists_in_vendored_stub` test enforces
//!   that every path below maps (via `:` -> `.`) to a real stub key.
//! - **Types, defaults, ranges and descriptions** are taken from the
//!   [Hyprland wiki — Configuring/Variables](https://wiki.hyprland.org/Configuring/Variables/).
//!   Defaults drift between releases; treat them as best-effort and verify with
//!   `hyprctl getoption <name>` on the target version when accuracy is critical.
//! - **`since` hints** are intentionally left `None` for now: the stub carries
//!   no version metadata, so populating them reliably needs a separate wiki
//!   scrape (tracked for a later step).
//!
//! This is a curated, representative subset — broad enough to exercise every
//! section and value kind — not yet the full 341-key surface. Extending it is
//! purely additive: append rows here and the cross-check test keeps them honest.

use super::{
    CollectionId, CollectionSpec, EnumVariant, NumericRange, OptionSpec, Schema, Section, ValueType,
};
use crate::value::{Color, Gradient, Value, Vec2};

/// Build the full embedded schema.
pub(super) fn build() -> Schema {
    Schema {
        sections: vec![
            general(),
            decoration(),
            animations(),
            input(),
            gestures(),
            group(),
            misc(),
            binds(),
            dwindle(),
            master(),
            xwayland(),
            cursor(),
            render(),
            debug(),
        ],
        collections: collections(),
    }
}

// ---------------------------------------------------------------------------
// terse builders (this file is data, not logic)
// ---------------------------------------------------------------------------

fn sec(id: &str, label: &str, description: &str, options: Vec<OptionSpec>) -> Section {
    Section {
        id: id.to_string(),
        label: label.to_string(),
        description: description.to_string(),
        options,
    }
}

fn spec(
    path: &str,
    label: &str,
    description: &str,
    value_type: ValueType,
    default: Value,
) -> OptionSpec {
    OptionSpec {
        path: path.to_string(),
        label: label.to_string(),
        description: description.to_string(),
        value_type,
        default,
        range: None,
        since: None,
    }
}

fn b(path: &str, label: &str, description: &str, default: bool) -> OptionSpec {
    spec(
        path,
        label,
        description,
        ValueType::Bool,
        Value::Bool(default),
    )
}

fn i(path: &str, label: &str, description: &str, default: i64, range: NumericRange) -> OptionSpec {
    let mut o = spec(
        path,
        label,
        description,
        ValueType::Int,
        Value::Int(default),
    );
    o.range = Some(range);
    o
}

fn fl(path: &str, label: &str, description: &str, default: f64, range: NumericRange) -> OptionSpec {
    let mut o = spec(
        path,
        label,
        description,
        ValueType::Float,
        Value::Float(default),
    );
    o.range = Some(range);
    o
}

fn s(path: &str, label: &str, description: &str, default: &str) -> OptionSpec {
    spec(
        path,
        label,
        description,
        ValueType::String,
        Value::String(default.to_string()),
    )
}

fn c(path: &str, label: &str, description: &str, default: Color) -> OptionSpec {
    spec(
        path,
        label,
        description,
        ValueType::Color,
        Value::Color(default),
    )
}

fn g(path: &str, label: &str, description: &str, default: Gradient) -> OptionSpec {
    spec(
        path,
        label,
        description,
        ValueType::Gradient,
        Value::Gradient(default),
    )
}

fn v(path: &str, label: &str, description: &str, default: Vec2) -> OptionSpec {
    spec(
        path,
        label,
        description,
        ValueType::Vec2,
        Value::Vec2(default),
    )
}

fn e(
    path: &str,
    label: &str,
    description: &str,
    default: &str,
    variants: &[(&str, &str)],
) -> OptionSpec {
    let variants = variants
        .iter()
        .map(|(name, desc)| EnumVariant::described(*name, *desc))
        .collect();
    spec(
        path,
        label,
        description,
        ValueType::Enum(variants),
        Value::Enum(default.to_string()),
    )
}

/// `0xAARRGGBB` -> [`Color`].
const fn argb(v: u32) -> Color {
    Color::rgba(
        ((v >> 16) & 0xff) as u8,
        ((v >> 8) & 0xff) as u8,
        (v & 0xff) as u8,
        ((v >> 24) & 0xff) as u8,
    )
}

/// `0xRRGGBB` -> opaque [`Color`].
const fn rgb(v: u32) -> Color {
    Color::rgba(
        ((v >> 16) & 0xff) as u8,
        ((v >> 8) & 0xff) as u8,
        (v & 0xff) as u8,
        0xff,
    )
}

fn solid(color: Color) -> Gradient {
    Gradient::solid(color)
}

// ---------------------------------------------------------------------------
// sections
// ---------------------------------------------------------------------------

#[rustfmt::skip]
fn general() -> Section {
    sec(
        "general",
        "General",
        "Core layout, gaps and border behaviour.",
        vec![
            i("general:border_size", "Border size", "Window border thickness in px.", 1, NumericRange::at_least(0.0)),
            i("general:gaps_in", "Inner gaps", "Gaps between adjacent windows.", 5, NumericRange::at_least(0.0)),
            i("general:gaps_out", "Outer gaps", "Gaps between windows and screen edges.", 20, NumericRange::at_least(0.0)),
            i("general:gaps_workspaces", "Workspace gaps", "Gaps between workspaces while swiping.", 0, NumericRange::at_least(0.0)),
            g("general:col.active_border", "Active border color", "Border color of the focused window.", solid(argb(0xffffffff))),
            g("general:col.inactive_border", "Inactive border color", "Border color of unfocused windows.", solid(argb(0xff444444))),
            e("general:layout", "Layout", "The tiling layout engine.", "dwindle", &[
                ("dwindle", "BSP-style binary tiling."),
                ("master", "Master/stack tiling."),
            ]),
            b("general:resize_on_border", "Resize on border", "Enable dragging window borders to resize.", false),
            i("general:extend_border_grab_area", "Border grab area", "Extra px around borders that respond to drags.", 15, NumericRange::at_least(0.0)),
            b("general:hover_icon_on_border", "Hover icon on border", "Show a resize cursor when hovering a border.", true),
            b("general:allow_tearing", "Allow tearing", "Permit tearing for windows that request it.", false),
            b("general:no_focus_fallback", "No focus fallback", "Do not refocus the last window when the active one closes.", false),
            b("general:snap:enabled", "Snap enabled", "Snap floating windows to edges.", false).since("0.42.0"),
            i("general:snap:window_gap", "Snap window gap", "Distance at which floating windows snap to each other.", 10, NumericRange::at_least(0.0)).since("0.42.0"),
            i("general:snap:monitor_gap", "Snap monitor gap", "Distance at which floating windows snap to monitor edges.", 10, NumericRange::at_least(0.0)).since("0.42.0"),
            b("general:snap:border_overlap", "Snap border overlap", "Allow snapped borders to overlap.", false).since("0.42.0"),
        ],
    )
}

#[rustfmt::skip]
fn decoration() -> Section {
    sec(
        "decoration",
        "Decoration",
        "Rounding, opacity, blur and shadow.",
        vec![
            i("decoration:rounding", "Rounding", "Corner rounding radius in layout px.", 0, NumericRange::at_least(0.0)),
            fl("decoration:rounding_power", "Rounding power", "Squircle-ness of rounded corners.", 2.0, NumericRange::bounded(1.0, 10.0)).since("0.45.0"),
            fl("decoration:active_opacity", "Active opacity", "Opacity of the focused window.", 1.0, NumericRange::bounded(0.0, 1.0)),
            fl("decoration:inactive_opacity", "Inactive opacity", "Opacity of unfocused windows.", 1.0, NumericRange::bounded(0.0, 1.0)),
            fl("decoration:fullscreen_opacity", "Fullscreen opacity", "Opacity of fullscreen windows.", 1.0, NumericRange::bounded(0.0, 1.0)),
            b("decoration:dim_inactive", "Dim inactive", "Dim windows that are not focused.", false),
            fl("decoration:dim_strength", "Dim strength", "How much to dim inactive windows.", 0.5, NumericRange::bounded(0.0, 1.0)),
            b("decoration:border_part_of_window", "Border part of window", "Count the border as part of the window.", true),
            b("decoration:blur:enabled", "Blur enabled", "Enable background blur.", true),
            i("decoration:blur:size", "Blur size", "Blur kernel size.", 8, NumericRange::at_least(1.0)),
            i("decoration:blur:passes", "Blur passes", "Number of blur passes.", 1, NumericRange::at_least(1.0)),
            b("decoration:blur:new_optimizations", "Blur optimizations", "Enable blur performance optimizations.", true),
            b("decoration:blur:xray", "Blur xray", "Blur behind floating windows as if transparent.", false),
            fl("decoration:blur:noise", "Blur noise", "Noise added to the blur.", 0.0117, NumericRange::bounded(0.0, 1.0)),
            fl("decoration:blur:contrast", "Blur contrast", "Contrast of the blur.", 0.8916, NumericRange::bounded(0.0, 2.0)),
            fl("decoration:blur:brightness", "Blur brightness", "Brightness of the blur.", 0.8172, NumericRange::bounded(0.0, 2.0)),
            fl("decoration:blur:vibrancy", "Blur vibrancy", "Saturation boost of the blur.", 0.1696, NumericRange::bounded(0.0, 1.0)),
            fl("decoration:blur:vibrancy_darkness", "Blur vibrancy darkness", "Vibrancy effect on dark areas.", 0.0, NumericRange::bounded(0.0, 1.0)),
            b("decoration:blur:special", "Blur special", "Blur the special workspace background.", false),
            b("decoration:blur:popups", "Blur popups", "Blur popups (e.g. menus).", false),
            b("decoration:shadow:enabled", "Shadow enabled", "Enable drop shadows.", true),
            i("decoration:shadow:range", "Shadow range", "Shadow size/spread in px.", 4, NumericRange::at_least(0.0)),
            i("decoration:shadow:render_power", "Shadow render power", "Falloff steepness of the shadow.", 3, NumericRange::bounded(1.0, 4.0)),
            c("decoration:shadow:color", "Shadow color", "Drop shadow color.", argb(0xee1a1a1a)),
            v("decoration:shadow:offset", "Shadow offset", "Shadow offset as an x/y vector.", Vec2::new(0.0, 0.0)),
            fl("decoration:shadow:scale", "Shadow scale", "Shadow scale factor.", 1.0, NumericRange::bounded(0.0, 1.0)),
            b("decoration:shadow:sharp", "Shadow sharp", "Render sharp (non-blurred) shadows.", false),
        ],
    )
}

#[rustfmt::skip]
fn animations() -> Section {
    sec(
        "animations",
        "Animations",
        "Global animation toggles. Bezier curves and per-target animations are managed as collections.",
        vec![
            b("animations:enabled", "Enabled", "Master switch for all animations.", true),
            b("animations:workspace_wraparound", "Workspace wraparound", "Animate wrap-around when cycling workspaces.", false),
        ],
    )
}

#[rustfmt::skip]
fn input() -> Section {
    sec(
        "input",
        "Input",
        "Keyboard, mouse and touchpad behaviour.",
        vec![
            s("input:kb_layout", "Keyboard layout", "XKB layout(s), comma-separated.", "us"),
            s("input:kb_variant", "Keyboard variant", "XKB variant(s).", ""),
            s("input:kb_model", "Keyboard model", "XKB model.", ""),
            s("input:kb_options", "Keyboard options", "XKB options, comma-separated.", ""),
            s("input:kb_rules", "Keyboard rules", "XKB rules.", ""),
            i("input:follow_mouse", "Follow mouse", "Focus-follows-mouse mode (0-3).", 1, NumericRange::bounded(0.0, 3.0)),
            b("input:mouse_refocus", "Mouse refocus", "Refocus the window under the cursor on motion.", true),
            fl("input:sensitivity", "Sensitivity", "Pointer sensitivity (libinput, -1.0 to 1.0).", 0.0, NumericRange::bounded(-1.0, 1.0)),
            s("input:accel_profile", "Accel profile", "Pointer acceleration profile (adaptive/flat).", ""),
            b("input:natural_scroll", "Natural scroll", "Invert scroll direction.", false),
            b("input:numlock_by_default", "Numlock by default", "Enable numlock on startup.", false),
            i("input:repeat_rate", "Repeat rate", "Key repeat rate (per second).", 25, NumericRange::at_least(0.0)),
            i("input:repeat_delay", "Repeat delay", "Delay before key repeat begins (ms).", 600, NumericRange::at_least(0.0)),
            b("input:left_handed", "Left handed", "Swap left/right mouse buttons.", false),
            fl("input:scroll_factor", "Scroll factor", "Multiplier for scroll distance.", 1.0, NumericRange::at_least(0.0)),
            b("input:touchpad:natural_scroll", "Touchpad natural scroll", "Invert touchpad scroll direction.", false),
            b("input:touchpad:disable_while_typing", "Disable while typing", "Disable the touchpad while typing.", true),
            b("input:touchpad:tap_to_click", "Tap to click", "Treat a tap as a click.", true),
            fl("input:touchpad:scroll_factor", "Touchpad scroll factor", "Multiplier for touchpad scrolling.", 1.0, NumericRange::at_least(0.0)),
            b("input:touchpad:clickfinger_behavior", "Clickfinger behavior", "Use finger count for click button.", false),
            b("input:touchpad:middle_button_emulation", "Middle button emulation", "Emulate the middle button.", false),
            b("input:touchpad:tap_and_drag", "Tap and drag", "Enable tap-and-drag.", true),
            b("input:touchpad:drag_lock", "Drag lock", "Keep dragging after lifting during tap-and-drag.", false),
        ],
    )
}

#[rustfmt::skip]
fn gestures() -> Section {
    sec(
        "gestures",
        "Gestures",
        "Touchpad and touch workspace-swipe gestures.",
        vec![
            i("gestures:workspace_swipe_distance", "Swipe distance", "Distance (px) for a full workspace swipe.", 300, NumericRange::at_least(0.0)),
            b("gestures:workspace_swipe_invert", "Swipe invert", "Invert swipe direction.", true),
            fl("gestures:workspace_swipe_cancel_ratio", "Swipe cancel ratio", "Fraction below which a swipe is cancelled.", 0.5, NumericRange::bounded(0.0, 1.0)),
            i("gestures:workspace_swipe_min_speed_to_force", "Min speed to force", "Min speed that forces a workspace change.", 30, NumericRange::at_least(0.0)),
            b("gestures:workspace_swipe_direction_lock", "Direction lock", "Lock swipe to one direction.", true),
            b("gestures:workspace_swipe_create_new", "Create new", "Allow swiping to create a new workspace.", true),
            b("gestures:workspace_swipe_forever", "Swipe forever", "Keep swiping past the last workspace.", false),
            b("gestures:workspace_swipe_touch", "Touch swipe", "Enable workspace swipe via touchscreen.", false),
        ],
    )
}

#[rustfmt::skip]
fn group() -> Section {
    sec(
        "group",
        "Group",
        "Window grouping and the group bar.",
        vec![
            b("group:auto_group", "Auto group", "Automatically group new windows into the focused group.", true),
            b("group:insert_after_current", "Insert after current", "Insert grouped windows after the active one.", true),
            b("group:focus_removed_window", "Focus removed window", "Focus a window when it leaves a group.", true),
            i("group:drag_into_group", "Drag into group", "Allow dragging windows into groups (0-2).", 1, NumericRange::bounded(0.0, 2.0)),
            b("group:merge_groups_on_drag", "Merge on drag", "Merge groups when dragged onto each other.", true),
            g("group:col.border_active", "Active group border", "Border of the active group.", solid(argb(0x66ffff00))),
            g("group:col.border_inactive", "Inactive group border", "Border of inactive groups.", solid(argb(0x66777700))),
            g("group:col.border_locked_active", "Locked active border", "Border of the active locked group.", solid(argb(0x66ff5500))),
            g("group:col.border_locked_inactive", "Locked inactive border", "Border of inactive locked groups.", solid(argb(0x66775500))),
            b("group:groupbar:enabled", "Groupbar enabled", "Render the group bar.", true),
            i("group:groupbar:font_size", "Groupbar font size", "Group bar title font size.", 8, NumericRange::at_least(1.0)),
            i("group:groupbar:height", "Groupbar height", "Group bar height in px.", 14, NumericRange::at_least(1.0)),
            b("group:groupbar:render_titles", "Render titles", "Show window titles in the group bar.", true),
            g("group:groupbar:col.active", "Groupbar active color", "Active tab color in the group bar.", solid(argb(0x66ffff00))),
            g("group:groupbar:col.inactive", "Groupbar inactive color", "Inactive tab color in the group bar.", solid(argb(0x66777700))),
        ],
    )
}

#[rustfmt::skip]
fn misc() -> Section {
    sec(
        "misc",
        "Misc",
        "Miscellaneous behaviour and cosmetics.",
        vec![
            b("misc:disable_hyprland_logo", "Disable logo", "Hide the Hyprland logo background.", false),
            b("misc:disable_splash_rendering", "Disable splash", "Hide the splash text.", false),
            i("misc:force_default_wallpaper", "Force default wallpaper", "Choose/force the default wallpaper (-1 to 2).", -1, NumericRange::bounded(-1.0, 2.0)),
            i("misc:vrr", "VRR", "Variable refresh rate mode (0-2).", 0, NumericRange::bounded(0.0, 2.0)),
            b("misc:mouse_move_enables_dpms", "Mouse wakes DPMS", "Wake displays on mouse movement.", false),
            b("misc:key_press_enables_dpms", "Key wakes DPMS", "Wake displays on key press.", false),
            b("misc:always_follow_on_dnd", "Follow on DnD", "Follow the cursor during drag-and-drop.", true),
            b("misc:layers_hog_keyboard_focus", "Layers hog focus", "Let layer surfaces keep keyboard focus.", true),
            b("misc:animate_manual_resizes", "Animate manual resizes", "Animate windows during manual resizes.", false),
            b("misc:animate_mouse_windowdragging", "Animate mouse dragging", "Animate windows during mouse dragging.", false),
            b("misc:focus_on_activate", "Focus on activate", "Focus windows that request activation.", false),
            c("misc:col.splash", "Splash color", "Color of the splash text.", argb(0xffffffff)),
            c("misc:background_color", "Background color", "Solid background color behind windows.", rgb(0x111111)),
            s("misc:font_family", "Font family", "Default font family for built-in text.", "Sans"),
            b("misc:enable_swallow", "Enable swallow", "Enable window swallowing.", false),
            b("misc:middle_click_paste", "Middle-click paste", "Enable primary-selection paste on middle click.", true),
            b("misc:close_special_on_empty", "Close empty special", "Auto-close the special workspace when empty.", true),
        ],
    )
}

#[rustfmt::skip]
fn binds() -> Section {
    sec(
        "binds",
        "Binds",
        "Behaviour of keybinds and dispatchers.",
        vec![
            b("binds:workspace_back_and_forth", "Back and forth", "Toggle between current and previous workspace.", false),
            b("binds:allow_workspace_cycles", "Allow cycles", "Allow workspace cycling with back-and-forth.", false),
            b("binds:pass_mouse_when_bound", "Pass mouse when bound", "Pass mouse events even when bound.", false),
            i("binds:scroll_event_delay", "Scroll event delay", "Debounce for scroll-triggered binds (ms).", 300, NumericRange::at_least(0.0)),
            i("binds:focus_preferred_method", "Focus method", "Preferred directional focus method (0-1).", 0, NumericRange::bounded(0.0, 1.0)),
            i("binds:workspace_center_on", "Workspace center on", "Where to center when switching workspaces (0-1).", 0, NumericRange::bounded(0.0, 1.0)),
            b("binds:movefocus_cycles_fullscreen", "Movefocus cycles fullscreen", "Cycle fullscreen windows with movefocus.", false),
            b("binds:disable_keybind_grabbing", "Disable keybind grabbing", "Disable global keybind grabbing.", false),
            i("binds:drag_threshold", "Drag threshold", "Pixels of motion before a drag begins (0 = instant).", 0, NumericRange::at_least(0.0)),
            b("binds:allow_pin_fullscreen", "Allow pin fullscreen", "Keep pinned windows visible in fullscreen.", false),
        ],
    )
}

#[rustfmt::skip]
fn dwindle() -> Section {
    sec(
        "dwindle",
        "Dwindle layout",
        "Options for the dwindle (BSP) layout.",
        vec![
            b("dwindle:preserve_split", "Preserve split", "Keep the split orientation when windows close.", false),
            b("dwindle:smart_split", "Smart split", "Choose split direction from cursor position.", false),
            b("dwindle:smart_resizing", "Smart resizing", "Resize relative to cursor quadrant.", true),
            i("dwindle:force_split", "Force split", "Force new windows to a side (0-2).", 0, NumericRange::bounded(0.0, 2.0)),
            fl("dwindle:default_split_ratio", "Default split ratio", "Initial split ratio for new windows.", 1.0, NumericRange::bounded(0.1, 1.9)),
            fl("dwindle:split_width_multiplier", "Split width multiplier", "Bias toward horizontal/vertical splits.", 1.0, NumericRange::at_least(0.0)),
            b("dwindle:use_active_for_splits", "Use active for splits", "Split based on the active window.", true),
            fl("dwindle:special_scale_factor", "Special scale factor", "Scale of the special workspace.", 1.0, NumericRange::bounded(0.0, 1.0)),
        ],
    )
}

#[rustfmt::skip]
fn master() -> Section {
    sec(
        "master",
        "Master layout",
        "Options for the master/stack layout.",
        vec![
            e("master:new_status", "New window status", "Role assigned to new windows.", "slave", &[
                ("master", "Become a new master."),
                ("slave", "Join the stack."),
                ("inherit", "Inherit from the focused window."),
            ]),
            b("master:new_on_top", "New on top", "Add new windows at the top of the stack.", false),
            e("master:new_on_active", "New on active", "Placement of new windows relative to the active one.", "none", &[
                ("before", "Before the active window."),
                ("after", "After the active window."),
                ("none", "Default placement."),
            ]),
            fl("master:mfact", "Master factor", "Fraction of the screen used by the master area.", 0.55, NumericRange::bounded(0.0, 1.0)),
            e("master:orientation", "Orientation", "Position of the master area.", "left", &[
                ("left", "Master on the left."),
                ("right", "Master on the right."),
                ("top", "Master on top."),
                ("bottom", "Master on the bottom."),
                ("center", "Master centered."),
            ]),
            fl("master:special_scale_factor", "Special scale factor", "Scale of the special workspace.", 1.0, NumericRange::bounded(0.0, 1.0)),
            b("master:smart_resizing", "Smart resizing", "Resize relative to cursor quadrant.", true),
            b("master:allow_small_split", "Allow small split", "Allow splitting the master into multiple windows.", false),
        ],
    )
}

#[rustfmt::skip]
fn xwayland() -> Section {
    sec(
        "xwayland",
        "XWayland",
        "X11 compatibility layer.",
        vec![
            b("xwayland:enabled", "Enabled", "Enable XWayland.", true),
            b("xwayland:use_nearest_neighbor", "Nearest neighbor", "Use nearest-neighbor scaling for X11 windows.", true),
            b("xwayland:force_zero_scaling", "Force zero scaling", "Force scale 1 for X11 windows.", false),
            b("xwayland:create_abstract_socket", "Abstract socket", "Create an abstract X11 socket.", false),
        ],
    )
}

#[rustfmt::skip]
fn cursor() -> Section {
    sec(
        "cursor",
        "Cursor",
        "Cursor appearance and behaviour.",
        vec![
            fl("cursor:inactive_timeout", "Inactive timeout", "Seconds before the cursor hides when idle (0 = never).", 0.0, NumericRange::at_least(0.0)),
            i("cursor:no_hardware_cursors", "No hardware cursors", "Disable hardware cursors (0/1/2 = off/on/auto).", 2, NumericRange::bounded(0.0, 2.0)),
            b("cursor:enable_hyprcursor", "Enable hyprcursor", "Use the hyprcursor format.", true),
            b("cursor:hide_on_key_press", "Hide on key press", "Hide the cursor when a key is pressed.", false),
            b("cursor:hide_on_touch", "Hide on touch", "Hide the cursor on touchscreen input.", true),
            b("cursor:invisible", "Invisible", "Make the cursor invisible.", false),
            fl("cursor:zoom_factor", "Zoom factor", "Cursor-centered zoom factor.", 1.0, NumericRange::at_least(1.0)),
            s("cursor:default_monitor", "Default monitor", "Monitor the cursor starts on.", ""),
            i("cursor:hotspot_padding", "Hotspot padding", "Padding before the cursor leaves an edge.", 0, NumericRange::at_least(0.0)),
        ],
    )
}

#[rustfmt::skip]
fn render() -> Section {
    sec(
        "render",
        "Render",
        "Low-level rendering options.",
        vec![
            i("render:direct_scanout", "Direct scanout", "Direct scanout mode (0/1/2).", 0, NumericRange::bounded(0.0, 2.0)),
            b("render:expand_undersized_textures", "Expand undersized textures", "Expand textures smaller than their window.", true),
            i("render:ctm_animation", "CTM animation", "Animate color-transform-matrix changes (0/1/2).", 2, NumericRange::bounded(0.0, 2.0)),
            b("render:new_render_scheduling", "New render scheduling", "Use the newer adaptive render scheduler.", false),
        ],
    )
}

#[rustfmt::skip]
fn debug() -> Section {
    sec(
        "debug",
        "Debug",
        "Diagnostics and logging. Usually left at defaults.",
        vec![
            b("debug:overlay", "Overlay", "Show the FPS/debug overlay.", false),
            b("debug:damage_blink", "Damage blink", "Flash damaged regions (epilepsy warning).", false),
            b("debug:disable_logs", "Disable logs", "Disable logging.", true),
            b("debug:disable_time", "Disable time", "Omit timestamps from logs.", true),
            b("debug:enable_stdout_logs", "Stdout logs", "Also log to stdout.", false),
            i("debug:damage_tracking", "Damage tracking", "Damage tracking mode (0/1/2).", 2, NumericRange::bounded(0.0, 2.0)),
            b("debug:disable_scale_checks", "Disable scale checks", "Skip fractional-scale sanity checks.", false),
            b("debug:vfr", "VFR", "Variable frame rate (render only on changes).", true),
        ],
    )
}

// ---------------------------------------------------------------------------
// structured collections
// ---------------------------------------------------------------------------

fn collection(
    id: CollectionId,
    label: &str,
    description: &str,
    element_type: ValueType,
    keywords: &[&str],
) -> CollectionSpec {
    CollectionSpec {
        id,
        label: label.to_string(),
        description: description.to_string(),
        element_type,
        keywords: keywords.iter().map(|k| (*k).to_string()).collect(),
        since: None,
    }
}

#[rustfmt::skip]
fn collections() -> Vec<CollectionSpec> {
    vec![
        collection(CollectionId::Monitors, "Monitors", "Per-output resolution, position, scale and transforms.", ValueType::MonitorRule, &["monitor"]),
        collection(CollectionId::Workspaces, "Workspaces", "Persistent/per-monitor workspace rules.", ValueType::Workspace, &["workspace"]),
        collection(CollectionId::WindowRules, "Window rules", "Per-window behaviour and appearance rules.", ValueType::WindowRule, &["windowrule", "windowrulev2"]),
        collection(CollectionId::LayerRules, "Layer rules", "Rules for layer-shell surfaces (bars, notifications).", ValueType::LayerRule, &["layerrule"]),
        collection(
            CollectionId::Keybinds,
            "Keybinds",
            "Key and mouse bindings with their flag variants.",
            ValueType::Keybind,
            &["bind", "bindm", "binde", "bindr", "bindl", "bindel", "bindn", "bindt", "bindi"],
        ),
        collection(CollectionId::Submaps, "Submaps", "Named bind scopes (modal keymaps).", ValueType::Submap, &["submap"]),
        collection(CollectionId::Env, "Environment", "Environment variables exported to the session.", ValueType::EnvVar, &["env", "envd"]),
        collection(CollectionId::Execs, "Exec", "Commands run on launch/reload/shutdown.", ValueType::Exec, &["exec", "exec-once", "exec-shutdown"]),
        collection(CollectionId::Variables, "Variables", "hyprlang `$variables` (textual macros).", ValueType::Variable, &[]),
        collection(CollectionId::Beziers, "Bezier curves", "Named bezier curves used by animations.", ValueType::Bezier, &["bezier"]),
        collection(CollectionId::Animations, "Animation rules", "Per-target animation settings.", ValueType::Animation, &["animation"]),
    ]
}
