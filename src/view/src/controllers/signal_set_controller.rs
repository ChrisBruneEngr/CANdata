//! Signal set controller — business logic for save/apply/clear/delete operations.

use crate::app::CanDataApp;
use crate::library::signal_sets::{
    SavedLane, SavedWindow, SignalSet, SignalSetEntry, parse_signal_id, save_signal_set_store,
    saved_windows, windows_from_set, windows_match_set,
};
use gpui::Context;

/// Save the current `selected_signals` and lanes as a new signal set under the active library.
/// Validates name, requires an active library, requires ≥1 parseable signal.
pub fn save_current_selection_as_signal_set(
    app: &mut CanDataApp,
    name: &str,
    cx: &mut Context<CanDataApp>,
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

    let entries = entries_from_selection(app);
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
    let (lanes, windows) = saved_layout(app);
    let sets = app
        .signal_set_store
        .sets_by_library
        .entry(library_id.clone())
        .or_default();
    sets.push(SignalSet { name: name.clone(), entries, lanes, windows });
    if let Err(e) = save_signal_set_store(&app.signal_set_store, app.config_file_path.as_deref()) {
        eprintln!("❌ Failed to save signal sets: {}", e);
        app.status_msg = format!("Warning: could not save signal set to disk ({})", e).into();
    } else {
        app.status_msg = format!("Saved set '{}' ({} signals)", name, count).into();
    }
    // Carry on editing under the new name.
    app.active_signal_set = Some((library_id, name));
    app.show_save_set_input = false;
    app.pending_signal_set_name = None;
    cx.notify();
}

/// The windows to save, and a copy of the first window's lanes for older
/// versions of the app.
fn saved_layout(app: &CanDataApp) -> (Vec<SavedLane>, Vec<SavedWindow>) {
    let windows = saved_windows(&app.plot_windows, &app.plot_lanes);
    let first_lanes = windows.first().map(|w| w.lanes.clone()).unwrap_or_default();
    (first_lanes, windows)
}

/// The selected signals as set entries. Unparseable IDs are skipped.
fn entries_from_selection(app: &CanDataApp) -> Vec<SignalSetEntry> {
    app.selected_signals
        .iter()
        .filter_map(|sig_id| {
            let entry = parse_signal_id(sig_id);
            if entry.is_none() {
                eprintln!("⚠️  Skipping unparseable signal_id: {}", sig_id);
            }
            entry
        })
        .collect()
}

/// The active set, if it belongs to the active library: its name, and whether
/// the lanes on screen differ from what is saved.
pub fn active_set_status(app: &CanDataApp) -> Option<(String, bool)> {
    let (library_id, set_name) = app.active_signal_set.as_ref()?;
    if app.app_config.active_library_id.as_ref() != Some(library_id) {
        return None;
    }
    let set = app
        .signal_set_store
        .sets_by_library
        .get(library_id)?
        .iter()
        .find(|s| &s.name == set_name)?;
    let channel_type = app.library_manager.find_library(library_id)?.channel_type;
    let modified = !windows_match_set(&app.plot_windows, &app.plot_lanes, set, channel_type);
    Some((set_name.clone(), modified))
}

/// Overwrite the active set with the current signals and lanes, in memory.
/// Returns the set's name and signal count. Context-free for tests.
pub fn update_active_signal_set_state(app: &mut CanDataApp) -> Result<(String, usize), &'static str> {
    let (library_id, set_name) = app.active_signal_set.clone().ok_or("No signal set loaded")?;
    if app.app_config.active_library_id.as_ref() != Some(&library_id) {
        return Err("The loaded set belongs to another library");
    }
    let entries = entries_from_selection(app);
    if entries.is_empty() {
        return Err("No valid signals to save");
    }
    let (lanes, windows) = saved_layout(app);
    let set = app
        .signal_set_store
        .sets_by_library
        .get_mut(&library_id)
        .and_then(|sets| sets.iter_mut().find(|s| s.name == set_name))
        .ok_or("Set not found")?;
    let count = entries.len();
    set.entries = entries;
    set.lanes = lanes;
    set.windows = windows;
    Ok((set_name, count))
}

/// Save the current signals and lanes over the active set, and write the file.
pub fn update_active_signal_set(app: &mut CanDataApp, cx: &mut Context<CanDataApp>) {
    app.status_msg = match update_active_signal_set_state(app) {
        Ok((name, count)) => {
            match save_signal_set_store(&app.signal_set_store, app.config_file_path.as_deref()) {
                Ok(()) => format!("Updated set '{}' ({} signals)", name, count).into(),
                Err(e) => {
                    eprintln!("❌ Failed to save signal sets: {}", e);
                    format!("Warning: could not save signal set to disk ({})", e).into()
                }
            }
        }
        Err(msg) => msg.into(),
    };
    cx.notify();
}

