//! The plot window bar: a button per named plot window, a right-click menu to
//! rename, add, duplicate or close windows, and the box for typing a name.

use crate::app::{CanDataApp, WindowNamePrompt};
use gpui::prelude::*;
use gpui::*;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::menu::{ContextMenuExt, PopupMenuItem};
use gpui_component::Sizable;

const MOUSE_HELP: &str = "Click = set cursor · ←/→ = move cursor (Shift = next sample) · \
drag or wheel = zoom · right-drag = pan · Ctrl + wheel = scroll lanes";

/// The window buttons, or the name box while one is being named.
pub fn render_window_bar(app: &CanDataApp, cx: &mut Context<CanDataApp>) -> AnyElement {
    if let (Some(prompt), Some(input)) = (app.window_name_prompt, app.window_name_input.clone()) {
        return render_name_box(app, prompt, &input, cx);
    }

    let view = cx.entity();
    let can_close = app.plot_windows.len() > 1;
    let active = app.plot_windows.active();

    div()
        .flex()
        .flex_1()
        .min_w_0()
        .items_center()
        .gap_1()
        .overflow_hidden()
        .children(app.plot_windows.names().iter().enumerate().map(|(index, name)| {
            let is_active = index == active;
            let view = view.clone();
            div()
                .id(("plot-window", index))
                .flex_none()
                .px_2()
                .py(px(2.0))
                .rounded(px(4.0))
                .cursor_pointer()
                .text_xs()
                .bg(if is_active { rgb(0x2563eb) } else { rgb(0x27272a) })
                .text_color(if is_active { rgb(0xffffff) } else { rgb(0xa1a1aa) })
                .hover(move |s| if is_active { s } else { s.bg(rgb(0x3f3f46)) })
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _, _, cx| {
                        switch_window(this, index);
                        cx.notify();
                    }),
                )
                .child(name.clone())
                .context_menu(move |menu, _window, _cx| {
                    let rename = view.clone();
                    let add = view.clone();
                    let duplicate = view.clone();
                    let close = view.clone();
                    menu.item(PopupMenuItem::new("Rename…").on_click(move |_, window, cx| {
                        rename.update(cx, |this, cx| {
                            open_name_box(this, WindowNamePrompt::Rename(index), window, cx)
                        });
                    }))
                    .item(PopupMenuItem::new("New window").on_click(move |_, _, cx| {
                        add.update(cx, |this, cx| {
                            this.plot_windows.add_window(&mut this.plot_lanes);
                            cx.notify();
                        });
                    }))
                    .item(PopupMenuItem::new("Duplicate…").on_click(move |_, window, cx| {
                        duplicate.update(cx, |this, cx| {
                            switch_window(this, index);
                            open_name_box(this, WindowNamePrompt::Duplicate, window, cx)
                        });
                    }))
                    .separator()
                    .item(
                        PopupMenuItem::new("Close")
                            .disabled(!can_close)
                            .on_click(move |_, _, cx| {
                                close.update(cx, |this, cx| {
                                    close_window(this, index);
                                    cx.notify();
                                });
                            }),
                    )
                })
        }))
        .child(
            div()
                .id("plot-mouse-help")
                .flex_none()
                .px_1p5()
                .rounded(px(8.0))
                .border_1()
                .border_color(rgb(0x3f3f46))
                .text_xs()
                .text_color(rgb(0x71717a))
                .child("?")
                .tooltip(|window, cx| gpui_component::tooltip::Tooltip::new(MOUSE_HELP).build(window, cx)),
        )
        .into_any_element()
}

/// "Rename window:" or "Duplicate as:", the name box, and Cancel.
fn render_name_box(
    app: &CanDataApp,
    prompt: WindowNamePrompt,
    input: &Entity<InputState>,
    cx: &mut Context<CanDataApp>,
) -> AnyElement {
    let label = match prompt {
        WindowNamePrompt::Rename(index) => format!(
            "Rename \"{}\" to:",
            app.plot_windows.names().get(index).map(String::as_str).unwrap_or("")
        ),
        WindowNamePrompt::Duplicate => "Name of the copy:".to_string(),
    };
    div()
        .flex()
        .flex_1()
        .min_w_0()
        .items_center()
        .gap_2()
        .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
            if event.keystroke.key == "escape" {
                this.window_name_prompt = None;
                cx.notify();
            }
        }))
        .child(div().flex_none().text_xs().text_color(rgb(0xa1a1aa)).child(label))
        .child(div().w(px(200.0)).child(Input::new(input).small().appearance(true)))
        .child(
            div()
                .flex_none()
                .px_2()
                .py(px(2.0))
                .rounded(px(4.0))
                .cursor_pointer()
                .bg(rgb(0x3f3f46))
                .hover(|s| s.bg(rgb(0x52525b)))
                .text_xs()
                .text_color(rgb(0xffffff))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.window_name_prompt = None;
                        cx.notify();
                    }),
                )
                .child("Cancel"),
        )
        .into_any_element()
}

