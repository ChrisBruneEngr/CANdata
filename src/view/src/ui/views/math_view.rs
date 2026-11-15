//! Math view — create and edit Math signals (signals defined by an equation of
//! decoded signals).
//!
//! Left: the list of Math signals, with the editor (name, unit, equation)
//! below it. Right: the signals of the active library; double-click one to
//! insert its name into the equation at the cursor.
//!
//! Math signals are stored in `library::math_signals`. Equations are checked
//! as they are typed (`math::resolve`) and computed for the plot by
//! `math::eval` (see `docs/design/2026-11-15-math-signals-design.md`).

use crate::app::CanDataApp;
use crate::library::math_signals::{Interpolation, MathSignal, TimeMode, save_math_signal_store};
use crate::ui::views::chart_view::MATH_ID_PREFIX;
use crate::math::resolve::{Check, SignalIndex, check};
use crate::models::SignalListMode;
use gpui::prelude::*;
use gpui::*;
use gpui_component::Sizable as _;
use gpui_component::input::{Input, InputEvent, InputState};
use std::sync::Arc;

/// UI state of the Math view. Not persisted.
#[derive(Default)]
pub struct MathViewState {
    /// Position in the store of the Math signal shown in the editor.
    pub selected: Option<usize>,
    pub name_input: Option<Entity<InputState>>,
    pub unit_input: Option<Entity<InputState>>,
    pub equation_input: Option<Entity<InputState>>,
    pub search_input: Option<Entity<InputState>>,
    /// Waiting for the user to discard or keep unsaved editor changes.
    pub pending: Option<PendingMathAction>,
    /// Waiting for the user to confirm deleting the selected Math signal.
    pub confirm_delete: bool,
    /// Time mode in the editor: resample (true) or sample and hold.
    pub resample: bool,
    pub interpolation: Interpolation,
    pub rate_input: Option<Entity<InputState>>,
}

/// Resample rate shown when a Math signal doesn't resample yet.
const DEFAULT_RATE_HZ: &str = "100";

/// Something that would replace the editor contents.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PendingMathAction {
    Select(usize),
    New,
}

/// One row of the signal browser.
#[derive(Clone, Debug, PartialEq)]
pub enum BrowserRow {
    Channel(String),
    Message(String),
    /// `note` tells apart signals that share a name (alphabetical list only).
    Signal { name: String, unit: String, note: Option<String> },
}

// ---------------------------------------------------------------------------
// Editor contents and actions
// ---------------------------------------------------------------------------

/// What the editor currently holds: (name, unit, equation).
fn editor_values(app: &CanDataApp, cx: &App) -> Option<(String, String, String)> {
    let m = &app.math;
    let read = |input: &Option<Entity<InputState>>| -> Option<String> {
        Some(input.as_ref()?.read(cx).value().to_string())
    };
    Some((read(&m.name_input)?, read(&m.unit_input)?, read(&m.equation_input)?))
}

/// The time mode set in the editor, or why the rate can't be used.
fn editor_time_mode(app: &CanDataApp, cx: &App) -> Result<TimeMode, String> {
    if !app.math.resample {
        return Ok(TimeMode::SampleAndHold);
    }
    let text = app
        .math
        .rate_input
        .as_ref()
        .map(|i| i.read(cx).value().trim().to_string())
        .unwrap_or_default();
    match text.parse::<f64>() {
        Ok(rate_hz) if rate_hz.is_finite() && rate_hz > 0.0 => {
            Ok(TimeMode::Resample { rate_hz, interpolation: app.math.interpolation })
        }
        Ok(_) => Err("The resample rate must be more than 0 Hz".into()),
        Err(_) => Err("The resample rate must be a number of Hz".into()),
    }
}

/// True if the editor holds changes that are not saved.
pub fn is_dirty(app: &CanDataApp, cx: &App) -> bool {
    let Some(signal) = app.math.selected.and_then(|i| app.math_store.signals.get(i)) else {
        return false;
    };
    editor_values(app, cx).is_some_and(|(name, unit, equation)| {
        name.trim() != signal.name || unit.trim() != signal.unit || equation != signal.equation
    }) || editor_time_mode(app, cx) != Ok(signal.time_mode)
}

/// Ask for an action that replaces the editor contents. With unsaved changes
/// it waits for the user to discard or keep them.
fn request(app: &mut CanDataApp, action: PendingMathAction, window: &mut Window, cx: &mut Context<CanDataApp>) {
    app.math.confirm_delete = false;
    if is_dirty(app, cx) {
        app.math.pending = Some(action);
    } else {
        perform(app, action, window, cx);
    }
    cx.notify();
}

