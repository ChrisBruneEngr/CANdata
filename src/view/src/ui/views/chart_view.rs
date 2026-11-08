use crate::app::CanViewerApp;
use gpui_component::input::{Input, InputEvent, InputState};
use crate::models::{DataPoint, Series};
use crate::plot_lanes::{auto_value_range, format_y_limit, pan_time_range, parse_y_range, PlotLane};
use super::plot_sidebar::DraggedSignal;
use blf::LogObject;
use gpui::prelude::*;
use gpui::*;
use gpui_component::ActiveTheme;
use gpui_component::PixelsExt;
use gpui_component::Sizable;
use gpui_component::v_flex;
use std::sync::Arc;

/// Left edge of a lane's plot area in the window: sidebar 320 + scroll area padding
/// 16 + lane padding 8 + canvas left padding 36.
const DRAW_ORIGIN_X: Pixels = px(380.0);
/// Gap from the plot area to the window's right edge: scroll area padding 16 + lane
/// padding 8 + canvas right padding 4.
const DRAW_RIGHT_MARGIN: Pixels = px(28.0);

/// Only this many signals are decoded and plotted.
const MAX_SIGNALS: usize = 20;

/// Render the plot view with signal charts
pub fn render_plot_view(window: &mut Window, app: &mut CanViewerApp, view: Entity<CanViewerApp>, cx: &mut Context<CanViewerApp>) -> impl IntoElement {
    // Safety check: prevent crash from invalid plot state after window operations
    let series_data = app.plot_data.clone();

    // Signals are added and removed from several places, so lanes catch up here.
    app.plot_lanes.sync_with_selection(&app.selected_signals);
    refresh_y_range_inputs(app, window, cx);

    // Check if plot data size is reasonable
    if series_data.len() > 100 {
        eprintln!("Warning: Excessive plot data ({} series). This may indicate a state corruption issue.", series_data.len());
        return div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgb(0x0f0f10))
            .child(div().px_4().py_2().text_sm().text_color(rgb(0xff0000))
                .child(format!("Error: Too many plot series ({}). Please reload the application.", series_data.len())));
    }

    div()
        .size_full()
        .flex()
        .flex_row() // Side-by-side: Signals on left, Charts on right
        .bg(rgb(0x0f0f10))
        .child(
            // Left Sidebar: Signal Selection
            div()
                .w(px(320.0))
                .h_full()
                .border_r_1()
                .border_color(rgb(0x27272a))
                .flex()
                .flex_col()
                .child(super::plot_sidebar::render_signal_sidebar(window, app, view.clone(), cx))
        )
        .child(
            // Right Main Area: Charts
            div()
                .flex_1()
                .h_full()
                .flex()
                .flex_col()
                .child(render_toolbar(app, cx))
                .child(render_lane_toolbar(app, cx))
                .child(render_chart_canvas(app, series_data.clone(), cx))
        )
}

/// Render the toolbar at the top of the plot area
fn render_toolbar(app: &CanViewerApp, cx: &mut Context<CanViewerApp>) -> impl IntoElement {
    let is_zoomed = app.plot_zoom_start.is_some() || app.plot_zoom_end.is_some();

    div()
        .flex()
        .items_center()
        .justify_between()
        .px_4()
        .py_1()
        .bg(rgb(0x18181b))
        .border_b_1()
        .border_color(rgb(0x27272a))
        .child(
            div()
                .text_xs()
                .text_color(rgb(0x71717a))
                .child("Click = set cursor · drag or wheel = zoom · right-drag = pan · ⌘/Ctrl + wheel = scroll lanes")
        )
        .when_some(app.plot_cursor_time, |this, time| {
            this.child(
                div()
                    .text_xs()
                    .text_color(rgb(0xd4d4d8))
                    .child(format!("Cursor: {}", format_time_relative_or_absolute(time, app.start_time)))
            )
        })
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .when(is_zoomed, |this| {
                    this.child(
                        div()
                            .px_2()
                            .py_1()
                            .bg(rgb(0x4b5563))
                            .rounded(px(4.0))
                            .cursor_pointer()
                            .hover(|s| s.bg(rgb(0x374151)))
                            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this: &mut CanViewerApp, _, _, cx| {
                                this.plot_zoom_start = None;
                                this.plot_zoom_end = None;
                                crate::ui::views::chart_view::extract_and_update_series_data(this);
                                cx.notify();
                            }))
                            .child(div().text_xs().text_color(rgb(0xffffff)).child("Reset Zoom"))
                    )
                })
        )
}


/// Render the lane buttons above the lanes.
fn render_lane_toolbar(app: &CanViewerApp, cx: &mut Context<CanViewerApp>) -> impl IntoElement {
    let lanes = &app.plot_lanes;
    let over_limit = app.selected_signals.len() > MAX_SIGNALS;

    div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap_2()
        .px_4()
        .py_1()
        .bg(rgb(0x131314))
        .border_b_1()
        .border_color(rgb(0x27272a))
        .child(render_lane_button("+ Add Lane", true, |app| app.plot_lanes.add_lane(), cx))
        .child(render_lane_button("Delete Lane", true, delete_selected_lane, cx))
        .child(render_lane_button(
            "↑ Move Up",
            lanes.can_move_up(),
            |app| app.plot_lanes.move_selected_up(),
            cx,
        ))
        .child(render_lane_button(
            "↓ Move Down",
            lanes.can_move_down(),
            |app| app.plot_lanes.move_selected_down(),
            cx,
        ))
        .child(render_toolbar_separator())
        .child(render_lane_checkbox(
            "Points",
            lanes.lanes[lanes.selected].show_points,
            toggle_lane_points,
            cx,
        ))
        .child(render_toolbar_separator())
        .child(render_y_range_controls(app, cx))
        .when(over_limit, |this| {
            this.child(
                div()
                    .text_xs()
                    .text_color(rgb(0xf59e0b))
                    .child(format!("Only the first {} signals are plotted", MAX_SIGNALS)),
            )
        })
}

/// Render the selected lane's Y-axis controls: Auto, then Min and Max.
fn render_y_range_controls(app: &CanViewerApp, cx: &mut Context<CanViewerApp>) -> impl IntoElement {
    let lane = &app.plot_lanes.lanes[app.plot_lanes.selected];
    let is_auto = lane.y_range.is_none();

    // While Auto is on, show the range it picked instead of editable boxes.
    let (min_field, max_field) = match (&app.plot_y_min_input, &app.plot_y_max_input) {
        (Some(min), Some(max)) if !is_auto => (render_y_input(min), render_y_input(max)),
        _ => {
            let (min, max) = lane_auto_range(app, lane)
                .map_or(("-".to_string(), "-".to_string()), |(min, max)| {
                    (format_y_limit(min), format_y_limit(max))
                });
            (render_y_auto_value(min), render_y_auto_value(max))
        }
    };
    let error = y_input_range(app, cx).and_then(Result::err).filter(|_| !is_auto);

    div()
        .flex()
        .items_center()
        .gap_2()
        .child(div().text_xs().text_color(rgb(0xa1a1aa)).child("Y:"))
        .child(render_lane_checkbox("Auto", is_auto, toggle_lane_auto_range, cx))
        .child(div().text_xs().text_color(rgb(0xa1a1aa)).child("Min"))
        .child(min_field)
        .child(div().text_xs().text_color(rgb(0xa1a1aa)).child("Max"))
        .child(max_field)
        .when_some(error, |this, message| {
            this.child(div().text_xs().text_color(rgb(0xf59e0b)).child(message))
        })
}

/// A divider between groups of lane controls.
fn render_toolbar_separator() -> impl IntoElement {
    div().w(px(1.0)).h(px(16.0)).mx_1().bg(rgb(0x3f3f46))
}

