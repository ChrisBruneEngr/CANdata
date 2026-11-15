//! Lane layout for the Plot view: which signals each lane plots, and which lane
//! is selected.
//!
//! This file has no dependencies, so its tests also run on their own:
//! `rustc --edition 2021 --test src/view/src/plot_lanes.rs -o <output>`

/// One horizontal plot area. A signal may appear in more than one lane.
#[derive(Debug, Clone, PartialEq)]
pub struct PlotLane {
    pub signals: Vec<String>,
    /// User-defined (min, max) Y limits. `None` means the range is automatic.
    pub y_range: Option<(f64, f64)>,
    /// Draw a dot at each sample as well as the line.
    pub show_points: bool,
    /// The range Auto picked from the data that was visible when it was worked out.
    /// It stays put while zooming and panning, and is worked out again when the
    /// lane's signals change or Auto is turned back on.
    pub auto_y_range: Option<(f64, f64)>,
    /// The lane's share of the plot height, relative to the other lanes
    /// (1.0 = the default size). Kept relative so lanes scale with the window.
    pub height: f32,
}

/// Lanes never get shorter than this, in pixels.
pub const MIN_LANE_HEIGHT: f32 = 100.0;

impl Default for PlotLane {
    fn default() -> Self {
        Self {
            signals: Vec::new(),
            y_range: None,
            show_points: true,
            auto_y_range: None,
            height: 1.0,
        }
    }
}

/// The ordered lanes. There is always at least one, and `selected` is always valid.
#[derive(Debug, Clone, PartialEq)]
pub struct LaneLayout {
    pub lanes: Vec<PlotLane>,
    pub selected: usize,
}

impl Default for LaneLayout {
    fn default() -> Self {
        Self {
            lanes: vec![PlotLane::default()],
            selected: 0,
        }
    }
}

impl LaneLayout {
    /// A layout of these lanes with the first selected. No lanes gives the default.
    pub fn from_lanes(lanes: Vec<PlotLane>) -> Self {
        if lanes.is_empty() {
            Self::default()
        } else {
            Self { lanes, selected: 0 }
        }
    }

    /// A layout with a lane for each signal.
    pub fn one_signal_per_lane(signals: Vec<String>) -> Self {
        Self::from_lanes(
            signals
                .into_iter()
                .map(|signal| PlotLane {
                    signals: vec![signal],
                    ..Default::default()
                })
                .collect(),
        )
    }

    pub fn select(&mut self, index: usize) {
        if index < self.lanes.len() {
            self.selected = index;
        }
    }

    /// Add an empty lane at the bottom and select it.
    pub fn add_lane(&mut self) {
        self.lanes.push(PlotLane::default());
        self.selected = self.lanes.len() - 1;
    }

    /// Delete the selected lane. The last remaining lane is emptied instead.
    pub fn delete_selected(&mut self) {
        if self.lanes.len() == 1 {
            self.lanes[0].signals.clear();
            self.lanes[0].auto_y_range = None;
            return;
        }
        self.lanes.remove(self.selected);
        self.selected = self.selected.min(self.lanes.len() - 1);
    }

    pub fn can_move_up(&self) -> bool {
        self.selected > 0
    }

    pub fn can_move_down(&self) -> bool {
        self.selected + 1 < self.lanes.len()
    }

    /// Swap the selected lane with the one above. It stays selected.
    pub fn move_selected_up(&mut self) {
        if self.can_move_up() {
            self.lanes.swap(self.selected, self.selected - 1);
            self.selected -= 1;
        }
    }

    /// Swap the selected lane with the one below. It stays selected.
    pub fn move_selected_down(&mut self) {
        if self.can_move_down() {
            self.lanes.swap(self.selected, self.selected + 1);
            self.selected += 1;
        }
    }

    /// Add a signal to a lane. Returns false if the lane doesn't exist or already has it.
    /// Rename a signal in every lane.
    pub fn rename_signal(&mut self, old: &str, new: &str) {
        for lane in &mut self.lanes {
            for signal in &mut lane.signals {
                if signal == old {
                    *signal = new.to_string();
                }
            }
        }
    }

    pub fn add_signal(&mut self, lane: usize, signal: &str) -> bool {
        match self.lanes.get_mut(lane) {
            Some(lane) if !lane.signals.iter().any(|s| s == signal) => {
                lane.signals.push(signal.to_string());
                lane.auto_y_range = None;
                true
            }
            _ => false,
        }
    }

