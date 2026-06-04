//! `hyprconf-gui` — the Iced desktop front-end for hyprconf.
//!
//! This first step is intentionally tiny: it boots an Iced 0.14 application
//! using the functional `iced::application(boot, update, view)` builder, renders
//! a top bar (title + a light/dark theme toggle) above an empty content area,
//! and wires the theme through application state. No Hyprland logic exists yet.

use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Element, Length, Theme};

fn main() -> anyhow::Result<()> {
    init_tracing();

    tracing::info!(
        gui_version = env!("CARGO_PKG_VERSION"),
        core_version = hyprconf_core::version(),
        "starting hyprconf",
    );

    // Iced 0.14 functional builder:
    //   application(boot, update, view) -> Application
    // where `boot` is `Fn() -> State` (or `Fn() -> (State, Task<Message>)`).
    //
    // ASSUMPTION (iced 0.14): the first argument is the state initialiser, and
    // `.title`/`.theme` take `Fn(&State) -> _`. Verified against the crate at
    // build time in this step.
    iced::application(App::new, App::update, App::view)
        .title(App::title)
        .theme(App::theme)
        .run()?;

    Ok(())
}

/// Initialise `tracing` with an `EnvFilter` so `RUST_LOG` is respected and we
/// fall back to `info` when it is unset.
fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // `try_init` so running tests or embedding the GUI twice never panics on a
    // double-initialised global subscriber.
    let _ = fmt().with_env_filter(filter).try_init();
}

/// Which built-in Iced theme the window is currently rendered with.
///
/// We model this explicitly (rather than storing an `iced::Theme`) so the
/// toggle logic stays trivial and we keep room to map to custom themes later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Appearance {
    Light,
    #[default]
    Dark,
}

impl Appearance {
    /// The opposite appearance, used by the toggle.
    fn toggled(self) -> Self {
        match self {
            Appearance::Light => Appearance::Dark,
            Appearance::Dark => Appearance::Light,
        }
    }

    /// Map our appearance to a concrete built-in Iced theme.
    fn theme(self) -> Theme {
        match self {
            Appearance::Light => Theme::Light,
            Appearance::Dark => Theme::Dark,
        }
    }

    /// Label for the button that switches to the *other* appearance.
    fn toggle_label(self) -> &'static str {
        match self {
            // Currently light -> offer to switch to dark, and vice versa.
            Appearance::Light => "Switch to dark",
            Appearance::Dark => "Switch to light",
        }
    }
}

/// Top-level application state (the Elm "model").
#[derive(Debug, Default)]
struct App {
    appearance: Appearance,
}

/// Messages produced by the UI / side effects (the Elm "messages").
#[derive(Debug, Clone)]
enum Message {
    /// The user toggled between the light and dark theme.
    ThemeToggled,
}

impl App {
    /// Boot function: produce the initial application state.
    fn new() -> Self {
        Self::default()
    }

    /// Window title (Iced calls this with `&State`).
    fn title(&self) -> String {
        format!("hyprconf {}", env!("CARGO_PKG_VERSION"))
    }

    /// Active theme (Iced calls this with `&State`).
    fn theme(&self) -> Theme {
        self.appearance.theme()
    }

    /// Elm `update`: fold a [`Message`] into the state.
    fn update(&mut self, message: Message) {
        match message {
            Message::ThemeToggled => {
                self.appearance = self.appearance.toggled();
                tracing::debug!(?self.appearance, "theme toggled");
            }
        }
    }

    /// Elm `view`: render the current state into widgets.
    fn view(&self) -> Element<'_, Message> {
        let top_bar = row![
            text("hyprconf").size(22),
            // Flexible spacer pushes the toggle to the right edge of the bar.
            Space::new().width(Length::Fill),
            button(text(self.appearance.toggle_label())).on_press(Message::ThemeToggled),
        ]
        .spacing(12)
        .padding(12)
        .align_y(Alignment::Center);

        // Empty content area for now — future steps fill this with config panels.
        let content = container(column![])
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(12);

        column![top_bar, content].into()
    }
}
