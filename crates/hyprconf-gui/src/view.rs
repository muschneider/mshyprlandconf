//! All rendering. Pure functions over `&App` producing Iced elements.

use iced::widget::{
    button, column, container, pick_list, row, scrollable, text, text_input, Column, Space,
};
use iced::{Alignment, Background, Border, Element, Font, Length, Theme};

use hyprconf_core::conf::value_to_conf;
use hyprconf_core::schema::{CollectionId, OptionSpec};
use hyprconf_core::structured::{
    Animation, Bezier, EnvVar, Exec, ExecKind, Keybind, LayerRule, MonitorRule, Submap, Variable,
    WindowRule, WorkspaceRule,
};
use hyprconf_core::Value;

use crate::load::{format_label, LoadState, Loaded};
use crate::{fuzzy, App, Message, Selection};

const SIDEBAR_WIDTH: f32 = 260.0;
const BOLD: Font = Font {
    weight: iced::font::Weight::Bold,
    ..Font::DEFAULT
};

/// The whole window: header / [sidebar | content] / status bar.
pub fn view(app: &App) -> Element<'_, Message> {
    column![
        header(app),
        row![sidebar(app), content(app)].height(Length::Fill),
        status_bar(app),
    ]
    .into()
}

// ---------------------------------------------------------------------------
// header
// ---------------------------------------------------------------------------

fn header(app: &App) -> Element<'_, Message> {
    let brand = row![
        text("❖").size(22).style(accent),
        text("hyprconf").size(20).font(BOLD),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let search = text_input("Search options…", &app.search)
        .on_input(Message::SearchChanged)
        .padding([8, 12])
        .size(15)
        .width(Length::Fill);

    let theme_picker = row![
        text("Theme").size(13).style(muted),
        pick_list(Theme::ALL, Some(app.theme.clone()), Message::ThemeSelected)
            .text_size(13)
            .padding([6, 10])
            .width(Length::Fixed(170.0)),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    container(
        row![brand, search, theme_picker]
            .spacing(20)
            .align_y(Alignment::Center),
    )
    .padding([12, 18])
    .width(Length::Fill)
    .style(bar_style)
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
        items.push(nav_button(
            section_icon(&section.id),
            section.label.clone(),
            selection,
            selected,
            None,
        ));
    }

    items.push(Space::new().height(Length::Fixed(14.0)).into());
    items.push(group_header("COLLECTIONS"));
    for collection in app.schema.collections() {
        let count = collection_count(app, collection.id);
        let selection = Selection::Collection(collection.id);
        let selected = app.selected == selection && app.search.trim().is_empty();
        items.push(nav_button(
            collection_icon(collection.id),
            collection.label.clone(),
            selection,
            selected,
            Some(count),
        ));
    }

    let list = Column::with_children(items)
        .spacing(3)
        .padding([12, 10])
        .width(Length::Fill);

    container(scrollable(list).height(Length::Fill))
        .width(Length::Fixed(SIDEBAR_WIDTH))
        .height(Length::Fill)
        .style(panel_style)
        .into()
}

fn group_header(label: &str) -> Element<'_, Message> {
    container(text(label.to_string()).size(11).font(BOLD).style(muted))
        .padding([8, 8])
        .into()
}

fn nav_button(
    icon: &'static str,
    label: String,
    selection: Selection,
    selected: bool,
    badge: Option<usize>,
) -> Element<'static, Message> {
    let mut inner = row![text(icon).size(15), text(label).size(14)]
        .spacing(10)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    if let Some(count) = badge {
        inner = inner.push(count_badge(count));
    }

    button(inner)
        .width(Length::Fill)
        .padding([7, 12])
        .on_press(Message::Selected(selection))
        .style(move |theme: &Theme, status| nav_style(theme, status, selected))
        .into()
}

fn count_badge(count: usize) -> Element<'static, Message> {
    container(text(count.to_string()).size(11))
        .padding([1, 7])
        .style(badge_style)
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
// content area
// ---------------------------------------------------------------------------

fn content(app: &App) -> Element<'_, Message> {
    let inner: Element<Message> = match &app.load {
        LoadState::Loading => centered(text("Loading configuration…").size(16).style(muted)),
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
        .padding(20)
        .into()
}

