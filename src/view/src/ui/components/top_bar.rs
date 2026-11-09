//! TopBar component
//!
//! Renders the top bar: File menu button (left) + Library button (right) +
//! window controls (Win/Linux only — macOS uses system traffic lights).
//! Data view tabs (Log/Plot) and the active-library badge live in the
//! StatusBar, not here.

use crate::app::{AppView, CanDataApp};
use crate::models::SignalListMode;
use crate::ui::theme::colors;
use crate::ui::theme::spacing;
use gpui::{prelude::*, *};

/// Render the top bar.
pub fn render_top_bar(
    app: &CanDataApp,
    view: Entity<CanDataApp>,
    _cx: &mut Context<CanDataApp>,
) -> impl IntoElement {
    let show_file_menu = app.show_file_menu;
    let in_library_view = app.current_view == AppView::LibraryView
        || app.current_view == AppView::ConfigView;

    // File menu button (left side). When the dropdown is open, use a deeper
    // background fill to indicate "pressed" state — visually distinct from
    // Library's bottom-indicator active state, so they can't be confused.
    let view_for_file = view.clone();
    let file_button = div()
        .px(spacing::SM)
        .h_full()
        .flex()
        .items_center()
        .cursor_pointer()
        .text_sm()
        .text_color(if show_file_menu {
            colors::TEXT_PRIMARY
        } else {
            colors::TEXT_MUTED
        })
        .when(show_file_menu, |el| el.bg(colors::SURFACE1))
        .hover(|s| {
            if show_file_menu {
                s
            } else {
                s.text_color(colors::TEXT_SECONDARY).bg(colors::SURFACE0)
            }
        })
        .child("File")
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            cx.stop_propagation();
            view_for_file.update(cx, |app, cx| {
                app.show_file_menu = !app.show_file_menu;
                cx.notify();
            });
        });

    // Active library badge moved to the status bar (bottom) to avoid
    // showing two library references simultaneously. See render_lib_badge_segment.

    // Library button (left, next to File). Acts as the configuration entry
    // point. Active state uses a bottom 2px indicator (not background fill)
    // so it can never visually clash with the File menu's open state.
    let view_for_library = view.clone();
    let library_button = div()
        .px(spacing::SM)
        .h_full()
        .flex()
        .items_center()
        .cursor_pointer()
        .text_sm()
        .text_color(if in_library_view {
            colors::TEXT_PRIMARY
        } else {
            colors::TEXT_MUTED
        })
        .hover(|s| {
            if in_library_view {
                s
            } else {
                s.text_color(colors::TEXT_SECONDARY)
            }
        })
        .when(in_library_view, |el| {
            el.border_b_2().border_color(colors::PRIMARY)
        })
        .child("Library")
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            cx.stop_propagation();
            view_for_library.update(cx, |app, cx| {
                app.current_view = AppView::LibraryView;
                app.library_picker_dismissed = false;
                cx.notify();
            });
        });

    // Help button (right side). Toggles the help dropdown (About / GitHub).
    // Styling mirrors the File menu button so the top bar reads consistently.
    let show_help_menu = app.show_help_menu;
    let view_for_help = view.clone();
    let help_button = div()
        .px(spacing::SM)
        .h_full()
        .flex()
        .items_center()
        .cursor_pointer()
        .text_sm()
        .text_color(if show_help_menu {
            colors::TEXT_PRIMARY
        } else {
            colors::TEXT_MUTED
        })
        .when(show_help_menu, |el| el.bg(colors::SURFACE1))
        .hover(|s| {
            if show_help_menu {
                s
            } else {
                s.text_color(colors::TEXT_SECONDARY).bg(colors::SURFACE0)
            }
        })
        .child("Help")
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            cx.stop_propagation();
            view_for_help.update(cx, |app, cx| {
                app.show_help_menu = !app.show_help_menu;
                app.show_settings_menu = false;
                cx.notify();
            });
        });

    // Settings button, styled to match Help.
    let show_settings_menu = app.show_settings_menu;
    let view_for_settings = view.clone();
    let settings_button = div()
        .px(spacing::SM)
        .h_full()
        .flex()
        .items_center()
        .cursor_pointer()
        .text_sm()
        .text_color(if show_settings_menu {
            colors::TEXT_PRIMARY
        } else {
            colors::TEXT_MUTED
        })
        .when(show_settings_menu, |el| el.bg(colors::SURFACE1))
        .hover(|s| {
            if show_settings_menu {
                s
            } else {
                s.text_color(colors::TEXT_SECONDARY).bg(colors::SURFACE0)
            }
        })
        .child("Settings")
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            cx.stop_propagation();
            view_for_settings.update(cx, |app, cx| {
                app.show_settings_menu = !app.show_settings_menu;
                app.show_help_menu = false;
                cx.notify();
            });
        });

    div()
        .h(px(36.))
        .bg(colors::BG_MUTED)
        .flex()
        .items_center()
        .px(spacing::LG)
        .border_b_1()
        .border_color(colors::BORDER_SUBTLE)
        .window_control_area(WindowControlArea::Drag)
        // Left: File menu + Library button (config entry)
        .child(file_button)
        .child(library_button)
        // Center: spacer that fills remaining width so the right side sticks to the edge
        .child(div().flex_1())
        // Right: Settings + Help buttons + window controls
        .child(settings_button)
        .child(help_button)
        .child(render_window_controls(view))
}