fn perform(app: &mut CanDataApp, action: PendingMathAction, window: &mut Window, cx: &mut Context<CanDataApp>) {
    app.math.pending = None;
    match action {
        PendingMathAction::Select(i) => {
            app.math.selected = (i < app.math_store.signals.len()).then_some(i);
            load_editor(app, window, cx);
        }
        PendingMathAction::New => {
            let name = app.math_store.next_free_name();
            app.math_store.signals.push(MathSignal::new(name.clone()));
            app.math.selected = Some(app.math_store.signals.len() - 1);
            persist(app, format!("Math signal {} created", name));
            load_editor(app, window, cx);
            if let Some(input) = &app.math.name_input {
                input.update(cx, |state, cx| state.focus(window, cx));
            }
        }
    }
    cx.notify();
}

/// Fill the editor from the selected Math signal (or empty it).
fn load_editor(app: &mut CanDataApp, window: &mut Window, cx: &mut Context<CanDataApp>) {
    let signal = app.math.selected.and_then(|i| app.math_store.signals.get(i)).cloned();
    let (name, unit, equation, time_mode) = signal
        .map(|s| (s.name, s.unit, s.equation, s.time_mode))
        .unwrap_or_default();
    let (resample, interpolation, rate) = match time_mode {
        TimeMode::SampleAndHold => (false, Interpolation::Hold, DEFAULT_RATE_HZ.to_string()),
        TimeMode::Resample { rate_hz, interpolation } => (true, interpolation, rate_hz.to_string()),
    };
    app.math.resample = resample;
    app.math.interpolation = interpolation;
    for (input, value) in [
        (&app.math.name_input, name),
        (&app.math.unit_input, unit),
        (&app.math.equation_input, equation),
        (&app.math.rate_input, rate),
    ] {
        if let Some(input) = input {
            input.update(cx, |state, cx| state.set_value(value, window, cx));
        }
    }
}

/// Check the name typed in the editor.
fn name_error(app: &CanDataApp, cx: &App) -> Option<String> {
    let (name, _, _) = editor_values(app, cx)?;
    app.math_store.validate_name(name.trim(), app.math.selected).err()
}

/// Save the editor into the selected Math signal.
fn save(app: &mut CanDataApp, cx: &mut Context<CanDataApp>) {
    let (Some(i), Some((name, unit, equation))) = (app.math.selected, editor_values(app, cx)) else {
        return;
    };
    let name = name.trim().to_string();
    if let Err(e) = app.math_store.validate_name(&name, Some(i)) {
        app.status_msg = format!("❌ {}", e).into();
        cx.notify();
        return;
    }
    if let Check::Syntax(e) = check(&equation, &SignalIndex::default()) {
        app.status_msg = format!("❌ {}", e.message).into();
        cx.notify();
        return;
    }
    let time_mode = match editor_time_mode(app, cx) {
        Ok(mode) => mode,
        Err(e) => {
            app.status_msg = format!("❌ {}", e).into();
            cx.notify();
            return;
        }
    };
    let signal = &mut app.math_store.signals[i];
    let old_name = std::mem::replace(&mut signal.name, name.clone());
    signal.unit = unit.trim().to_string();
    signal.equation = equation;
    signal.time_mode = time_mode;
    persist(app, format!("Math signal {} saved", name));
    if old_name != name {
        rename_everywhere(app, &old_name, &name);
    }
    replot_if_plotted(app, &name);
    cx.notify();
}

/// After a rename, point the plot and the saved signal sets at the new name.
fn rename_everywhere(app: &mut CanDataApp, old: &str, new: &str) {
    let (old_id, new_id) = (format!("{}{}", MATH_ID_PREFIX, old), format!("{}{}", MATH_ID_PREFIX, new));
    app.plot_windows.rename_signal(&mut app.plot_lanes, &old_id, &new_id);
    for id in app.selected_signals.iter_mut().filter(|id| **id == old_id) {
        *id = new_id.clone();
    }
    if app.signal_set_store.rename_math_signal(old, new) {
        if let Err(e) = crate::library::save_signal_set_store(&app.signal_set_store, app.config_file_path.as_deref()) {
            app.status_msg = format!("❌ {}", e).into();
        }
    }
}

