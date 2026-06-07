//! `hyprconf-gui` — the Iced desktop front-end for hyprconf.
//!
//! This step wires `hyprconf-core` into the UI: on launch it locates and parses
//! the user's Hyprland config off the UI thread (via an `iced::Task`), then lets
//! the user browse and edit it. It also detects a running Hyprland (via
//! `hyprctl`) for optional live-apply/reload, keeps an undo/redo history, and
//! persists window/theme/format/recent-file settings between runs.

mod diff;
mod edit;
mod fuzzy;
mod load;
mod profiles;
mod save;
mod settings;
mod view;

use std::path::PathBuf;
use std::sync::Arc;

use iced::{Element, Size, Task, Theme};

use hyprconf_core::hyprctl::HyprlandInfo;
use hyprconf_core::schema::{CollectionId, Schema};
use hyprconf_core::ConfigFormat;

use crate::edit::EditSnapshot;
use crate::load::{LoadState, Loaded};
use crate::settings::Settings;

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

    let settings = Settings::load();
    let size = Size::new(settings.window_width, settings.window_height);
    let explicit = args.config;

    iced::application(
        move || App::boot(explicit.clone(), settings.clone()),
        App::update,
        App::view,
    )
    .title(App::title)
    .theme(App::theme)
    .subscription(App::subscription)
    .window_size(size)
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
    /// Undo the last edit.
    Undo,
    /// Redo the last undone edit.
    Redo,
    /// Toggle the pending-changes (diff) view.
    ToggleChanges,
    /// Open/close the save panel.
    ToggleSave,
    /// Choose the output format in the save panel.
    SetOutputFormat(ConfigFormat),
    /// Toggle "save despite warnings".
    ToggleOverride(bool),
    /// Write the current plan to disk.
    PerformSave,
    /// A background Hyprland detection finished.
    HyprlandDetected(Option<HyprlandInfo>),
    /// Toggle live-apply (push committed scalar edits via `hyprctl keyword`).
    ToggleLiveApply(bool),
    /// Ask the running Hyprland to reload its config.
    Reload,
    /// The result of a `hyprctl` invocation (apply/reload).
    HyprResult(Result<String, String>),
    /// The window was resized (persisted for next launch).
    WindowResized(f32, f32),
    /// Open/close the profiles & recent-files panel.
    ToggleProfiles,
    /// The profile-name field changed.
    ProfileNameChanged(String),
    /// Save the current config as a named profile.
    SaveProfile,
    /// The import-path field changed.
    ImportPathChanged(String),
    /// Open a config from an explicit path (recent / profile / import).
    OpenPath(PathBuf),
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
    /// Whether the save panel is open.
    pub(crate) show_save: bool,
    /// The chosen output format (defaults to the loaded format).
    pub(crate) output_format: Option<ConfigFormat>,
    /// "Save despite soft warnings".
    pub(crate) override_warnings: bool,
    /// The last save's status line, if any.
    pub(crate) save_status: Option<Result<String, String>>,
    /// Undo history (newest last).
    pub(crate) undo: Vec<EditSnapshot>,
    /// Redo history (newest last).
    pub(crate) redo: Vec<EditSnapshot>,
    /// The coalescing key of the last continuous edit, if any.
    pub(crate) last_key: Option<String>,
    /// A detected running Hyprland, if any.
    pub(crate) hyprland: Option<HyprlandInfo>,
    /// Whether committed scalar edits are pushed live via `hyprctl`.
    pub(crate) live_apply: bool,
    /// The last `hyprctl` status line, if any.
    pub(crate) hypr_status: Option<Result<String, String>>,
    /// Persisted settings (theme/format/window/recents).
    pub(crate) settings: Settings,
    /// Whether the profiles & recents panel is open.
    pub(crate) show_profiles: bool,
    /// The in-progress profile name.
    pub(crate) profile_name: String,
    /// The in-progress import path.
    pub(crate) import_path: String,
}

impl App {
    /// Boot: build the initial state and kick off the (non-blocking) load and
    /// Hyprland detection.
    fn boot(explicit: Option<PathBuf>, settings: Settings) -> (Self, Task<Message>) {
        let schema = Schema::shared();
        let selected = schema
            .sections()
            .first()
            .map(|s| Selection::Section(s.id.clone()))
            .unwrap_or(Selection::Collection(CollectionId::Keybinds));

        let app = Self {
            theme: theme_from_name(&settings.theme),
            schema,
            load: LoadState::Loading,
            selected,
            search: String::new(),
            show_changes: false,
            show_save: false,
            output_format: Some(format_from_name(&settings.last_format)),
            override_warnings: false,
            save_status: None,
            undo: Vec::new(),
            redo: Vec::new(),
            last_key: None,
            hyprland: None,
            live_apply: false,
            hypr_status: None,
            settings,
            show_profiles: false,
            profile_name: String::new(),
            import_path: String::new(),
        };

        let load = Task::perform(async move { load::load_config(explicit) }, |state| {
            Message::Loaded(Arc::new(state))
        });
        let detect = Task::perform(
            async { hyprconf_core::hyprctl::detect() },
            Message::HyprlandDetected,
        );

        (app, Task::batch([load, detect]))
    }