/// Apply a named set to `selected_signals` and the lanes, and trigger plot refresh.
pub fn apply_signal_set(
    app: &mut CanDataApp,
    library_id: &str,
    set_name: &str,
    cx: &mut Context<CanDataApp>,
) {
    app.status_msg = match apply_signal_set_state(app, library_id, set_name) {
        Ok(count) => format!("Loaded set '{}' ({} signals)", set_name, count).into(),
        Err(msg) => msg.into(),
    };
    cx.notify();
}

/// Apply a named set to `selected_signals` and the lanes, mark it active and
/// refresh the plot data. Returns the number of signals in the set.
/// Context-free so it can run from a file load and from tests.
pub fn apply_signal_set_state(
    app: &mut CanDataApp,
    library_id: &str,
    set_name: &str,
) -> Result<usize, &'static str> {
    let channel_type = app
        .library_manager
        .find_library(library_id)
        .ok_or("Library not found")?
        .channel_type;
    let set = app
        .signal_set_store
        .sets_by_library
        .get(library_id)
        .and_then(|sets| sets.iter().find(|s| s.name == set_name))
        .ok_or("Set not found")?
        .clone();

    let (windows, current) = windows_from_set(&set, channel_type);
    app.plot_windows = windows;
    app.plot_lanes = current;
    app.selected_signals = app.plot_windows.plotted_signals(&app.plot_lanes, usize::MAX);
    app.active_signal_set = Some((library_id.to_string(), set_name.to_string()));

    // Trigger plot refresh
    crate::ui::views::chart_view::extract_and_update_series_data(app);

    Ok(set.entries.len())
}

/// After a new log replaces the old one, plot the active set again on the new
/// data. If the set no longer applies (its library isn't the active one, or
/// it was removed), drop the highlight so it doesn't claim a set that isn't
/// plotted. Returns the set's name when it was applied.
pub fn reapply_active_signal_set(app: &mut CanDataApp) -> Option<String> {
    let (library_id, set_name) = app.active_signal_set.clone()?;
    if app.app_config.active_library_id.as_deref() != Some(library_id.as_str()) {
        app.active_signal_set = None;
        return None;
    }
    match apply_signal_set_state(app, &library_id, &set_name) {
        Ok(_) => Some(set_name),
        Err(_) => {
            app.active_signal_set = None;
            None
        }
    }
}

/// Clear the active set selection AND clear selected_signals.
/// Triggered by the dropdown's "✕" item.
pub fn clear_active_signal_set(app: &mut CanDataApp, cx: &mut Context<CanDataApp>) {
    app.active_signal_set = None;
    app.selected_signals.clear();
    crate::ui::views::chart_view::extract_and_update_series_data(app);
    app.status_msg = "Set selection cleared".into();
    cx.notify();
}

