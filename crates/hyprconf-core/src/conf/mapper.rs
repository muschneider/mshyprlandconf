//! Builds the semantic [`Config`] from a parsed `.conf` document/bundle.
//!
//! This is where the lossless document is interpreted: `$variables` are
//! expanded, scalar assignments are mapped onto the [`Schema`]'s [`OptionSpec`]s
//! and parsed into typed [`Value`]s, and the repeatable directives are turned
//! into the structured collections. Includes are followed in evaluation order
//! so later values win, exactly as hyprlang would evaluate them.
//!
//! Nothing is ever dropped: unknown keys and unparseable values are recorded as
//! [`ConfWarning`]s and still stored (unknown scalars as `Value::String`), while
//! the lossless document remains the source of truth for serialization.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::conf::document::{Assignment, ConfDocument, Directive, LineKind};
use crate::conf::parser::ConfBundle;
use crate::model::{Config, ConfigFormat, Provenance, Tracked};
use crate::schema::{Schema, ValueType};
use crate::structured::{
    Animation, Bezier, EnvVar, Exec, ExecKind, Keybind, KeybindFlags, LayerRule, MonitorRule,
    Submap, Variable, WindowRule, WorkspaceRule,
};
use crate::value::{Color, Gradient, Value, Vec2};

/// A non-fatal issue encountered while mapping a document onto the schema.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ConfWarning {
    /// An assignment used a key the schema does not know about. It is kept
    /// verbatim in the document and stored as a `String` value.
    #[error("unknown option `{path}`{}", at(.file, *.line))]
    UnknownOption {
        /// The unknown dotted path.
        path: String,
        /// The source file, if known.
        file: Option<PathBuf>,
        /// The 1-based line number, if known.
        line: Option<u32>,
    },

    /// A known option's value could not be parsed into its declared type.
    #[error("could not parse value {value:?} for `{path}`: {reason}{}", at(.file, *.line))]
    UnparsableValue {
        /// The option path.
        path: String,
        /// The offending value text.
        value: String,
        /// Why parsing failed.
        reason: String,
        /// The source file, if known.
        file: Option<PathBuf>,
        /// The 1-based line number, if known.
        line: Option<u32>,
    },

    /// A directive's arguments could not be parsed.
    #[error("could not parse {keyword} directive {args:?}: {reason}{}", at(.file, *.line))]
    UnparsableDirective {
        /// The directive keyword.
        keyword: String,
        /// The raw argument text.
        args: String,
        /// Why parsing failed.
        reason: String,
        /// The source file, if known.
        file: Option<PathBuf>,
        /// The 1-based line number, if known.
        line: Option<u32>,
    },
}

fn at(file: &Option<PathBuf>, line: Option<u32>) -> String {
    match (file, line) {
        (Some(p), Some(l)) => format!(" ({}:{l})", p.display()),
        (Some(p), None) => format!(" ({})", p.display()),
        (None, Some(l)) => format!(" (line {l})"),
        (None, None) => String::new(),
    }
}

/// Build a [`Config`] from a single document (does not follow includes).
#[must_use]
pub fn document_to_config(document: &ConfDocument, schema: &Schema) -> (Config, Vec<ConfWarning>) {
    let docs = std::slice::from_ref(document);
    let mut ctx = Ctx::new(schema, docs);
    ctx.config.source = document.path.clone();
    ctx.eval(0, &mut Vec::new());
    (ctx.config, ctx.warnings)
}

/// Build a [`Config`] from a full bundle, following includes in evaluation
/// order (so later assignments override earlier ones).
#[must_use]
pub fn bundle_to_config(bundle: &ConfBundle, schema: &Schema) -> (Config, Vec<ConfWarning>) {
    let mut ctx = Ctx::new(schema, &bundle.documents);
    ctx.config.source = bundle.root().path.clone();
    ctx.eval(bundle.root, &mut Vec::new());
    (ctx.config, ctx.warnings)
}

struct Ctx<'a> {
    schema: &'a Schema,
    docs: &'a [ConfDocument],
    config: Config,
    vars: HashMap<String, String>,
    submap: Option<String>,
    warnings: Vec<ConfWarning>,
}

impl<'a> Ctx<'a> {
    fn new(schema: &'a Schema, docs: &'a [ConfDocument]) -> Self {
        let mut config = Config::empty();
        config.format = Some(ConfigFormat::Conf);
        Self {
            schema,
            docs,
            config,
            vars: HashMap::new(),
            submap: None,
            warnings: Vec::new(),
        }
    }