    fn title(&self) -> String {
        format!("hyprconf {}", env!("CARGO_PKG_VERSION"))
    }

    fn theme(&self) -> Theme {
        self.theme.clone()
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        iced::Subscription::batch([
            iced::event::listen_with(handle_event),
            iced::window::resize_events()
                .map(|(_id, size)| Message::WindowResized(size.width, size.height)),
        ])
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ThemeSelected(theme) => {
                self.settings.theme = theme.to_string();
                self.settings.save();
                self.theme = theme;
            }
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
                // A fresh load invalidates the edit history.
                self.undo.clear();
                self.redo.clear();
                self.last_key = None;

                let recent = match &self.load {
                    LoadState::Loaded(loaded) => Some(loaded.source.display().to_string()),
                    _ => None,
                };
                if let Some(source) = recent {
                    self.settings.add_recent(&source);
                    self.settings.save();
                }
            }
            Message::Selected(selection) => {
                self.selected = selection;
                self.search.clear();
                self.show_changes = false;
                self.show_save = false;
                self.show_profiles = false;
            }
            Message::SearchChanged(query) => self.search = query,
            Message::Edit(action) => {
                self.record(action.coalesce_key());
                let path = action.option_path().map(str::to_string);
                if let LoadState::Loaded(loaded) = &mut self.load {
                    loaded.apply(action, self.schema);
                }
                return self.live_apply_task(path);
            }
            Message::CollectionEdit(action) => {
                self.record(action.coalesce_key());
                if let LoadState::Loaded(loaded) = &mut self.load {
                    loaded.apply_collection(action);
                }
            }
            Message::Undo => {
                self.last_key = None;
                let Some(prev) = self.undo.pop() else {
                    return Task::none();
                };
                if let LoadState::Loaded(loaded) = &mut self.load {
                    let current = loaded.snapshot();
                    loaded.restore(prev);
                    self.redo.push(current);
                } else {
                    self.undo.push(prev);
                }
            }
            Message::Redo => {
                self.last_key = None;
                let Some(next) = self.redo.pop() else {
                    return Task::none();
                };
                if let LoadState::Loaded(loaded) = &mut self.load {
                    let current = loaded.snapshot();
                    loaded.restore(next);
                    self.undo.push(current);
                } else {
                    self.redo.push(next);
                }
            }
            Message::ToggleChanges => {
                self.show_changes = !self.show_changes;
                if self.show_changes {
                    self.show_save = false;
                    self.show_profiles = false;
                }
            }
            Message::ToggleSave => {
                self.show_save = !self.show_save;
                if self.show_save {
                    self.show_changes = false;
                    self.show_profiles = false;
                    self.save_status = None;
                    if self.output_format.is_none() {
                        self.output_format = self.load.loaded().map(|l| l.format);
                    }
                }
            }
            Message::SetOutputFormat(format) => {
                self.output_format = Some(format);
                self.save_status = None;
                self.settings.last_format = format_to_name(format).to_string();
                self.settings.save();
            }
            Message::ToggleOverride(value) => self.override_warnings = value,
            Message::PerformSave => return self.perform_save(),
            Message::HyprlandDetected(info) => {
                if info.is_none() {
                    self.live_apply = false;
                }
                tracing::info!(detected = info.is_some(), "hyprland detection");
                self.hyprland = info;
            }
            Message::ToggleLiveApply(on) => {
                self.live_apply = on && self.hyprland.is_some();
            }
            Message::Reload => {
                return Task::perform(
                    async { hyprconf_core::hyprctl::reload().map_err(|e| e.to_string()) },
                    Message::HyprResult,
                );
            }
            Message::HyprResult(result) => {
                match &result {
                    Ok(msg) => tracing::info!(message = %msg, "hyprctl ok"),
                    Err(e) => tracing::warn!(error = %e, "hyprctl failed"),
                }
                self.hypr_status = Some(result);
            }
            Message::WindowResized(width, height) => {
                self.settings.window_width = width;
                self.settings.window_height = height;
                self.settings.save();
            }
            Message::ToggleProfiles => {
                self.show_profiles = !self.show_profiles;
                if self.show_profiles {
                    self.show_changes = false;
                    self.show_save = false;
                    self.save_status = None;
                }
            }
            Message::ProfileNameChanged(name) => self.profile_name = name,
            Message::SaveProfile => {
                if let Some(loaded) = self.load.loaded() {
                    let format = self.output_format.unwrap_or(loaded.format);
                    self.save_status = Some(
                        profiles::save(&self.profile_name, format, &loaded.config)
                            .map(|path| format!("Saved profile → {}", path.display())),
                    );
                }
            }
            Message::ImportPathChanged(path) => self.import_path = path,
            Message::OpenPath(path) => {
                return Task::perform(async move { load::load_config(Some(path)) }, |state| {
                    Message::Loaded(Arc::new(state))
                });
            }
        }
        Task::none()
    }

    /// Push an undo snapshot for an edit, coalescing consecutive continuous
    /// edits (typing/dragging) that share a `key` into a single step.
    fn record(&mut self, key: Option<String>) {
        let coalesce = key.is_some() && self.last_key == key;
        self.last_key = key;
        if coalesce {
            return;
        }
        let snapshot = self.load.loaded().map(Loaded::snapshot);
        if let Some(snapshot) = snapshot {
            self.undo.push(snapshot);
            self.redo.clear();
            const MAX_UNDO: usize = 200;
            if self.undo.len() > MAX_UNDO {
                self.undo.remove(0);
            }
        }
    }

    /// If live-apply is on and the edited option is valid, push it to the
    /// running Hyprland via `hyprctl keyword`.
    fn live_apply_task(&self, path: Option<String>) -> Task<Message> {
        if !self.live_apply || self.hyprland.is_none() {
            return Task::none();
        }
        let Some(path) = path else {
            return Task::none();
        };
        let Some(loaded) = self.load.loaded() else {
            return Task::none();
        };
        if loaded.first_error(&path).is_some() {
            return Task::none();
        }
        let Some(value) = loaded.config.get(&path) else {
            return Task::none();
        };
        let value_text = hyprconf_core::conf::value_to_conf(value);
        Task::perform(
            async move {
                hyprconf_core::hyprctl::apply_keyword(&path, &value_text).map_err(|e| e.to_string())
            },
            Message::HyprResult,
        )
    }

    /// Validate, write the plan, and reload from disk on success.
    fn perform_save(&mut self) -> Task<Message> {
        let outcome: Option<(Result<String, String>, Option<PathBuf>)> = match self.load.loaded() {
            Some(loaded) => {
                let target = self.output_format.unwrap_or(loaded.format);
                let problems = save::review(loaded, self.schema);
                if let Some(reason) = save::blocked(&problems, self.override_warnings) {
                    Some((Err(reason), None))
                } else {
                    let plan = save::plan_save(loaded, target);
                    match save::perform_save(&plan) {
                        Ok(reports) => {
                            let backups = reports.iter().filter(|r| r.backup.is_some()).count();
                            let summary = format!(
                                "Saved {} file(s){}",
                                reports.len(),
                                if backups > 0 {
                                    format!(" · {backups} backup(s)")
                                } else {
                                    String::new()
                                }
                            );
                            Some((Ok(summary), Some(plan.root)))
                        }
                        Err(e) => Some((Err(format!("write failed: {e}")), None)),
                    }
                }
            }
            None => None,
        };

        if let Some((status, reload)) = outcome {
            self.save_status = Some(status);
            if let Some(root) = reload {
                self.show_save = false;
                self.override_warnings = false;
                self.output_format = None;
                return Task::perform(async move { load::load_config(Some(root)) }, |state| {
                    Message::Loaded(Arc::new(state))
                });
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        view::view(self)
    }
}

/// Translate a raw window event into a [`Message`] (keyboard shortcuts).
fn handle_event(
    event: iced::Event,
    _status: iced::event::Status,
    _window: iced::window::Id,
) -> Option<Message> {
    use iced::keyboard::{Event as KeyEvent, Key};

    let iced::Event::Keyboard(KeyEvent::KeyPressed { key, modifiers, .. }) = event else {
        return None;
    };
    if !modifiers.command() {
        return None;
    }
    match key.as_ref() {
        Key::Character("z") if modifiers.shift() => Some(Message::Redo),
        Key::Character("z") => Some(Message::Undo),
        Key::Character("y") => Some(Message::Redo),
        Key::Character("s") => Some(Message::ToggleSave),
        _ => None,
    }
}

/// Resolve a persisted theme name to a [`Theme`] (defaults to Catppuccin Mocha).
fn theme_from_name(name: &str) -> Theme {
    Theme::ALL
        .iter()
        .find(|t| t.to_string() == name)
        .cloned()
        .unwrap_or(Theme::CatppuccinMocha)
}

/// Resolve a persisted format name to a [`ConfigFormat`].
fn format_from_name(name: &str) -> ConfigFormat {
    if name.eq_ignore_ascii_case("lua") {
        ConfigFormat::Lua
    } else {
        ConfigFormat::Conf
    }
}

/// The persisted name for a [`ConfigFormat`].
fn format_to_name(format: ConfigFormat) -> &'static str {
    match format {
        ConfigFormat::Lua => "lua",
        ConfigFormat::Conf => "conf",
    }
}