/// A checkbox in the lane toolbar.
fn render_lane_checkbox(
    label: &'static str,
    checked: bool,
    action: fn(&mut CanViewerApp),
    cx: &mut Context<CanViewerApp>,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_1()
        .cursor_pointer()
        .on_mouse_down(
            gpui::MouseButton::Left,
            cx.listener(move |this: &mut CanViewerApp, _, _, cx| {
                action(this);
                cx.notify();
            }),
        )
        .child(
            div()
                .size(px(12.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(2.0))
                .border_1()
                .border_color(if checked { rgb(0x3b82f6) } else { rgb(0x52525b) })
                .when(checked, |this| this.bg(rgb(0x3b82f6)))
                .text_size(px(9.0))
                .text_color(rgb(0xffffff))
                .child(if checked { "✓" } else { "" }),
        )
        .child(div().text_xs().text_color(rgb(0xe4e4e7)).child(label))
}

/// Show or hide sample dots on the selected lane.
fn toggle_lane_points(app: &mut CanViewerApp) {
    let selected = app.plot_lanes.selected;
    let lane = &mut app.plot_lanes.lanes[selected];
    lane.show_points = !lane.show_points;
}

/// An editable Min or Max box.
fn render_y_input(input: &Entity<InputState>) -> AnyElement {
    div().w(px(90.0)).child(Input::new(input).small()).into_any_element()
}

/// A read-only Min or Max value, shown while Auto is on.
fn render_y_auto_value(value: String) -> AnyElement {
    div()
        .w(px(90.0))
        .px_2()
        .text_xs()
        .text_color(rgb(0x71717a))
        .child(value)
        .into_any_element()
}

/// The range Auto picks for a lane: everything visible in its signals.
fn lane_auto_range(app: &CanViewerApp, lane: &PlotLane) -> Option<(f64, f64)> {
    auto_value_range(
        lane.signals
            .iter()
            .filter_map(|id| app.plot_data.iter().find(|series| &series.name == id))
            .flat_map(|series| series.points.iter().map(|p| p.value)),
    )
}

/// Switch the selected lane between Auto and its own limits. Turning Auto off starts
/// from the current range, so the plot doesn't jump.
fn toggle_lane_auto_range(app: &mut CanViewerApp) {
    let selected = app.plot_lanes.selected;
    let lane = &app.plot_lanes.lanes[selected];
    let new_range = match lane.y_range {
        Some(_) => None,
        None => Some(lane_auto_range(app, lane).unwrap_or((0.0, 1.0))),
    };
    app.plot_lanes.lanes[selected].y_range = new_range;
}

/// Reload the Min and Max boxes when the selected lane's limits differ from what they
/// hold, such as after selecting another lane. New boxes are created rather than
/// updated, so loading a value can't look like typing and change a lane.
fn refresh_y_range_inputs(app: &mut CanViewerApp, window: &mut Window, cx: &mut Context<CanViewerApp>) {
    let range = app.plot_lanes.lanes[app.plot_lanes.selected].y_range;
    let boxes_exist = app.plot_y_min_input.is_some();
    if range == app.plot_y_inputs_loaded && boxes_exist == range.is_some() {
        return;
    }

    app.plot_y_inputs_loaded = range;
    app.plot_y_min_input = range.map(|(min, _)| new_y_limit_input(min, window, cx));
    app.plot_y_max_input = range.map(|(_, max)| new_y_limit_input(max, window, cx));
}

/// Create a Min or Max box. Valid values apply as they're typed, so nothing depends
/// on the box losing focus and a value can't land on the wrong lane.
fn new_y_limit_input(
    value: f64,
    window: &mut Window,
    cx: &mut Context<CanViewerApp>,
) -> Entity<InputState> {
    let input = cx.new(|cx| InputState::new(window, cx).default_value(format_y_limit(value)));
    cx.subscribe(&input, |this, _input, event, cx| {
        if matches!(event, InputEvent::Change) {
            if let Some(Ok(range)) = y_input_range(this, cx) {
                let selected = this.plot_lanes.selected;
                this.plot_lanes.lanes[selected].y_range = Some(range);
                this.plot_y_inputs_loaded = Some(range);
            }
            cx.notify();
        }
    })
    .detach();
    input
}

/// What the Min and Max boxes currently hold, if they're showing.
fn y_input_range(app: &CanViewerApp, cx: &App) -> Option<Result<(f64, f64), &'static str>> {
    let min = app.plot_y_min_input.as_ref()?.read(cx).value();
    let max = app.plot_y_max_input.as_ref()?.read(cx).value();
    Some(parse_y_range(&min, &max))
}

/// A lane toolbar button. Disabled buttons are dimmed and do nothing.
fn render_lane_button(
    label: &'static str,
    enabled: bool,
    action: fn(&mut CanViewerApp),
    cx: &mut Context<CanViewerApp>,
) -> impl IntoElement {
    div()
        .px_2()
        .py_1()
        .rounded(px(4.0))
        .bg(rgb(0x27272a))
        .text_xs()
        .text_color(if enabled { rgb(0xe4e4e7) } else { rgb(0x52525b) })
        .when(enabled, |this| {
            this.cursor_pointer()
                .hover(|s| s.bg(rgb(0x3f3f46)))
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(move |this: &mut CanViewerApp, _, _, cx| {
                        action(this);
                        cx.notify();
                    }),
                )
        })
        .child(label)
}

/// Plot a signal in a lane and select that lane. A lane holds each signal only once.
pub fn add_signal_to_lane(app: &mut CanViewerApp, lane: usize, signal_id: &str) {
    app.plot_lanes.select(lane);
    if !app.plot_lanes.add_signal(lane, signal_id) {
        return;
    }
    if !app.selected_signals.iter().any(|s| s == signal_id) {
        app.selected_signals.push(signal_id.to_string());
    }
    app.active_signal_set = None;
    extract_and_update_series_data(app);
}

/// Take a signal out of one lane, and stop plotting it if no other lane uses it.
fn remove_signal_from_lane(app: &mut CanViewerApp, lane: usize, signal_id: &str) {
    app.plot_lanes.remove_signal(lane, signal_id);
    let lanes = &app.plot_lanes;
    app.selected_signals.retain(|signal| lanes.contains(signal));
    app.active_signal_set = None;
    extract_and_update_series_data(app);
}

/// Delete the selected lane and stop plotting signals no other lane uses.
fn delete_selected_lane(app: &mut CanViewerApp) {
    app.plot_lanes.delete_selected();
    let lanes = &app.plot_lanes;
    app.selected_signals.retain(|signal| lanes.contains(signal));
    app.active_signal_set = None;
    extract_and_update_series_data(app);
}

/// The time range every lane shows: the zoom window, or all plotted data.
fn visible_time_range(app: &CanViewerApp) -> Option<(f64, f64)> {
    if let (Some(start), Some(end)) = (app.plot_zoom_start, app.plot_zoom_end) {
        return Some((start, end));
    }
    app.plot_data
        .iter()
        .filter_map(|series| Some((series.points.first()?.time, series.points.last()?.time)))
        .reduce(|(lo, hi), (first, last)| (lo.min(first), hi.max(last)))
}

/// Line colour for the signal at `position` within a lane.
pub fn lane_color(position: usize) -> Hsla {
    const COLORS: [(f32, f32, f32); 10] = [
        (217.0, 0.91, 0.60),
        (142.0, 0.71, 0.45),
        (35.0, 0.92, 0.50),
        (280.0, 0.61, 0.60),
        (0.0, 0.84, 0.60),
        (180.0, 0.71, 0.50),
        (55.0, 0.85, 0.55),
        (320.0, 0.70, 0.65),
        (95.0, 0.60, 0.55),
        (20.0, 0.50, 0.60),
    ];
    let (hue, saturation, lightness) = COLORS[position % COLORS.len()];
    hsla(hue / 360.0, saturation, lightness, 1.0)
}