    /// Remove a signal from one lane, leaving any other lanes that plot it alone.
    pub fn remove_signal(&mut self, lane: usize, signal: &str) {
        if let Some(lane) = self.lanes.get_mut(lane) {
            let before = lane.signals.len();
            lane.signals.retain(|s| s != signal);
            if lane.signals.len() != before {
                lane.auto_y_range = None;
            }
        }
    }

    /// Set the heights of lane `upper` and the lane below it.
    pub fn set_pair_heights(&mut self, upper: usize, heights: (f32, f32)) {
        if upper + 1 < self.lanes.len() {
            self.lanes[upper].height = heights.0;
            self.lanes[upper + 1].height = heights.1;
        }
    }

    /// Give lane `upper` and the lane below it the same height, keeping their
    /// combined height.
    pub fn even_out_pair(&mut self, upper: usize) {
        if upper + 1 < self.lanes.len() {
            let half = (self.lanes[upper].height + self.lanes[upper + 1].height) / 2.0;
            self.set_pair_heights(upper, (half, half));
        }
    }

    /// The signal's position within the first lane that plots it. Line colours
    /// follow this position.
    pub fn first_position(&self, signal: &str) -> Option<usize> {
        self.lanes
            .iter()
            .find_map(|lane| lane.signals.iter().position(|s| s == signal))
    }

    /// Every plotted signal once, in lane order.
    pub fn unique_signals(&self) -> Vec<String> {
        let mut all: Vec<String> = Vec::new();
        for signal in self.lanes.iter().flat_map(|lane| &lane.signals) {
            if !all.contains(signal) {
                all.push(signal.clone());
            }
        }
        all
    }

    /// Take out signals that aren't in the list. Lanes stay, even if emptied.
    pub fn retain_signals(&mut self, keep: &[String]) {
        for lane in &mut self.lanes {
            let before = lane.signals.len();
            lane.signals.retain(|s| keep.contains(s));
            if lane.signals.len() != before {
                lane.auto_y_range = None;
            }
        }
    }

    /// True if any lane plots this signal.
    pub fn contains(&self, signal: &str) -> bool {
        self.lanes
            .iter()
            .any(|lane| lane.signals.iter().any(|s| s == signal))
    }

    /// Bring the lanes in line with the app's list of plotted signals.
    ///
    /// Signals no longer listed leave every lane. One new signal joins the selected
    /// lane. Several at once replace any empty lanes and get a lane each.
    pub fn sync_with_selection(&mut self, selected_signals: &[String]) {
        for lane in &mut self.lanes {
            let before = lane.signals.len();
            lane.signals.retain(|s| selected_signals.contains(s));
            if lane.signals.len() != before {
                lane.auto_y_range = None;
            }
        }

        let new_signals: Vec<&String> = selected_signals
            .iter()
            .filter(|s| !self.contains(s))
            .collect();

        match new_signals.as_slice() {
            [] => {}
            [signal] => {
                let signal = (*signal).clone();
                let lane = &mut self.lanes[self.selected];
                lane.signals.push(signal);
                lane.auto_y_range = None;
            }
            signals => {
                let added: Vec<PlotLane> = signals
                    .iter()
                    .map(|s| PlotLane {
                        signals: vec![(*s).clone()],
                        ..Default::default()
                    })
                    .collect();
                self.lanes.retain(|lane| !lane.signals.is_empty());
                self.lanes.extend(added);
                self.selected = self.selected.min(self.lanes.len() - 1);
            }
        }
    }
}

/// New heights for two neighbouring lanes after the divider between them is
/// dragged `dy` pixels (down is positive). `start_px` is each lane's height on
/// screen when the drag began and `start_heights` their `height` values then.
/// The upper lane grows by `dy` and the lower one shrinks by the same amount,
/// so their combined size is unchanged, and neither goes below `min_px`.
pub fn resize_pair(
    start_px: (f32, f32),
    start_heights: (f32, f32),
    dy: f32,
    min_px: f32,
) -> (f32, f32) {
    let total_px = start_px.0 + start_px.1;
    if total_px <= 2.0 * min_px {
        return start_heights; // no room to move
    }
    let upper_px = (start_px.0 + dy).clamp(min_px, total_px - min_px);
    let total = start_heights.0 + start_heights.1;
    let upper = total * upper_px / total_px;
    (upper, total - upper)
}

/// The Y range that fits `values`. When they are all equal the range is widened to
/// include zero, so a flat line doesn't sit against the top of the lane.
pub fn auto_value_range(values: impl Iterator<Item = f64>) -> Option<(f64, f64)> {
    let (mut min, mut max) = values.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
        (lo.min(v), hi.max(v))
    });
    if min > max {
        return None;
    }
    if min == max {
        min = min.min(0.0);
        max = max.max(0.0);
        if min == max {
            max = min + 1.0;
        }
    }
    Some((min, max))
}