    fn eval(&mut self, doc_index: usize, visiting: &mut Vec<usize>) {
        if visiting.contains(&doc_index) {
            return; // defensive: parse_file already rejects real cycles
        }
        visiting.push(doc_index);

        let doc = &self.docs[doc_index];
        let source = doc.path.clone();
        let mut pending_comments: Vec<String> = Vec::new();

        for (idx, line) in doc.lines.iter().enumerate() {
            let line_no = (idx + 1) as u32;
            match &line.kind {
                LineKind::Comment => pending_comments.push(line.raw.clone()),
                LineKind::Blank => pending_comments.clear(),
                LineKind::Assignment(a) => {
                    let prov =
                        self.provenance(&source, line_no, &pending_comments, a.trailing.as_str());
                    self.record_assignment(a, prov);
                    pending_comments.clear();
                }
                LineKind::Directive(d) => {
                    let prov =
                        self.provenance(&source, line_no, &pending_comments, d.trailing.as_str());
                    self.record_directive(d, prov, &source, line_no);
                    pending_comments.clear();
                }
                LineKind::Source(s) => {
                    pending_comments.clear();
                    let children = s.resolved.clone();
                    for child in children {
                        self.eval(child, visiting);
                    }
                }
                LineKind::SectionOpen { .. } | LineKind::SectionClose | LineKind::Unknown => {
                    pending_comments.clear();
                }
            }
        }

        visiting.pop();
    }

    fn provenance(
        &self,
        source: &Option<PathBuf>,
        line: u32,
        leading: &[String],
        trailing: &str,
    ) -> Provenance {
        let trailing_comment = trailing.find('#').map(|i| trailing[i..].to_string());
        Provenance {
            source: source.clone(),
            span: None,
            line: Some(line),
            leading_comments: leading.to_vec(),
            trailing_comment,
        }
    }

    fn record_assignment(&mut self, a: &Assignment, prov: Provenance) {
        let expanded = unescape_hash(&expand_vars(&a.value, &self.vars));

        if a.is_variable {
            let name = a.key.trim_start_matches('$').to_string();
            self.vars.insert(name.clone(), expanded.clone());
            self.config.variables.push(Tracked::with_provenance(
                Variable {
                    name,
                    value: expanded,
                },
                prov,
            ));
            return;
        }

        let path = a.full_path.clone();
        match self.schema.option(&path) {
            Some(spec) => match parse_value(&spec.value_type, &expanded) {
                Ok(value) => {
                    self.config
                        .options
                        .insert(path, Tracked::with_provenance(value, prov));
                }
                Err(reason) => {
                    self.warnings.push(ConfWarning::UnparsableValue {
                        path: path.clone(),
                        value: expanded.clone(),
                        reason,
                        file: prov.source.clone(),
                        line: prov.line,
                    });
                    // Keep the value so nothing is lost semantically.
                    self.config.options.insert(
                        path,
                        Tracked::with_provenance(Value::String(expanded), prov),
                    );
                }
            },
            None => {
                self.warnings.push(ConfWarning::UnknownOption {
                    path: path.clone(),
                    file: prov.source.clone(),
                    line: prov.line,
                });
                self.config.options.insert(
                    path,
                    Tracked::with_provenance(Value::String(expanded), prov),
                );
            }
        }
    }