/// Compute the plot again if it shows this Math signal.
fn replot_if_plotted(app: &mut CanDataApp, name: &str) {
    let id = format!("{}{}", MATH_ID_PREFIX, name);
    if app.plot_windows.contains(&app.plot_lanes, &id) {
        crate::ui::views::chart_view::extract_and_update_series_data(app);
    }
}

/// Delete the selected Math signal.
fn delete(app: &mut CanDataApp, window: &mut Window, cx: &mut Context<CanDataApp>) {
    app.math.confirm_delete = false;
    let Some(i) = app.math.selected.filter(|i| *i < app.math_store.signals.len()) else {
        return;
    };
    let removed = app.math_store.signals.remove(i);
    app.math.selected = None;
    persist(app, format!("Math signal {} deleted", removed.name));
    replot_if_plotted(app, &removed.name);
    load_editor(app, window, cx);
    cx.notify();
}

/// Write the store to disk and report the result in the status bar.
fn persist(app: &mut CanDataApp, success: String) {
    app.status_msg = match save_math_signal_store(&app.math_store, app.config_file_path.as_deref()) {
        Ok(()) => success.into(),
        Err(e) => format!("❌ {}", e).into(),
    };
}

/// Insert text into the equation at the cursor and put the focus there.
fn insert_into_equation(app: &mut CanDataApp, text: &str, window: &mut Window, cx: &mut Context<CanDataApp>) {
    if app.math.selected.is_none() {
        return;
    }
    if let Some(input) = &app.math.equation_input {
        let text = text.to_string();
        input.update(cx, |state, cx| {
            state.insert(text, window, cx);
            state.focus(window, cx);
        });
    }
    cx.notify();
}

// ---------------------------------------------------------------------------
// Signal browser
// ---------------------------------------------------------------------------

/// One signal of the loaded channels, as the browser lists it.
struct BrowserEntry {
    ch: u16,
    is_can: bool,
    msg_id: u32,
    msg_name: String,
    name: String,
    unit: String,
}

/// Every signal of the loaded channels whose signal or message name contains
/// `filter` (ignoring case), sorted by channel, message ID and signal name.
fn browser_entries(app: &CanDataApp, filter: &str) -> Vec<BrowserEntry> {
    let filter = filter.trim().to_lowercase();
    let matches = |s: &str| filter.is_empty() || s.to_lowercase().contains(&filter);
    let mut entries = Vec::new();

    for (&ch, dbc) in &app.dbc_channels {
        for msg in dbc.messages.values() {
            let msg_match = matches(&msg.name);
            for s in msg.signals.values().filter(|s| msg_match || matches(&s.name)) {
                entries.push(BrowserEntry {
                    ch,
                    is_can: true,
                    msg_id: msg.id,
                    msg_name: msg.name.clone(),
                    name: s.name.clone(),
                    unit: s.unit.clone(),
                });
            }
        }
    }
    for (&ch, ldf) in &app.ldf_channels {
        for frame in ldf.frames.values() {
            let frame_match = matches(&frame.name);
            for m in frame.signals.iter().filter(|m| frame_match || matches(&m.signal_name)) {
                entries.push(BrowserEntry {
                    ch,
                    is_can: false,
                    msg_id: frame.id,
                    msg_name: frame.name.clone(),
                    name: m.signal_name.clone(),
                    unit: String::new(),
                });
            }
        }
    }
    entries.sort_by(|a, b| (a.ch, a.msg_id, &a.name).cmp(&(b.ch, b.msg_id, &b.name)));
    entries
}

/// The rows of the signal browser, laid out as chosen in Settings -> Plot
/// signal list: grouped by channel and message, or one alphabetical list.
pub fn browser_rows(app: &CanDataApp, filter: &str) -> Vec<BrowserRow> {
    let entries = browser_entries(app, filter);
    match app.app_config.signal_list_mode {
        SignalListMode::Grouped => grouped_rows(entries),
        SignalListMode::Alphabetical => alphabetical_rows(entries),
    }
}

fn grouped_rows(entries: Vec<BrowserEntry>) -> Vec<BrowserRow> {
    let mut rows = Vec::new();
    let mut last_ch = None;
    let mut last_msg = None;
    for e in entries {
        if last_ch != Some(e.ch) {
            let bus = if e.is_can { "CAN" } else { "LIN" };
            rows.push(BrowserRow::Channel(format!("Channel {} ({})", e.ch, bus)));
            last_ch = Some(e.ch);
            last_msg = None;
        }
        if last_msg != Some(e.msg_id) {
            rows.push(BrowserRow::Message(format!(
                "{} ({})",
                e.msg_name,
                crate::rendering::format_message_id(e.msg_id, false)
            )));
            last_msg = Some(e.msg_id);
        }
        rows.push(BrowserRow::Signal { name: e.name, unit: e.unit, note: None });
    }
    rows
}

