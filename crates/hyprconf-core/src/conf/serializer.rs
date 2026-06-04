//! Serialization of a [`ConfDocument`] back to hyprlang text, and rendering of
//! typed [`Value`]s into their `.conf` textual form.
//!
//! The heavy lifting of round-trip fidelity lives in [`ConfDocument::to_text`];
//! this module is a thin, explicitly-named facade plus the value renderer used
//! by edits.

use crate::conf::document::ConfDocument;
use crate::value::Value;

/// Emits valid hyprlang from a [`ConfDocument`].
#[derive(Debug, Default, Clone, Copy)]
pub struct ConfSerializer;

impl ConfSerializer {
    /// Serialize a document to text (byte-for-byte identical to the input when
    /// the document has not been edited).
    #[must_use]
    pub fn serialize(document: &ConfDocument) -> String {
        document.to_text()
    }
}

/// Render a typed [`Value`] into its hyprlang `.conf` textual form.
#[must_use]
pub fn value_to_conf(value: &Value) -> String {
    match value {
        Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        Value::Int(i) => i.to_string(),
        Value::Float(x) => format_f64(*x),
        Value::Color(c) => c.to_rgba_string(),
        Value::Gradient(g) => g.to_hyprland_string(),
        Value::String(s) => s.clone(),
        Value::Enum(name) => name.clone(),
        Value::Vec2(v) => v.to_hyprland_string(),
    }
}

/// Format an `f64` without a trailing `.0` for whole numbers.
fn format_f64(value: f64) -> String {
    if value.is_finite() && value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::{Color, Vec2};

    #[test]
    fn renders_scalars() {
        assert_eq!(value_to_conf(&Value::Bool(true)), "true");
        assert_eq!(value_to_conf(&Value::Int(10)), "10");
        assert_eq!(value_to_conf(&Value::Float(0.5)), "0.5");
        assert_eq!(value_to_conf(&Value::Float(1.0)), "1");
        assert_eq!(value_to_conf(&Value::String("us".into())), "us");
        assert_eq!(value_to_conf(&Value::Enum("dwindle".into())), "dwindle");
        assert_eq!(
            value_to_conf(&Value::Color(Color::rgba(0x1a, 0x2b, 0x3c, 0xff))),
            "rgba(1a2b3cff)"
        );
        assert_eq!(value_to_conf(&Value::Vec2(Vec2::new(0.0, 0.0))), "0 0");
    }
}
