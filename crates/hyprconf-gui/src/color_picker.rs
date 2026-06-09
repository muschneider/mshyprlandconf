// SPDX-License-Identifier: MIT OR Apache-2.0
//! The visual color picker: a draggable saturation/value square and a hue
//! strip, both drawn with `iced`'s `canvas`. They report geometric positions
//! ([`Message::PickSatVal`] / [`Message::PickHue`]); the app turns those into a
//! concrete color (combined with the model's current alpha) and applies it
//! live. HSV is kept in [`ColorDraft`] — separate from the model — so hue and
//! saturation survive excursions to value 0 / saturation 0 (where they'd
//! otherwise be undefined).

use iced::mouse;
use iced::widget::canvas::{self, gradient::Linear, Event, Frame, Geometry, Path, Program};
use iced::{Color as IcedColor, Point, Rectangle, Renderer, Size, Theme};

use hyprconf_core::value::Color;

use crate::Message;

/// What an open color picker is editing: either a whole scalar `Color` option,
/// or one stop of a `Gradient` option.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColorTarget {
    /// A scalar color option, by path.
    Option(String),
    /// A single stop of a gradient option, by path + stop index.
    Stop {
        /// The gradient option's path.
        path: String,
        /// The stop index.
        index: usize,
    },
}

impl ColorTarget {
    /// The underlying option path (shared by both variants).
    #[must_use]
    pub fn path(&self) -> &str {
        match self {
            ColorTarget::Option(path) | ColorTarget::Stop { path, .. } => path,
        }
    }
}

/// The live HSV state of an open picker (the single source of truth for the
/// indicators). Alpha is read from the model, not stored here.
#[derive(Debug, Clone)]
pub struct ColorDraft {
    /// What is being edited.
    pub target: ColorTarget,
    /// Hue in degrees, `0.0..360.0`.
    pub hue: f32,
    /// Saturation, `0.0..=1.0`.
    pub sat: f32,
    /// Value/brightness, `0.0..=1.0`.
    pub val: f32,
}

impl ColorDraft {
    /// Derive a draft for `target` from its existing color.
    #[must_use]
    pub fn from_color(target: ColorTarget, color: Color) -> Self {
        let (hue, sat, val) = color.to_hsv();
        Self {
            target,
            hue: hue as f32,
            sat: sat as f32,
            val: val as f32,
        }
    }
}

fn iced_color(c: Color) -> IcedColor {
    IcedColor::from_rgba8(c.r, c.g, c.b, f32::from(c.a) / 255.0)
}

/// The fully-saturated, full-value color for a hue (the right edge of the SV
/// square / the hue strip).
fn hue_color(hue: f32) -> IcedColor {
    iced_color(Color::from_hsv(f64::from(hue), 1.0, 1.0, 255))
}

// ---------------------------------------------------------------------------
// saturation / value square
// ---------------------------------------------------------------------------

/// The 2D saturation (x) / value (y) picking area for the current hue.
#[derive(Debug)]
pub struct SvSquare {
    /// Current hue (fixes the square's color ramp).
    pub hue: f32,
    /// Current saturation (indicator x).
    pub sat: f32,
    /// Current value (indicator y).
    pub val: f32,
}

fn sat_val_message(p: Point, bounds: Rectangle) -> Message {
    let s = (p.x / bounds.width).clamp(0.0, 1.0);
    let v = (1.0 - p.y / bounds.height).clamp(0.0, 1.0);
    Message::PickSatVal(s, v)
}

impl Program<Message> for SvSquare {
    type State = bool; // currently dragging

    fn update(
        &self,
        dragging: &mut bool,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(p) = cursor.position_in(bounds) {
                    *dragging = true;
                    return Some(canvas::Action::publish(sat_val_message(p, bounds)).and_capture());
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) if *dragging => {
                if let Some(p) = cursor.position_in(bounds) {
                    return Some(canvas::Action::publish(sat_val_message(p, bounds)));
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                *dragging = false;
            }
            _ => {}
        }
        None
    }

    fn draw(
        &self,
        _state: &bool,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let (w, h) = (bounds.width, bounds.height);
        let rect = Path::rectangle(Point::ORIGIN, Size::new(w, h));

        // White → hue, left to right.
        frame.fill(
            &rect,
            Linear::new(Point::ORIGIN, Point::new(w, 0.0))
                .add_stop(0.0, IcedColor::WHITE)
                .add_stop(1.0, hue_color(self.hue)),
        );
        // Transparent → black, top to bottom (darkens toward the bottom).
        frame.fill(
            &rect,
            Linear::new(Point::ORIGIN, Point::new(0.0, h))
                .add_stop(0.0, IcedColor::TRANSPARENT)
                .add_stop(1.0, IcedColor::BLACK),
        );

        // Indicator: a white ring around the picked color.
        let center = Point::new(
            self.sat.clamp(0.0, 1.0) * w,
            (1.0 - self.val.clamp(0.0, 1.0)) * h,
        );
        frame.fill(&Path::circle(center, 7.0), IcedColor::WHITE);
        frame.fill(
            &Path::circle(center, 5.0),
            iced_color(Color::from_hsv(
                f64::from(self.hue),
                f64::from(self.sat),
                f64::from(self.val),
                255,
            )),
        );

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &bool,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if cursor.is_over(bounds) {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::default()
        }
    }
}

// ---------------------------------------------------------------------------
// hue strip
// ---------------------------------------------------------------------------

/// A vertical hue selector.
#[derive(Debug)]
pub struct HueStrip {
    /// Current hue (indicator y).
    pub hue: f32,
}

fn hue_message(p: Point, bounds: Rectangle) -> Message {
    Message::PickHue((p.y / bounds.height).clamp(0.0, 1.0) * 360.0)
}

impl Program<Message> for HueStrip {
    type State = bool; // currently dragging

    fn update(
        &self,
        dragging: &mut bool,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(p) = cursor.position_in(bounds) {
                    *dragging = true;
                    return Some(canvas::Action::publish(hue_message(p, bounds)).and_capture());
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) if *dragging => {
                if let Some(p) = cursor.position_in(bounds) {
                    return Some(canvas::Action::publish(hue_message(p, bounds)));
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                *dragging = false;
            }
            _ => {}
        }
        None
    }

    fn draw(
        &self,
        _state: &bool,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let (w, h) = (bounds.width, bounds.height);

        frame.fill(
            &Path::rectangle(Point::ORIGIN, Size::new(w, h)),
            Linear::new(Point::ORIGIN, Point::new(0.0, h))
                .add_stop(0.0, hue_color(0.0))
                .add_stop(1.0 / 6.0, hue_color(60.0))
                .add_stop(2.0 / 6.0, hue_color(120.0))
                .add_stop(3.0 / 6.0, hue_color(180.0))
                .add_stop(4.0 / 6.0, hue_color(240.0))
                .add_stop(5.0 / 6.0, hue_color(300.0))
                .add_stop(1.0, hue_color(360.0)),
        );

        // Indicator: a white bar with a black backing for contrast.
        let y = (self.hue / 360.0).clamp(0.0, 1.0) * h;
        frame.fill(
            &Path::rectangle(Point::new(0.0, y - 3.5), Size::new(w, 7.0)),
            IcedColor::BLACK,
        );
        frame.fill(
            &Path::rectangle(Point::new(0.0, y - 1.5), Size::new(w, 3.0)),
            IcedColor::WHITE,
        );

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &bool,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if cursor.is_over(bounds) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}
