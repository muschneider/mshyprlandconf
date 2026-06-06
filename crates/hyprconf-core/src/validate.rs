//! Whole-model validation of a [`Config`] against the [`Schema`].
//!
//! Returns every problem found (not just the first), each tagged with a
//! [`Severity`] so a front-end can block saving on hard [`Severity::Error`]s
//! while letting the user override soft [`Severity::Warning`]s.

use crate::model::Config;
use crate::schema::Schema;
use crate::CoreError;

/// How serious a [`ConfigProblem`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// A hard error: the value is invalid for its option (blocks save).
    Error,
    /// A soft warning: unusual but writable (overridable).
    Warning,
}

/// A single validation finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigProblem {
    /// The dotted option path the problem concerns.
    pub path: String,
    /// A human-readable description.
    pub message: String,
    /// How serious it is.
    pub severity: Severity,
}

/// Validate every scalar option in `config` against the schema.
///
/// - A value that fails its [`crate::schema::OptionSpec`] (wrong type, out of
///   range, invalid enum variant) is a [`Severity::Error`].
/// - A key the schema does not know is a [`Severity::Warning`] (it is preserved
///   verbatim, but the user should be aware it is unrecognised).
#[must_use]
pub fn validate_config(schema: &Schema, config: &Config) -> Vec<ConfigProblem> {
    let mut problems = Vec::new();

    for (path, tracked) in &config.options {
        match schema.option(path) {
            Some(spec) => {
                if let Err(CoreError::Validation { reason, .. }) = spec.validate(&tracked.value) {
                    problems.push(ConfigProblem {
                        path: path.clone(),
                        message: reason,
                        severity: Severity::Error,
                    });
                }
            }
            None => problems.push(ConfigProblem {
                path: path.clone(),
                message: "unknown option (not in the Hyprland schema)".to_string(),
                severity: Severity::Warning,
            }),
        }
    }

    problems
}

/// Whether `problems` contains any hard error.
#[must_use]
pub fn has_errors(problems: &[ConfigProblem]) -> bool {
    problems.iter().any(|p| p.severity == Severity::Error)
}

/// Options that the *running* Hyprland is too old to support.
///
/// For every option that is set and carries a `since` version newer than
/// `running_version`, returns a [`Severity::Warning`] problem. (Most schema
/// options have no `since`, so this is empty unless that metadata is present.)
#[must_use]
pub fn unsupported_options(
    schema: &Schema,
    config: &Config,
    running_version: &str,
) -> Vec<ConfigProblem> {
    let mut problems = Vec::new();
    for (path, _) in &config.options {
        let Some(spec) = schema.option(path) else {
            continue;
        };
        let Some(since) = &spec.since else {
            continue;
        };
        if crate::hyprctl::is_newer(since, running_version) == Some(true) {
            problems.push(ConfigProblem {
                path: path.clone(),
                message: format!("requires Hyprland {since}, but {running_version} is running"),
                severity: Severity::Warning,
            });
        }
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;
    use crate::Tracked;

    #[test]
    fn flags_out_of_range_as_error() {
        let schema = Schema::shared();
        let mut config = Config::default_from_schema(schema);
        // rounding has min 0; -5 is out of range.
        config.set("decoration:rounding", Value::Int(-5));

        let problems = validate_config(schema, &config);
        assert!(has_errors(&problems));
        assert!(problems
            .iter()
            .any(|p| p.path == "decoration:rounding" && p.severity == Severity::Error));
    }

    #[test]
    fn flags_unknown_key_as_warning() {
        let schema = Schema::shared();
        let mut config = Config::empty();
        config.options.insert(
            "general:not_a_real_key".into(),
            Tracked::new(Value::String("x".into())),
        );

        let problems = validate_config(schema, &config);
        assert!(!has_errors(&problems));
        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0].severity, Severity::Warning);
    }

    #[test]
    fn clean_config_has_no_problems() {
        let schema = Schema::shared();
        let config = Config::default_from_schema(schema);
        assert!(validate_config(schema, &config).is_empty());
    }

    #[test]
    fn surfaces_options_newer_than_running_hyprland() {
        let schema = Schema::shared();
        let config = Config::default_from_schema(schema);

        // An ancient Hyprland is missing the `since`-tagged options.
        let old = unsupported_options(schema, &config, "0.41.0");
        assert!(old.iter().any(|p| p.path == "decoration:rounding_power"));
        assert!(old.iter().all(|p| p.severity == Severity::Warning));

        // The current Hyprland supports everything in the schema.
        assert!(unsupported_options(schema, &config, "0.55.2").is_empty());
    }
}