/// One list sorted by signal name. A name found more than once is marked with
/// its channel and message ID, as in the plot signal list.
fn alphabetical_rows(mut entries: Vec<BrowserEntry>) -> Vec<BrowserRow> {
    entries.sort_by(|a, b| {
        (a.name.to_lowercase(), a.ch, a.msg_id).cmp(&(b.name.to_lowercase(), b.ch, b.msg_id))
    });
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for e in &entries {
        *counts.entry(e.name.to_lowercase()).or_insert(0) += 1;
    }
    entries
        .into_iter()
        .map(|e| {
            let duplicate = counts[&e.name.to_lowercase()] > 1;
            BrowserRow::Signal {
                note: duplicate.then(|| {
                    format!("Ch{} {}", e.ch, crate::rendering::format_message_id(e.msg_id, false))
                }),
                name: e.name,
                unit: e.unit,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

const PANEL_BG: u32 = 0x131314;
const BORDER: u32 = 0x27272a;
const TEXT: u32 = 0xe4e4e7;
const MUTED: u32 = 0x71717a;
const SELECTED_BG: u32 = 0x1e3a5f;
const HOVER_BG: u32 = 0x1f1f23;

/// Create the inputs the first time the view is shown.
fn ensure_inputs(app: &mut CanDataApp, window: &mut Window, cx: &mut Context<CanDataApp>) {
    // Redraw the view as the user types, for the unsaved mark and name check.
    fn redraw_on_change(input: &Entity<InputState>, cx: &mut Context<CanDataApp>) {
        cx.subscribe(input, |_, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        })
        .detach();
    }

    if app.math.name_input.is_none() {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Name"));
        redraw_on_change(&input, cx);
        app.math.name_input = Some(input);
    }
    if app.math.unit_input.is_none() {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Unit"));
        redraw_on_change(&input, cx);
        app.math.unit_input = Some(input);
    }
    if app.math.equation_input.is_none() {
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(6)
                .placeholder("e.g.  Motor_Speed * Motor_Torque / 9549")
        });
        redraw_on_change(&input, cx);
        app.math.equation_input = Some(input);
        // The editor starts out showing whatever is selected.
        load_editor(app, window, cx);
    }
    if app.math.rate_input.is_none() {
        let input = cx.new(|cx| InputState::new(window, cx).default_value(DEFAULT_RATE_HZ));
        redraw_on_change(&input, cx);
        app.math.rate_input = Some(input);
    }
    if app.math.search_input.is_none() {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Search signals / messages..."));
        redraw_on_change(&input, cx);
        app.math.search_input = Some(input);
    }
}

/// Render the Math view.
pub fn render_math_view(
    window: &mut Window,
    app: &mut CanDataApp,
    view: Entity<CanDataApp>,
    cx: &mut Context<CanDataApp>,
) -> impl IntoElement {
    ensure_inputs(app, window, cx);
    let index = SignalIndex::from_app(app);

    div()
        .size_full()
        .flex()
        .bg(rgb(0x0c0c0e))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .h_full()
                .flex()
                .flex_col()
                .border_r_1()
                .border_color(rgb(BORDER))
                .child(render_signal_list(app, &index, view.clone(), cx))
                .child(render_editor(app, &index, view.clone(), cx)),
        )
        .child(
            div()
                .w(px(340.))
                .flex_shrink_0()
                .h_full()
                .child(render_browser(app, view, cx)),
        )
}

fn section_header(title: &'static str) -> Div {
    div()
        .px_3()
        .h(px(36.))
        .flex()
        .items_center()
        .gap_2()
        .border_b_1()
        .border_color(rgb(BORDER))
        .bg(rgb(PANEL_BG))
        .child(
            div()
                .flex_1()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT))
                .child(title),
        )
}

