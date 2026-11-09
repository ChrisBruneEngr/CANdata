//! StatusBar component
//!
//! Renders the bottom status bar with file info (left) and library
//! status (right). Single 24px row.

use crate::app::{AppView, CanDataApp};
use crate::ui::theme::colors;
use crate::ui::theme::radius;
use crate::ui::theme::spacing;
use gpui::{prelude::*, *};
use gpui_component::scroll::ScrollableElement;

/// Maximum number of visible rows in the Files popover list. Up to this many rows
/// the list sizes to its content with no scrollbar; beyond it the list has a fixed height and scrolls.
const MAX_VISIBLE_ROWS: usize = 8;

/// Estimated height of one row (p(XS)=4 top and bottom + text_xs ~16 + row gap 4 ≈ 28).
/// Used to compute the fixed height of the scrolling list.
const ROW_HEIGHT: f32 = 28.;

/// Format a count with thousands separators.
fn format_count(n: usize) -> String {
    let s = n.to_string();
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(*b as char);
    }
    out
}

/// Format a byte count with units (1024-based): 0B, 1023B, 1.0KB, 1.5MB, 2.3GB.
pub fn format_bytes(n: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if n >= GB {
        format!("{:.1}GB", n as f64 / GB as f64)
    } else if n >= MB {
        format!("{:.1}MB", n as f64 / MB as f64)
    } else if n >= KB {
        format!("{:.1}KB", n as f64 / KB as f64)
    } else {
        format!("{}B", n)
    }
}

/// Render the left-side file segment.
///
/// Five states (by `app.files.len()` × `app.blf_parse_errors.is_empty()`):
///   - 0 files            → `📂 No file loaded — File > Open BLF...` (display only)
///   - 1 file, no errors  → `📂 <file_name>` (display only)
///   - 1 file, errors     → `📂 <file_name> ⚠️` (clickable, opens the files popover)
///   - ≥2 files, no errors → `📂 N files` (clickable, opens the files popover)
///   - ≥2 files, errors   → `📂 N files ⚠️` (clickable, opens the files popover)
///
/// ⚠️ goes after the label (file name / N files) and is clicked together with it.
/// The files popover already shows each file's ❌/✅ icon and error list, so the ⚠️ here
/// is only an overall "some file failed to parse" hint.
fn render_file_segment(app: &CanDataApp, view: Entity<CanDataApp>) -> impl IntoElement {
    let files_count = app.files.len();
    // Multi-file loads don't fill `blf_parse_errors` (that's for the single-file path only),
    // so also check whether any segment has errors.
    let has_errors =
        !app.blf_parse_errors.is_empty() || app.files.iter().any(|f| !f.errors.is_empty());
    let clickable = files_count >= 1 && (files_count >= 2 || has_errors);

    let label = if files_count == 0 {
        "No file loaded — File > Open BLF...".to_string()
    } else if files_count >= 2 {
        format!("{} files", files_count)
    } else {
        app.current_file_name.clone().unwrap_or_default()
    };
    let label_color = if files_count == 0 {
        colors::TEXT_PLACEHOLDER
    } else {
        colors::TEXT_SECONDARY
    };

    let view_for_click = view.clone();
    div()
        .flex()
        .items_center()
        .gap(px(6.))
        .px(spacing::SM)
        .py(px(1.))
        .rounded(px(3.))
        .when(clickable, |el| {
            el.cursor_pointer()
                .hover(|s| s.bg(colors::SURFACE0))
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    cx.stop_propagation();
                    view_for_click.update(cx, |app, cx| {
                        app.show_files_popover = !app.show_files_popover;
                        cx.notify();
                    });
                })
        })
        .child(div().text_color(colors::TEXT_MUTED).child("📂"))
        .child(div().text_color(label_color).child(label))
        .when(has_errors, |el| {
            el.child(div().text_color(colors::WARNING).child("⚠️"))
        })
}