fn centered(content: impl Into<Element<'static, Message>>) -> Element<'static, Message> {
    container(content.into()).center(Length::Fill).into()
}

fn not_found_view(searched: &[std::path::PathBuf]) -> Element<'static, Message> {
    let mut lines: Vec<Element<Message>> = vec![
        text("🔍").size(40).into(),
        text("No Hyprland configuration found")
            .size(20)
            .font(BOLD)
            .into(),
        text("Looked for:").size(13).style(muted).into(),
    ];
    for path in searched {
        lines.push(text(format!("• {}", path.display())).size(13).into());
    }
    lines.push(Space::new().height(Length::Fixed(8.0)).into());
    lines.push(
        text("Pass --config <path> to load a specific file.")
            .size(13)
            .style(muted)
            .into(),
    );
    centered(
        Column::with_children(lines)
            .spacing(8)
            .align_x(Alignment::Center),
    )
}

fn error_view(path: &str, message: &str) -> Element<'static, Message> {
    centered(
        Column::with_children(vec![
            text("⚠").size(40).style(danger).into(),
            text("Failed to load configuration")
                .size(20)
                .font(BOLD)
                .into(),
            text(path.to_string()).size(13).style(muted).into(),
            text(message.to_string()).size(13).style(danger).into(),
        ])
        .spacing(8)
        .align_x(Alignment::Center),
    )
}

fn pane_header(
    icon: &str,
    title: String,
    subtitle: String,
    trailing: String,
) -> Element<'static, Message> {
    let mut left = row![text(icon.to_string()).size(26)]
        .spacing(12)
        .align_y(Alignment::Center);
    left = left.push(
        column![
            text(title).size(22).font(BOLD),
            text(subtitle).size(13).style(muted),
        ]
        .spacing(2),
    );

    container(
        row![
            left.width(Length::Fill),
            text(trailing).size(13).style(muted),
        ]
        .align_y(Alignment::Center),
    )
    .padding([14, 18])
    .width(Length::Fill)
    .style(card_style)
    .into()
}

fn section_view(app: &App, loaded: &Loaded, id: &str) -> Element<'static, Message> {
    let Some(section) = app.schema.section(id) else {
        return centered(text("Unknown section"));
    };

    let set = section
        .options
        .iter()
        .filter(|o| loaded.config.get(&o.path).is_some())
        .count();
    let trailing = format!("{set} set · {} total", section.options.len());

    let mut items: Vec<Element<Message>> = vec![
        pane_header(
            section_icon(id),
            section.label.clone(),
            section.description.clone(),
            trailing,
        ),
        Space::new().height(Length::Fixed(4.0)).into(),
    ];
    for opt in &section.options {
        items.push(option_card(opt, loaded));
    }

    scroll(items)
}