/// A small text button. Disabled buttons are dimmed and do nothing.
fn small_button(
    id: &'static str,
    label: &'static str,
    enabled: bool,
    on_click: impl Fn(&mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    div()
        .id(id)
        .px_2()
        .py(px(2.))
        .rounded(px(4.))
        .border_1()
        .border_color(rgb(0x3f3f46))
        .text_xs()
        .text_color(if enabled { rgb(TEXT) } else { rgb(0x52525b) })
        .when(enabled, |el| {
            el.cursor_pointer()
                .hover(|s| s.bg(rgb(0x27272a)))
                .on_click(move |_, window, cx| on_click(window, cx))
        })
        .child(label)
}

fn render_signal_list(
    app: &CanDataApp,
    index: &SignalIndex,
    view: Entity<CanDataApp>,
    cx: &App,
) -> impl IntoElement {
    let selected = app.math.selected;
    let dirty = is_dirty(app, cx);
    let has_selection = selected.is_some();

    let view_new = view.clone();
    let view_delete = view.clone();
    let header = section_header("Math signals")
        .child(small_button("math-new", "New", true, move |window, cx| {
            view_new.update(cx, |app, cx| request(app, PendingMathAction::New, window, cx));
        }))
        .child(small_button("math-delete", "Delete", has_selection, move |_, cx| {
            view_delete.update(cx, |app, cx| {
                app.math.pending = None;
                app.math.confirm_delete = true;
                cx.notify();
            });
        }));

    let rows: Vec<AnyElement> = app
        .math_store
        .signals
        .iter()
        .enumerate()
        .map(|(i, signal)| {
            let is_selected = selected == Some(i);
            let view = view.clone();
            let (mark, mark_color) = status_mark(&check(&signal.equation, index));
            div()
                .id(("math-row", i))
                .px_3()
                .py_1()
                .flex()
                .items_center()
                .gap_3()
                .cursor_pointer()
                .when(is_selected, |el| el.bg(rgb(SELECTED_BG)))
                .when(!is_selected, |el| el.hover(|s| s.bg(rgb(HOVER_BG))))
                .on_click(move |_, window, cx| {
                    view.update(cx, |app, cx| {
                        if app.math.selected != Some(i) {
                            request(app, PendingMathAction::Select(i), window, cx);
                        }
                    });
                })
                .child(div().w(px(14.)).flex_shrink_0().text_sm().text_color(mark_color).child(mark))
                .child(
                    div()
                        .w(px(220.))
                        .truncate()
                        .text_sm()
                        .text_color(rgb(TEXT))
                        .child(if is_selected && dirty {
                            format!("{} •", signal.name)
                        } else {
                            signal.name.clone()
                        }),
                )
                .child(div().w(px(80.)).truncate().text_xs().text_color(rgb(MUTED)).child(signal.unit.clone()))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_xs()
                        .text_color(rgb(MUTED))
                        .child(if signal.equation.trim().is_empty() {
                            "(no equation)".to_string()
                        } else {
                            signal.equation.replace('\n', " ")
                        }),
                )
                .into_any_element()
        })
        .collect();

    div()
        .h(px(240.))
        .flex_shrink_0()
        .flex()
        .flex_col()
        .border_b_1()
        .border_color(rgb(BORDER))
        .child(header)
        .child(
            div()
                .id("math-signal-list")
                .flex_1()
                .overflow_y_scroll()
                .py_1()
                .when(rows.is_empty(), |el| {
                    el.child(
                        div()
                            .px_3()
                            .py_2()
                            .text_sm()
                            .text_color(rgb(MUTED))
                            .child("No Math signals yet. Click New to create one."),
                    )
                })
                .children(rows),
        )
}

/// A button in a group of choices; the chosen one is highlighted.
fn choice_button(
    id: &'static str,
    label: &'static str,
    chosen: bool,
    on_click: impl Fn(&mut App) + 'static,
) -> Stateful<Div> {
    div()
        .id(id)
        .px_2()
        .py(px(2.))
        .rounded(px(4.))
        .border_1()
        .border_color(if chosen { rgb(0x3b82f6) } else { rgb(0x3f3f46) })
        .when(chosen, |el| el.bg(rgb(SELECTED_BG)))
        .text_xs()
        .text_color(rgb(TEXT))
        .cursor_pointer()
        .hover(|s| s.bg(rgb(0x27272a)))
        .on_click(move |_, _, cx| on_click(cx))
        .child(label)
}