/// Render the chart canvas — zoom box + one card per lane.
fn render_chart_canvas(app: &CanViewerApp, series_data: Arc<[Series]>, cx: &mut Context<CanViewerApp>) -> AnyElement {
    let is_dragging = app.is_dragging_zoom;
    let drag_start = app.zoom_drag_start_x;
    let drag_current = app.zoom_drag_current_x;
    let start_time = app.start_time;
    let cursor_time = app.plot_cursor_time;

    // Constant offsets based on layout:
    //   sidebar 320 → v_flex p_4 left 16 → card p_2 left 8 → canvas pad_left 36
    // 每张 chart 卡片的实际绘图区原点（相对于窗口左边）= 380px，
    // 右边界 = window_width - 16 - 8 - 4 = window_width - 28。
    // 之前用 chart_start_x = 336 当绘图原点，导致 hover 竖线、zoom 拖拽框、
    // 缩放中心都向左偏 ~44px。
    let sidebar_offset = px(320.0);
    let draw_origin_x = DRAW_ORIGIN_X;
    let draw_right_margin = DRAW_RIGHT_MARGIN;

    let time_range = visible_time_range(app);
    let mut lanes = Vec::with_capacity(app.plot_lanes.lanes.len());
    for (index, lane) in app.plot_lanes.lanes.iter().enumerate() {
        lanes.push(render_lane(
            index,
            lane,
            index == app.plot_lanes.selected,
            app,
            &series_data,
            time_range,
            start_time,
            cursor_time,
            cx,
        ));
    }

    // The v_flex is the scroll container. Lanes share its height equally until they
    // reach their minimum height, and then it scrolls. The zoom-box overlay is
    // positioned absolutely inside it so it stays aligned with the lanes.
    v_flex()
        .id("plot-scroll-container")
        .flex_1()
        .min_h_0()  // allow the flex child to shrink so its content can scroll
        .relative()
        .p_4()
        .gap_2()
        .overflow_y_scroll()
        .track_scroll(&app.plot_scroll_handle)
        .when(app.plot_pan_start.is_some(), |this| this.cursor_grabbing())
        .children(lanes)
        // Zoom Box Overlay Layer
        .when(is_dragging, |this| {
            if let (Some(start), Some(current)) = (drag_start, drag_current) {
                // Convert global mouse coordinates to local container coordinates.
                // The overlay is a child of this v_flex, whose bounds.origin.x =
                // sidebar_offset (320). Use sidebar_offset, NOT chart_start_x (336),
                // otherwise the box is shifted 16px left of the actual drag.
                let left_global = start.min(current);
                let right_global = start.max(current);

                let left_local = (left_global - sidebar_offset).max(px(0.0));
                let width = right_global - left_global; // Width is the delta

                this.child(
                    div()
                        .absolute()
                        .top_0()
                        .left(left_local)
                        .w(width)
                        .h_full()
                        .bg(rgba(0x3b82f633)) // Light blue with transparency
                        .border_x_1()
                        .border_color(rgb(0x3b82f6))
                )
            } else {
                this
            }
        })
        // Global Canvas Interactions
        .on_mouse_down(MouseButton::Left, cx.listener(move |this: &mut CanViewerApp, event: &MouseDownEvent, _, cx| {
             // Check for double-click to reset zoom
             if event.click_count == 2 {
                 this.plot_zoom_start = None;
                 this.plot_zoom_end = None;
                 crate::ui::views::chart_view::extract_and_update_series_data(this);
                 eprintln!("🔄 Double-click detected - resetting zoom");
                 cx.notify();
                 return;
             }
             
             this.is_dragging_zoom = true;
             this.zoom_drag_start_x = Some(event.position.x);
             this.zoom_drag_current_x = Some(event.position.x);
             cx.notify();
        }))
        // Right-drag pans, which only means something while zoomed in.
        .on_mouse_down(MouseButton::Right, cx.listener(move |this: &mut CanViewerApp, event: &MouseDownEvent, _, cx| {
            if let (Some(start), Some(end)) = (this.plot_zoom_start, this.plot_zoom_end) {
                this.plot_pan_start = Some((event.position.x, start, end));
                cx.notify();
            }
        }))
        .on_mouse_up(MouseButton::Right, cx.listener(move |this: &mut CanViewerApp, _, _, cx| {
            this.plot_pan_start = None;
            cx.notify();
        }))
        .on_mouse_move(cx.listener(move |this: &mut CanViewerApp, event: &MouseMoveEvent, window, cx| {
            // Handle Zoom Dragging
            if this.is_dragging_zoom {
                this.zoom_drag_current_x = Some(event.position.x);
                cx.notify();
            }

            if this.plot_pan_start.is_some() {
                // The button may have been released outside the plot.
                if event.pressed_button == Some(MouseButton::Right) {
                    pan_plot(this, event.position.x, window);
                } else {
                    this.plot_pan_start = None;
                }
                cx.notify();
            }
        }))
        .on_mouse_up(MouseButton::Left, cx.listener(move |this: &mut CanViewerApp, event: &MouseUpEvent, window, cx| {
            if !this.is_dragging_zoom { return; }
            
            if let Some(start_x) = this.zoom_drag_start_x {
                let end_x = event.position.x;
                let (x1, x2) = (start_x.min(end_x), start_x.max(end_x));
                
                // Only zoom if the selection is significant (> 10 pixels)
                if (x1 - x2).abs() > px(10.0) {
                    // Use stored full data time range for accurate mapping
                    let (min_t, max_t) = match (this.plot_full_time_min, this.plot_full_time_max) {
                        (Some(mn), Some(mx)) if mn < mx => (mn, mx),
                        _ => {
                            let mut min_t = f64::MAX;
                            let mut max_t = f64::MIN;
                            for series in this.plot_full_data.iter() {
                                for p in series.points.iter() {
                                    if p.time < min_t { min_t = p.time; }
                                    if p.time > max_t { max_t = p.time; }
                                }
                            }
                            if min_t == f64::MAX { (0.0, 1.0) } else { (min_t, max_t) }
                        }
                    };
                    
                    // Get current zoomed range for coordinate mapping
                    let (cur_min, cur_max) = if let (Some(s), Some(e)) = (this.plot_zoom_start, this.plot_zoom_end) {
                        (s, e)
                    } else {
                        (min_t, max_t)
                    };
                    
                    if cur_min < cur_max {
                        let window_width = window.bounds().size.width;
                        // Match the actual chart drawing area used by render_lane
                        // (sidebar + v_flex p_4 + card p_2 + canvas pad_left = 380).
                        let plot_width = f32::from(window_width - draw_origin_x - draw_right_margin);

                        // Map relative x to time within current visible range
                        let start_rel = f32::from(x1 - draw_origin_x) / plot_width;
                        let end_rel = f32::from(x2 - draw_origin_x) / plot_width;

                        let cur_range = cur_max - cur_min;
                        let new_start = cur_min + cur_range * start_rel as f64;
                        let new_end = cur_min + cur_range * end_rel as f64;

                        this.plot_zoom_start = Some(new_start.max(min_t));
                        this.plot_zoom_end = Some(new_end.min(max_t));
                        // Fast filter: just slice plot_full_data, no message re-scan
                        crate::ui::views::chart_view::apply_zoom_to_full_data(this);
                    }
                } else {
                    // Too short to be a zoom drag: treat it as a click to set the cursor.
                    let draw_right = window.bounds().size.width - draw_right_margin;
                    set_plot_cursor(this, end_x, draw_origin_x, draw_right);
                }
            }
            
            this.is_dragging_zoom = false;
            this.zoom_drag_start_x = None;
            this.zoom_drag_current_x = None;
            cx.notify();
        }))
        .into_any_element()
}