/// Render the Settings dropdown over a full-window layer that closes it on an outside click.
pub fn render_settings_menu(app: &CanDataApp, view: Entity<CanDataApp>) -> impl IntoElement {
    let current = app.app_config.signal_list_mode;
    let view_for_close = view.clone();
    // Sit under the Settings button, clear of the window controls.
    let right_offset = px(124.);

    div()
        .absolute()
        .top_0()
        .left_0()
        .w_full()
        .h_full()
        .bg(rgba(0x00000033))
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            view_for_close.update(cx, |app, cx| {
                app.show_settings_menu = false;
                cx.notify();
            });
        })
        .child(
            div()
                .absolute()
                .top(px(36.))
                .right(right_offset)
                .w(px(240.))
                .bg(rgb(0x313244))
                .border_1()
                .border_color(rgb(0x45475a))
                .rounded(px(6.))
                .shadow_lg()
                .flex()
                .flex_col()
                .py_1()
                .on_mouse_down(MouseButton::Left, |_, _, cx| {
                    cx.stop_propagation();
                })
                .child(
                    div()
                        .px_3()
                        .py_1()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(0xa6adc8))
                        .child("Plot signal list"),
                )
                .child(render_signal_list_option(
                    "Grouped by message",
                    SignalListMode::Grouped,
                    current,
                    view.clone(),
                ))
                .child(render_signal_list_option(
                    "Alphabetical signals only",
                    SignalListMode::Alphabetical,
                    current,
                    view,
                )),
        )
}

/// One selectable row in the Settings dropdown; the current choice shows a check mark.
fn render_signal_list_option(
    label: &'static str,
    mode: SignalListMode,
    current: SignalListMode,
    view: Entity<CanDataApp>,
) -> impl IntoElement {
    div()
        .px_3()
        .py_1()
        .flex()
        .items_center()
        .gap_2()
        .text_xs()
        .text_color(rgb(0xcdd6f4))
        .hover(|style| style.bg(rgb(0x45475a)))
        .cursor_pointer()
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            cx.stop_propagation();
            view.update(cx, |app, cx| {
                app.app_config.signal_list_mode = mode;
                app.show_settings_menu = false;
                app.save_config(cx);
                cx.notify();
            });
        })
        .child(div().w(px(12.)).child(if mode == current { "✓" } else { "" }))
        .child(label)
}

/// Render the Win/Linux window controls (minimize, maximize, close).
fn render_window_controls(view: Entity<CanDataApp>) -> impl IntoElement {
    let view_min = view.clone();
    let view_max = view.clone();
    let _view_close = view.clone();

    div()
        .flex()
        .items_center()
        .h_full()
        .child(
            div()
                .w(px(36.))
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(|s| s.bg(colors::SURFACE1))
                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    cx.stop_propagation();
                    window.minimize_window();
                    view_min.update(cx, |_, cx| cx.notify());
                })
                .child(div().w(px(10.)).h(px(1.)).bg(colors::TEXT_MUTED)),
        )
        .child(
            div()
                .w(px(36.))
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(|s| s.bg(colors::SURFACE1))
                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    cx.stop_propagation();
                    view_max.update(cx, |app, cx| {
                        app.toggle_maximize(window, cx);
                        cx.notify();
                    });
                })
                .child(
                    div()
                        .w(px(10.))
                        .h(px(10.))
                        .border_1()
                        .border_color(colors::TEXT_MUTED),
                ),
        )
        .child(
            div()
                .w(px(36.))
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(|s| s.bg(colors::CLOSE_HOVER))
                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    cx.stop_propagation();
                    window.remove_window();
                })
                .child(
                    div()
                        .text_sm()
                        .text_color(colors::TEXT_MUTED)
                        .hover(|s| s.text_color(colors::TEXT_PRIMARY))
                        .child("✕"),
                ),
        )
}

#[cfg(test)]
mod tests {
    #[test]
    fn placeholder() {}
}
