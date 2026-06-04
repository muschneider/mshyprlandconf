//! All rendering. Pure functions over `&App` producing Iced elements.

use iced::widget::{button, column, container, row, scrollable, text, text_input, Column, Space};
use iced::{Alignment, Element, Length, Theme};

use hyprconf_core::conf::value_to_conf;
use hyprconf_core::schema::{CollectionId, OptionSpec};
use hyprconf_core::structured::{
    Animation, Bezier, EnvVar, Exec, ExecKind, Keybind, LayerRule, MonitorRule, Submap, Variable,
    WindowRule, WorkspaceRule,
};
use hyprconf_core::Value;

use crate::load::{format_label, LoadState, Loaded};
use crate::{fuzzy, App, Message, Selection};

const SIDEBAR_WIDTH: f32 = 240.0;

/// The whole window.
pub fn view(app: &App) -> Element<'_, Message> {
    column![top_bar(app), body(app), status_bar(app)].into()
}

fn top_bar(app: &App) -> Element<'_, Message> {
    row![
        text("hyprconf").size(20),
        text_input("Search options…", &app.search)
            .on_input(Message::SearchChanged)
            .padding(6)
            .width(Length::Fill),
        button(text(app.appearance.toggle_label())).on_press(Message::ThemeToggled),
    ]
    .spacing(12)
    .padding(10)
    .align_y(Alignment::Center)
    .into()
}

fn body(app: &App) -> Element<'_, Message> {
    row![sidebar(app), main_pane(app)]
        .height(Length::Fill)
        .into()
}

// ---------------------------------------------------------------------------
// sidebar
// ---------------------------------------------------------------------------

fn sidebar(app: &App) -> Element<'_, Message> {
    let mut items: Vec<Element<Message>> = Vec::new();
    items.push(group_header("SECTIONS"));
    for section in app.schema.sections() {
        let selection = Selection::Section(section.id.clone());
        let selected = app.selected == selection && app.search.trim().is_empty();
        items.push(nav_button(section.label.clone(), selection, selected));
    }

    items.push(Space::new().height(Length::Fixed(8.0)).into());
    items.push(group_header("COLLECTIONS"));
    for collection in app.schema.collections() {
        let count = collection_count(app, collection.id);
        let label = format!("{}  ({count})", collection.label);
        let selection = Selection::Collection(collection.id);
        let selected = app.selected == selection && app.search.trim().is_empty();
        items.push(nav_button(label, selection, selected));
    }

    let list = Column::with_children(items)
        .spacing(2)
        .padding(8)
        .width(Length::Fill);
    container(scrollable(list).height(Length::Fill))
        .width(Length::Fixed(SIDEBAR_WIDTH))
        .height(Length::Fill)
        .into()
}

fn group_header(label: &str) -> Element<'_, Message> {
    text(label.to_string()).size(11).style(muted).into()
}

fn nav_button(label: String, selection: Selection, selected: bool) -> Element<'static, Message> {
    let style: fn(&Theme, button::Status) -> button::Style = if selected {
        button::primary
    } else {
        button::text
    };
    button(text(label).size(14))
        .width(Length::Fill)
        .padding([6, 10])
        .on_press(Message::Selected(selection))
        .style(style)
        .into()
}

fn collection_count(app: &App, id: CollectionId) -> usize {
    let Some(loaded) = app.load.loaded() else {
        return 0;
    };
    let c = &loaded.config;
    match id {
        CollectionId::Monitors => c.monitors.len(),
        CollectionId::Workspaces => c.workspaces.len(),
        CollectionId::WindowRules => c.window_rules.len(),
        CollectionId::LayerRules => c.layer_rules.len(),
        CollectionId::Keybinds => c.keybinds.len(),
        CollectionId::Submaps => c.submaps.len(),
        CollectionId::Env => c.env.len(),
        CollectionId::Execs => c.execs.len(),
        CollectionId::Variables => c.variables.len(),
        CollectionId::Beziers => c.beziers.len(),
        CollectionId::Animations => c.animations.len(),
    }
}

// ---------------------------------------------------------------------------
// main pane
// ---------------------------------------------------------------------------

fn main_pane(app: &App) -> Element<'_, Message> {
    let inner: Element<Message> = match &app.load {
        LoadState::Loading => centered(text("Loading configuration…").size(16)),
        LoadState::NotFound { searched } => not_found_view(searched),
        LoadState::Error { path, message } => error_view(&path.display().to_string(), message),
        LoadState::Loaded(loaded) => {
            if app.search.trim().is_empty() {
                match &app.selected {
                    Selection::Section(id) => section_view(app, loaded, id),
                    Selection::Collection(id) => collection_view(app, loaded, *id),
                }
            } else {
                search_results(app, loaded)
            }
        }
    };

    container(inner)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(16)
        .into()
}