/// Move a time window by `shift` seconds, keeping its width and staying within
/// `min..max`. A window at least as wide as that range doesn't move.
pub fn pan_time_range(start: f64, end: f64, shift: f64, min: f64, max: f64) -> (f64, f64) {
    let span = end - start;
    if span >= max - min {
        return (start, end);
    }
    let new_start = (start + shift).clamp(min, max - span);
    (new_start, new_start + span)
}

/// Read typed Y limits, or explain why they can't be used.
pub fn parse_y_range(min: &str, max: &str) -> Result<(f64, f64), &'static str> {
    let parse = |text: &str| text.trim().parse::<f64>().ok().filter(|v| v.is_finite());
    let (Some(min), Some(max)) = (parse(min), parse(max)) else {
        return Err("Min and Max must be numbers");
    };
    if min >= max {
        return Err("Min must be less than Max");
    }
    Ok((min, max))
}

/// Format a Y limit for display, without trailing zeros.
pub fn format_y_limit(value: f64) -> String {
    let text = format!("{:.6}", value);
    let trimmed = text.trim_end_matches('0').trim_end_matches('.');
    match trimmed {
        "" | "-" | "-0" => "0".to_string(),
        _ => trimmed.to_string(),
    }
}

/// Name of the first window, and the start of new windows' names.
pub const DEFAULT_WINDOW_NAME: &str = "Unnamed";

/// The named plot windows. Only one is on screen at a time.
///
/// The on-screen window's lanes live outside this struct (in the app's
/// `plot_lanes`), so the lane code works on one layout as before. Every method
/// that needs them takes them as `current`. While a window is on screen its
/// slot in `stored` holds an empty placeholder.
#[derive(Debug, Clone, PartialEq)]
pub struct PlotWindows {
    names: Vec<String>,
    stored: Vec<LaneLayout>,
    active: usize,
}

impl Default for PlotWindows {
    fn default() -> Self {
        Self {
            names: vec![DEFAULT_WINDOW_NAME.to_string()],
            stored: vec![LaneLayout::default()],
            active: 0,
        }
    }
}

impl PlotWindows {
    /// Windows from (name, lanes) pairs, with the first on screen. Returns its
    /// lanes, which the caller keeps as `current`. No windows gives the default.
    pub fn from_windows(windows: Vec<(String, LaneLayout)>) -> (Self, LaneLayout) {
        if windows.is_empty() {
            return (Self::default(), LaneLayout::default());
        }
        let (names, mut stored): (Vec<String>, Vec<LaneLayout>) = windows.into_iter().unzip();
        let current = std::mem::take(&mut stored[0]);
        (Self { names, stored, active: 0 }, current)
    }