/// Zoom around the mouse with the wheel. Returns false if the event wasn't for zooming,
/// so the wheel can scroll the lanes instead: Ctrl/Cmd is held, nothing is plotted, or
/// the mouse is outside the plot area.
fn zoom_with_wheel(this: &mut CanViewerApp, event: &ScrollWheelEvent, window: &Window) -> bool {
    if event.modifiers.secondary() || this.plot_data.is_empty() {
        return false;
    }

    let draw_origin_x = DRAW_ORIGIN_X;
    let draw_right_margin = DRAW_RIGHT_MARGIN;
    let window_width = window.bounds().size.width;
    let mouse_x = event.position.x;
    if mouse_x < draw_origin_x || mouse_x > (window_width - draw_right_margin) {
        return false;
    }

    // plot_data is filtered by zoom range, so its min/max is the zoomed
    // range, not the full data range. Prefer the stored full-time bounds.
    let (abs_min_t, abs_max_t) = match (this.plot_full_time_min, this.plot_full_time_max) {
        (Some(min_t), Some(max_t)) if min_t < max_t => (min_t, max_t),
        _ => {
            let mut min_t = f64::MAX;
            let mut max_t = f64::MIN;
            for series in this.plot_data.iter() {
                for p in series.points.iter() {
                    if p.time < min_t { min_t = p.time; }
                    if p.time > max_t { max_t = p.time; }
                }
            }
            if min_t == f64::MAX { return true; }
            (min_t, max_t)
        }
    };

    let (current_min, current_max) = if let (Some(s), Some(e)) = (this.plot_zoom_start, this.plot_zoom_end) {
        (s, e)
    } else {
        (abs_min_t, abs_max_t)
    };

    let current_range = current_max - current_min;
    if current_range <= 0.0 {
        return true;
    }

    let scroll_delta = match event.delta {
        gpui::ScrollDelta::Lines(point) => point.y,
        gpui::ScrollDelta::Pixels(pixels) => f32::from(pixels.y),
    };

    // Mac 触控板会产生大量微小滚动事件，过低的阈值会导致轻微触碰就触发缩放。
    if scroll_delta.abs() < 0.5 {
        return true;
    }

    // Try reversed direction:
    // Scroll backward (toward user) = positive delta = zoom IN
    // Scroll forward (away from user) = negative delta = zoom OUT
    let zoom_in = scroll_delta > 0.0;

    // 单次缩放系数：1.1 比之前的 1.2 更平缓，避免一次滚动跳太多
    let zoom_factor = 1.1;

    let (new_min, new_max) = if zoom_in {
        // Zoom in: reduce range
        let new_range = current_range / zoom_factor;

        // Minimum range = the smallest interval that still contains at
        // least one data point. Use a fraction of the full data range,
        // but never smaller than the time between two adjacent points
        // (approximated by abs_range / total_point_count).
        let abs_range = abs_max_t - abs_min_t;
        let total_points: usize = this.plot_full_data.iter().map(|s| s.points.len()).sum();
        let min_range = if total_points > 1 {
            // smallest gap between any two adjacent points across all series
            let mut smallest_gap = f64::MAX;
            for series in this.plot_full_data.iter() {
                let mut prev: Option<f64> = None;
                for p in series.points.iter() {
                    if let Some(prev_t) = prev {
                        let gap = p.time - prev_t;
                        if gap > 0.0 && gap < smallest_gap {
                            smallest_gap = gap;
                        }
                    }
                    prev = Some(p.time);
                }
            }
            if smallest_gap == f64::MAX {
                // all points at the same time → use 1/1000 of range
                (abs_range / 1000.0).max(1e-6)
            } else {
                // 至少保留 2 个点可见：窗口宽度 >= 2 * 最小间隔，
                // 即使窗口偏移到中间也至少能覆盖 2 个相邻点。
                (smallest_gap * 2.0).max(1e-6)
            }
        } else {
            // less than 2 points total → no zoom-in needed
            (abs_range / 1000.0).max(1e-6)
        };

        if new_range < min_range {
            return true;
        }

        // Zoom centered on the mouse position, not the chart center,
        // so the data under the cursor stays put.
        let chart_width_px = window_width - draw_origin_x - draw_right_margin;
        let mouse_t_ratio = if current_max > current_min && chart_width_px > gpui::px(0.0) {
            let r = (mouse_x - draw_origin_x) / chart_width_px;
            f32::max(0.0, f32::min(1.0, r)) as f64
        } else {
            0.5
        };
        let focus_t = current_min + (current_max - current_min) * mouse_t_ratio;
        let new_min = focus_t - new_range * mouse_t_ratio;
        let new_max = focus_t + new_range * (1.0 - mouse_t_ratio);
        (new_min, new_max)
    } else {
        // Zoom out: expand toward full data range, centered on mouse.
        // 用一个更激进的步长：至少扩展 abs_range 的 10%，避免从极小 zoom
        // 花几十次滚动才能缩回来。
        let abs_range = abs_max_t - abs_min_t;
        let new_range = (current_range * zoom_factor).max(current_range + 0.1 * abs_range);

        let chart_width_px = window_width - draw_origin_x - draw_right_margin;
        let mouse_t_ratio = if current_max > current_min && chart_width_px > gpui::px(0.0) {
            let r = (mouse_x - draw_origin_x) / chart_width_px;
            f32::max(0.0, f32::min(1.0, r)) as f64
        } else {
            0.5
        };
        let focus_t = current_min + (current_max - current_min) * mouse_t_ratio;
        let mut new_min = focus_t - new_range * mouse_t_ratio;
        let mut new_max = focus_t + new_range * (1.0 - mouse_t_ratio);

        // Clamp to data boundaries
        new_min = new_min.max(abs_min_t);
        new_max = new_max.min(abs_max_t);

        (new_min, new_max)
    };

    let new_range = new_max - new_min;

    // Check if we've reached or exceeded the full range
    let abs_range = abs_max_t - abs_min_t;
    // Use a more lenient threshold (0.95 instead of 0.999) to make it easier to reset
    let is_at_or_near_full_range = new_range >= abs_range * 0.95;

    if is_at_or_near_full_range {
        // Reset to show full data
        this.plot_zoom_start = None;
        this.plot_zoom_end = None;
    } else {
        this.plot_zoom_start = Some(new_min);
        this.plot_zoom_end = Some(new_max);
    }

    // Fast filter: slice plot_full_data without re-decoding messages
    crate::ui::views::chart_view::apply_zoom_to_full_data(this);
    true
}

/// The time range of all decoded data, which panning stays within.
fn full_time_range(app: &CanViewerApp) -> Option<(f64, f64)> {
    match (app.plot_full_time_min, app.plot_full_time_max) {
        (Some(min), Some(max)) if min < max => Some((min, max)),
        _ => None,
    }
}

/// Move the zoom window to follow a right-drag. Dragging right shows earlier data, as
/// if pulling the plot along with the mouse.
fn pan_plot(app: &mut CanViewerApp, mouse_x: Pixels, window: &Window) {
    let (Some((start_x, start, end)), Some((min, max))) = (app.plot_pan_start, full_time_range(app)) else {
        return;
    };
    let width = window.bounds().size.width - DRAW_ORIGIN_X - DRAW_RIGHT_MARGIN;
    if width <= px(0.0) {
        return;
    }

    let shift = -((mouse_x - start_x) / width) as f64 * (end - start);
    let (new_start, new_end) = pan_time_range(start, end, shift, min, max);

    // A window with no points at all would reset the zoom, so hold position over gaps
    // in the data until the drag reaches more of it.
    let has_data = app.plot_full_data.iter().any(|series| {
        let first = series.points.partition_point(|p| p.time < new_start);
        series.points.get(first).is_some_and(|p| p.time <= new_end)
    });
    if has_data {
        app.plot_zoom_start = Some(new_start);
        app.plot_zoom_end = Some(new_end);
        apply_zoom_to_full_data(app);
    }
}

/// Helper to format time as absolute `YYYY-MM-DD HH:MM:SS.ffffff` (microsecond
/// precision, matching the log view's `%H:%M:%S%.6f`).
///
/// `time` is **Unix epoch seconds** (abs_ns / 1e9). The MergedView rewrites
/// every LogObject's `object_time_stamp` to absolute Unix nanoseconds during
/// multi-file merge (see `domain/multi_file.rs:160`), so we don't add
/// `start_time` here — `time` already is the wall-clock instant.
///
/// `start_time` is kept in the signature only for backward compatibility with
/// callers that still thread it through; it's not used.
fn format_time_relative_or_absolute(time: f64, _start_time: Option<chrono::NaiveDateTime>) -> String {
    use chrono::{TimeZone, Utc};
    let ns = (time * 1_000_000_000.0) as i64;
    let dt = Utc.timestamp_nanos(ns);
    dt.naive_utc().format("%Y-%m-%d %H:%M:%S%.6f").to_string()
}