/// The time mode row: sample and hold, or resample at a rate with hold or
/// linear interpolation.
fn render_time_mode(
    resample: bool,
    interpolation: Interpolation,
    rate_input: Option<Entity<InputState>>,
    view: Entity<CanDataApp>,
) -> impl IntoElement {
    let set = move |view: &Entity<CanDataApp>, f: fn(&mut MathViewState)| {
        let view = view.clone();
        move |cx: &mut App| {
            view.update(cx, |app, cx| {
                f(&mut app.math);
                cx.notify();
            })
        }
    };
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(div().w(px(40.)).text_xs().text_color(rgb(MUTED)).child("Time"))
        .child(choice_button("math-hold-mode", "Sample and hold", !resample, set(&view, |m| m.resample = false)))
        .child(choice_button("math-resample-mode", "Resample", resample, set(&view, |m| m.resample = true)))
        .when(resample, |el| {
            el.when_some(rate_input, |el, input| {
                el.child(div().w(px(80.)).child(Input::new(&input).small()))
            })
            .child(div().text_xs().text_color(rgb(MUTED)).child("Hz"))
            .child(choice_button(
                "math-interp-hold",
                "Hold",
                interpolation == Interpolation::Hold,
                set(&view, |m| m.interpolation = Interpolation::Hold),
            ))
            .child(choice_button(
                "math-interp-linear",
                "Linear",
                interpolation == Interpolation::Linear,
                set(&view, |m| m.interpolation = Interpolation::Linear),
            ))
        })
}

/// The list mark and status colour for a check: ✓ usable with the active
/// library, ⚠ fine but not usable with it, ✗ can't be read.
fn status_mark(check: &Check) -> (&'static str, Rgba) {
    match check {
        Check::Ok { .. } => ("✓", rgb(0x4ade80)),
        Check::NoLibrary | Check::Names(_) => ("⚠", rgb(0xfbbf24)),
        Check::Syntax(_) => ("✗", rgb(0xf87171)),
    }
}

/// A one-line bar asking the user to confirm something.
fn confirm_bar(message: String, buttons: Vec<Stateful<Div>>) -> Div {
    div()
        .mx_3()
        .mt_2()
        .px_3()
        .py_2()
        .rounded(px(4.))
        .bg(rgb(0x3a2e12))
        .border_1()
        .border_color(rgb(0x854d0e))
        .flex()
        .items_center()
        .gap_2()
        .child(div().flex_1().text_sm().text_color(rgb(0xfde68a)).child(message))
        .children(buttons)
}

fn render_editor(
    app: &CanDataApp,
    index: &SignalIndex,
    view: Entity<CanDataApp>,
    cx: &App,
) -> impl IntoElement {
    let Some(signal) = app.math.selected.and_then(|i| app.math_store.signals.get(i)) else {
        return div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .text_sm()
            .text_color(rgb(MUTED))
            .child("Select a Math signal to edit it, or click New.")
            .into_any_element();
    };
    let (Some(name_input), Some(unit_input), Some(equation_input)) = (
        app.math.name_input.clone(),
        app.math.unit_input.clone(),
        app.math.equation_input.clone(),
    ) else {
        return div().into_any_element();
    };

    let dirty = is_dirty(app, cx);
    let name_error = name_error(app, cx);
    let equation = equation_input.read(cx).value().to_string();
    let equation_check = check(&equation, index);
    let (_, status_color) = status_mark(&equation_check);
    let time_mode = editor_time_mode(app, cx);
    let (resample, interpolation) = (app.math.resample, app.math.interpolation);
    let rate_input = app.math.rate_input.clone();

    let mut el = div().flex_1().min_h_0().flex().flex_col();

    if let Some(action) = app.math.pending {
        let (view_discard, view_keep) = (view.clone(), view.clone());
        el = el.child(confirm_bar(
            format!("{} has unsaved changes.", signal.name),
            vec![
                small_button("math-discard", "Discard", true, move |window, cx| {
                    view_discard.update(cx, |app, cx| perform(app, action, window, cx));
                }),
                small_button("math-keep", "Keep editing", true, move |_, cx| {
                    view_keep.update(cx, |app, cx| {
                        app.math.pending = None;
                        cx.notify();
                    });
                }),
            ],
        ));
    } else if app.math.confirm_delete {
        let (view_delete, view_cancel) = (view.clone(), view.clone());
        el = el.child(confirm_bar(
            {
                let sets = app.signal_set_store.sets_using_math_signal(&signal.name);
                if sets.is_empty() {
                    format!("Delete {}? This cannot be undone.", signal.name)
                } else {
                    format!(
                        "Delete {}? It is used in signal sets: {}. This cannot be undone.",
                        signal.name,
                        sets.join(", ")
                    )
                }
            },
            vec![
                small_button("math-confirm-delete", "Delete", true, move |window, cx| {
                    view_delete.update(cx, |app, cx| delete(app, window, cx));
                }),
                small_button("math-cancel-delete", "Cancel", true, move |_, cx| {
                    view_cancel.update(cx, |app, cx| {
                        app.math.confirm_delete = false;
                        cx.notify();
                    });
                }),
            ],
        ));
    }

    let label = |text: &'static str| div().text_xs().text_color(rgb(MUTED)).child(text);

    let (view_revert, view_save) = (view.clone(), view.clone());
    el.child(
        div()
            .p_3()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .gap_3()
                    .child(
                        div()
                            .w(px(260.))
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(label("Name"))
                            .child(Input::new(&name_input).small()),
                    )
                    .child(
                        div()
                            .w(px(120.))
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(label("Unit"))
                            .child(Input::new(&unit_input).small()),
                    ),
            )
            .when_some(name_error.clone(), |el, e| {
                el.child(div().text_xs().text_color(rgb(0xf87171)).child(e))
            })
            .child(label("Equation"))
            .child(Input::new(&equation_input).h(px(140.)))
            .child(
                div()
                    .text_xs()
                    .text_color(status_color)
                    .child(equation_check.summary(&equation)),
            )
            .child(render_time_mode(resample, interpolation, rate_input, view.clone()))
            .when_some(time_mode.clone().err(), |el, e| {
                el.child(div().text_xs().text_color(rgb(0xf87171)).child(e))
            })
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(small_button("math-revert", "Revert", dirty, move |window, cx| {
                        view_revert.update(cx, |app, cx| {
                            load_editor(app, window, cx);
                            cx.notify();
                        });
                    }))
                    .child(small_button("math-save", "Save", dirty && name_error.is_none() && !equation_check.is_error() && time_mode.is_ok(), move |_, cx| {
                        view_save.update(cx, |app, cx| save(app, cx));
                    })),
            ),
    )
    .into_any_element()
}