/// Render a vertical separator (1px wide, 12px tall).
fn render_separator() -> impl IntoElement {
    div().w(px(1.)).h(px(12.)).bg(colors::BORDER_SUBTLE)
}

/// Render the BLF bytes-progress segment: "521.0KB / 521.0KB (100.0%)".
/// Returns None when no file is loaded (blf_bytes_total == 0) so the
/// caller can skip rendering via .when_some().
fn render_blf_progress_segment(app: &CanDataApp) -> Option<impl IntoElement> {
    if app.blf_bytes_total == 0 {
        return None;
    }
    let total = format_bytes(app.blf_bytes_total);
    let consumed = format_bytes(app.blf_bytes_consumed);
    // Clamp at 100% — consumed should never exceed total, but float math
    // can introduce tiny overshoot (e.g. 100.01%). The min() in the
    // parser already prevents this at the source; the cap here is a
    // last-line-of-defense so users never see >100%.
    let pct = ((app.blf_bytes_consumed as f64 / app.blf_bytes_total as f64) * 100.0).min(100.0);
    let text = format!("{} / {} ({:.1}%)", consumed, total, pct);
    // Color: green at 100%, yellow if < 100% (partial parse)
    let color = if pct >= 100.0 {
        colors::TEXT_SECONDARY
    } else {
        colors::WARNING
    };
    Some(div().text_color(color).child(text))
}

/// Render the library badge segment (right side, segment 3).
fn render_lib_badge_segment(app: &CanDataApp) -> impl IntoElement {
    if let (Some(lib_id), Some(ver)) = (&app.active_library_id, &app.active_version_name) {
        let lib_name = app
            .library_manager
            .find_library(lib_id)
            .map(|l| l.name.clone())
            .unwrap_or_else(|| lib_id.clone());
        let text = format!("📚 {} / {}", lib_name, ver);
        div()
            .text_color(colors::ACCENT_GREEN_LIGHT)
            .child(text)
            .into_any_element()
    } else {
        div().into_any_element()
    }
}

/// Render the status message segment (right side, segment 2).
///
/// Returns `Some(element)` when `app.status_msg` is non-empty, otherwise `None`
/// so the caller can skip the surrounding separators via `.when_some()`.
///
/// The parse-error entry point moved to `render_file_segment` on the left (it shows ⚠️ and opens
/// the files popover, which lists each file's errors). This segment no longer adds an error entry,
/// so the function doesn't need the view.
fn render_status_msg_segment(app: &CanDataApp) -> Option<impl IntoElement> {
    if app.status_msg.is_empty() {
        return None;
    }

    Some(
        div()
            .flex()
            .items_center()
            .gap(px(4.))
            .text_color(colors::TEXT_MUTED)
            .text_xs()
            .truncate()
            .child(app.status_msg.clone()),
    )
}