/// Render one lane: a header naming its signals, then their shared plot.
#[allow(clippy::too_many_arguments)]
fn render_lane(
    index: usize,
    lane: &PlotLane,
    is_selected: bool,
    app: &CanViewerApp,
    series_data: &Arc<[Series]>,
    time_range: Option<(f64, f64)>,
    start_time: Option<chrono::NaiveDateTime>,
    cursor_time: Option<f64>,
    cx: &mut Context<CanViewerApp>,
) -> AnyElement {
    let mut header_items = Vec::with_capacity(lane.signals.len());
    // (index into series_data, line colour) for each signal with points to draw
    let mut traces: Vec<(usize, Hsla)> = Vec::with_capacity(lane.signals.len());

    for (position, signal_id) in lane.signals.iter().enumerate() {
        let color = lane_color(position);
        let name = signal_id.split(':').last().unwrap_or(signal_id);
        let series_index = series_data.iter().position(|s| &s.name == signal_id);

        let label = match series_index.map(|i| &series_data[i]) {
            Some(series) => super::plot_sidebar::signal_label(name, series.unit.as_deref().unwrap_or("")),
            None => format!("{} (no data)", name),
        };
        let title = match app.plot_cursor_values.get(signal_id) {
            Some(value) => format!("{} {}", label, value),
            None => label,
        };
        let id_to_remove = signal_id.clone();
        header_items.push(
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(div().size(px(8.0)).rounded(px(2.0)).bg(color))
                .child(div().text_xs().text_color(color).child(title))
                .child(
                    div()
                        .px_1()
                        .text_xs()
                        .text_color(rgb(0x71717a))
                        .cursor_pointer()
                        .hover(|s| s.text_color(rgb(0xf38ba8)))
                        .child("✕")
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(move |this: &mut CanViewerApp, _, _, cx| {
                                cx.stop_propagation();
                                remove_signal_from_lane(this, index, &id_to_remove);
                                cx.notify();
                            }),
                        ),
                ),
        );

        if let Some(i) = series_index.filter(|i| !series_data[*i].points.is_empty()) {
            traces.push((i, color));
        }
    }

    let body = match time_range.filter(|_| !traces.is_empty()) {
        Some(range) => render_lane_plot(
            series_data.clone(),
            traces,
            range,
            lane.y_range,
            start_time,
            lane.show_points,
            cursor_time,
        )
        .into_any_element(),
        None => div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .text_xs()
            .text_color(rgb(0x52525b))
            .child(if lane.signals.is_empty() {
                "Drag a signal here, or double-click it to add it to the selected lane"
            } else {
                "No data: check that channel IDs match (DBC vs log), or check the time range"
            })
            .into_any_element(),
    };

    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_h(px(100.0))
        .bg(rgb(0x18181b))
        .border_1()
        .border_color(if is_selected { rgb(0x3b82f6) } else { rgb(0x27272a) })
        .rounded_lg()
        .p_2()
        .drag_over::<DraggedSignal>(|style, _, _, _| {
            style.bg(rgb(0x1e293b)).border_color(rgb(0x60a5fa))
        })
        .on_drop(cx.listener(move |this: &mut CanViewerApp, signal: &DraggedSignal, _, cx| {
            add_signal_to_lane(this, index, &signal.id);
            cx.notify();
        }))
        .on_mouse_down(
            gpui::MouseButton::Left,
            cx.listener(move |this: &mut CanViewerApp, _, _, cx| {
                this.plot_lanes.select(index);
                cx.notify();
            }),
        )
        // Stopping the event here prevents the scroll area from also scrolling.
        .on_scroll_wheel(cx.listener(
            |this: &mut CanViewerApp, event: &ScrollWheelEvent, window, cx| {
                if zoom_with_wheel(this, event, window) {
                    cx.stop_propagation();
                    cx.notify();
                }
            },
        ))
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_x_3()
                .min_h(px(16.0))
                // Clicking the header selects the lane without moving the cursor.
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(move |this: &mut CanViewerApp, _, _, cx| {
                        cx.stop_propagation();
                        this.plot_lanes.select(index);
                        cx.notify();
                    }),
                )
                .children(header_items),
        )
        .child(div().flex_1().min_h_0().py_1().child(body))
        .into_any_element()
}

/// Draw a lane's signals on one set of axes — gpui canvas self-drawn, no grid.
fn render_lane_plot(
    series_data: Arc<[Series]>,
    traces: Vec<(usize, Hsla)>,
    time_range: (f64, f64),
    y_range: Option<(f64, f64)>,
    start_time: Option<chrono::NaiveDateTime>,
    show_points: bool,
    cursor_time: Option<f64>,
) -> impl IntoElement {
    let prepaint_series = series_data.clone();
    let prepaint_traces = traces.clone();

    gpui::canvas(
        move |bounds, _window, _cx| {
            // All signals in a lane share one Y range; every lane shares one time range.
            let (min_v, max_v) = match y_range {
                Some(range) => range,
                None => auto_value_range(
                    prepaint_traces
                        .iter()
                        .flat_map(|(i, _)| prepaint_series[*i].points.iter().map(|p| p.value)),
                )?,
            };
            let (min_t, max_t) = time_range;
            Some(ChartLayout {
                bounds,
                min_t,
                min_v,
                max_v,
                t_range: (max_t - min_t).max(1e-9),
                v_range: (max_v - min_v).max(1e-9),
            })
        },
        move |_bounds, layout, window, cx| {
            let layout = match layout {
                Some(l) => l,
                None => return,
            };
            let bounds = layout.bounds;
            let pad_left = px(36.0);
            let pad_right = px(4.0);
            let pad_top = px(4.0);
            let pad_bottom = px(14.0);
            let chart_w = bounds.size.width - pad_left - pad_right;
            let chart_h = bounds.size.height - pad_top - pad_bottom;

            let origin_x = bounds.origin.x + pad_left;
            let origin_y = bounds.origin.y + pad_top;
            let x_axis_y = bounds.origin.y + pad_top + chart_h;

            let to_x = |time: f64| {
                origin_x + px(((time - layout.min_t) / layout.t_range * chart_w.as_f32() as f64) as f32)
            };
            let to_y = |value: f64| {
                origin_y + px(((layout.max_v - value) / layout.v_range * chart_h.as_f32() as f64) as f32)
            };

            let stroke_color = cx.theme().border;

            // Limits can cut through a signal, so its lines stay inside the plot area.
            // The small margin keeps a line along the very edge fully visible.
            let plot_area = ContentMask {
                bounds: gpui::Bounds::new(
                    point(origin_x - px(2.), origin_y - px(2.)),
                    size(chart_w + px(4.), chart_h + px(4.)),
                ),
            };

            // === Axis lines ===
            {
                let mut b = PathBuilder::stroke(px(1.));
                b.move_to(point(origin_x, origin_y));
                b.line_to(point(origin_x, x_axis_y));
                if let Ok(path) = b.build() {
                    window.paint_path(path, stroke_color);
                }
            }
            {
                let mut b = PathBuilder::stroke(px(1.));
                b.move_to(point(origin_x, x_axis_y));
                b.line_to(point(bounds.origin.x + pad_left + chart_w, x_axis_y));
                if let Ok(path) = b.build() {
                    window.paint_path(path, stroke_color);
                }
            }

            // === Y ticks and labels ===
            let n_y_ticks = calc_y_tick_count(chart_h.as_f32());
            let mut y_labels: Vec<gpui_component::plot::label::Text> = Vec::with_capacity(n_y_ticks);
            for i in 0..n_y_ticks {
                // i=0 is max_v at the top; the last tick is min_v at the bottom
                let ratio = if n_y_ticks == 1 { 0.0 } else { i as f32 / (n_y_ticks - 1) as f32 };
                let v = layout.max_v - (layout.max_v - layout.min_v) * ratio as f64;
                let y_px = origin_y + chart_h * ratio;
                {
                    let mut b = PathBuilder::stroke(px(1.));
                    b.move_to(point(origin_x - px(4.), y_px));
                    b.line_to(point(origin_x, y_px));
                    if let Ok(path) = b.build() {
                        window.paint_path(path, stroke_color);
                    }
                }
                // PlotLabel::paint adds bounds.origin itself, so these coordinates
                // must be relative to the canvas.
                y_labels.push(
                    gpui_component::plot::label::Text::new(
                        format!("{:.1}", v),
                        point(
                            origin_x - px(6.) - bounds.origin.x,
                            y_px - px(5.) - bounds.origin.y,
                        ),
                        cx.theme().muted_foreground,
                    )
                    .font_size(px(10.))
                    .align(gpui::TextAlign::Right),
                );
            }
            gpui_component::plot::PlotLabel::new(y_labels).paint(&bounds, window, cx);

            // === X ticks and labels ===
            let n_ticks = calc_x_tick_count(chart_w.as_f32());
            let mut x_labels: Vec<gpui_component::plot::label::Text> = Vec::with_capacity(n_ticks);
            for i in 0..n_ticks {
                let ratio = if n_ticks == 1 { 0.0 } else { i as f32 / (n_ticks - 1) as f32 };
                let x_px = origin_x + chart_w * ratio;
                {
                    let mut b = PathBuilder::stroke(px(1.));
                    b.move_to(point(x_px, x_axis_y));
                    b.line_to(point(x_px, x_axis_y + px(4.)));
                    if let Ok(path) = b.build() {
                        window.paint_path(path, stroke_color);
                    }
                }
                // Drop the date (the first 11 characters) from axis labels.
                let t = layout.min_t + layout.t_range * ratio as f64;
                let full_label = format_time_relative_or_absolute(t, start_time);
                let label_text = if full_label.starts_with("Time: ") {
                    full_label
                } else {
                    full_label.get(11..).unwrap_or(&full_label).to_string()
                };
                x_labels.push(
                    gpui_component::plot::label::Text::new(
                        label_text,
                        point(x_px - bounds.origin.x, x_axis_y + px(4.) - bounds.origin.y),
                        cx.theme().muted_foreground,
                    )
                    .font_size(px(10.))
                    .align(gpui::TextAlign::Center),
                );
            }
            gpui_component::plot::PlotLabel::new(x_labels).paint(&bounds, window, cx);

            // === Signal lines and optional points ===
            window.with_content_mask(Some(plot_area.clone()), |window| {
                for (series_index, color) in traces.iter() {
                    let points = &series_data[*series_index].points;

                    let mut builder = PathBuilder::stroke(px(1.0));
                    let mut started = false;
                    for p in points.iter() {
                        let position = point(to_x(p.time), to_y(p.value));
                        if started {
                            builder.line_to(position);
                        } else {
                            builder.move_to(position);
                            started = true;
                        }
                    }
                    if started {
                        if let Ok(path) = builder.build() {
                            window.paint_path(path, *color);
                        }
                    }

                    if show_points {
                        for p in points.iter() {
                            let dot_bounds = gpui::Bounds::new(
                                point(to_x(p.time) - px(1.5), to_y(p.value) - px(1.5)),
                                size(px(3.), px(3.)),
                            );
                            window.paint_quad(gpui::fill(dot_bounds, *color));
                        }
                    }
                }
            });

            // === Cursor line, and a marker on each signal ===
            let Some(cursor) = cursor_time else {
                return;
            };
            if cursor < layout.min_t || cursor > layout.min_t + layout.t_range {
                return;
            }
            let line_x = to_x(cursor);
            let mut b = PathBuilder::stroke(px(1.));
            b.move_to(point(line_x, origin_y));
            b.line_to(point(line_x, x_axis_y));
            if let Ok(path) = b.build() {
                window.paint_path(path, gpui::rgba(0xd4d4d8cc));
            }

            window.with_content_mask(Some(plot_area), |window| {
                for (series_index, color) in traces.iter() {
                    let points = &series_data[*series_index].points;
                    // Latest sample at or before the cursor: the same rule as the
                    // signal list, so the two always agree.
                    let idx = points.partition_point(|p| p.time <= cursor);
                    if let Some(marker) = idx.checked_sub(1).and_then(|i| points.get(i)) {
                        window.paint_quad(gpui::fill(
                            gpui::Bounds::new(
                                point(to_x(marker.time) - px(2.), to_y(marker.value) - px(2.)),
                                size(px(4.), px(4.)),
                            ),
                            *color,
                        ));
                    }
                }
            });
        },
    )
    .size_full()
}