    /// Every window's name and lanes, in order.
    pub fn windows<'a>(&'a self, current: &'a LaneLayout) -> Vec<(&'a str, &'a LaneLayout)> {
        self.names
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let lanes = if i == self.active { current } else { &self.stored[i] };
                (name.as_str(), lanes)
            })
            .collect()
    }

    pub fn names(&self) -> &[String] {
        &self.names
    }

    pub fn active(&self) -> usize {
        self.active
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Rename a signal in every window's lanes.
    pub fn rename_signal(&mut self, current: &mut LaneLayout, old: &str, new: &str) {
        for (i, lanes) in self.stored.iter_mut().enumerate() {
            if i != self.active {
                lanes.rename_signal(old, new);
            }
        }
        current.rename_signal(old, new);
    }

    /// True if any window plots this signal.
    pub fn contains(&self, current: &LaneLayout, signal: &str) -> bool {
        self.windows(current).iter().any(|(_, lanes)| lanes.contains(signal))
    }

    /// The signals to decode: up to `per_window` from each window, in window
    /// and lane order, each once.
    pub fn plotted_signals(&self, current: &LaneLayout, per_window: usize) -> Vec<String> {
        let mut all: Vec<String> = Vec::new();
        for (_, lanes) in self.windows(current) {
            for signal in lanes.unique_signals().into_iter().take(per_window) {
                if !all.contains(&signal) {
                    all.push(signal);
                }
            }
        }
        all
    }

    /// Put another window on screen.
    pub fn switch_to(&mut self, index: usize, current: &mut LaneLayout) {
        if index >= self.len() || index == self.active {
            return;
        }
        std::mem::swap(current, &mut self.stored[self.active]);
        self.active = index;
        std::mem::swap(current, &mut self.stored[self.active]);
    }

    /// Add a window after the last one and put it on screen.
    fn push_and_show(&mut self, name: String, lanes: LaneLayout, current: &mut LaneLayout) {
        self.names.push(name);
        self.stored.push(lanes);
        let index = self.len() - 1;
        self.switch_to(index, current);
    }

    /// Add a window with one empty lane, named "Unnamed", "Unnamed 2", ... and show it.
    pub fn add_window(&mut self, current: &mut LaneLayout) {
        let name = self.next_default_name();
        self.push_and_show(name, LaneLayout::default(), current);
    }

    /// Copy the on-screen window under a new name and show the copy.
    pub fn duplicate_active(&mut self, name: &str, current: &mut LaneLayout) -> Result<(), &'static str> {
        let name = self.check_name(name, None)?;
        let copy = current.clone();
        self.push_and_show(name, copy, current);
        Ok(())
    }

    pub fn rename(&mut self, index: usize, name: &str) -> Result<(), &'static str> {
        if index >= self.len() {
            return Err("No such window");
        }
        self.names[index] = self.check_name(name, Some(index))?;
        Ok(())
    }

    /// Close a window. The last window can't be closed. Closing the one on
    /// screen shows its neighbour.
    pub fn close(&mut self, index: usize, current: &mut LaneLayout) -> bool {
        if self.len() <= 1 || index >= self.len() {
            return false;
        }
        self.names.remove(index);
        self.stored.remove(index);
        if index == self.active {
            self.active = index.min(self.len() - 1);
            *current = std::mem::take(&mut self.stored[self.active]);
        } else if index < self.active {
            self.active -= 1;
        }
        true
    }

    /// Bring every window in line with the app's list of plotted signals.
    /// Signals no longer listed leave every window. New signals join the
    /// on-screen window, as with a single layout.
    pub fn sync_with_selection(&mut self, current: &mut LaneLayout, selected_signals: &[String]) {
        let active = self.active;
        for (i, lanes) in self.stored.iter_mut().enumerate() {
            if i != active {
                lanes.retain_signals(selected_signals);
            }
        }
        let others = &self.stored;
        let for_current: Vec<String> = selected_signals
            .iter()
            .filter(|s| !others.iter().enumerate().any(|(i, l)| i != active && l.contains(s)))
            .cloned()
            .collect();
        current.sync_with_selection(&for_current);
    }

    /// The first free name of "Unnamed", "Unnamed 2", "Unnamed 3", ...
    pub fn next_default_name(&self) -> String {
        (1..)
            .map(|n| {
                if n == 1 {
                    DEFAULT_WINDOW_NAME.to_string()
                } else {
                    format!("{} {}", DEFAULT_WINDOW_NAME, n)
                }
            })
            .find(|name| !self.name_taken(name, None))
            .unwrap()
    }

    /// True if another window (not `except`) has this name, ignoring case.
    pub fn name_taken(&self, name: &str, except: Option<usize>) -> bool {
        self.names
            .iter()
            .enumerate()
            .any(|(i, n)| Some(i) != except && n.eq_ignore_ascii_case(name.trim()))
    }

    fn check_name(&self, name: &str, except: Option<usize>) -> Result<String, &'static str> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Window name cannot be empty");
        }
        if self.name_taken(name, except) {
            return Err("A window with that name already exists");
        }
        Ok(name.to_string())
    }
}

#[cfg(test)]
mod window_tests {
    use super::*;

    fn lanes_with(signals: &[&str]) -> LaneLayout {
        LaneLayout::one_signal_per_lane(signals.iter().map(|s| s.to_string()).collect())
    }

    #[test]
    fn starts_with_one_unnamed_window() {
        let windows = PlotWindows::default();
        assert_eq!(windows.names(), ["Unnamed"]);
        assert_eq!(windows.active(), 0);
    }

    #[test]
    fn new_windows_get_numbered_names_and_one_empty_lane() {
        let mut windows = PlotWindows::default();
        let mut current = lanes_with(&["a"]);
        windows.add_window(&mut current);
        windows.add_window(&mut current);
        assert_eq!(windows.names(), ["Unnamed", "Unnamed 2", "Unnamed 3"]);
        assert_eq!(windows.active(), 2);
        assert_eq!(current, LaneLayout::default());
    }