/// Render the BLF errors popover as a standalone element (sibling of the
/// status bar) so it isn't clipped by the status_msg segment's truncate().
/// Returns None when no errors or when the popover is closed.
fn render_blf_errors_popover(app: &CanDataApp, view: Entity<CanDataApp>) -> Option<impl IntoElement> {
    if app.blf_parse_errors.is_empty() || !app.show_blf_errors_popover {
        return None;
    }
    let errors_count = app.blf_parse_errors.len();
    let errors: Vec<String> = app.blf_parse_errors.clone();
    let view_for_close = view.clone();
    let view_for_click_outside = view.clone();

    Some(
        div()
            .absolute()
            .top_0()
            .left_0()
            .w_full()
            .h_full()
            .bg(rgba(0x00000055))
            .flex()
            .items_end()
            .justify_end()
            .p(spacing::LG)
            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                view_for_click_outside.update(cx, |app, cx| {
                    app.show_blf_errors_popover = false;
                    cx.notify();
                });
            })
            .child(
                div()
                    .w(px(460.))
                    .max_h(px(360.))
                    .bg(colors::BG_ELEVATED)
                    .border_1()
                    .border_color(colors::BORDER_DEFAULT)
                    .rounded(px(6.))
                    .shadow_lg()
                    .flex()
                    .flex_col()
                    .p(spacing::SM)
                    .gap(spacing::XS)
                    .on_mouse_down(MouseButton::Left, |_, _, cx| {
                        cx.stop_propagation();
                    })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_color(colors::TEXT_PRIMARY)
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(format!("BLF parse errors ({})", errors_count)),
                            )
                            .child(
                                div()
                                    .cursor_pointer()
                                    .text_color(colors::TEXT_MUTED)
                                    .hover(|s| s.text_color(colors::TEXT_PRIMARY))
                                    .child("✕")
                                    .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                        cx.stop_propagation();
                                        view_for_close.update(cx, |app, cx| {
                                            app.show_blf_errors_popover = false;
                                            cx.notify();
                                        });
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .children(
                                errors
                                    .iter()
                                    .enumerate()
                                    .map(|(i, e)| {
                                        div()
                                            .text_color(colors::TEXT_MUTED)
                                            .text_xs()
                                            .child(format!("{}. {}", i + 1, e))
                                    })
                                    .collect::<Vec<_>>(),
                            ),
                    ),
            ),
    )
}

/// Render the ❌ Cancel button segment (right side, only during loading).
///
/// Returns `Some` only while `loading_progress` shows a load in progress that isn't cancelled. Clicking sets
/// `is_cancelled = true`, so later `apply_blf_result_append_one` calls return early and
/// unfinished files aren't appended. Segments that already finished stay in `files`.
fn render_cancel_button_segment(app: &CanDataApp, view: Entity<CanDataApp>) -> Option<impl IntoElement> {
    let is_loading = app
        .loading_progress
        .as_ref()
        .map(|p| p.completed_files < p.total_files && !p.is_cancelled)
        .unwrap_or(false);
    if !is_loading {
        return None;
    }
    let view_for_click = view.clone();
    Some(
        div()
            .flex()
            .items_center()
            .gap(px(4.))
            .px(spacing::SM)
            .py(px(1.))
            .rounded(px(3.))
            .text_xs()
            .text_color(colors::ERROR)
            .cursor_pointer()
            .hover(|s| s.bg(colors::SURFACE0))
            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                cx.stop_propagation();
                view_for_click.update(cx, |app, cx| {
                    if let Some(p) = &mut app.loading_progress {
                        p.is_cancelled = true;
                    }
                    app.status_msg = "❌ Loading cancelled".into();
                    cx.notify();
                });
            })
            .child("❌ Cancel"),
    )
}

/// Render the 📁 Files (N) button segment (right side).
///
/// **Deprecated**: the multi-file entry point moved to `render_file_segment` on the left. This function
/// is kept but always returns `None`, so the right-side entry can be restored quickly if needed. The caller
/// (`.when_some(render_files_button_segment(...))` in `render_status_bar`)
/// skips the None case automatically.
fn render_files_button_segment(_app: &CanDataApp, _view: Entity<CanDataApp>) -> Option<Div> {
    None
}