/// Canvas 自绘坐标变换参数
struct ChartLayout {
    bounds: gpui::Bounds<gpui::Pixels>,
    min_t: f64,
    t_range: f64,
    min_v: f64,
    max_v: f64,
    v_range: f64,
}


/// Filter existing plot data by zoom range
/// This is used when messages is empty but plot_data has already been extracted
fn filter_series_data_by_zoom(app: &CanViewerApp) -> Arc<[Series]> {
    eprintln!("🔍 Filtering existing plot data by zoom range...");

    let has_zoom = app.plot_zoom_start.is_some() || app.plot_zoom_end.is_some();

    if !has_zoom {
        // No zoom filter, return original data
        eprintln!("📊 No zoom filter, returning original plot_data ({} series)", app.plot_data.len());
        return app.plot_data.clone();
    }

    // Apply zoom filter to existing plot_data
    let filtered_series: Vec<Series> = app.plot_data.iter().map(|series| {
        let filtered_points: Vec<DataPoint> = series.points.iter()
            .filter(|p| {
                if let Some(start) = app.plot_zoom_start {
                    if p.time < start { return false; }
                }
                if let Some(end) = app.plot_zoom_end {
                    if p.time > end { return false; }
                }
                true
            })
            .map(|p| *p)
            .collect();

        eprintln!("  📈 Filtered '{}' from {} to {} points",
            series.name, series.points.len(), filtered_points.len());

        Series {
            name: series.name.clone(),
            color: series.color,
            unit: series.unit.clone(),
            points: filtered_points,
            time_labels: Vec::new(), // Will be regenerated by renderer
        }
    }).collect();

    eprintln!("✅ Filtering complete: {} series", filtered_series.len());
    Arc::from(filtered_series)
}