/// Put another window on screen.
fn switch_window(app: &mut CanDataApp, index: usize) {
    app.plot_lane_resize = None;
    app.plot_windows.switch_to(index, &mut app.plot_lanes);
}

/// Close a window, and stop decoding signals no other window plots.
fn close_window(app: &mut CanDataApp, index: usize) {
    app.plot_lane_resize = None;
    if app.plot_windows.close(index, &mut app.plot_lanes) {
        let before = app.selected_signals.len();
        let (windows, lanes) = (&app.plot_windows, &app.plot_lanes);
        app.selected_signals.retain(|signal| windows.contains(lanes, signal));
        if app.selected_signals.len() != before {
            crate::ui::views::chart_view::extract_and_update_series_data(app);
        }
    }
}

/// Show the name box, filled with a starting name and focused.
fn open_name_box(
    app: &mut CanDataApp,
    prompt: WindowNamePrompt,
    window: &mut Window,
    cx: &mut Context<CanDataApp>,
) {
    let start = match prompt {
        WindowNamePrompt::Rename(index) => app.plot_windows.names().get(index).cloned().unwrap_or_default(),
        WindowNamePrompt::Duplicate => String::new(),
    };
    let input = app
        .window_name_input
        .get_or_insert_with(|| {
            let input = cx.new(|cx| InputState::new(window, cx).placeholder("Window name"));
            cx.subscribe(&input, |this, input, event, cx| {
                if let InputEvent::PressEnter { .. } = event {
                    let name = input.read(cx).value().to_string();
                    commit_name(this, &name);
                    cx.notify();
                }
            })
            .detach();
            input
        })
        .clone();
    input.update(cx, |state, cx| {
        state.set_value(start, window, cx);
        state.focus(window, cx);
    });
    app.window_name_prompt = Some(prompt);
    cx.notify();
}

/// Rename or duplicate with the typed name. A bad name keeps the box open
/// and says why.
fn commit_name(app: &mut CanDataApp, name: &str) {
    let Some(prompt) = app.window_name_prompt else { return };
    let result = match prompt {
        WindowNamePrompt::Rename(index) => app.plot_windows.rename(index, name),
        WindowNamePrompt::Duplicate => app.plot_windows.duplicate_active(name, &mut app.plot_lanes),
    };
    match result {
        Ok(()) => app.window_name_prompt = None,
        Err(msg) => app.status_msg = msg.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::prelude::v1::test; // `use gpui::*` brings in gpui's own `test` macro

    #[test]
    fn closing_a_window_stops_decoding_signals_only_it_plotted() {
        let mut app = CanDataApp::new_state();
        app.plot_lanes.add_signal(0, "CAN:1:256:A");
        app.plot_windows.duplicate_active("Copy", &mut app.plot_lanes).unwrap();
        app.plot_lanes.add_signal(0, "CAN:1:256:B");
        app.selected_signals = vec!["CAN:1:256:A".into(), "CAN:1:256:B".into()];

        close_window(&mut app, 1);
        assert_eq!(app.plot_windows.names(), ["Unnamed"]);
        assert_eq!(app.selected_signals, ["CAN:1:256:A"]);
    }

    #[test]
    fn a_taken_name_keeps_the_box_open() {
        let mut app = CanDataApp::new_state();
        app.window_name_prompt = Some(WindowNamePrompt::Duplicate);
        commit_name(&mut app, "unnamed");
        assert_eq!(app.window_name_prompt, Some(WindowNamePrompt::Duplicate));
        commit_name(&mut app, "Temps");
        assert_eq!(app.window_name_prompt, None);
        assert_eq!(app.plot_windows.names(), ["Unnamed", "Temps"]);
    }
}