/// Delete a set from the store (no UI in this spec; symmetric API).
pub fn delete_signal_set(
    app: &mut CanDataApp,
    library_id: &str,
    set_name: &str,
    cx: &mut Context<CanDataApp>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::LibraryManager;
    use crate::library::signal_sets::SignalSetEntry;
    use crate::models::{ChannelType, SignalLibrary};

    /// App with library `lib1` active, holding a two-signal set `Engine`
    /// that is marked active but not plotted (as right after a file load).
    fn app_with_active_set() -> CanDataApp {
        let mut app = CanDataApp::new_state();
        app.library_manager = LibraryManager::from_libraries(vec![SignalLibrary {
            id: "lib1".into(),
            name: "Lib 1".into(),
            channel_type: ChannelType::CAN,
            versions: Vec::new(),
        }]);
        app.app_config.active_library_id = Some("lib1".into());
        let entries = ["Speed", "Rpm"]
            .iter()
            .map(|name| SignalSetEntry { channel_id: 1, msg_id: 0x100, signal_name: name.to_string() })
            .collect();
        app.signal_set_store.sets_by_library.insert(
            "lib1".into(),
            vec![SignalSet { name: "Engine".into(), entries, lanes: Vec::new(), windows: Vec::new() }],
        );
        app.active_signal_set = Some(("lib1".into(), "Engine".into()));
        app
    }

    /// Load `Engine` so the lanes on screen match it.
    fn app_with_loaded_set() -> CanDataApp {
        let mut app = app_with_active_set();
        apply_signal_set_state(&mut app, "lib1", "Engine").unwrap();
        app
    }

    #[test]
    fn a_freshly_loaded_set_is_not_modified() {
        let app = app_with_loaded_set();
        assert_eq!(active_set_status(&app), Some(("Engine".into(), false)));
    }

    #[test]
    fn changing_a_lane_marks_the_set_modified_and_undoing_it_clears_that() {
        let mut app = app_with_loaded_set();
        app.plot_lanes.lanes[0].show_points = !app.plot_lanes.lanes[0].show_points;
        assert_eq!(active_set_status(&app), Some(("Engine".into(), true)));
        app.plot_lanes.lanes[0].show_points = !app.plot_lanes.lanes[0].show_points;
        assert_eq!(active_set_status(&app), Some(("Engine".into(), false)));
    }

    #[test]
    fn adding_a_signal_keeps_the_set_name_and_marks_it_modified() {
        let mut app = app_with_loaded_set();
        crate::ui::views::chart_view::add_signal_to_lane(&mut app, 0, "CAN:1:256:Torque");
        assert_eq!(active_set_status(&app), Some(("Engine".into(), true)));
    }

    #[test]
    fn update_overwrites_the_set_with_the_current_lanes() {
        let mut app = app_with_loaded_set();
        crate::ui::views::chart_view::add_signal_to_lane(&mut app, 0, "CAN:1:256:Torque");
        app.plot_lanes.lanes[1].y_range = Some((0.0, 100.0));
        assert_eq!(update_active_signal_set_state(&mut app), Ok(("Engine".into(), 3)));
        assert_eq!(active_set_status(&app), Some(("Engine".into(), false)));
        let set = &app.signal_set_store.sets_by_library["lib1"][0];
        assert_eq!(set.entries.len(), 3);
        assert_eq!(set.lanes.len(), 2);
        assert_eq!(set.lanes[0].entries.len(), 2);
        assert_eq!(set.lanes[1].y_range, Some((0.0, 100.0)));
        // Only one set: updating doesn't add a copy.
        assert_eq!(app.signal_set_store.sets_by_library["lib1"].len(), 1);
    }

    #[test]
    fn update_saves_every_window_and_a_window_change_marks_the_set_modified() {
        let mut app = app_with_loaded_set();
        app.plot_windows.duplicate_active("Copy", &mut app.plot_lanes).unwrap();
        assert_eq!(active_set_status(&app), Some(("Engine".into(), true)));
        update_active_signal_set_state(&mut app).unwrap();
        assert_eq!(active_set_status(&app), Some(("Engine".into(), false)));
        let set = &app.signal_set_store.sets_by_library["lib1"][0];
        let names: Vec<&str> = set.windows.iter().map(|w| w.name.as_str()).collect();
        assert_eq!(names, ["Unnamed", "Copy"]);
        // Loading it again brings both windows back.
        apply_signal_set_state(&mut app, "lib1", "Engine").unwrap();
        assert_eq!(app.plot_windows.names(), ["Unnamed", "Copy"]);
        assert_eq!(app.plot_windows.active(), 0);
    }

    #[test]
    fn update_needs_a_set_in_the_active_library() {
        let mut app = app_with_loaded_set();
        app.app_config.active_library_id = Some("other".into());
        assert!(update_active_signal_set_state(&mut app).is_err());
        assert_eq!(active_set_status(&app), None);
        app.app_config.active_library_id = Some("lib1".into());
        app.active_signal_set = None;
        assert!(update_active_signal_set_state(&mut app).is_err());
    }

    #[test]
    fn reapply_restores_the_active_sets_signals() {
        let mut app = app_with_active_set();
        assert_eq!(reapply_active_signal_set(&mut app).as_deref(), Some("Engine"));
        assert_eq!(app.selected_signals.len(), 2);
        assert_eq!(app.active_signal_set, Some(("lib1".into(), "Engine".into())));
    }

    #[test]
    fn reapply_drops_the_set_when_its_library_is_not_active() {
        let mut app = app_with_active_set();
        app.app_config.active_library_id = Some("other".into());
        assert_eq!(reapply_active_signal_set(&mut app), None);
        assert!(app.selected_signals.is_empty());
        assert_eq!(app.active_signal_set, None);
    }

    #[test]
    fn reapply_drops_the_set_when_it_no_longer_exists() {
        let mut app = app_with_active_set();
        app.signal_set_store.sets_by_library.clear();
        assert_eq!(reapply_active_signal_set(&mut app), None);
        assert!(app.selected_signals.is_empty());
        assert_eq!(app.active_signal_set, None);
    }

    #[test]
    fn reapply_without_an_active_set_leaves_selection_alone() {
        let mut app = app_with_active_set();
        app.active_signal_set = None;
        app.selected_signals = vec!["CAN:1:0x200:Other".into()];
        assert_eq!(reapply_active_signal_set(&mut app), None);
        assert_eq!(app.selected_signals, vec!["CAN:1:0x200:Other".to_string()]);
    }
}