/// Extract series data from application state - SAFE VERSION
pub fn extract_series_data(app: &CanViewerApp) -> Arc<[Series]> {
    eprintln!("🔍 Starting data extraction...");

    // If messages is empty but we have plot_data, filter the existing plot_data
    // This handles the case after window maximize/restore where messages is cleared
    if app.messages.is_empty() && !app.plot_data.is_empty() {
        eprintln!("📊 Messages empty, using existing plot_data ({} series)", app.plot_data.len());
        return filter_series_data_by_zoom(app);
    }

    // Limit processing to avoid stack overflow, but high enough for complete logs
    const MAX_MESSAGES: usize = 10_000_000;
    const MAX_RAW_POINTS: usize = 1_000_000; // Max points: 1M (needed for 10min @ 1ms = 600K points)
    const SAMPLING_INTERVAL_MS: f64 = 1.0; // 1ms sampling interval

    let signal_count = app.selected_signals.len().min(MAX_SIGNALS);
    let message_count = app.messages.len().min(MAX_MESSAGES);

    eprintln!("📊 Processing {} signals from {} messages", signal_count, message_count);

    if signal_count == 0 || message_count == 0 {
        eprintln!("⚠️  No data to process");
        return Arc::from([]);
    }

    let mut all_series: Vec<Series> = Vec::with_capacity(signal_count);
    
    let colors = [
        hsla(217.0 / 360.0, 0.91, 0.6, 1.0),
        hsla(142.0 / 360.0, 0.71, 0.45, 1.0),
        hsla(35.0 / 360.0, 0.92, 0.5, 1.0),
        hsla(280.0 / 360.0, 0.61, 0.5, 1.0),
        hsla(0.0 / 360.0, 0.84, 0.6, 1.0),
        hsla(180.0 / 360.0, 0.71, 0.5, 1.0),
    ];

    // Process each signal
    for (idx, sig_id) in app.selected_signals.iter().take(signal_count).enumerate() {
        eprintln!("  📡 Signal {}: {}", idx + 1, sig_id);
        
        // Parse signal ID: BUS:CHANNEL:MSG_ID:SIG_NAME or BUS:MSG_ID:SIG_NAME (compat)
        let parts: Vec<&str> = sig_id.split(':').collect();
        if parts.len() < 3 {
            eprintln!("    ❌ Skip: invalid format");
            continue;
        }

        let (bus_type, channel_filter, msg_id_str, sig_name) = if parts.len() >= 4 {
            let bus_type = parts[0];
            let channel_id = parts[1];
            let msg_id = parts[2];
            let sig_name = parts[3..].join(":");
            (bus_type, Some(channel_id.to_string()), msg_id.to_string(), sig_name)
        } else {
            (parts[0], None, parts[1].to_string(), parts[2].to_string())
        };

        let msg_id_res = if msg_id_str.starts_with("0x") {
            u32::from_str_radix(&msg_id_str[2..], 16)
        } else {
            msg_id_str.parse::<u32>()
        };
        
        let msg_id = match msg_id_res {
            Ok(id) => id,
            Err(_) => {
                eprintln!("    ❌ Skip: invalid message ID");
                continue;
            }
        };

        let target_channel = channel_filter.and_then(|s| s.parse::<u16>().ok());
        let mut unit = None;
        let mut points = Vec::new();
        let mut collected = 0;

        // Single pass: collect points with early termination if too many
        for msg in app.messages.iter().take(message_count) {
            // Early termination to prevent memory issues
            if collected >= MAX_RAW_POINTS {
                eprintln!("    ⚠️  Reached max raw points limit (1M), stopping collection");
                break;
            }

            if bus_type != "CAN" && bus_type != "LIN" {
                continue;
            }

            let (m_id, ch, timestamp, data, _flags): (u32, u16, u64, &[u8], u32) = match msg {
                LogObject::CanMessage(m) => (m.id, m.channel, m.header.object_time_stamp, &m.data, m.header.object_flags),
                LogObject::CanMessage2(m) => (m.id, m.channel, m.header.object_time_stamp, &m.data, m.header.object_flags),
                LogObject::CanFdMessage(m) => (m.id, m.channel, m.header.object_time_stamp, &m.data, m.header.object_flags),
                LogObject::CanFdMessage64(m) => (m.id, m.channel as u16, m.header.object_time_stamp, &m.data, m.header.object_flags),
                LogObject::LinMessage(m) => (m.id as u32, m.channel, m.header.object_time_stamp, &m.data, m.header.object_flags),
                LogObject::LinMessage2(m) => (0u32, 0u16, m.header.object_time_stamp, &m.data, m.header.object_flags),
                _ => continue,
            };

            if m_id != msg_id {
                continue;
            }

            // Check channel filter
            if let Some(ch_filter) = target_channel {
                if ch != ch_filter {
                    continue;
                }
            }

            // Convert timestamp to seconds. After MergedView::from_segments,
            // object_time_stamp is always absolute Unix nanoseconds (abs_ns),
            // regardless of the original object_flags. The flags-based
            // conversion (TimeTenMics / TimeOneNans) is done at merge time.
            let time = timestamp as f64 / 1_000_000_000.0;
            if collected == 0 {
                eprintln!("    🕐 Using absolute Unix nanosecond timestamp");
            }

            
            // Apply zoom filter if active
            if let Some(start) = app.plot_zoom_start {
                if time < start { continue; }
            }
            if let Some(end) = app.plot_zoom_end {
                if time > end { continue; }
            }

            // Try to decode the signal
            if bus_type == "CAN" {
                if let Some(dbc) = app.dbc_channels.get(&ch) {
                    if let Some(dbc_msg) = dbc.messages.get(&m_id) {
                        if let Some(sig) = dbc_msg.signals.get(&sig_name) {
                            if unit.is_none() && !sig.unit.is_empty() {
                                unit = Some(display_unit(&sig.unit).to_string());
                            }
                            let val = sig.decode(data);
                            points.push(DataPoint { time, value: val, index: 0 });
                            collected += 1;
                        }
                    }
                }
            } else if bus_type == "LIN" {
                if let Some(ldf) = app.ldf_channels.get(&ch) {
                    // LDF frames are keyed by frame name, not by numeric ID,
                    // so we scan for the frame whose `id` matches m_id.
                    let frame = ldf.frames.values().find(|f| f.id == m_id);
                    if let Some(frame) = frame {
                        // Find signal in frame mappings
                        if let Some(mapping) = frame.signals.iter().find(|s| s.signal_name == sig_name) {
                            if let Some(sig) = ldf.signals.get(&mapping.signal_name) {
                                if unit.is_none() {
                                    unit = Some("".to_string()); // LIN usually doesn't have units in this parser
                                }
                                let val = sig.decode(data, mapping.offset) as f64;
                                points.push(DataPoint { time, value: val, index: 0 });
                                collected += 1;
                            }
                        }
                    }
                }
            }
        }

        eprintln!("    📈 Collected {} raw points", points.len());

        if points.is_empty() {
            eprintln!("    ⚠️  No points found for this signal");
            continue;
        }

        // Show time range
        let min_time = points.iter().map(|p| p.time).fold(f64::INFINITY, f64::min);
        let max_time = points.iter().map(|p| p.time).fold(f64::NEG_INFINITY, f64::max);
        eprintln!("    ⏱️  Time range: {:.3}s to {:.3}s (span: {:.3}s)", min_time, max_time, max_time - min_time);

        // Apply 1ms sampling if data is too dense
        if points.len() > 1 {
            let time_span = points.last().unwrap().time - points[0].time;
            let avg_interval_ms = (time_span * 1000.0) / (points.len() as f64);
            
            if avg_interval_ms < SAMPLING_INTERVAL_MS {
                eprintln!("    🔽 Applying 1ms sampling (avg interval: {:.3}ms)", avg_interval_ms);
                
                // Group points into 1ms buckets and keep one per bucket
                let mut sampled = Vec::new();
                let start_time = points[0].time;
                let mut current_bucket = 0i64;
                let mut bucket_point: Option<DataPoint> = None;
                
                for point in points.iter() {
                    let time_ms = (point.time - start_time) * 1000.0;
                    let bucket = (time_ms / SAMPLING_INTERVAL_MS).floor() as i64;
                    
                    if bucket != current_bucket {
                        // New bucket, save previous bucket's point
                        if let Some(p) = bucket_point {
                            sampled.push(p);
                        }
                        current_bucket = bucket;
                        bucket_point = Some(*point);
                    } else {
                        // Same bucket, keep the point closest to bucket start
                        if let Some(existing) = bucket_point {
                            let existing_offset = (existing.time - start_time) * 1000.0 - (current_bucket as f64 * SAMPLING_INTERVAL_MS);
                            let new_offset = time_ms - (bucket as f64 * SAMPLING_INTERVAL_MS);
                            if new_offset < existing_offset {
                                bucket_point = Some(*point);
                            }
                        }
                    }
                }
                
                // Don't forget the last bucket
                if let Some(p) = bucket_point {
                    sampled.push(p);
                }
                
                eprintln!("    ✅ Sampled to {} points", sampled.len());
                points = sampled;
            } else {
                eprintln!("    ✅ Data already sparse enough (avg interval: {:.3}ms)", avg_interval_ms);
            }
        }

        // Assign indices for label spacing
        for (i, p) in points.iter_mut().enumerate() {
            p.index = i;
        }

        // time_labels field is now unused by render (the X axis draws via
        // format_time_relative_or_absolute directly). Leave it empty to avoid
        // building misleading strings; apply_zoom_to_full_data handles the
        // empty case via unwrap_or_default().
        let time_labels: Vec<String> = Vec::new();
        let _ = app.start_time; // silence unused warning

        eprintln!("    ✅ Final point count: {}", points.len());

        all_series.push(Series {
            name: sig_id.clone(),
            unit,
            points: points.into(),
            color: colors[idx % colors.len()],
            time_labels,
        });
    }

    eprintln!("✅ Extraction complete: {} series generated", all_series.len());
    Arc::from(all_series)
}

/// Fast zoom filter: slice plot_full_data by zoom range without re-decoding messages.
/// This is called every time zoom changes; it's O(points), not O(messages).
pub fn apply_zoom_to_full_data(app: &mut CanViewerApp) {
    if app.plot_full_data.is_empty() {
        // plot_full_data hasn't been populated yet.
        // Do NOT call extract_and_update_series_data here — that would cause
        // mutual recursion (extract_and_update_series_data → apply_zoom_to_full_data).
        // Just keep plot_data empty; the caller is responsible for filling
        // plot_full_data first before calling us.
        app.plot_data = std::sync::Arc::from([]);
        return;
    }

    let zoom_start = app.plot_zoom_start;
    let zoom_end = app.plot_zoom_end;

    if zoom_start.is_none() && zoom_end.is_none() {
        // No zoom: display full data directly
        app.plot_data = app.plot_full_data.clone();
        return;
    }

    // Filter each series by zoom range and reassign point indices
    let filtered: Vec<crate::models::Series> = app.plot_full_data.iter().map(|series| {
        // Collect filtered points while remembering each point's original index
        // (original index → lookup in time_labels; new sequential index → label spacing)
        let orig_indexed: Vec<(usize, crate::models::DataPoint)> = series.points.iter()
            .filter(|p| {
                if let Some(s) = zoom_start { if p.time < s { return false; } }
                if let Some(e) = zoom_end   { if p.time > e { return false; } }
                true
            })
            .map(|p| (p.index, *p))  // save original index before we overwrite it
            .collect();

        // Build time_labels using the ORIGINAL index, then reassign sequential index
        let time_labels: Vec<String> = orig_indexed.iter()
            .map(|(orig_idx, _)| series.time_labels.get(*orig_idx).cloned().unwrap_or_default())
            .collect();

        let mut filtered_pts: Vec<crate::models::DataPoint> = orig_indexed.into_iter()
            .enumerate()
            .map(|(new_i, (_, mut p))| { p.index = new_i; p })
            .collect();

        // Sanity: ensure pts len matches labels len (they always should)
        debug_assert_eq!(filtered_pts.len(), time_labels.len());
        let _ = &mut filtered_pts; // suppress unused-mut warning

        crate::models::Series {
            name: series.name.clone(),
            color: series.color,
            unit: series.unit.clone(),
            points: filtered_pts,
            time_labels,
        }
    }).collect();

    // Guard: if all series have 0 points (zoom window is between points), the
    // chart would be empty. Fall back to the full range so user never sees a
    // blank chart.
    let all_empty = filtered.iter().all(|s| s.points.is_empty());
    if all_empty {
        eprintln!("⚠️  Zoom window contains no points — resetting to full range");
        app.plot_zoom_start = None;
        app.plot_zoom_end = None;
        app.plot_data = app.plot_full_data.clone();
        return;
    }

    app.plot_data = std::sync::Arc::from(filtered);
}

