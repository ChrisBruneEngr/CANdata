//! Signal set controller — business logic for save/apply/clear/delete operations.

use crate::app::CanViewerApp;
use crate::library::signal_sets::{
    SignalSet, build_selected_signals_from_set, layout_from_set, parse_signal_id,
    save_signal_set_store, saved_lanes_from_layout,
};
use gpui::Context;

/// Save the current `selected_signals` and lanes as a new signal set under the active library.
/// Validates name, requires an active library, requires ≥1 parseable signal.
pub fn save_current_selection_as_signal_set(
    app: &mut CanViewerApp,
    name: &str,
    cx: &mut Context<CanViewerApp>,
) {
    let name = name.trim().to_string();
    if name.is_empty() {
        app.status_msg = "Set name cannot be empty".into();
        cx.notify();
        return;
    }

    let library_id = match &app.app_config.active_library_id {
        Some(id) => id.clone(),
        None => {
            app.status_msg = "Activate a signal library first".into();
            cx.notify();
            return;
        }
    };

    // Parse selected_signals → entries (silently skip unparseable ones)
    let mut entries: Vec<_> = Vec::new();
    for sig_id in &app.selected_signals {
        match parse_signal_id(sig_id) {
            Some(entry) => entries.push(entry),
            None => eprintln!("⚠️  Skipping unparseable signal_id: {}", sig_id),
        }
    }
    if entries.is_empty() {
        app.status_msg = "No valid signals to save".into();
        cx.notify();
        return;
    }

    // Check duplicate name
    let sets = app
        .signal_set_store
        .sets_by_library
        .entry(library_id.clone())
        .or_default();
    if sets.iter().any(|s| s.name == name) {
        app.status_msg = format!("Set '{}' already exists", name).into();
        cx.notify();
        return;
    }

    let count = entries.len();
    sets.push(SignalSet {
        name: name.clone(),
        entries,
        lanes: saved_lanes_from_layout(&app.plot_lanes),
    });
    if let Err(e) = save_signal_set_store(&app.signal_set_store, app.config_file_path.as_deref()) {
        eprintln!("❌ Failed to save signal sets: {}", e);
        app.status_msg = format!("Warning: could not save signal set to disk ({})", e).into();
    } else {
        app.status_msg = format!("Saved set '{}' ({} signals)", name, count).into();
    }
    app.show_save_set_input = false;
    app.pending_signal_set_name = None;
    cx.notify();
}

/// Apply a named set to `selected_signals` and the lanes, and trigger plot refresh.
pub fn apply_signal_set(
    app: &mut CanViewerApp,
    library_id: &str,
    set_name: &str,
    cx: &mut Context<CanViewerApp>,
) {
    let library = match app.library_manager.find_library(library_id) {
        Some(lib) => lib.clone(),
        None => {
            app.status_msg = "Library not found".into();
            cx.notify();
            return;
        }
    };
    let set = match app
        .signal_set_store
        .sets_by_library
        .get(library_id)
        .and_then(|sets| sets.iter().find(|s| s.name == set_name))
    {
        Some(s) => s.clone(),
        None => {
            app.status_msg = "Set not found".into();
            cx.notify();
            return;
        }
    };

    app.selected_signals.clear();
    let rebuilt = build_selected_signals_from_set(&set, library.channel_type);
    app.selected_signals.extend(rebuilt);
    app.plot_lanes = layout_from_set(&set, library.channel_type);
    app.active_signal_set = Some((library_id.to_string(), set_name.to_string()));

    // Trigger plot refresh
    crate::ui::views::chart_view::extract_and_update_series_data(app);

    app.status_msg = format!("Loaded set '{}' ({} signals)", set_name, set.entries.len()).into();
    cx.notify();
}

/// Clear the active set selection AND clear selected_signals.
/// Triggered by the dropdown's "✕" item.
pub fn clear_active_signal_set(app: &mut CanViewerApp, cx: &mut Context<CanViewerApp>) {
    app.active_signal_set = None;
    app.selected_signals.clear();
    crate::ui::views::chart_view::extract_and_update_series_data(app);
    app.status_msg = "Set selection cleared".into();
    cx.notify();
}

/// Delete a set from the store (no UI in this spec; symmetric API).
pub fn delete_signal_set(
    app: &mut CanViewerApp,
    library_id: &str,
    set_name: &str,
    cx: &mut Context<CanViewerApp>,
) {
    let mut removed = false;
    if let Some(sets) = app.signal_set_store.sets_by_library.get_mut(library_id) {
        if let Some(pos) = sets.iter().position(|s| s.name == set_name) {
            sets.remove(pos);
            removed = true;
        }
    }

    if !removed {
        app.status_msg = "Set not found".into();
        cx.notify();
        return;
    }

    let _ = save_signal_set_store(&app.signal_set_store, app.config_file_path.as_deref());
    // Only clear the active set if the deleted set WAS the active set.
    if let Some((lid, sname)) = &app.active_signal_set {
        if lid == library_id && sname == set_name {
            app.active_signal_set = None;
        }
    }
    app.status_msg = format!("Deleted set '{}'", set_name).into();
    cx.notify();
}