/// Render the file-management popover as a sibling of the status bar.
///
/// Positioned like `render_blf_errors_popover`: a full-window absolute layer with a translucent backdrop
/// handles outside clicks; the content is anchored to the bottom-right so the status_msg segment's
/// `truncate()` doesn't clip it. Returns `Some` only when `show_files_popover && !files.is_empty()`;
/// if every file is removed while the popover is open, it closes automatically.
fn render_files_popover(app: &CanDataApp, view: Entity<CanDataApp>) -> Option<impl IntoElement> {
    if !app.show_files_popover || app.files.is_empty() {
        return None;
    }
    let files_count = app.files.len();
    // Estimate the width each row needs and use the largest as the popover width, so it isn't too wide for few files.
    // Estimate: icon (16) + file name (chars * 6.5) + info text (~110) + ✕ button (20) +
    //       gaps/padding (~40). The text-xs font is ≈ 11px, about 6.5px per character.
    // Clamped to [280, 460]: narrower truncates rows, wider wastes space with few files.
    let estimated_max_row_width = app
        .files
        .iter()
        .map(|seg| {
            let name_w = seg.file_name.chars().count() as f32 * 6.5;
            let info_w = format_count(seg.object_count).len() as f32 * 6.5
                + format_bytes(seg.bytes_total).len() as f32 * 6.5
                + 30.0;
            let errors_w = if seg.errors.is_empty() {
                0.0
            } else {
                50.0
            };
            16.0 + name_w + info_w + errors_w + 20.0 + 40.0
        })
        .fold(0f32, f32::max);
    let popover_width = estimated_max_row_width.clamp(280., 460.);
    // Render each row as its own element up front, so GPUI children()'s 'static lifetime
    // requirement doesn't conflict with borrowing the local rows (E0597). Each row's on_mouse_down closure
    // moves in its own file_id + view clone, so nothing borrows rows.
    let row_elements: Vec<Div> = app
        .files
        .iter()
        .map(|seg| {
            let file_id = seg.file_id;
            let file_name = seg.file_name.clone();
            let msg_count = seg.object_count;
            let bytes_total = seg.bytes_total;
            let has_errors = !seg.errors.is_empty();
            // De-duplicate identical errors (the BLF parser may report the same error for each corrupted chunk)
            // Order-preserving de-dup with a Vec, to avoid needing a HashSet
            let mut errors_list: Vec<String> = Vec::new();
            for e in seg.errors.iter() {
                if !errors_list.iter().any(|existing| existing == e) {
                    errors_list.push(e.clone());
                }
            }
            // Truncate each error to 200 characters, with "..." when longer
            let errors_list: Vec<String> = errors_list
                .iter()
                .map(|e| {
                    if e.chars().count() > 200 {
                        let truncated: String = e.chars().take(200).collect();
                        format!("{}...", truncated)
                    } else {
                        e.clone()
                    }
                })
                .collect();
            let status_icon = if has_errors { "❌" } else { "✅" };
            let status_color = if has_errors {
                colors::WARNING
            } else {
                colors::SUCCESS
            };
            let row_bg = if has_errors {
                colors::SURFACE0
            } else {
                colors::BG_DEFAULT
            };
            let view_for_remove = view.clone();
            div()
                .p(spacing::XS)
                .bg(row_bg)
                .rounded(radius::MD)
                .flex()
                .flex_col()
                .gap(px(4.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .text_xs()
                                .truncate()
                                .child(div().text_color(status_color).child(status_icon))
                                .child(div().text_color(colors::TEXT_PRIMARY).child(file_name))
                                .child(
                                    div()
                                        .text_color(colors::TEXT_MUTED)
                                        .child(format!(
                                            "{} msgs, {}",
                                            format_count(msg_count),
                                            format_bytes(bytes_total),
                                        )),
                                )
                                .when(has_errors, |el| {
                                    el.child(div().text_color(colors::WARNING).child("(errors)"))
                                }),
                        )
                        .child(
                            div()
                                .px(spacing::SM)
                                .text_xs()
                                .text_color(colors::ERROR)
                                .cursor_pointer()
                                .hover(|s| s.bg(colors::SURFACE0))
                                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                    cx.stop_propagation();
                                    view_for_remove.update(cx, |app, cx| {
                                        app.remove_file(file_id);
                                        cx.notify();
                                    });
                                })
                                .child("✕"),
                        ),
                )
                .when(has_errors, |el| {
                    el.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .pl(px(18.))
                            .text_xs()
                            .text_color(colors::WARNING)
                            .children(
                                errors_list
                                    .iter()
                                    .map(|e| div().child(format!("• {}", e))),
                            ),
                    )
                })
        })
        .collect();
    let view_for_remove_all = view.clone();
    let view_for_done = view.clone();
    let view_for_title_close = view.clone();
    let view_for_click_outside = view.clone();

    Some(
        div()
            .absolute()
            .top_0()
            .left_0()
            .w_full()
            .h_full()
            .bg(rgba(0x00000055))
            .flex()
            .items_end()
            .justify_end()
            .p(spacing::LG)
            // Clicking outside the popover closes it
            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                view_for_click_outside.update(cx, |app, cx| {
                    app.show_files_popover = false;
                    cx.notify();
                });
            })
            .child(
                div()
                    .w(px(popover_width))
                    .mb(px(28.))
                    .bg(colors::BG_ELEVATED)
                    .border_1()
                    .border_color(colors::BORDER_DEFAULT)
                    .rounded(radius::LG)
                    .shadow_lg()
                    .flex()
                    .flex_col()
                    .p(spacing::SM)
                    .gap(spacing::XS)
                    // Stop clicks inside the popover from reaching the backdrop's close handler
                    .on_mouse_down(MouseButton::Left, |_, _, cx| {
                        cx.stop_propagation();
                    })
                    // Title bar: file count + ✕ close
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_color(colors::TEXT_PRIMARY)
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(format!("Loaded Files ({})", files_count)),
                            )
                            .child(
                                div()
                                    .cursor_pointer()
                                    .text_color(colors::TEXT_MUTED)
                                    .hover(|s| s.text_color(colors::TEXT_PRIMARY))
                                    .child("✕")
                                    .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                        cx.stop_propagation();
                                        view_for_title_close.update(cx, |app, cx| {
                                            app.show_files_popover = false;
                                            cx.notify();
                                        });
                                    }),
                            ),
                    )
                    // File list (each row: status icon + name + msgs + size + ✕ remove).
                    // With few files it sizes to its content (no scrollbar, compact popover);
                    // beyond MAX_VISIBLE_ROWS it gets a fixed height and a scrollbar, so the bottom buttons
                    // stay visible. Key point: the scroll container needs a fixed h. The Scrollable wrapper only
                    // inherits style.size, and flex_1/min_h_0/max_h aren't part of size, so they can't give
                    // the wrapper flex constraints; it would grow to its content, never scroll,
                    // and push the bottom buttons out of view. size_full + pr spaces the scrollbar from the content.
                    .child(
                        if files_count > MAX_VISIBLE_ROWS {
                            div()
                                .h(px(MAX_VISIBLE_ROWS as f32 * ROW_HEIGHT))
                                .child(
                                    div()
                                        .size_full()
                                        .pr(px(4.))
                                        .overflow_y_scrollbar()
                                        .child(
                                            div()
                                                .flex()
                                                .flex_col()
                                                .gap(px(2.))
                                                .children(row_elements),
                                        ),
                                )
                                .into_any_element()
                        } else {
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(2.))
                                .children(row_elements)
                                .into_any_element()
                        },
                    )
                    // Bottom: Remove All (left) + Done (right)
                    .child(
                        div()
                            .mt(px(2.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .px(spacing::MD)
                                    .py(px(1.))
                                    .text_xs()
                                    .text_color(colors::ERROR)
                                    .cursor_pointer()
                                    .rounded(px(3.))
                                    .hover(|s| s.bg(colors::SURFACE0))
                                    .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                        cx.stop_propagation();
                                        view_for_remove_all.update(cx, |app, cx| {
                                            app.remove_all_files();
                                            cx.notify();
                                        });
                                    })
                                    .child("Remove All"),
                            )
                            .child(
                                div()
                                    .px(spacing::MD)
                                    .py(px(1.))
                                    .text_xs()
                                    .text_color(colors::TEXT_PRIMARY)
                                    .cursor_pointer()
                                    .rounded(px(3.))
                                    .hover(|s| s.bg(colors::SURFACE0))
                                    .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                        cx.stop_propagation();
                                        view_for_done.update(cx, |app, cx| {
                                            app.show_files_popover = false;
                                            cx.notify();
                                        });
                                    })
                                    .child("Done"),
                            ),
                    ),
            ),
    )
}

