//! Structured, repeatable configuration constructs.
//!
//! Unlike scalar [`crate::value::Value`]s — which are keyed by a single dotted
//! path — these directives are *ordered collections*: a config has a list of
//! keybinds, a list of window rules, a list of monitors, and so on. Their order
//! is semantically meaningful (rules are applied top-to-bottom, variables must
//! be defined before use), so the [`crate::model::Config`] stores them in
//! `Vec`s rather than a map.
//!
//! In this step these types only need to *exist* and be constructible; the
//! parsers and serializers that populate them arrive in later steps.

use crate::value::Vec2;

/// The flag letters that may be appended to a `bind` keyword (`bindel`, `bindm`,
/// ...). Each corresponds to a documented Hyprland bind modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KeybindFlags {
    /// `l` — works while the session is locked.
    pub locked: bool,
    /// `r` — fires on key release instead of press.
    pub release: bool,
    /// `e` — repeats while held.
    pub repeat: bool,
    /// `n` — non-consuming (event passes through to the focused client).
    pub non_consuming: bool,
    /// `m` — mouse bind (`bindm`).
    pub mouse: bool,
    /// `t` — transparent (does not block other binds).
    pub transparent: bool,
    /// `i` — ignores modifier state.
    pub ignore_mods: bool,
}

impl KeybindFlags {
    /// The canonical `bind` keyword for these flags (e.g. `binde`, `bindml`).
    ///
    /// Flag letters are emitted in Hyprland's conventional order so output is
    /// deterministic.
    #[must_use]
    pub fn keyword(&self) -> String {
        let mut kw = String::from("bind");
        for (enabled, letter) in [
            (self.mouse, 'm'),
            (self.locked, 'l'),
            (self.release, 'r'),
            (self.repeat, 'e'),
            (self.non_consuming, 'n'),
            (self.ignore_mods, 'i'),
            (self.transparent, 't'),
        ] {
            if enabled {
                kw.push(letter);
            }
        }
        kw
    }
}

/// A single key/mouse binding: `bind<flags> = MODS, KEY, DISPATCHER, ARGS`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keybind {
    /// Which `bind*` keyword variant produced this entry.
    pub flags: KeybindFlags,
    /// The raw modifier expression, e.g. `SUPER SHIFT` or `$mainMod`.
    pub mods: String,
    /// The key or button, e.g. `Q`, `code:24`, `mouse:272`.
    pub key: String,
    /// The dispatcher name, e.g. `exec`, `killactive`, `movefocus`.
    pub dispatcher: String,
    /// Dispatcher arguments (may be empty).
    pub args: String,
    /// The submap this bind belongs to (`None` = the global/default submap).
    pub submap: Option<String>,
}

/// A window rule: legacy `windowrule = RULE, REGEX` or `windowrulev2 = RULE, MATCHERS`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowRule {
    /// `true` for `windowrulev2`, `false` for legacy `windowrule`.
    pub v2: bool,
    /// The rule body, e.g. `float`, `opacity 0.9`, `workspace 2 silent`.
    pub rule: String,
    /// The matcher text: a window regex (v1) or `key:value` matchers (v2).
    pub matchers: String,
}

/// A layer-surface rule: `layerrule = RULE, NAMESPACE`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerRule {
    /// The rule body, e.g. `blur`, `ignorezero`.
    pub rule: String,
    /// The target layer namespace, e.g. `waybar`, `^(notifications)$`.
    pub namespace: String,
}

/// A monitor directive: `monitor = NAME, MODE, POSITION, SCALE[, extra...]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorRule {
    /// Connector name or selector (`DP-1`, `desc:...`, `` (empty) or `*`).
    pub name: String,
    /// Resolution/refresh, e.g. `1920x1080@144`, `preferred`, `highres`, `disable`.
    pub mode: String,
    /// Position, e.g. `0x0`, `auto`, `auto-right`.
    pub position: String,
    /// Scale factor, e.g. `1`, `1.5`, `auto`.
    pub scale: String,
    /// Trailing modifiers (`transform`, `mirror`, `bitdepth`, `vrr`, ...).
    pub extra: Vec<String>,
}

/// A workspace rule: `workspace = SELECTOR, RULES`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRule {
    /// The workspace selector, e.g. `1`, `name:web`, `special:magic`.
    pub selector: String,
    /// The comma-separated rule list, e.g. `monitor:DP-1, default:true`.
    pub rules: String,
}

