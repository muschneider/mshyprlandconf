// SPDX-License-Identifier: MIT OR Apache-2.0
//! Theme-aware widget styling for the views.
//!
//! Pure functions from a `&Theme` (and, where relevant, a widget status) to
//! the matching iced `Style`. Kept in their own module so the view modules
//! stay focused on layout rather than palette plumbing.

use iced::widget::{button, container, text, text_input};
use iced::{Background, Border, Color, Theme};

pub(super) fn accent(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(theme.extended_palette().primary.base.color),
    }
}

pub(super) fn muted(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(theme.palette().text.scale_alpha(0.6)),
    }
}

pub(super) fn danger(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(theme.extended_palette().danger.base.color),
    }
}

pub(super) fn success(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(theme.extended_palette().success.base.color),
    }
}

pub(super) fn warn_style(_theme: &Theme) -> text::Style {
    text::Style {
        color: Some(Color::from_rgb8(0xe0, 0xa0, 0x30)),
    }
}

/// Background for the diff/code block.
pub(super) fn code_style(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(p.background.weakest.color.into()),
        border: Border {
            radius: 6.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// Header & status bars: a panel-tinted strip.
pub(super) fn bar_style(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(p.background.weak.color.into()),
        ..container::Style::default()
    }
}

/// Sidebar panel.
pub(super) fn panel_style(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(p.background.weak.color.into()),
        ..container::Style::default()
    }
}

/// A raised card on the base background.
pub(super) fn card_style(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(p.background.weak.color.into()),
        border: Border {
            color: p.background.strong.color.scale_alpha(0.5),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..container::Style::default()
    }
}

/// Small count/tag badge.
pub(super) fn badge_style(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(p.background.strong.color.into()),
        text_color: Some(p.background.strong.text),
        border: Border {
            radius: 8.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// The format badge in the status bar (accent-tinted).
pub(super) fn format_badge_style(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(p.primary.base.color.into()),
        text_color: Some(p.primary.base.text),
        border: Border {
            radius: 6.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// Sidebar nav button: accent fill when selected, subtle hover otherwise.
pub(super) fn nav_style(theme: &Theme, status: button::Status, selected: bool) -> button::Style {
    let p = theme.extended_palette();
    let (background, text_color) = if selected {
        (Some(p.primary.base.color.into()), p.primary.base.text)
    } else {
        match status {
            button::Status::Hovered | button::Status::Pressed => (
                Some(p.background.strong.color.scale_alpha(0.5).into()),
                p.background.base.text,
            ),
            _ => (None, p.background.base.text),
        }
    };
    button::Style {
        background,
        text_color,
        border: Border {
            radius: 7.0.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

/// Search-result row: invisible until hovered.
pub(super) fn result_style(theme: &Theme, status: button::Status) -> button::Style {
    let p = theme.extended_palette();
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => {
            Some(Background::from(p.background.weak.color))
        }
        _ => None,
    };
    button::Style {
        background,
        text_color: p.background.base.text,
        border: Border {
            radius: 8.0.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

/// A subtle, borderless button (reset/remove/add controls).
pub(super) fn ghost_button(theme: &Theme, status: button::Status) -> button::Style {
    let p = theme.extended_palette();
    let (background, alpha) = match status {
        button::Status::Hovered | button::Status::Pressed => (
            Some(p.background.strong.color.scale_alpha(0.5).into()),
            0.95,
        ),
        button::Status::Disabled => (None, 0.35),
        button::Status::Active => (None, 0.8),
    };
    button::Style {
        background,
        text_color: p.background.base.text.scale_alpha(alpha),
        border: Border {
            radius: 6.0.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

/// A small toggle chip (modifiers, bind flags, v2): accent when active.
pub(super) fn chip_style(theme: &Theme, status: button::Status, active: bool) -> button::Style {
    let p = theme.extended_palette();
    let (background, text_color) = if active {
        (Some(p.primary.base.color.into()), p.primary.base.text)
    } else {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let color = if hovered {
            p.background.strong.color
        } else {
            p.background.weak.color
        };
        (Some(color.into()), p.background.base.text.scale_alpha(0.85))
    };
    button::Style {
        background,
        text_color,
        border: Border {
            radius: 6.0.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

/// The "N unsaved" indicator pill (accent-tinted).
pub(super) fn changes_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let p = theme.extended_palette();
    let base = p.primary.base.color;
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => base,
        _ => base.scale_alpha(0.85),
    };
    button::Style {
        background: Some(background.into()),
        text_color: p.primary.base.text,
        border: Border {
            radius: 6.0.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

/// The floating tooltip box.
pub(super) fn tooltip_style(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(p.background.strong.color.into()),
        text_color: Some(p.background.strong.text),
        border: Border {
            color: p.background.stronger.color,
            width: 1.0,
            radius: 6.0.into(),
        },
        ..container::Style::default()
    }
}

/// A text input that flags validation errors with a danger-colored border.
pub(super) fn input_style(
    theme: &Theme,
    status: text_input::Status,
    has_error: bool,
) -> text_input::Style {
    let mut style = text_input::default(theme, status);
    if has_error {
        style.border.color = theme.extended_palette().danger.base.color;
        style.border.width = 1.5;
    }
    style
}