/// Render the current view name segment (right side, segment 4).
fn render_view_name_segment(view_val: AppView) -> impl IntoElement {
    let name = match view_val {
        AppView::LogView => "log mode",
        AppView::PlotView => "plot mode",
        AppView::LibraryView => "library mode",
        AppView::ConfigView => "config mode",
    };
    div().text_color(colors::TEXT_MUTED).child(name.to_string())
}

/// Render the data-view toggle (Log ⇄ Plot) in the center of the status bar.
///
/// Two mutually-exclusive buttons. Clicking one switches the current view to
/// that data view. When the app is in Library/Config view, both buttons are
/// rendered but neither is active — clicking either switches back to data
/// view (this is the "return to msg list" path).
fn render_data_view_toggle(app: &CanDataApp, view: Entity<CanDataApp>) -> impl IntoElement {
    let current = app.current_view;
    let log_active = current == AppView::LogView;
    let plot_active = current == AppView::PlotView;

    let view_for_log = view.clone();
    let view_for_plot = view.clone();

    div()
        .flex()
        .items_center()
        .gap(px(2.))
        .bg(colors::BG_DEFAULT)
        .rounded(px(4.))
        .border_1()
        .border_color(colors::BORDER_SUBTLE)
        .px(px(2.))
        // Log button
        .child(
            div()
                .px(spacing::SM)
                .py(px(1.))
                .rounded(px(3.))
                .cursor_pointer()
                .text_color(if log_active {
                    colors::TEXT_PRIMARY
                } else {
                    colors::TEXT_MUTED
                })
                .when(log_active, |el| {
                    el.bg(colors::PRIMARY).text_color(colors::BG_DEFAULT)
                })
                .hover(|s| {
                    if log_active {
                        s
                    } else {
                        s.bg(colors::SURFACE0).text_color(colors::TEXT_SECONDARY)
                    }
                })
                .child("Log")
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    cx.stop_propagation();
                    view_for_log.update(cx, |app, cx| {
                        app.current_view = AppView::LogView;
                        app.library_picker_dismissed = false;
                        cx.notify();
                    });
                }),
        )
        // Plot button
        .child(
            div()
                .px(spacing::SM)
                .py(px(1.))
                .rounded(px(3.))
                .cursor_pointer()
                .text_color(if plot_active {
                    colors::TEXT_PRIMARY
                } else {
                    colors::TEXT_MUTED
                })
                .when(plot_active, |el| {
                    el.bg(colors::PRIMARY).text_color(colors::BG_DEFAULT)
                })
                .hover(|s| {
                    if plot_active {
                        s
                    } else {
                        s.bg(colors::SURFACE0).text_color(colors::TEXT_SECONDARY)
                    }
                })
                .child("Plot")
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    cx.stop_propagation();
                    view_for_plot.update(cx, |app, cx| {
                        app.current_view = AppView::PlotView;
                        app.library_picker_dismissed = false;
                        crate::ui::views::chart_view::extract_and_update_series_data(app);
                        cx.notify();
                    });
                }),
        )
}