    #[test]
    fn switching_keeps_each_windows_lanes() {
        let mut windows = PlotWindows::default();
        let mut current = lanes_with(&["a", "b"]);
        windows.add_window(&mut current);
        current.add_signal(0, "c");
        windows.switch_to(0, &mut current);
        assert_eq!(current, lanes_with(&["a", "b"]));
        windows.switch_to(1, &mut current);
        assert_eq!(current, lanes_with(&["c"]));
    }

    #[test]
    fn duplicate_copies_the_lanes_and_needs_a_free_name() {
        let mut windows = PlotWindows::default();
        let mut original = lanes_with(&["a"]);
        original.lanes[0].y_range = Some((0.0, 5.0));
        let mut current = original.clone();
        assert!(windows.duplicate_active("unnamed", &mut current).is_err());
        assert!(windows.duplicate_active("  ", &mut current).is_err());
        windows.duplicate_active(" Temps ", &mut current).unwrap();
        assert_eq!(windows.names(), ["Unnamed", "Temps"]);
        assert_eq!(windows.active(), 1);
        assert_eq!(current, original);
        // The copy is separate from the original.
        current.add_signal(0, "b");
        windows.switch_to(0, &mut current);
        assert_eq!(current, original);
    }

    #[test]
    fn rename_allows_a_new_case_of_its_own_name_but_not_anothers() {
        let mut windows = PlotWindows::default();
        let mut current = LaneLayout::default();
        windows.add_window(&mut current);
        assert!(windows.rename(1, "UNNAMED").is_err());
        windows.rename(0, "UNNAMED").unwrap();
        windows.rename(1, "Torque").unwrap();
        assert_eq!(windows.names(), ["UNNAMED", "Torque"]);
    }

    #[test]
    fn closing_the_shown_window_shows_its_neighbour() {
        let (mut windows, mut current) = PlotWindows::from_windows(vec![
            ("A".into(), lanes_with(&["a"])),
            ("B".into(), lanes_with(&["b"])),
            ("C".into(), lanes_with(&["c"])),
        ]);
        windows.switch_to(2, &mut current);
        assert!(windows.close(2, &mut current));
        assert_eq!(windows.names(), ["A", "B"]);
        assert_eq!(windows.active(), 1);
        assert_eq!(current, lanes_with(&["b"]));
        // Closing one before the shown window keeps the same window on screen.
        assert!(windows.close(0, &mut current));
        assert_eq!(windows.active(), 0);
        assert_eq!(current, lanes_with(&["b"]));
        // The last window stays.
        assert!(!windows.close(0, &mut current));
    }

    #[test]
    fn rename_signal_in_every_window() {
        let (mut windows, mut current) = PlotWindows::from_windows(vec![
            ("A".into(), lanes_with(&["a", "m"])),
            ("B".into(), lanes_with(&["m"])),
        ]);
        windows.rename_signal(&mut current, "m", "n");
        assert_eq!(windows.plotted_signals(&current, 10), ["a", "n"]);
        assert!(!windows.contains(&current, "m"));
    }

    #[test]
    fn plotted_signals_cover_every_window_once_with_a_per_window_limit() {
        let (windows, current) = PlotWindows::from_windows(vec![
            ("A".into(), lanes_with(&["a", "b", "c"])),
            ("B".into(), lanes_with(&["b", "d"])),
        ]);
        assert_eq!(windows.plotted_signals(&current, 10), ["a", "b", "c", "d"]);
        assert_eq!(windows.plotted_signals(&current, 2), ["a", "b", "d"]);
    }