fn option_card(opt: &OptionSpec, loaded: &Loaded) -> Element<'static, Message> {
    let (display, is_default) = match loaded.config.get(&opt.path) {
        Some(value) => (render_value(value), false),
        None => (render_value(&opt.default), true),
    };

    let value_widget: Element<Message> = if is_default {
        row![
            text(display).size(14).style(muted),
            container(text("default").size(10).style(muted))
                .padding([1, 6])
                .style(badge_style),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    } else {
        text(display).size(14).style(accent).into()
    };

    let header = row![
        text(opt.label.clone())
            .size(15)
            .width(Length::FillPortion(2)),
        container(value_widget)
            .width(Length::FillPortion(3))
            .align_x(Alignment::End),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    container(column![header, text(opt.path.clone()).size(11).style(muted)].spacing(3))
        .padding([10, 14])
        .width(Length::Fill)
        .style(card_style)
        .into()
}

fn collection_view(app: &App, loaded: &Loaded, id: CollectionId) -> Element<'static, Message> {
    let (label, description) = app
        .schema
        .collection(id)
        .map(|c| (c.label.clone(), c.description.clone()))
        .unwrap_or_default();

    let lines = collection_lines(loaded, id);
    let trailing = format!(
        "{} entr{}",
        lines.len(),
        if lines.len() == 1 { "y" } else { "ies" }
    );

    let mut items: Vec<Element<Message>> = vec![
        pane_header(collection_icon(id), label, description, trailing),
        Space::new().height(Length::Fixed(4.0)).into(),
    ];

    if lines.is_empty() {
        items.push(
            container(
                text("No entries in this configuration.")
                    .size(13)
                    .style(muted),
            )
            .padding([10, 14])
            .into(),
        );
    } else {
        for line in lines {
            items.push(
                container(text(line).size(13))
                    .padding([8, 14])
                    .width(Length::Fill)
                    .style(card_style)
                    .into(),
            );
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
        items.push(text("No matches.").size(15).into());
    }

    for (_score, section_id, opt) in scored.into_iter().take(300) {
        let value = match loaded.config.get(&opt.path) {
            Some(v) => render_value(v),
            None => render_value(&opt.default),
        };
        let inner = row![
            row![
                text(section_icon(section_id)).size(13),
                text(opt.label.clone()).size(14)
            ]
            .spacing(8)
            .width(Length::FillPortion(3)),
            container(text(value).size(13).style(accent))
                .width(Length::FillPortion(2))
                .align_x(Alignment::End),
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        items.push(
            button(inner)
                .width(Length::Fill)
                .padding([8, 14])
                .on_press(Message::Selected(Selection::Section(
                    section_id.to_string(),
                )))
                .style(result_style)
                .into(),
        );
    }

    scroll(items)
}

/// A vertical, scrollable, fill-width column.
fn scroll(items: Vec<Element<'static, Message>>) -> Element<'static, Message> {
    scrollable(
        Column::with_children(items)
            .spacing(8)
            .width(Length::Fill)
            .padding([0, 8]),
    )
    .height(Length::Fill)
    .into()
}

// ---------------------------------------------------------------------------
// status bar
// ---------------------------------------------------------------------------

fn status_bar(app: &App) -> Element<'_, Message> {
    let content: Element<Message> = match &app.load {
        LoadState::Loading => text("Loading…").size(12).style(muted).into(),
        LoadState::NotFound { .. } => text("No configuration loaded").size(12).style(muted).into(),
        LoadState::Error { .. } => text("Error loading configuration")
            .size(12)
            .style(danger)
            .into(),
        LoadState::Loaded(loaded) => {
            let mut segs = row![
                container(text(format_label(loaded.format)).size(11).font(BOLD))
                    .padding([1, 8])
                    .style(format_badge_style),
                text(loaded.source.display().to_string())
                    .size(12)
                    .style(muted),
            ]
            .spacing(10)
            .align_y(Alignment::Center);

            if loaded.included_files > 0 {
                segs = segs.push(
                    text(format!("+{} included", loaded.included_files))
                        .size(12)
                        .style(muted),
                );
            }
            segs = segs.push(Space::new().width(Length::Fill));
            segs = segs.push(
                text(format!("{} options set", loaded.config.option_count()))
                    .size(12)
                    .style(muted),
            );
            if loaded.warnings > 0 {
                segs = segs.push(
                    text(format!("⚠ {} warnings", loaded.warnings))
                        .size(12)
                        .style(danger),
                );
            }
            segs.into()
        }
    };

    container(content)
        .width(Length::Fill)
        .padding([6, 16])
        .style(bar_style)
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
        k.dispatcher
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

// ---------------------------------------------------------------------------
// icons
// ---------------------------------------------------------------------------

fn section_icon(id: &str) -> &'static str {
    match id {
        "general" => "🪟",
        "decoration" => "🎨",
        "animations" => "✨",
        "input" => "⌨",
        "gestures" => "✋",
        "group" => "🗂",
        "misc" => "🧩",
        "binds" => "🎹",
        "dwindle" => "🌿",
        "master" => "📐",
        "xwayland" => "🩹",
        "cursor" => "🖱",
        "render" => "🖼",
        "debug" => "🐞",
        _ => "•",
    }
}

fn collection_icon(id: CollectionId) -> &'static str {
    match id {
        CollectionId::Monitors => "🖥",
        CollectionId::Workspaces => "🔳",
        CollectionId::WindowRules => "📏",
        CollectionId::LayerRules => "🧅",
        CollectionId::Keybinds => "⌨",
        CollectionId::Submaps => "🗺",
        CollectionId::Env => "🌐",
        CollectionId::Execs => "▶",
        CollectionId::Variables => "🔣",
        CollectionId::Beziers => "〰",
        CollectionId::Animations => "🎞",
    }
}

// ---------------------------------------------------------------------------
// theme-aware styles
// ---------------------------------------------------------------------------

fn accent(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(theme.extended_palette().primary.base.color),
    }
}

fn muted(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(theme.palette().text.scale_alpha(0.6)),
    }
}

fn danger(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(theme.extended_palette().danger.base.color),
    }
}

/// Header & status bars: a panel-tinted strip.
fn bar_style(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(p.background.weak.color.into()),
        ..container::Style::default()
    }
}

/// Sidebar panel.
fn panel_style(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(p.background.weak.color.into()),
        ..container::Style::default()
    }
}

/// A raised card on the base background.
fn card_style(theme: &Theme) -> container::Style {
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
fn badge_style(theme: &Theme) -> container::Style {
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
fn format_badge_style(theme: &Theme) -> container::Style {
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
fn nav_style(theme: &Theme, status: button::Status, selected: bool) -> button::Style {
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
fn result_style(theme: &Theme, status: button::Status) -> button::Style {
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
