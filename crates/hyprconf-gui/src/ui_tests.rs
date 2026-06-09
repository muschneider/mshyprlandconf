// SPDX-License-Identifier: MIT OR Apache-2.0
//! Headless UI tests: drive the real `App` (boot → `update` → `view`) through
//! the key flows without opening a window or a renderer. `update` exercises all
//! application logic; calling `view` builds the full Iced element tree (which is
//! renderer-independent), so these catch panics and state regressions across the
//! load → edit → add keybind → choose format → preview → save cycle.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Once};

use hyprconf_core::schema::CollectionId;
use hyprconf_core::{ConfigFormat, HyprlandInfo, Value};

use crate::color_picker::ColorTarget;
use crate::edit::{CollectionAction, EditAction, KeybindEdit};
use crate::load::{self, LoadState};
use crate::settings::Settings;
use crate::{App, Message, Selection};

/// Point XDG dirs at a throwaway location so `Settings`/profile writes during
/// `update` never touch the developer's real config. Set once per test binary.
fn init_xdg() {
    static XDG: Once = Once::new();
    XDG.call_once(|| {
        let base = std::env::temp_dir().join(format!("hyprconf-uitest-xdg-{}", std::process::id()));
        std::env::set_var("XDG_CONFIG_HOME", base.join("config"));
        std::env::set_var("XDG_DATA_HOME", base.join("data"));
    });
}