fn centered(content: impl Into<Element<'static, Message>>) -> Element<'static, Message> {
    container(content.into()).center(Length::Fill).into()
}

fn not_found_view(searched: &[std::path::PathBuf]) -> Element<'static, Message> {
    let mut lines: Vec<Element<Message>> = vec![
        text("No Hyprland configuration found").size(18).into(),
        text("Looked for:").size(13).style(muted).into(),
    ];
    for path in searched {
        lines.push(text(format!("  • {}", path.display())).size(13).into());
    }
    lines.push(
        text("Pass --config <path> to load a specific file.")
            .size(13)
            .style(muted)
            .into(),
    );
    centered(Column::with_children(lines).spacing(6))
}

fn error_view(path: &str, message: &str) -> Element<'static, Message> {
    centered(
        Column::with_children(vec![
            text("⚠  Failed to load configuration").size(18).into(),
            text(path.to_string()).size(13).style(muted).into(),
            text(message.to_string()).size(13).into(),
        ])
        .spacing(6),
    )
}

fn section_view(app: &App, loaded: &Loaded, id: &str) -> Element<'static, Message> {
    let Some(section) = app.schema.section(id) else {
        return centered(text("Unknown section"));
    };

    let mut items: Vec<Element<Message>> = vec![
        text(section.label.clone()).size(20).into(),
        text(section.description.clone())
            .size(13)
            .style(muted)
            .into(),
        Space::new().height(Length::Fixed(8.0)).into(),
    ];
    for opt in &section.options {
        items.push(option_row(opt, loaded));
    }

    scroll(items)
}

fn option_row(opt: &OptionSpec, loaded: &Loaded) -> Element<'static, Message> {
    let (display, is_default) = match loaded.config.get(&opt.path) {
        Some(value) => (render_value(value), false),
        None => (render_value(&opt.default), true),
    };

    let value_widget = if is_default {
        text(format!("{display}   (default)")).style(muted)
    } else {
        text(display)
    };

    let header = row![
        text(opt.label.clone()).width(Length::FillPortion(2)),
        value_widget.width(Length::FillPortion(3)),
    ]
    .spacing(12);

    column![header, text(opt.path.clone()).size(11).style(muted)]
        .spacing(1)
        .into()
}

fn collection_view(app: &App, loaded: &Loaded, id: CollectionId) -> Element<'static, Message> {
    let label = app
        .schema
        .collection(id)
        .map(|c| c.label.clone())
        .unwrap_or_default();
    let description = app
        .schema
        .collection(id)
        .map(|c| c.description.clone())
        .unwrap_or_default();

    let lines = collection_lines(loaded, id);

    let mut items: Vec<Element<Message>> = vec![
        text(label).size(20).into(),
        text(description).size(13).style(muted).into(),
        Space::new().height(Length::Fixed(8.0)).into(),
    ];

    if lines.is_empty() {
        items.push(text("No entries.").size(13).style(muted).into());
    } else {
        for line in lines {
            items.push(text(line).size(13).into());
        }
    }

    scroll(items)
}

fn search_results(app: &App, loaded: &Loaded) -> Element<'static, Message> {
    let query = app.search.trim();

    let mut scored: Vec<(i32, &str, &OptionSpec)> = Vec::new();
    for section in app.schema.sections() {
        for opt in &section.options {
            if let Some(score) = fuzzy::option_score(query, &opt.label, &opt.path, &opt.description)
            {
                scored.push((score, section.id.as_str(), opt));
            }
        }
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.2.path.cmp(&b.2.path)));

    let mut items: Vec<Element<Message>> = vec![text(format!(
        "{} option{} matching “{query}”",
        scored.len(),
        if scored.len() == 1 { "" } else { "s" }
    ))
    .size(13)
    .style(muted)
    .into()];

    if scored.is_empty() {
        items.push(text("No matches.").size(14).into());
    }

    for (_score, section_id, opt) in scored.into_iter().take(300) {
        let value = match loaded.config.get(&opt.path) {
            Some(v) => render_value(v),
            None => render_value(&opt.default),
        };
        let label = format!("{} › {}", section_id, opt.label);
        let row = row![
            text(label).width(Length::FillPortion(3)),
            text(value).width(Length::FillPortion(2)).style(muted),
        ]
        .spacing(12);
        items.push(
            button(row)
                .width(Length::Fill)
                .padding([4, 6])
                .on_press(Message::Selected(Selection::Section(
                    section_id.to_string(),
                )))
                .style(button::text)
                .into(),
        );
    }

    scroll(items)
}