/// Render the StatusBar.
pub fn render_status_bar(app: &CanDataApp, view: Entity<CanDataApp>) -> impl IntoElement {
    let current_view = app.current_view;

    div()
        .relative()
        .h(px(24.))
        .bg(colors::BG_MUTED)
        .border_t_1()
        .border_color(colors::BORDER_SUBTLE)
        .flex()
        .items_center()
        .justify_between()
        .px(spacing::MD)
        .text_xs()
        // Left side: Log/Plot toggle | file | BLF progress | msgs | DBC | LDF
        .child(
            div()
                .flex()
                .items_center()
                .gap(spacing::SM)
                .child(render_data_view_toggle(app, view.clone()))
                .child(render_separator())
                .child(render_file_segment(app, view.clone()))
                .when_some(render_blf_progress_segment(app), |el, seg| {
                    el.child(render_separator()).child(seg)
                })
                .child(render_separator())
                .child(
                    div()
                        .text_color(colors::TEXT_MUTED)
                        .child(format!("{} msgs", format_count(app.messages.len()))),
                )
                .child(render_separator())
                .child(
                    div()
                        .text_color(colors::TEXT_MUTED)
                        .child(format!("DBC: {}", app.dbc_channels.len())),
                )
                .child(render_separator())
                .child(
                    div()
                        .text_color(colors::TEXT_MUTED)
                        .child(format!("LDF: {}", app.ldf_channels.len())),
                ),
        )
        // Right side: status_msg | lib badge | files btn | view name (separated by vertical bars)
        .child(
            div()
                .flex()
                .items_center()
                .gap(spacing::SM)
                .when_some(render_status_msg_segment(app), |el, status_seg| {
                    el.child(status_seg).child(render_separator())
                })
                .child(render_lib_badge_segment(app))
                .when_some(render_cancel_button_segment(app, view.clone()), |el, cancel_btn| {
                    el.child(render_separator()).child(cancel_btn)
                })
                .when_some(render_files_button_segment(app, view.clone()), |el, files_btn| {
                    el.child(render_separator()).child(files_btn)
                })
                .child(render_separator())
                .child(render_view_name_segment(current_view)),
        )
        // Popovers (blf_errors + files) are rendered as siblings of the status
        // bar at the outer layout level, NOT inside this div — otherwise the
        // status bar's border_t_1 stacking context clips the popover's top
        // edge. See impls_rendering.rs where render_status_bar_popovers is
        // called as a sibling of render_status_bar.
}