/// Wrapper that extracts series data and updates the full time range in app state.
/// Call this instead of extract_series_data directly.
pub fn extract_and_update_series_data(app: &mut CanViewerApp) {
    // Always decode ALL data (ignoring zoom range) into plot_full_data
    let saved_start = app.plot_zoom_start.take();
    let saved_end = app.plot_zoom_end.take();

    app.plot_full_data = extract_series_data(app);

    app.plot_zoom_start = saved_start;
    app.plot_zoom_end = saved_end;

    // Store the full data time range for zoom-out reference
    {
        let mut min_t = f64::MAX;
        let mut max_t = f64::MIN;
        for series in app.plot_full_data.iter() {
            for p in series.points.iter() {
                if p.time < min_t { min_t = p.time; }
                if p.time > max_t { max_t = p.time; }
            }
        }
        if min_t < max_t {
            app.plot_full_time_min = Some(min_t);
            app.plot_full_time_max = Some(max_t);
            eprintln!("📏 Stored full time range: {:.3}s - {:.3}s", min_t, max_t);
        }
    }

    // Now apply zoom filter to set plot_data
    apply_zoom_to_full_data(app);

    // The library or selection may have changed, so refresh the cursor values.
    update_cursor_values(app);
}

/// Set the cursor from a click at `mouse_x`. Clicks outside the chart drawing
/// area are ignored.
fn set_plot_cursor(app: &mut CanViewerApp, mouse_x: Pixels, draw_left: Pixels, draw_right: Pixels) {
    if mouse_x < draw_left || mouse_x > draw_right || draw_right <= draw_left {
        return;
    }

    let Some((min_t, max_t)) = visible_time_range(app) else {
        return; // Nothing plotted yet
    };

    let rel_x = ((mouse_x - draw_left) / (draw_right - draw_left)).clamp(0.0, 1.0);
    app.plot_cursor_time = Some(min_t + (max_t - min_t) * rel_x as f64);
    update_cursor_values(app);
}

/// The identifying fields of a CAN or LIN frame: (ID, channel, timestamp, data).
fn frame_fields(msg: &LogObject) -> Option<(u32, u16, u64, &[u8])> {
    match msg {
        LogObject::CanMessage(m) => Some((m.id, m.channel, m.header.object_time_stamp, &m.data[..])),
        LogObject::CanMessage2(m) => Some((m.id, m.channel, m.header.object_time_stamp, &m.data[..])),
        LogObject::CanFdMessage(m) => Some((m.id, m.channel, m.header.object_time_stamp, &m.data[..])),
        LogObject::CanFdMessage64(m) => {
            Some((m.id, m.channel as u16, m.header.object_time_stamp, &m.data[..]))
        }
        LogObject::LinMessage(m) => {
            Some((m.id as u32, m.channel, m.header.object_time_stamp, &m.data[..]))
        }
        _ => None,
    }
}

/// The part of a DBC unit to show. Units may be written "physical property:unit",
/// such as "voltage:V"; anything else is shown as it is.
pub fn display_unit(unit: &str) -> &str {
    unit.rsplit(':').next().unwrap_or(unit).trim()
}

/// Format a value for the signal list, matching the precision shown on the charts.
fn format_cursor_value(value: f64, decimals: usize) -> String {
    format!("{:.*}", decimals, value)
}

/// Decode every signal at the cursor, plotted or not, from the latest frame of each
/// message at or before that time. Runs once per click rather than on every frame.
fn update_cursor_values(app: &mut CanViewerApp) {
    let Some(cursor) = app.plot_cursor_time else {
        app.plot_cursor_values.clear();
        return;
    };

    let mut latest: std::collections::HashMap<(u16, u32), (u64, &[u8])> =
        std::collections::HashMap::new();
    for msg in app.messages.iter() {
        let Some((id, channel, timestamp, data)) = frame_fields(msg) else {
            continue;
        };
        // object_time_stamp is absolute Unix nanoseconds, as in extract_series_data.
        if timestamp as f64 / 1_000_000_000.0 > cursor {
            continue;
        }
        let slot = latest.entry((channel, id)).or_insert((timestamp, data));
        if timestamp >= slot.0 {
            *slot = (timestamp, data);
        }
    }

    let mut values = std::collections::HashMap::new();
    for ((channel, id), (_, data)) in &latest {
        if let Some(message) = app.dbc_channels.get(channel).and_then(|db| db.messages.get(id)) {
            for sig in message.signals.values() {
                values.insert(
                    format!("CAN:{}:{}:{}", channel, id, sig.name),
                    format_cursor_value(sig.decode(data), sig.decimal_places()),
                );
            }
        }
        if let Some(ldf) = app.ldf_channels.get(channel) {
            if let Some(frame) = ldf.frames.values().find(|f| f.id == *id) {
                for mapping in &frame.signals {
                    if let Some(sig) = ldf.signals.get(&mapping.signal_name) {
                        values.insert(
                            format!("LIN:{}:{}:{}", channel, id, mapping.signal_name),
                            format_cursor_value(sig.decode(data, mapping.offset) as f64, 0),
                        );
                    }
                }
            }
        }
    }

    app.plot_cursor_values = values;
}

/// 按 80px/刻度估算 X 轴刻度数量，clamp [2, 6]。
/// chart_w_px 是 canvas 内可用于画折线的水平像素（已扣除左右 padding）。
pub fn calc_x_tick_count(chart_w_px: f32) -> usize {
    let approx = (chart_w_px / 80.0).floor() as usize;
    approx.clamp(2, 6)
}

/// 按 50px/刻度估算 Y 轴刻度数量，clamp [3, 8]。
/// chart_h_px 是 canvas 内可用于画折线的垂直像素（已扣除上下 padding）。
/// 至少 3 个 (max / mid / min)，最多 8 个（避免标签过密）。
pub fn calc_y_tick_count(chart_h_px: f32) -> usize {
    let approx = (chart_h_px / 50.0).floor() as usize;
    approx.clamp(3, 8)
}

/// 时间标签 fallback（series.time_labels 为空时用）。
/// span 是 max_t - min_t（秒）。
/// < 60s → 三位小数秒；< 1h → 一位小数秒；否则 → 一位小数分钟。
pub fn format_time_label(t: f64, span: f64) -> String {
    if span < 60.0 {
        format!("{:.3}s", t)
    } else if span < 3600.0 {
        format!("{:.1}s", t)
    } else {
        format!("{:.1}min", t / 60.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that the signal ID format produced by the sidebar matches what
    /// `extract_series_data` will set as `Series.name`. This is a structural
    /// test of the ID format — it doesn't run `extract_series_data` (which
    /// needs a full message log) but locks the format convention.
    #[test]
    fn signal_id_format_matches_series_name_convention() {
        let sidebar_signal_id = "CAN:1:256:EngineSpeed";
        let parts: Vec<&str> = sidebar_signal_id.split(':').collect();
        assert!(parts.len() >= 4, "signal id must have BUS:CH:MSG:NAME structure");
        // The Series.name after this task's change will equal the full signal_id;
        // display names extract the last segment:
        let display_name = sidebar_signal_id.split(':').last().unwrap();
        assert_eq!(display_name, "EngineSpeed");
    }

    #[test]
    fn calc_x_tick_count_basic() {
        assert_eq!(calc_x_tick_count(400.0), 5);
        assert_eq!(calc_x_tick_count(80.0), 2);   // 80px → 1 → clamp 2
        assert_eq!(calc_x_tick_count(40.0), 2);   // 40px → 0 → clamp 2
        assert_eq!(calc_x_tick_count(600.0), 6);  // 600/80=7.5 → 7 → clamp 6
        assert_eq!(calc_x_tick_count(160.0), 2);
        assert_eq!(calc_x_tick_count(320.0), 4);
    }

    #[test]
    fn calc_y_tick_count_basic() {
        assert_eq!(calc_y_tick_count(150.0), 3);  // 150/50=3 → clamp 3
        assert_eq!(calc_y_tick_count(100.0), 3);  // 100/50=2 → clamp 3
        assert_eq!(calc_y_tick_count(50.0), 3);   // 50/50=1 → clamp 3
        assert_eq!(calc_y_tick_count(300.0), 6);  // 300/50=6
        assert_eq!(calc_y_tick_count(500.0), 8);  // 500/50=10 → clamp 8
        assert_eq!(calc_y_tick_count(250.0), 5);
    }

    #[test]
    fn format_time_label_ranges() {
        assert_eq!(format_time_label(12.345, 30.0), "12.345s");
        assert_eq!(format_time_label(5.0, 300.0), "5.0s");
        assert_eq!(format_time_label(120.0, 4000.0), "2.0min");
        // Boundary cases: at exactly 60.0 the first branch (< 60.0) fails → second branch → "60.0s";
        // at exactly 3600.0 the second branch (< 3600.0) fails → else → 3600.0/60.0 = 60.0 → "60.0min".
        assert_eq!(format_time_label(60.0, 60.0), "60.0s");
        assert_eq!(format_time_label(3600.0, 3600.0), "60.0min");
    }
}
