//! `hyprconf-gui` — the Iced desktop front-end for hyprconf.
//!
//! This step wires `hyprconf-core` into the UI: on launch it locates and parses
//! the user's Hyprland config off the UI thread (via an `iced::Task`), then lets
//! the user browse it through a sidebar of sections/collections, a live
//! fuzzy-search box, and a status bar. Editing/saving arrive later.

mod edit;
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
    /// A theme was chosen from the picker.
    ThemeSelected(Theme),
    /// The background load finished.
    Loaded(Arc<LoadState>),
    /// A sidebar entry was selected.
    Selected(Selection),
    /// The search query changed.
    SearchChanged(String),
    /// An option was edited.
    Edit(edit::EditAction),
    /// A structured collection was edited.
    CollectionEdit(edit::CollectionAction),
    /// Toggle the pending-changes (diff) view.
    ToggleChanges,
}

/// Top-level application state.
#[derive(Debug)]
pub(crate) struct App {
    pub(crate) theme: Theme,
    pub(crate) schema: &'static Schema,
    pub(crate) load: LoadState,
    pub(crate) selected: Selection,
    pub(crate) search: String,
    pub(crate) show_changes: bool,
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
            theme: Theme::CatppuccinMocha,
            schema,
            load: LoadState::Loading,
            selected,
            search: String::new(),
            show_changes: false,
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
        self.theme.clone()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ThemeSelected(theme) => self.theme = theme,
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
                self.show_changes = false;
            }
            Message::SearchChanged(query) => self.search = query,
            Message::Edit(action) => {
                if let LoadState::Loaded(loaded) = &mut self.load {
                    loaded.apply(action, self.schema);
                }
            }
            Message::CollectionEdit(action) => {
                if let LoadState::Loaded(loaded) = &mut self.load {
                    loaded.apply_collection(action);
                }
            }
            Message::ToggleChanges => self.show_changes = !self.show_changes,
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        view::view(self)
    }
}