fn render_browser(app: &CanDataApp, view: Entity<CanDataApp>, cx: &App) -> impl IntoElement {
    let filter = app
        .math
        .search_input
        .as_ref()
        .map(|i| i.read(cx).value().to_string())
        .unwrap_or_default();
    let rows = Arc::new(browser_rows(app, &filter));
    let no_channels = app.dbc_channels.is_empty() && app.ldf_channels.is_empty();
    let can_insert = app.math.selected.is_some();
    let grouped = app.app_config.signal_list_mode == SignalListMode::Grouped;

    let body = if no_channels {
        div()
            .p_3()
            .text_sm()
            .text_color(rgb(MUTED))
            .child("No signals loaded. Activate a library to list its signals here.")
            .into_any_element()
    } else if rows.is_empty() {
        div()
            .p_3()
            .text_sm()
            .text_color(rgb(MUTED))
            .child("No signals match the search.")
            .into_any_element()
    } else {
        let count = rows.len();
        uniform_list("math-browser", count, move |range, _window, _cx| {
            range
                .map(|i| render_browser_row(&rows[i], grouped, can_insert, view.clone()))
                .collect::<Vec<_>>()
        })
        .size_full()
        .into_any_element()
    };

    div()
        .size_full()
        .flex()
        .flex_col()
        .child(section_header("Signals"))
        .when_some(app.math.search_input.clone(), |el, input| {
            el.child(
                div()
                    .p_2()
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .child(Input::new(&input).small()),
            )
        })
        .child(
            div()
                .px_3()
                .py_1()
                .text_xs()
                .text_color(rgb(MUTED))
                .child("Double-click a signal to insert it into the equation."),
        )
        .child(div().flex_1().min_h_0().child(body))
}

