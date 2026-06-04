//! `hyprconf-gui` — the Iced desktop front-end for hyprconf.
//!
//! This step wires `hyprconf-core` into the UI: on launch it locates and parses
//! the user's Hyprland config off the UI thread (via an `iced::Task`), then lets
//! the user browse it through a sidebar of sections/collections, a live
//! fuzzy-search box, and a status bar. Editing/saving arrive later.

mod fuzzy;
mod load;
mod view;

use std::path::PathBuf;
use std::sync::Arc;

use iced::{Element, Task, Theme};

use hyprconf_core::schema::{CollectionId, Schema};

use crate::load::LoadState;

fn main() -> anyhow::Result<()> {
    init_tracing();

    let args = parse_args();
    tracing::info!(
        gui_version = env!("CARGO_PKG_VERSION"),
        core_version = hyprconf_core::version(),
        config = ?args.config,
        check = args.check,
        "starting hyprconf",
    );

    // Headless sanity check: load and report, without opening a window.
    if args.check {
        return run_check(args.config);
    }

    let explicit = args.config;
    iced::application(move || App::boot(explicit.clone()), App::update, App::view)
        .title(App::title)
        .theme(App::theme)
        .run()?;

    Ok(())
}

/// Parsed command-line arguments.
#[derive(Debug, Default)]
struct Args {
    config: Option<PathBuf>,
    check: bool,
}

/// Parse `--config <path>` / `--config=<path>` and `--check`.
fn parse_args() -> Args {
    let mut parsed = Args::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if let Some(value) = arg.strip_prefix("--config=") {
            parsed.config = Some(PathBuf::from(value));
        } else if arg == "--config" {
            parsed.config = args.next().map(PathBuf::from);
        } else if arg == "--check" {
            parsed.check = true;
        }
    }
    parsed
}

/// Load the config and print a one-line summary; used by `--check` (no window).
fn run_check(explicit: Option<PathBuf>) -> anyhow::Result<()> {
    match load::load_config(explicit) {
        LoadState::Loaded(loaded) => {
            println!(
                "loaded {} config: {} ({} options set, {} warnings, {} included file(s))",
                load::format_label(loaded.format),
                loaded.source.display(),
                loaded.config.option_count(),
                loaded.warnings,
                loaded.included_files,
            );
            Ok(())
        }
        LoadState::NotFound { searched } => {
            let searched = searched
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            anyhow::bail!("no configuration found (searched: {searched})")
        }
        LoadState::Error { path, message } => {
            anyhow::bail!("failed to load {}: {message}", path.display())
        }
        LoadState::Loading => Ok(()),
    }
}

fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = fmt().with_env_filter(filter).try_init();
}

/// Light/dark appearance, mapped to a built-in Iced theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Appearance {
    Light,
    #[default]
    Dark,
}

impl Appearance {
    fn toggled(self) -> Self {
        match self {
            Appearance::Light => Appearance::Dark,
            Appearance::Dark => Appearance::Light,
        }
    }

    fn theme(self) -> Theme {
        match self {
            Appearance::Light => Theme::Light,
            Appearance::Dark => Theme::Dark,
        }
    }

    pub(crate) fn toggle_label(self) -> &'static str {
        match self {
            Appearance::Light => "Switch to dark",
            Appearance::Dark => "Switch to light",
        }
    }
}

/// Which sidebar entry is selected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Selection {
    /// A schema section, by id.
    Section(String),
    /// A structured collection.
    Collection(CollectionId),
}

/// Messages produced by the UI and async tasks.
#[derive(Debug, Clone)]
pub(crate) enum Message {
    /// The light/dark theme was toggled.
    ThemeToggled,
    /// The background load finished.
    Loaded(Arc<LoadState>),
    /// A sidebar entry was selected.
    Selected(Selection),
    /// The search query changed.
    SearchChanged(String),
}

/// Top-level application state.
#[derive(Debug)]
pub(crate) struct App {
    pub(crate) appearance: Appearance,
    pub(crate) schema: &'static Schema,
    pub(crate) load: LoadState,
    pub(crate) selected: Selection,
    pub(crate) search: String,
}

impl App {
    /// Boot: build the initial state and kick off the (non-blocking) load.
    fn boot(explicit: Option<PathBuf>) -> (Self, Task<Message>) {
        let schema = Schema::shared();
        let selected = schema
            .sections()
            .first()
            .map(|s| Selection::Section(s.id.clone()))
            .unwrap_or(Selection::Collection(CollectionId::Keybinds));

        let app = Self {
            appearance: Appearance::default(),
            schema,
            load: LoadState::Loading,
            selected,
            search: String::new(),
        };

        let task = Task::perform(async move { load::load_config(explicit) }, |state| {
            Message::Loaded(Arc::new(state))
        });

        (app, task)
    }

    fn title(&self) -> String {
        format!("hyprconf {}", env!("CARGO_PKG_VERSION"))
    }

    fn theme(&self) -> Theme {
        self.appearance.theme()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ThemeToggled => self.appearance = self.appearance.toggled(),
            Message::Loaded(state) => {
                match &*state {
                    LoadState::Loaded(loaded) => tracing::info!(
                        format = load::format_label(loaded.format),
                        source = %loaded.source.display(),
                        options = loaded.config.option_count(),
                        warnings = loaded.warnings,
                        "configuration loaded",
                    ),
                    LoadState::NotFound { searched } => {
                        tracing::warn!(?searched, "no configuration found");
                    }
                    LoadState::Error { path, message } => {
                        tracing::error!(path = %path.display(), %message, "failed to load configuration");
                    }
                    LoadState::Loading => {}
                }
                self.load = (*state).clone();
            }
            Message::Selected(selection) => {
                self.selected = selection;
                self.search.clear();
            }
            Message::SearchChanged(query) => self.search = query,
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        view::view(self)
    }
}