/// A unique temp file path (its parent directory is created).
fn temp_path(tag: &str, name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir =
        std::env::temp_dir().join(format!("hyprconf-ui-{tag}-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

/// Boot the app and synchronously complete the initial load (boot's load runs in
/// a `Task` we don't have a runtime for, so we feed the result directly).
fn boot_loaded(path: &Path) -> App {
    init_xdg();
    let (mut app, _task) = App::boot(Some(path.to_path_buf()), Settings::default());
    let state = load::load_config(Some(path.to_path_buf()));
    let _ = app.update(Message::Loaded(Arc::new(state)));
    app
}

#[test]
fn full_flow_load_edit_addbind_convert_preview_save() {
    // 1. Load a real (temp) .conf.
    let path = temp_path("flow", "hyprland.conf");
    std::fs::write(
        &path,
        "decoration:rounding = 4\nbind = SUPER, Q, killactive\n",
    )
    .unwrap();
    let mut app = boot_loaded(&path);
    assert!(matches!(app.load, LoadState::Loaded(_)));
    assert_eq!(
        app.load.loaded().unwrap().config.get("decoration:rounding"),
        Some(&Value::Int(4))
    );

    // 2. Edit a scalar.
    let _ = app.update(Message::Edit(EditAction::SetIntSlider(
        "decoration:rounding".into(),
        12,
    )));
    let loaded = app.load.loaded().unwrap();
    assert_eq!(
        loaded.config.get("decoration:rounding"),
        Some(&Value::Int(12))
    );
    assert!(loaded.is_dirty("decoration:rounding"));

    // 3. Add a keybind and give it a key.
    let before = app.load.loaded().unwrap().config.keybinds.len();
    let _ = app.update(Message::CollectionEdit(CollectionAction::Add(
        CollectionId::Keybinds,
    )));
    let _ = app.update(Message::CollectionEdit(CollectionAction::Keybind(
        before,
        KeybindEdit::Key("T".into()),
    )));
    assert_eq!(app.load.loaded().unwrap().config.keybinds.len(), before + 1);

    // 4. Choose Lua output and open the save panel (preview).
    let _ = app.update(Message::SetOutputFormat(ConfigFormat::Lua));
    let _ = app.update(Message::ToggleSave);
    assert!(app.show_save);
    // Building the view must not panic (renders the conversion diff/preview).
    let _ = crate::view::view(&app);

    // 5. Save — converts to a sibling hyprland.lua.
    let _ = app.update(Message::PerformSave);
    assert!(
        matches!(app.save_status, Some(Ok(_))),
        "save status: {:?}",
        app.save_status
    );
    let lua_path = path.with_extension("lua");
    let written = std::fs::read_to_string(&lua_path).expect("hyprland.lua written");
    assert!(written.contains("hl.bind"), "{written}");
    assert!(written.contains("rounding = 12"), "{written}");

    // 6. Reload the written Lua and confirm the full cycle preserved everything.
    let reloaded = load::load_config(Some(lua_path.clone()));
    let _ = app.update(Message::Loaded(Arc::new(reloaded)));
    let l = app.load.loaded().unwrap();
    assert_eq!(l.format, ConfigFormat::Lua);
    assert_eq!(l.config.get("decoration:rounding"), Some(&Value::Int(12)));
    assert!(l.config.keybinds.iter().any(|t| t.value.key == "T"));

    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn undo_redo_round_trips_a_scalar_edit() {
    let path = temp_path("undo", "hyprland.conf");
    std::fs::write(&path, "decoration:rounding = 4\n").unwrap();
    let mut app = boot_loaded(&path);

    let _ = app.update(Message::Edit(EditAction::SetIntSlider(
        "decoration:rounding".into(),
        20,
    )));
    assert_eq!(
        app.load.loaded().unwrap().config.get("decoration:rounding"),
        Some(&Value::Int(20))
    );

    let _ = app.update(Message::Undo);
    assert_eq!(
        app.load.loaded().unwrap().config.get("decoration:rounding"),
        Some(&Value::Int(4)),
        "undo restores the loaded value"
    );

    let _ = app.update(Message::Redo);
    assert_eq!(
        app.load.loaded().unwrap().config.get("decoration:rounding"),
        Some(&Value::Int(20)),
        "redo re-applies the edit"
    );

    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn views_build_headlessly_across_panels() {
    let path = temp_path("views", "hyprland.conf");
    std::fs::write(
        &path,
        "decoration:rounding = 4\ngeneral:col.active_border = rgba(33ccffee) rgba(00ff99ee) 45deg\n",
    )
    .unwrap();
    let mut app = boot_loaded(&path);

    // Search.
    let _ = app.update(Message::SearchChanged("round".into()));
    let _ = crate::view::view(&app);
    let _ = app.update(Message::SearchChanged(String::new()));

    // Navigate to a collection and back to a section.
    let _ = app.update(Message::Selected(Selection::Collection(
        CollectionId::Keybinds,
    )));
    let _ = crate::view::view(&app);
    let _ = app.update(Message::Selected(Selection::Section("decoration".into())));
    let _ = crate::view::view(&app);

    // Pending-changes and profiles panels.
    let _ = app.update(Message::ToggleChanges);
    let _ = crate::view::view(&app);
    let _ = app.update(Message::ToggleProfiles);
    let _ = crate::view::view(&app);

    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn color_picker_open_pick_and_close() {
    let path = temp_path("color", "hyprland.conf");
    std::fs::write(&path, "decoration:shadow:color = rgba(11223344)\n").unwrap();
    let mut app = boot_loaded(&path);

    // Open the picker for a scalar color option; the modal must build.
    let _ = app.update(Message::OpenColorPicker("decoration:shadow:color".into()));
    assert!(matches!(
        app.color_picker.as_ref().map(|c| &c.target),
        Some(ColorTarget::Option(_))
    ));
    let _ = crate::view::view(&app);

    // Drag the saturation/value area and the hue strip — the model updates live.
    let _ = app.update(Message::PickSatVal(0.5, 0.5));
    let _ = app.update(Message::PickHue(200.0));
    assert!(matches!(
        app.load
            .loaded()
            .unwrap()
            .config
            .get("decoration:shadow:color"),
        Some(Value::Color(_))
    ));

    // Open a gradient stop picker too.
    let _ = app.update(Message::OpenStopColorPicker(
        "general:col.active_border".into(),
        0,
    ));
    let _ = crate::view::view(&app);

    let _ = app.update(Message::CloseColorPicker);
    assert!(app.color_picker.is_none());

    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn live_apply_gated_on_detection() {
    let path = temp_path("live", "hyprland.conf");
    std::fs::write(&path, "decoration:rounding = 4\n").unwrap();
    let mut app = boot_loaded(&path);

    // Without a detected Hyprland, live-apply cannot be turned on.
    let _ = app.update(Message::ToggleLiveApply(true));
    assert!(!app.live_apply);

    // With one detected, it can — and a subsequent edit builds its (unpolled)
    // hyprctl task without panicking.
    let _ = app.update(Message::HyprlandDetected(Some(HyprlandInfo {
        version: "0.55.2".into(),
        tag: None,
    })));
    let _ = app.update(Message::ToggleLiveApply(true));
    assert!(app.live_apply);
    let _ = app.update(Message::Edit(EditAction::SetIntSlider(
        "decoration:rounding".into(),
        6,
    )));
    assert_eq!(
        app.load.loaded().unwrap().config.get("decoration:rounding"),
        Some(&Value::Int(6))
    );

    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}