    fn record_directive(
        &mut self,
        d: &Directive,
        prov: Provenance,
        source: &Option<PathBuf>,
        line: u32,
    ) {
        let args = unescape_hash(&expand_vars(&d.args, &self.vars));
        let keyword = d.keyword.as_str();

        match keyword {
            "monitor" => {
                let f = split_commas(&args);
                self.config.monitors.push(Tracked::with_provenance(
                    MonitorRule {
                        name: nth(&f, 0),
                        mode: nth(&f, 1),
                        position: nth(&f, 2),
                        scale: nth(&f, 3),
                        extra: f.iter().skip(4).map(|s| s.trim().to_string()).collect(),
                    },
                    prov,
                ));
            }
            "workspace" => {
                let (selector, rules) = split_once_trim(&args, ',');
                self.config.workspaces.push(Tracked::with_provenance(
                    WorkspaceRule { selector, rules },
                    prov,
                ));
            }
            "windowrule" | "windowrulev2" => {
                let (rule, matchers) = split_once_trim(&args, ',');
                self.config.window_rules.push(Tracked::with_provenance(
                    WindowRule {
                        v2: keyword == "windowrulev2",
                        rule,
                        matchers,
                    },
                    prov,
                ));
            }
            "layerrule" => {
                let (rule, namespace) = split_once_trim(&args, ',');
                self.config.layer_rules.push(Tracked::with_provenance(
                    LayerRule { rule, namespace },
                    prov,
                ));
            }
            "env" | "envd" => {
                let (name, value) = split_once_trim(&args, ',');
                self.config
                    .env
                    .push(Tracked::with_provenance(EnvVar { name, value }, prov));
            }
            "exec" | "exec-once" | "exec-shutdown" | "execr" | "exec-once-r" => {
                let kind = match keyword {
                    "exec-once" | "exec-once-r" => ExecKind::ExecOnce,
                    "exec-shutdown" => ExecKind::ExecShutdown,
                    _ => ExecKind::Exec,
                };
                self.config.execs.push(Tracked::with_provenance(
                    Exec {
                        kind,
                        command: args,
                    },
                    prov,
                ));
            }
            "submap" => {
                if args == "reset" {
                    self.submap = None;
                } else {
                    self.submap = Some(args.clone());
                }
                self.config
                    .submaps
                    .push(Tracked::with_provenance(Submap { name: args }, prov));
            }
            "bezier" => match parse_bezier(&args) {
                Ok(bezier) => self
                    .config
                    .beziers
                    .push(Tracked::with_provenance(bezier, prov)),
                Err(reason) => self.warnings.push(ConfWarning::UnparsableDirective {
                    keyword: keyword.to_string(),
                    args,
                    reason,
                    file: source.clone(),
                    line: Some(line),
                }),
            },
            "animation" => match parse_animation(&args) {
                Ok(animation) => self
                    .config
                    .animations
                    .push(Tracked::with_provenance(animation, prov)),
                Err(reason) => self.warnings.push(ConfWarning::UnparsableDirective {
                    keyword: keyword.to_string(),
                    args,
                    reason,
                    file: source.clone(),
                    line: Some(line),
                }),
            },
            _ if is_bind(keyword) => {
                let f = splitn_commas(&args, 4);
                self.config.keybinds.push(Tracked::with_provenance(
                    Keybind {
                        flags: parse_bind_flags(keyword),
                        mods: nth(&f, 0),
                        key: nth(&f, 1),
                        dispatcher: nth(&f, 2),
                        args: nth(&f, 3),
                        submap: self.submap.clone(),
                    },
                    prov,
                ));
            }
            _ => {
                // Keywords we recognise as directives but do not yet model
                // (e.g. `plugin`, `permission`) are preserved in the document
                // and surfaced as a warning rather than dropped.
                self.warnings.push(ConfWarning::UnparsableDirective {
                    keyword: keyword.to_string(),
                    args,
                    reason: "directive not modelled".to_string(),
                    file: source.clone(),
                    line: Some(line),
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// value / directive parsing helpers
// ---------------------------------------------------------------------------

fn parse_value(value_type: &ValueType, text: &str) -> Result<Value, String> {
    match value_type {
        ValueType::Bool => parse_bool(text)
            .map(Value::Bool)
            .ok_or_else(|| "expected a boolean".to_string()),
        ValueType::Int => text
            .trim()
            .parse::<i64>()
            .map(Value::Int)
            .map_err(|e| e.to_string()),
        ValueType::Float => text
            .trim()
            .parse::<f64>()
            .map(Value::Float)
            .map_err(|e| e.to_string()),
        ValueType::Color => Color::from_hyprland_str(text)
            .map(Value::Color)
            .map_err(|e| e.to_string()),
        ValueType::Gradient => Gradient::from_hyprland_str(text)
            .map(Value::Gradient)
            .map_err(|e| e.to_string()),
        ValueType::String => Ok(Value::String(text.to_string())),
        ValueType::Enum(variants) => {
            if variants.iter().any(|v| v.name == text) {
                Ok(Value::Enum(text.to_string()))
            } else {
                Err(format!("`{text}` is not a valid variant"))
            }
        }
        ValueType::Vec2 => Vec2::from_hyprland_str(text)
            .map(Value::Vec2)
            .map_err(|e| e.to_string()),
        _ => Err("option has a non-scalar value type".to_string()),
    }
}

fn parse_bool(text: &str) -> Option<bool> {
    match text.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" | "1" => Some(true),
        "false" | "no" | "off" | "0" => Some(false),
        _ => None,
    }
}

fn parse_bezier(args: &str) -> Result<Bezier, String> {
    let f = split_commas(args);
    if f.len() < 5 {
        return Err("expected NAME, x0, y0, x1, y1".to_string());
    }
    let num = |s: &str| s.trim().parse::<f64>().map_err(|e| e.to_string());
    Ok(Bezier {
        name: nth(&f, 0),
        p0: Vec2::new(num(&f[1])?, num(&f[2])?),
        p1: Vec2::new(num(&f[3])?, num(&f[4])?),
    })
}

fn parse_animation(args: &str) -> Result<Animation, String> {
    let f = split_commas(args);
    if f.len() < 4 {
        return Err("expected NAME, ONOFF, SPEED, CURVE[, STYLE]".to_string());
    }
    let enabled = parse_bool(&f[1]).unwrap_or(false);
    let speed = f[2].trim().parse::<f64>().map_err(|e| e.to_string())?;
    let style = if f.len() > 4 {
        let joined = f[4..].join(",").trim().to_string();
        (!joined.is_empty()).then_some(joined)
    } else {
        None
    };
    Ok(Animation {
        name: nth(&f, 0),
        enabled,
        speed,
        curve: nth(&f, 3),
        style,
    })
}

fn is_bind(keyword: &str) -> bool {
    keyword == "bind"
        || keyword.strip_prefix("bind").is_some_and(|flags| {
            !flags.is_empty() && flags.bytes().all(|b| b"lrenmtiopdcg".contains(&b))
        })
}

fn parse_bind_flags(keyword: &str) -> KeybindFlags {
    let flags = keyword.strip_prefix("bind").unwrap_or("");
    KeybindFlags {
        locked: flags.contains('l'),
        release: flags.contains('r'),
        repeat: flags.contains('e'),
        non_consuming: flags.contains('n'),
        mouse: flags.contains('m'),
        transparent: flags.contains('t'),
        ignore_mods: flags.contains('i'),
    }
}

/// Expand `$name` / `${name}` references using `vars`; unknown references are
/// left verbatim (e.g. environment variables we don't resolve here).
fn expand_vars(text: &str, vars: &HashMap<String, String>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(pos) = rest.find('$') {
        out.push_str(&rest[..pos]);
        let after = &rest[pos + 1..];

        if let Some(stripped) = after.strip_prefix('{') {
            if let Some(end) = stripped.find('}') {
                let name = &stripped[..end];
                if let Some(value) = vars.get(name) {
                    out.push_str(value);
                    rest = &stripped[end + 1..];
                    continue;
                }
            }
            out.push('$');
            rest = after;
            continue;
        }

        let name_len = after
            .char_indices()
            .take_while(|(_, c)| c.is_ascii_alphanumeric() || *c == '_')
            .map(|(i, c)| i + c.len_utf8())
            .last()
            .unwrap_or(0);
        if name_len > 0 {
            if let Some(value) = vars.get(&after[..name_len]) {
                out.push_str(value);
                rest = &after[name_len..];
                continue;
            }
        }

        out.push('$');
        rest = after;
    }
    out.push_str(rest);
    out
}

/// Un-escape hyprlang's `##` -> `#`.
fn unescape_hash(text: &str) -> String {
    text.replace("##", "#")
}

fn split_commas(s: &str) -> Vec<String> {
    s.split(',').map(|p| p.trim().to_string()).collect()
}

fn splitn_commas(s: &str, n: usize) -> Vec<String> {
    s.splitn(n, ',').map(|p| p.trim().to_string()).collect()
}

fn split_once_trim(s: &str, sep: char) -> (String, String) {
    match s.split_once(sep) {
        Some((a, b)) => (a.trim().to_string(), b.trim().to_string()),
        None => (s.trim().to_string(), String::new()),
    }
}

fn nth(parts: &[String], i: usize) -> String {
    parts
        .get(i)
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_known_variables_only() {
        let mut vars = HashMap::new();
        vars.insert("mainMod".to_string(), "SUPER".to_string());
        assert_eq!(expand_vars("$mainMod, Q", &vars), "SUPER, Q");
        assert_eq!(expand_vars("${mainMod} SHIFT", &vars), "SUPER SHIFT");
        assert_eq!(expand_vars("$HOME/x", &vars), "$HOME/x");
        assert_eq!(expand_vars("no vars here", &vars), "no vars here");
    }

    #[test]
    fn unescape_hash_collapses_double() {
        assert_eq!(unescape_hash("a##b"), "a#b");
        assert_eq!(unescape_hash("plain"), "plain");
    }

    #[test]
    fn parse_bool_accepts_hyprlang_forms() {
        for t in ["true", "yes", "on", "1"] {
            assert_eq!(parse_bool(t), Some(true));
        }
        for f in ["false", "no", "off", "0"] {
            assert_eq!(parse_bool(f), Some(false));
        }
        assert_eq!(parse_bool("maybe"), None);
    }

    #[test]
    fn parses_bezier_and_animation() {
        let b = parse_bezier("myCurve, 0.05, 0.9, 0.1, 1.0").unwrap();
        assert_eq!(b.name, "myCurve");
        assert_eq!(b.p0, Vec2::new(0.05, 0.9));

        let a = parse_animation("windows, 1, 7, myCurve, slide").unwrap();
        assert!(a.enabled);
        assert_eq!(a.speed, 7.0);
        assert_eq!(a.curve, "myCurve");
        assert_eq!(a.style.as_deref(), Some("slide"));
    }

    #[test]
    fn bind_flags_parse_from_keyword() {
        let f = parse_bind_flags("bindel");
        assert!(f.repeat && f.locked);
        assert!(!f.mouse);
        assert!(is_bind("bind"));
        assert!(is_bind("bindm"));
        assert!(!is_bind("binds"));
    }
}