    #[test]
    fn sync_leaves_other_windows_signals_where_they_are() {
        let (mut windows, mut current) = PlotWindows::from_windows(vec![
            ("A".into(), lanes_with(&["a"])),
            ("B".into(), lanes_with(&["b", "c"])),
        ]);
        let selected: Vec<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
        windows.sync_with_selection(&mut current, &selected);
        assert_eq!(current, lanes_with(&["a"]));
        // Dropping a signal from the list takes it out of the hidden window too.
        windows.sync_with_selection(&mut current, &selected[..2]);
        windows.switch_to(1, &mut current);
        assert_eq!(current.lanes[0].signals, ["b"]);
        assert!(current.lanes[1].signals.is_empty());
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn dragging_down_grows_the_upper_lane_and_shrinks_the_lower() {
        // Two equal 200 px lanes; drag the divider down 50 px.
        let (upper, lower) = resize_pair((200.0, 200.0), (1.0, 1.0), 50.0, 100.0);
        assert!((upper - 1.25).abs() < 1e-6);
        assert!((lower - 0.75).abs() < 1e-6);
    }

    #[test]
    fn dragging_up_shrinks_the_upper_lane_and_keeps_the_total() {
        let (upper, lower) = resize_pair((300.0, 300.0), (2.0, 2.0), -150.0, 100.0);
        assert!((upper - 1.0).abs() < 1e-6);
        assert!((lower - 3.0).abs() < 1e-6);
    }

    #[test]
    fn resizing_stops_at_the_minimum_height() {
        let (upper, lower) = resize_pair((200.0, 200.0), (1.0, 1.0), 1_000.0, 100.0);
        assert!((upper - 1.5).abs() < 1e-6); // 300 px of 400
        assert!((lower - 0.5).abs() < 1e-6); // 100 px, the minimum
        let (upper, _) = resize_pair((200.0, 200.0), (1.0, 1.0), -1_000.0, 100.0);
        assert!((upper - 0.5).abs() < 1e-6);
    }

    #[test]
    fn lanes_already_at_the_minimum_do_not_resize() {
        assert_eq!(resize_pair((100.0, 100.0), (1.0, 1.0), 40.0, 100.0), (1.0, 1.0));
    }

    #[test]
    fn evening_out_a_pair_leaves_other_lanes_alone() {
        let mut layout = LaneLayout::from_lanes(vec![PlotLane::default(); 3]);
        layout.set_pair_heights(0, (1.5, 0.5));
        layout.lanes[2].height = 3.0;
        layout.even_out_pair(0);
        let heights: Vec<f32> = layout.lanes.iter().map(|l| l.height).collect();
        assert_eq!(heights, vec![1.0, 1.0, 3.0]);
    }

    #[test]
    fn a_moved_lane_keeps_its_height() {
        let mut layout = LaneLayout::from_lanes(vec![PlotLane::default(); 2]);
        layout.lanes[1].height = 2.0;
        layout.select(1);
        layout.move_selected_up();
        assert_eq!(layout.lanes[0].height, 2.0);
    }

    use super::*;

    fn signals(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    fn layout(lanes: &[&[&str]], selected: usize) -> LaneLayout {
        LaneLayout {
            lanes: lanes
                .iter()
                .map(|names| PlotLane {
                    signals: signals(names),
                    ..Default::default()
                })
                .collect(),
            selected,
        }
    }

    #[test]
    fn starts_with_one_empty_selected_lane() {
        assert_eq!(LaneLayout::default(), layout(&[&[]], 0));
    }

    #[test]
    fn a_layout_from_lanes_selects_the_first() {
        let lanes = layout(&[&["a"], &["b"]], 0).lanes;
        assert_eq!(LaneLayout::from_lanes(lanes), layout(&[&["a"], &["b"]], 0));
    }

    #[test]
    fn a_layout_always_has_a_lane() {
        assert_eq!(LaneLayout::from_lanes(Vec::new()), LaneLayout::default());
        assert_eq!(LaneLayout::one_signal_per_lane(Vec::new()), LaneLayout::default());
    }

    #[test]
    fn one_signal_per_lane_keeps_the_order() {
        let l = LaneLayout::one_signal_per_lane(signals(&["a", "b"]));
        assert_eq!(l, layout(&[&["a"], &["b"]], 0));
    }

    #[test]
    fn new_lanes_show_points() {
        let mut l = LaneLayout::default();
        l.add_lane();
        assert!(l.lanes.iter().all(|lane| lane.show_points));
    }

    #[test]
    fn add_lane_goes_to_the_bottom_and_is_selected() {
        let mut l = layout(&[&["a"]], 0);
        l.add_lane();
        assert_eq!(l, layout(&[&["a"], &[]], 1));
    }

    #[test]
    fn delete_removes_the_selected_lane() {
        let mut l = layout(&[&["a"], &["b"], &["c"]], 1);
        l.delete_selected();
        assert_eq!(l, layout(&[&["a"], &["c"]], 1));
    }

    #[test]
    fn deleting_the_bottom_lane_selects_the_new_bottom_lane() {
        let mut l = layout(&[&["a"], &["b"]], 1);
        l.delete_selected();
        assert_eq!(l, layout(&[&["a"]], 0));
    }

    #[test]
    fn deleting_the_last_lane_empties_it() {
        let mut l = layout(&[&["a", "b"]], 0);
        l.delete_selected();
        assert_eq!(l, layout(&[&[]], 0));
    }

    #[test]
    fn move_up_swaps_lanes_and_keeps_the_selection() {
        let mut l = layout(&[&["a"], &["b"]], 1);
        l.move_selected_up();
        assert_eq!(l, layout(&[&["b"], &["a"]], 0));
    }

    #[test]
    fn move_down_swaps_lanes_and_keeps_the_selection() {
        let mut l = layout(&[&["a"], &["b"]], 0);
        l.move_selected_down();
        assert_eq!(l, layout(&[&["b"], &["a"]], 1));
    }

    #[test]
    fn moves_stop_at_the_ends() {
        let mut top = layout(&[&["a"], &["b"]], 0);
        assert!(!top.can_move_up());
        top.move_selected_up();
        assert_eq!(top, layout(&[&["a"], &["b"]], 0));

        let mut bottom = layout(&[&["a"], &["b"]], 1);
        assert!(!bottom.can_move_down());
        bottom.move_selected_down();
        assert_eq!(bottom, layout(&[&["a"], &["b"]], 1));
    }

    #[test]
    fn select_ignores_lanes_that_do_not_exist() {
        let mut l = layout(&[&["a"], &["b"]], 0);
        l.select(5);
        assert_eq!(l.selected, 0);
        l.select(1);
        assert_eq!(l.selected, 1);
    }

    #[test]
    fn a_signal_can_be_added_to_several_lanes() {
        let mut l = layout(&[&["a"], &[]], 0);
        assert!(l.add_signal(1, "a"));
        assert_eq!(l, layout(&[&["a"], &["a"]], 0));
    }

    #[test]
    fn a_lane_cannot_hold_the_same_signal_twice() {
        let mut l = layout(&[&["a"]], 0);
        assert!(!l.add_signal(0, "a"));
        assert_eq!(l, layout(&[&["a"]], 0));
    }

    #[test]
    fn adding_to_a_missing_lane_does_nothing() {
        let mut l = layout(&[&[]], 0);
        assert!(!l.add_signal(3, "a"));
        assert_eq!(l, layout(&[&[]], 0));
    }

    #[test]
    fn removing_from_one_lane_leaves_other_lanes_alone() {
        let mut l = layout(&[&["a", "b"], &["a"]], 0);
        l.remove_signal(0, "a");
        assert_eq!(l, layout(&[&["b"], &["a"]], 0));
        assert!(l.contains("a"));
    }

    #[test]
    fn first_position_uses_the_first_lane_that_has_the_signal() {
        let l = layout(&[&["a", "b"], &["x", "y", "b"]], 0);
        assert_eq!(l.first_position("b"), Some(1));
        assert_eq!(l.first_position("y"), Some(1));
        assert_eq!(l.first_position("missing"), None);
    }

    #[test]
    fn one_new_signal_joins_the_selected_lane() {
        let mut l = layout(&[&["a"], &[]], 1);
        l.sync_with_selection(&signals(&["a", "b"]));
        assert_eq!(l, layout(&[&["a"], &["b"]], 1));
    }

    #[test]
    fn unlisted_signals_leave_every_lane_but_lanes_remain() {
        let mut l = layout(&[&["a", "b"], &["a"]], 0);
        l.sync_with_selection(&signals(&["b"]));
        assert_eq!(l, layout(&[&["b"], &[]], 0));
    }

    #[test]
    fn a_signal_already_in_a_lane_is_not_added_again() {
        let mut l = layout(&[&["a"], &["a", "b"]], 0);
        l.sync_with_selection(&signals(&["a", "b"]));
        assert_eq!(l, layout(&[&["a"], &["a", "b"]], 0));
    }

    #[test]
    fn several_new_signals_get_a_lane_each_and_replace_empty_lanes() {
        let mut l = layout(&[&["old"], &[]], 1);
        l.sync_with_selection(&signals(&["a", "b", "c"]));
        assert_eq!(l, layout(&[&["a"], &["b"], &["c"]], 1));
    }

    #[test]
    fn several_new_signals_keep_lanes_that_still_have_signals() {
        let mut l = layout(&[&["keep"], &[]], 0);
        l.sync_with_selection(&signals(&["keep", "a", "b"]));
        assert_eq!(l, layout(&[&["keep"], &["a"], &["b"]], 0));
    }

    #[test]
    fn clearing_every_signal_keeps_the_lanes() {
        let mut l = layout(&[&["a"], &["b"]], 1);
        l.sync_with_selection(&[]);
        assert_eq!(l, layout(&[&[], &[]], 1));
    }

    #[test]
    fn auto_range_covers_every_signal_in_the_lane() {
        let values = [3.0, -2.0, 7.5, 1.0];
        assert_eq!(auto_value_range(values.into_iter()), Some((-2.0, 7.5)));
    }

    #[test]
    fn auto_range_widens_flat_lines_to_include_zero() {
        assert_eq!(auto_value_range([5.0, 5.0].into_iter()), Some((0.0, 5.0)));
        assert_eq!(auto_value_range([-5.0].into_iter()), Some((-5.0, 0.0)));
        assert_eq!(auto_value_range([0.0].into_iter()), Some((0.0, 1.0)));
    }

    #[test]
    fn y_limits_parse_with_spaces_decimals_and_negatives() {
        assert_eq!(parse_y_range(" -10 ", "2.5"), Ok((-10.0, 2.5)));
    }

    #[test]
    fn y_limits_must_be_finite_numbers() {
        let message = Err("Min and Max must be numbers");
        assert_eq!(parse_y_range("", "5"), message);
        assert_eq!(parse_y_range("5", "abc"), message);
        assert_eq!(parse_y_range("-", "5"), message);
        assert_eq!(parse_y_range("inf", "5"), message);
        assert_eq!(parse_y_range("0", "NaN"), message);
    }

    #[test]
    fn y_min_must_be_below_y_max() {
        let message = Err("Min must be less than Max");
        assert_eq!(parse_y_range("5", "5"), message);
        assert_eq!(parse_y_range("10", "5"), message);
    }

    #[test]
    fn y_limits_format_without_trailing_zeros() {
        assert_eq!(format_y_limit(100.0), "100");
        assert_eq!(format_y_limit(2.5), "2.5");
        assert_eq!(format_y_limit(-40.0), "-40");
        assert_eq!(format_y_limit(0.0), "0");
        assert_eq!(format_y_limit(-0.0000001), "0");
        assert_eq!(format_y_limit(0.30000000000000004), "0.3");
    }

    #[test]
    fn moving_a_lane_keeps_its_settings() {
        let mut l = layout(&[&["a"], &["b"]], 1);
        l.lanes[1].y_range = Some((0.0, 10.0));
        l.lanes[1].show_points = false;
        l.move_selected_up();
        assert_eq!(l.lanes[0].y_range, Some((0.0, 10.0)));
        assert!(!l.lanes[0].show_points);
        assert_eq!(l.lanes[1].y_range, None);
        assert!(l.lanes[1].show_points);
    }

    #[test]
    fn panning_keeps_the_zoom_level() {
        assert_eq!(pan_time_range(20.0, 30.0, 5.0, 0.0, 100.0), (25.0, 35.0));
        assert_eq!(pan_time_range(20.0, 30.0, -5.0, 0.0, 100.0), (15.0, 25.0));
    }

    #[test]
    fn panning_stops_at_the_ends_of_the_data() {
        assert_eq!(pan_time_range(20.0, 30.0, -50.0, 0.0, 100.0), (0.0, 10.0));
        assert_eq!(pan_time_range(80.0, 90.0, 50.0, 0.0, 100.0), (90.0, 100.0));
    }

    #[test]
    fn a_window_covering_all_the_data_does_not_pan() {
        assert_eq!(pan_time_range(0.0, 100.0, 10.0, 0.0, 100.0), (0.0, 100.0));
    }

    #[test]
    fn changing_a_lanes_signals_forgets_its_auto_range() {
        let mut l = layout(&[&["a"], &["b"]], 0);
        for lane in &mut l.lanes {
            lane.auto_y_range = Some((0.0, 1.0));
        }
        l.add_signal(0, "c");
        assert_eq!(l.lanes[0].auto_y_range, None);
        assert_eq!(l.lanes[1].auto_y_range, Some((0.0, 1.0)));

        l.lanes[0].auto_y_range = Some((0.0, 1.0));
        l.remove_signal(1, "b");
        assert_eq!(l.lanes[0].auto_y_range, Some((0.0, 1.0)));
        assert_eq!(l.lanes[1].auto_y_range, None);
    }

    #[test]
    fn syncing_keeps_auto_ranges_of_unchanged_lanes() {
        let mut l = layout(&[&["a"], &["b"]], 1);
        for lane in &mut l.lanes {
            lane.auto_y_range = Some((0.0, 1.0));
        }
        l.sync_with_selection(&signals(&["a", "b", "c"]));
        assert_eq!(l.lanes[0].auto_y_range, Some((0.0, 1.0)));
        assert_eq!(l.lanes[1].auto_y_range, None);
    }

    #[test]
    fn auto_range_is_none_without_values() {
        assert_eq!(auto_value_range(std::iter::empty()), None);
    }
}