/// Render the popovers that belong to the status bar (blf_errors + files)
/// as a sibling of the status bar, not as a child. This avoids the status
/// bar's border_t_1 stacking context clipping the popover's top edge.
pub fn render_status_bar_popovers(app: &CanDataApp, view: Entity<CanDataApp>) -> impl IntoElement {
    div()
        .when_some(render_blf_errors_popover(app, view.clone()), |el, popover| {
            el.child(popover)
        })
        .when_some(render_files_popover(app, view.clone()), |el, popover| {
            el.child(popover)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::prelude::v1::test; // `use gpui::*` brings in gpui's own `test` macro

    #[test]
    fn test_format_count_zero() {
        assert_eq!(format_count(0), "0");
    }

    #[test]
    fn test_format_count_small() {
        assert_eq!(format_count(123), "123");
    }

    #[test]
    fn test_format_count_thousands() {
        assert_eq!(format_count(12345), "12,345");
    }

    #[test]
    fn test_format_count_millions() {
        assert_eq!(format_count(1234567), "1,234,567");
    }

    #[test]
    fn test_format_count_exact_thousand() {
        assert_eq!(format_count(1000), "1,000");
    }

    #[test]
    fn test_format_bytes_zero() {
        assert_eq!(format_bytes(0), "0B");
    }

    #[test]
    fn test_format_bytes_small() {
        assert_eq!(format_bytes(1023), "1023B");
    }

    #[test]
    fn test_format_bytes_exact_kb() {
        assert_eq!(format_bytes(1024), "1.0KB");
    }

    #[test]
    fn test_format_bytes_kb_decimal() {
        assert_eq!(format_bytes(1536), "1.5KB");
    }

    #[test]
    fn test_format_bytes_exact_mb() {
        assert_eq!(format_bytes(1024 * 1024), "1.0MB");
    }

    #[test]
    fn test_format_bytes_mb_decimal() {
        assert_eq!(format_bytes(2_400_819), "2.3MB");
    }

    #[test]
    fn test_format_bytes_exact_gb() {
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0GB");
    }

    #[test]
    fn test_format_bytes_gb_decimal() {
        assert_eq!(format_bytes(2_700_000_000), "2.5GB");
    }
}