/// Wrap a list of elements in a vertical, scrollable, fill-width column.
fn scroll(items: Vec<Element<'static, Message>>) -> Element<'static, Message> {
    scrollable(Column::with_children(items).spacing(6).width(Length::Fill))
        .height(Length::Fill)
        .into()
}

// ---------------------------------------------------------------------------
// status bar
// ---------------------------------------------------------------------------

fn status_bar(app: &App) -> Element<'_, Message> {
    let label = match &app.load {
        LoadState::Loading => "Loading…".to_string(),
        LoadState::NotFound { .. } => "No configuration loaded".to_string(),
        LoadState::Error { .. } => "Error loading configuration".to_string(),
        LoadState::Loaded(loaded) => {
            let includes = if loaded.included_files > 0 {
                format!(" (+{} included)", loaded.included_files)
            } else {
                String::new()
            };
            let warnings = if loaded.warnings > 0 {
                format!("  ·  {} warnings", loaded.warnings)
            } else {
                String::new()
            };
            format!(
                "{}  ·  {}{includes}  ·  {} options set{warnings}",
                format_label(loaded.format),
                loaded.source.display(),
                loaded.config.option_count(),
            )
        }
    };

    container(text(label).size(12).style(muted))
        .width(Length::Fill)
        .padding([4, 10])
        .into()
}

// ---------------------------------------------------------------------------
// value & collection rendering
// ---------------------------------------------------------------------------

fn render_value(value: &Value) -> String {
    value_to_conf(value)
}

fn collection_lines(loaded: &Loaded, id: CollectionId) -> Vec<String> {
    let c = &loaded.config;
    match id {
        CollectionId::Monitors => c.monitors.iter().map(|t| monitor_line(&t.value)).collect(),
        CollectionId::Workspaces => c
            .workspaces
            .iter()
            .map(|t| workspace_line(&t.value))
            .collect(),
        CollectionId::WindowRules => c
            .window_rules
            .iter()
            .map(|t| window_rule_line(&t.value))
            .collect(),
        CollectionId::LayerRules => c
            .layer_rules
            .iter()
            .map(|t| layer_rule_line(&t.value))
            .collect(),
        CollectionId::Keybinds => c.keybinds.iter().map(|t| keybind_line(&t.value)).collect(),
        CollectionId::Submaps => c.submaps.iter().map(|t| submap_line(&t.value)).collect(),
        CollectionId::Env => c.env.iter().map(|t| env_line(&t.value)).collect(),
        CollectionId::Execs => c.execs.iter().map(|t| exec_line(&t.value)).collect(),
        CollectionId::Variables => c
            .variables
            .iter()
            .map(|t| variable_line(&t.value))
            .collect(),
        CollectionId::Beziers => c.beziers.iter().map(|t| bezier_line(&t.value)).collect(),
        CollectionId::Animations => c
            .animations
            .iter()
            .map(|t| animation_line(&t.value))
            .collect(),
    }
}

fn keybind_line(k: &Keybind) -> String {
    let mods = if k.mods.is_empty() {
        String::new()
    } else {
        format!("{} ", k.mods)
    };
    let args = if k.args.is_empty() {
        String::new()
    } else {
        format!(" {}", k.args)
    };
    let submap = k
        .submap
        .as_ref()
        .map(|s| format!("   [submap: {s}]"))
        .unwrap_or_default();
    format!(
        "{} · {mods}{} → {}{args}{submap}",
        k.flags.keyword(),
        k.key,
        k.dispatcher,
    )
}

fn window_rule_line(r: &WindowRule) -> String {
    let kw = if r.v2 { "windowrulev2" } else { "windowrule" };
    format!("{kw} · {}  ⟵  {}", r.rule, r.matchers)
}

fn layer_rule_line(r: &LayerRule) -> String {
    format!("{}  ⟵  {}", r.rule, r.namespace)
}

fn monitor_line(m: &MonitorRule) -> String {
    let name = if m.name.is_empty() { "(all)" } else { &m.name };
    let extra = if m.extra.is_empty() {
        String::new()
    } else {
        format!("  {}", m.extra.join(", "))
    };
    format!("{name}: {} @ {} ×{}{extra}", m.mode, m.position, m.scale)
}

fn workspace_line(w: &WorkspaceRule) -> String {
    format!("{}: {}", w.selector, w.rules)
}

fn env_line(e: &EnvVar) -> String {
    format!("{} = {}", e.name, e.value)
}

fn exec_line(e: &Exec) -> String {
    let kind = match e.kind {
        ExecKind::Exec => "exec",
        ExecKind::ExecOnce => "exec-once",
        ExecKind::ExecShutdown => "exec-shutdown",
    };
    format!("{kind} · {}", e.command)
}

fn submap_line(s: &Submap) -> String {
    s.name.clone()
}

fn variable_line(v: &Variable) -> String {
    format!("${} = {}", v.name, v.value)
}

fn bezier_line(b: &Bezier) -> String {
    format!(
        "{}: ({}, {}) ({}, {})",
        b.name, b.p0.x, b.p0.y, b.p1.x, b.p1.y
    )
}

fn animation_line(a: &Animation) -> String {
    let onoff = if a.enabled { "on" } else { "off" };
    let style = a
        .style
        .as_deref()
        .map(|s| format!(", {s}"))
        .unwrap_or_default();
    format!("{}: {onoff}, speed {}, {}{style}", a.name, a.speed, a.curve)
}

/// Muted text style derived from the active theme.
fn muted(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(theme.palette().text.scale_alpha(0.6)),
    }
}