fn render_browser_row(
    row: &BrowserRow,
    grouped: bool,
    can_insert: bool,
    view: Entity<CanDataApp>,
) -> AnyElement {
    let base = div().h(px(22.)).flex().items_center().text_xs();
    match row {
        BrowserRow::Channel(label) => base
            .px_2()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(TEXT))
            .child(label.clone())
            .into_any_element(),
        BrowserRow::Message(label) => base
            .pl_4()
            .text_color(rgb(0xa1a1aa))
            .child(label.clone())
            .into_any_element(),
        BrowserRow::Signal { name, unit, note } => {
            let insert = name.clone();
            base.pl(if grouped { px(32.) } else { px(8.) })
                .pr_2()
                .gap_2()
                .text_color(rgb(TEXT))
                .when(can_insert, |el| {
                    el.cursor_pointer()
                        .hover(|s| s.bg(rgb(HOVER_BG)))
                        .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                            if event.click_count == 2 {
                                view.update(cx, |app, cx| insert_into_equation(app, &insert, window, cx));
                            }
                        })
                })
                .child(div().flex_1().min_w_0().truncate().child(name.clone()))
                .when_some(note.clone(), |el, note| {
                    el.child(div().flex_shrink_0().text_color(rgb(MUTED)).child(note))
                })
                .when(!unit.is_empty(), |el| {
                    el.child(div().flex_shrink_0().text_color(rgb(MUTED)).child(unit.clone()))
                })
                .into_any_element()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::prelude::v1::test; // `use gpui::*` brings in gpui's own `test` macro
    use parser::dbc::{DbcDatabase, Message, Signal};

    fn signal(name: &str, unit: &str) -> Signal {
        Signal {
            name: name.into(),
            start_bit: 0,
            signal_size: 8,
            byte_order: 1,
            value_type: '+',
            factor: 1.0,
            offset: 0.0,
            min: 0.0,
            max: 0.0,
            unit: unit.into(),
            receivers: vec![],
            comment: None,
        }
    }

    fn message(id: u32, name: &str, signals: &[(&str, &str)]) -> Message {
        Message {
            id,
            name: name.into(),
            dlc: 8,
            transmitter: String::new(),
            signals: signals.iter().map(|(n, u)| (n.to_string(), signal(n, u))).collect(),
            comment: None,
        }
    }

    fn app_with_dbc() -> CanDataApp {
        let mut app = CanDataApp::new_state();
        let mut dbc = DbcDatabase { messages: Default::default(), version: String::new(), description: None };
        dbc.messages.insert(0x200, message(0x200, "Brake", &[("Pressure", "bar")]));
        dbc.messages.insert(
            0x100,
            message(0x100, "Motor", &[("Motor_Torque", "Nm"), ("Motor_Speed", "rpm")]),
        );
        app.dbc_channels.insert(1, dbc);
        app
    }

    fn sig(name: &str, unit: &str) -> BrowserRow {
        BrowserRow::Signal { name: name.into(), unit: unit.into(), note: None }
    }

    #[test]
    fn browser_rows_alphabetical_marks_duplicates() {
        let mut app = app_with_dbc();
        let mut dbc2 = DbcDatabase { messages: Default::default(), version: String::new(), description: None };
        dbc2.messages.insert(0x300, message(0x300, "Other", &[("motor_speed", "rpm")]));
        app.dbc_channels.insert(2, dbc2);
        app.app_config.signal_list_mode = SignalListMode::Alphabetical;

        let rows = browser_rows(&app, "");
        let names: Vec<(&str, bool)> = rows
            .iter()
            .map(|r| match r {
                BrowserRow::Signal { name, note, .. } => (name.as_str(), note.is_some()),
                other => panic!("unexpected row {:?}", other),
            })
            .collect();
        assert_eq!(
            names,
            [("Motor_Speed", true), ("motor_speed", true), ("Motor_Torque", false), ("Pressure", false)]
        );
        assert_eq!(
            rows[0],
            BrowserRow::Signal {
                name: "Motor_Speed".into(),
                unit: "rpm".into(),
                note: Some(format!("Ch1 {}", crate::rendering::format_message_id(0x100, false))),
            }
        );
    }

    #[test]
    fn browser_rows_grouped_and_sorted() {
        let app = app_with_dbc();
        let rows = browser_rows(&app, "");
        assert_eq!(rows.len(), 6);
        assert_eq!(rows[0], BrowserRow::Channel("Channel 1 (CAN)".into()));
        assert!(matches!(&rows[1], BrowserRow::Message(m) if m.starts_with("Motor")));
        assert_eq!(rows[2], sig("Motor_Speed", "rpm"));
        assert_eq!(rows[3], sig("Motor_Torque", "Nm"));
        assert!(matches!(&rows[4], BrowserRow::Message(m) if m.starts_with("Brake")));
        assert_eq!(rows[5], sig("Pressure", "bar"));
    }

    #[test]
    fn browser_rows_filter_by_signal_or_message() {
        let app = app_with_dbc();
        let rows = browser_rows(&app, "TORQUE");
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[2], sig("Motor_Torque", "Nm"));

        // A matching message keeps all its signals.
        assert_eq!(browser_rows(&app, "brake").len(), 3);
        assert!(browser_rows(&app, "nothing").is_empty());
    }

    #[test]
    fn browser_rows_empty_without_channels() {
        assert!(browser_rows(&CanDataApp::new_state(), "").is_empty());
    }
}