/// An environment variable directive: `env = NAME, VALUE`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvVar {
    /// The variable name.
    pub name: String,
    /// The variable value.
    pub value: String,
}

/// Which flavour of `exec` directive a command uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExecKind {
    /// `exec` — run on every config (re)load.
    #[default]
    Exec,
    /// `exec-once` — run once, at startup.
    ExecOnce,
    /// `exec-shutdown` — run when Hyprland exits.
    ExecShutdown,
}

/// An exec directive: `exec` / `exec-once` / `exec-shutdown = COMMAND`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exec {
    /// Which exec flavour this is.
    pub kind: ExecKind,
    /// The shell command to run.
    pub command: String,
}

/// A bezier curve definition: `bezier = NAME, X0, Y0, X1, Y1`.
#[derive(Debug, Clone, PartialEq)]
pub struct Bezier {
    /// The curve's name (referenced by [`Animation::curve`]).
    pub name: String,
    /// First control point.
    pub p0: Vec2,
    /// Second control point.
    pub p1: Vec2,
}

/// An animation directive: `animation = NAME, ONOFF, SPEED, CURVE[, STYLE]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Animation {
    /// The animation target name, e.g. `windows`, `workspaces`, `global`.
    pub name: String,
    /// Whether the animation is enabled (`ONOFF`).
    pub enabled: bool,
    /// Speed in deciseconds.
    pub speed: f64,
    /// The bezier curve name to use.
    pub curve: String,
    /// Optional style argument (e.g. `slide`, `popin 80%`).
    pub style: Option<String>,
}

/// A submap marker: `submap = NAME` (or `submap = reset`).
///
/// Binds that follow a submap declaration belong to it until the next
/// `submap = reset`; that association is reconstructed by the parser and is
/// also recorded on [`Keybind::submap`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Submap {
    /// The submap name (`reset` is represented by the literal name `reset`).
    pub name: String,
}

/// A hyprlang variable definition: `$NAME = VALUE`.
///
/// Variables are textual macros expanded before evaluation. They have no
/// dedicated value type; the value is stored verbatim for faithful round-trips.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variable {
    /// The variable name *without* the leading `$`.
    pub name: String,
    /// The raw replacement text.
    pub value: String,
}

/// A uniform wrapper over every structured construct.
///
/// The typed [`crate::model::Config`] collections are preferred for editing;
/// this enum exists for code that needs to handle any structured item
/// generically (e.g. a future generic serializer dispatch).
#[derive(Debug, Clone, PartialEq)]
pub enum StructuredValue {
    /// See [`Keybind`].
    Keybind(Keybind),
    /// See [`WindowRule`].
    WindowRule(WindowRule),
    /// See [`LayerRule`].
    LayerRule(LayerRule),
    /// See [`MonitorRule`].
    MonitorRule(MonitorRule),
    /// See [`WorkspaceRule`].
    Workspace(WorkspaceRule),
    /// See [`EnvVar`].
    EnvVar(EnvVar),
    /// See [`Exec`].
    Exec(Exec),
    /// See [`Bezier`].
    Bezier(Bezier),
    /// See [`Animation`].
    Animation(Animation),
    /// See [`Submap`].
    Submap(Submap),
    /// See [`Variable`].
    Variable(Variable),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keybind_keyword_is_plain_for_no_flags() {
        assert_eq!(KeybindFlags::default().keyword(), "bind");
    }

    #[test]
    fn keybind_keyword_combines_flags_in_order() {
        let flags = KeybindFlags {
            repeat: true,
            locked: true,
            ..Default::default()
        };
        assert_eq!(flags.keyword(), "bindle");

        let mouse = KeybindFlags {
            mouse: true,
            ..Default::default()
        };
        assert_eq!(mouse.keyword(), "bindm");
    }

    #[test]
    fn exec_kind_defaults_to_exec() {
        assert_eq!(ExecKind::default(), ExecKind::Exec);
    }

    #[test]
    fn structured_value_is_constructible() {
        let v = StructuredValue::EnvVar(EnvVar {
            name: "XCURSOR_SIZE".into(),
            value: "24".into(),
        });
        assert!(matches!(v, StructuredValue::EnvVar(_)));
    }
}
