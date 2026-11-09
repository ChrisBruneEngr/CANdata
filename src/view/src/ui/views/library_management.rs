//! Three-column library management layout
//!
//! Three-column layout:
//! - Left column: library list
//! - Middle column: version list
//! - Right column: channel configuration

use crate::models::{ChannelDatabase, ChannelMapping, LibraryVersion, SignalLibrary};
use gpui::prelude::*;
use gpui::*;
use gpui_component::input::{Input, InputState};
/// Render the three-column library management view (Zed IDE style).
///
/// A strict three-column layout separated by divider lines:
/// - Left column: library list (fixed width)
/// - Middle column: version list (fixed width)
/// - Right column: channel configuration (fills the remaining width)
pub fn render_library_management_view(
    libraries: &[SignalLibrary],
    selected_library_id: &Option<String>,
    selected_version_id: &Option<String>, // Add selected version ID parameter
    mappings: &[ChannelMapping],
    show_new_library_input: bool,
    show_add_version_input: bool,
    new_library_name: &str,
    new_version_name: &str,
    focused_input: &Option<String>,
    library_cursor_pos: usize,
    version_cursor_pos: usize,
    library_name_input: Option<&gpui::Entity<gpui_component::input::InputState>>,
    version_name_input: Option<&gpui::Entity<gpui_component::input::InputState>>,
    show_add_channel_input: bool,
    channel_id_input: Option<&gpui::Entity<gpui_component::input::InputState>>,
    channel_name_input: Option<&gpui::Entity<gpui_component::input::InputState>>,
    channel_db_path_input: Option<&gpui::Entity<gpui_component::input::InputState>>,
    new_channel_db_path: &str, // Add this parameter to avoid reading entity in render
    new_channel_type: crate::models::ChannelType, // Add channel type parameter
    editing_channel_index: Option<usize>, // Row being edited, if any
    active_library_id: Option<&str>,
    active_version_name: Option<&str>,
    renaming_library_id: Option<&str>,
    rename_library_input: Option<&gpui::Entity<gpui_component::input::InputState>>,
    renaming_version_name: Option<&str>,
    rename_version_input: Option<&gpui::Entity<gpui_component::input::InputState>>,
    cx: &mut Context<crate::CanDataApp>,
) -> impl IntoElement {
    div()
        .flex_1()
        .flex()
        .flex_row()
        .bg(rgb(0x0a0a0a))
        .overflow_hidden() // keep the main container from scrolling
        // Left column: library list
        .child(render_left_column(
            libraries,
            selected_library_id,
            mappings,
            show_new_library_input,
            new_library_name,
            focused_input,
            library_cursor_pos,
            library_name_input,
            renaming_library_id,
            rename_library_input,
            active_library_id,
            active_version_name,
            cx,
        ))
        // Vertical divider 1 - Zed IDE subtle divider
        .child(
            div()
                .w(px(1.0))
                .h_full()
                .bg(rgb(0x252525)) // Zed-style subtle divider
                .flex_shrink_0(),
        )
        // Middle column: version list
        .child(render_middle_column(
            libraries,
            selected_library_id,
            selected_version_id,
            mappings,
            show_add_version_input,
            new_version_name,
            focused_input,
            version_cursor_pos,
            version_name_input,
            renaming_version_name,
            rename_version_input,
            active_library_id,
            active_version_name,
            cx,
        ))
        // Vertical divider 2 - Zed IDE subtle divider
        .child(
            div()
                .w(px(1.0))
                .h_full()
                .bg(rgb(0x252525)) // Zed-style subtle divider
                .flex_shrink_0(),
        )
        // Right column: channel configuration
        .child(render_right_column(
            libraries,
            selected_library_id,
            selected_version_id,
            mappings,
            show_add_channel_input,
            channel_id_input,
            channel_name_input,
            channel_db_path_input,
            new_channel_db_path,
            new_channel_type,
            editing_channel_index,
            cx,
        ))
}

/// Left column: library list (Zed IDE style)
fn render_left_column(
    libraries: &[SignalLibrary],
    selected_library_id: &Option<String>,
    mappings: &[ChannelMapping],
    show_new_library_input: bool,
    new_library_name: &str,
    _focused_input: &Option<String>,
    _cursor_pos: usize,
    library_name_input: Option<&gpui::Entity<gpui_component::input::InputState>>,
    renaming_library_id: Option<&str>,
    rename_library_input: Option<&gpui::Entity<gpui_component::input::InputState>>,
    active_library_id: Option<&str>,
    active_version_name: Option<&str>,
    cx: &mut Context<crate::CanDataApp>,
) -> impl IntoElement {
    div()
        .w(px(280.0)) // slightly narrower, more compact
        .h_full()
        .flex()
        .flex_col()
        .overflow_hidden()
        .child(
            // Fixed top area: title
            div()
                .flex()
                .items_center()
                .justify_between()
                .px_3()
                .py_2()
                .border_b_1()
                .border_color(rgb(0x252525))
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgb(0x6c7086)) // Zed muted
                        .child("LIBRARIES"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x6c7086)) // Zed muted
                        .child(format!("{}", libraries.len())),
                ),
        )
        .child(
            // Scrollable library list
            div()
                .flex_1()
                .overflow_hidden()
                .px_2()
                .py_2()
                .when(libraries.is_empty() && !show_new_library_input, |this| {
                    this.child(
                        div().px_3().py_8().items_center().justify_center().child(
                            div()
                                .flex()
                                .flex_col()
                                .items_center()
                                .gap_3()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(0x646473))
                                        .child("No libraries yet"),
                                )
                                .child(render_add_library_button(cx)),
                        ),
                    )
                })
                .when(!libraries.is_empty() || show_new_library_input, |this| {
                    let mut list = this;
                    // While adding, show the input row first
                    if show_new_library_input {
                        list = list.child(render_add_library_input_row(
                            library_name_input,
                            new_library_name,
                            cx,
                        ));
                    }
                    // Then all library items
                    for library in libraries {
                        list = list.child(render_library_item(
                            library,
                            selected_library_id,
                            mappings,
                            renaming_library_id,
                            rename_library_input,
                            active_library_id,
                            active_version_name,
                            cx,
                        ));
                    }
                    // Finally the add button
                    list.child(render_add_library_button(cx))
                }),
        )
        // Bottom action bar: Export & Import library files
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .px_3()
                .py_2()
                .border_t_1()
                .border_color(rgb(0x252525))
                .child({
                    let can_export = selected_library_id.is_some();
                    div()
                        .flex_1()
                        .px_2()
                        .py_1()
                        .text_xs()
                        .text_color(if can_export { rgb(0xcdd6f4) } else { rgb(0x6c7086) })
                        .bg(rgb(0x1e1e2e))
                        .rounded(px(4.0))
                        .cursor_pointer()
                        .text_center()
                        .hover(|s| s.bg(rgb(0x313244)))
                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                            this.export_selected_library(cx);
                        }))
                        .child("📤 Export")
                })
                .child(
                    div()
                        .flex_1()
                        .px_2()
                        .py_1()
                        .text_xs()
                        .text_color(rgb(0xcdd6f4))
                        .bg(rgb(0x1e1e2e))
                        .rounded(px(4.0))
                        .cursor_pointer()
                        .text_center()
                        .hover(|s| s.bg(rgb(0x313244)))
                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                            this.import_library_file(cx);
                        }))
                        .child("📥 Import"),
                ),
        )
}

/// Render the inline "add library" input row, blended into the list.
fn render_add_library_input_row(
    library_name_input: Option<&gpui::Entity<gpui_component::input::InputState>>,
    _new_library_name: &str,
    cx: &mut Context<crate::CanDataApp>,
) -> impl IntoElement {
    div()
        .px_3()
        .py_1p5()
        .h(px(32.))
        .border_1()
        .border_color(rgb(0x252525))
        .flex()
        .items_center()
        .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
            if event.keystroke.key == "escape" {
                // Close the input without saving
                this.show_library_dialog = false;
                this.library_name_input = None;
                cx.notify();
            }
        }))
        .child(if let Some(input) = library_name_input {
            div()
                .flex_1()
                .child(Input::new(input).appearance(true))
                .into_any_element()
        } else {
            div()
                .text_color(gpui::rgb(0xffffff))
                .text_sm()
                .child("Library name...")
                .into_any_element()
        })
}

/// Render one library item (Zed IDE style).
fn render_library_item(
    library: &SignalLibrary,
    selected_library_id: &Option<String>,
    mappings: &[ChannelMapping],
    renaming_library_id: Option<&str>,
    rename_library_input: Option<&gpui::Entity<gpui_component::input::InputState>>,
    active_library_id: Option<&str>,
    active_version_name: Option<&str>,
    cx: &mut Context<crate::CanDataApp>,
) -> impl IntoElement {
    let is_selected = selected_library_id.as_ref() == Some(&library.id);
    let is_used = library.is_used(mappings);
    let db_type = library.database_type();
    let icon = db_type.icon();
    let library_id = library.id.clone();
    let is_renaming = renaming_library_id == Some(library.id.as_str());
    let is_active = active_library_id == Some(library.id.as_str());
    let active_ver = if is_active { active_version_name } else { None };

    div()
        .id(format!("lib-{}", library_id))
        .px_3()
        .py_1p5()
        .h(px(32.))
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0x1a1a1a))) // subtler hover
        .when(is_selected, |el| {
            el.bg(rgb(0x252525)) // darker background when selected
                .border_l_1()
                .border_color(rgb(0x89b4fa)) // Zed blue accent
        })
        .flex()
        .items_center()
        .justify_between()
        .when(!is_renaming, |el| {
            el.on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener({
                    let library_id = library_id.clone();
                    move |this, _event, _window, cx| {
                        cx.stop_propagation();
                        eprintln!("🖱️ Selected library: {}", library_id);
                        this.selected_library_id = Some(library_id.clone());
                        // Reset selected version when library changes
                        this.selected_version_id = None;
                        // Cancel any in-progress version rename when switching libraries
                        this.renaming_version_name = None;
                        this.rename_version_input = None;
                        // Reset add channel input when switching libraries
                        this.hide_add_channel_input(cx);
                        cx.notify();
                    }
                }),
            )
        })
        .when(is_renaming, |el| {
            // Inline rename row: input + confirm + cancel
            el.child(
                div()
                    .flex()
                    .flex_1()
                    .items_center()
                    .gap_1()
                    .child(if let Some(input) = rename_library_input {
                        div()
                            .flex_1()
                            .child(Input::new(input).appearance(true))
                            .into_any_element()
                    } else {
                        div().flex_1().into_any_element()
                    })
                    .child(
                        div()
                            .id("lib-rename-ok")
                            .px_1()
                            .text_xs()
                            .text_color(rgb(0xa6e3a1))
                            .cursor_pointer()
                            .hover(|s| s.text_color(rgb(0xcdd6f4)))
                            .on_mouse_down(
                                gpui::MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.commit_rename_library(cx);
                                }),
                            )
                            .child("✓"),
                    )
                    .child(
                        div()
                            .id("lib-rename-cancel")
                            .px_1()
                            .text_xs()
                            .text_color(rgb(0xf38ba8))
                            .cursor_pointer()
                            .hover(|s| s.text_color(rgb(0xcdd6f4)))
                            .on_mouse_down(
                                gpui::MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.cancel_rename_library(cx);
                                }),
                            )
                            .child("✕"),
                    ),
            )
        })
        .when(!is_renaming, |el| {
            el.child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(if is_selected {
                                        rgb(0x89b4fa) // Zed blue
                                    } else {
                                        rgb(0x6c7086) // Zed muted
                                    })
                                    .child(icon.to_string()),
                            )
                            .when_some(active_ver, |el, ver| {
                                el.child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(0xa6e3a1)) // green
                                        .child(format!("({})", ver)),
                                )
                            }),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0xcdd6f4)) // Zed text
                            .child(library.name.clone()),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .when(is_used, |el| {
                        el.child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x6c7086))
                                .child(format!("{}", library.versions.len())),
                        )
                    })
                    .child(
                        div()
                            .id(format!("lib-rename-btn-{}", library_id))
                            .px_1()
                            .text_xs()
                            .text_color(rgb(0x45475a))
                            .cursor_pointer()
                            .hover(|s| s.text_color(rgb(0x89b4fa)))
                            .on_mouse_down(
                                gpui::MouseButton::Left,
                                cx.listener({
                                    let library_id = library_id.clone();
                                    let library_name = library.name.clone();
                                    move |this, _, window, cx| {
                                        cx.stop_propagation();
                                        this.start_rename_library(
                                            library_id.clone(),
                                            library_name.clone(),
                                            window,
                                            cx,
                                        );
                                    }
                                }),
                            )
                            .child("✎"),
                    )
                    .child(
                        div()
                            .id(format!("lib-delete-btn-{}", library_id))
                            .px_1()
                            .text_xs()
                            .text_color(rgb(0x45475a))
                            .cursor_pointer()
                            .hover(|s| s.text_color(rgb(0xf38ba8)))
                            .on_mouse_down(
                                gpui::MouseButton::Left,
                                cx.listener({
                                    let library_id = library_id.clone();
                                    move |this, _, _window, cx| {
                                        cx.stop_propagation();
                                        this.delete_library(&library_id, cx);
                                    }
                                }),
                            )
                            .child("🗑"),
                    ),
            )
        })
}

/// Render the "add library" button (Zed IDE style).
fn render_add_library_button(cx: &mut Context<crate::CanDataApp>) -> impl IntoElement {
    div()
        .id("add-lib-btn")
        .px_3()
        .py_1p5()
        .h(px(32.))
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0x252525)))
        .flex()
        .items_center()
        .gap_2()
        .on_mouse_down(
            gpui::MouseButton::Left,
            cx.listener(|this, _event, window, cx| {
                cx.stop_propagation();
                eprintln!("🖱️ Add library button clicked");
                // Initialize input state when showing input
                if this.library_name_input.is_none() {
                    let input = cx
                        .new(|cx| InputState::new(window, cx).placeholder("Enter library name..."));

                    this.library_name_input = Some(input.clone());

                    // Subscribe to input events - detach so the subscription lives
                    // until the input entity is dropped (when library_name_input = None)
                    cx.subscribe(
                        &input,
                        |this: &mut crate::CanDataApp,
                         _input_entity,
                         event: &gpui_component::input::InputEvent,
                         cx| {
                            match event {
                                gpui_component::input::InputEvent::PressEnter { .. } => {
                                    eprintln!(
                                        "📥 Received PressEnter event from library_name_input"
                                    );
                                    let name = this
                                        .library_name_input
                                        .as_ref()
                                        .map(|i| i.read(cx).value().to_string())
                                        .unwrap_or_default();
                                    eprintln!("⏎ Creating library: '{}'", name);

                                    if !name.trim().is_empty() {
                                        this.new_library_name = name.clone();
                                        this.create_library(cx);
                                    }

                                    // Close the dialog
                                    this.show_library_dialog = false;
                                    this.library_name_input = None;
                                    cx.notify();
                                }
                                gpui_component::input::InputEvent::Change => {
                                    // Sync text to state
                                    let name = this
                                        .library_name_input
                                        .as_ref()
                                        .map(|i| i.read(cx).value().to_string())
                                        .unwrap_or_default();
                                    this.new_library_name = name;
                                }
                                _ => {}
                            }
                        },
                    )
                    .detach();

                    eprintln!("✅ Created input and subscribed to events");
                }
                this.show_library_dialog = true;
                cx.notify();
            }),
        )
        .child(
            div()
                .text_sm()
                .text_color(rgb(0x6c7086)) // Zed muted
                .child("+"),
        )
        .child(
            div()
                .text_sm()
                .text_color(rgb(0x9399b2)) // Zed muted text
                .child("New Library"),
        )
}

/// Middle column: version list (Zed IDE style)
fn render_middle_column(
    libraries: &[SignalLibrary],
    selected_library_id: &Option<String>,
    selected_version_id: &Option<String>, // Add selected version ID parameter
    _mappings: &[ChannelMapping],
    show_add_version_input: bool,
    new_version_name: &str,
    _focused_input: &Option<String>,
    _cursor_pos: usize,
    version_name_input: Option<&gpui::Entity<gpui_component::input::InputState>>,
    renaming_version_name: Option<&str>,
    rename_version_input: Option<&gpui::Entity<gpui_component::input::InputState>>,
    active_library_id: Option<&str>,
    active_version_name: Option<&str>,
    cx: &mut Context<crate::CanDataApp>,
) -> impl IntoElement {
    // Find the selected library
    let selected_library = selected_library_id
        .as_ref()
        .and_then(|id| libraries.iter().find(|lib| &lib.id == id));

    let version_count = selected_library.map(|l| l.versions.len()).unwrap_or(0);

    div()
        .w(px(280.0)) // same width as the left column
        .h_full()
        .flex()
        .flex_col()
        .overflow_hidden()
        .child(
            // Fixed top area: title
            div()
                .flex()
                .items_center()
                .justify_between()
                .px_3()
                .py_2()
                .border_b_1()
                .border_color(rgb(0x252525))
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgb(0x6c7086)) // Zed muted
                        .child("VERSIONS"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x6c7086)) // Zed muted
                        .child(format!("{}", version_count)),
                ),
        )
        .child(
            // Scrollable version list
            div()
                .flex_1()
                .overflow_hidden()
                .px_2()
                .py_2()
                .when(selected_library.is_none(), |this| {
                    this.child(
                        div().px_3().py_8().items_center().justify_center().child(
                            div().flex().flex_col().items_center().gap_3().child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x646473))
                                    .child("Select a library first"),
                            ),
                        ),
                    )
                })
                .when_some(selected_library, |this, library| {
                    let mut list = this;
                    let lib_id = library.id.clone();
                    // Show the existing versions
                    for version in &library.versions {
                        let version_name = version.name.clone();
                        let is_selected = selected_version_id.as_ref() == Some(&version_name);
                        let is_active = active_library_id == Some(lib_id.as_str())
                            && active_version_name == Some(version_name.as_str());
                        list = list.child(render_version_item(
                            version,
                            version_name,
                            lib_id.clone(),
                            is_selected,
                            is_active,
                            renaming_version_name,
                            rename_version_input,
                            cx,
                        ));
                    }
                    // Inline "add version" input row (when show_add_version_input is true)
                    if show_add_version_input {
                        list = list.child(render_add_version_input_row(
                            version_name_input,
                            new_version_name,
                            cx,
                        ));
                    }
                    // "Add Version" button
                    list.child(render_add_version_button(cx))
                }),
        )
}

/// Render one version item (Zed IDE style).
fn render_version_item(
    version: &LibraryVersion,
    version_name: String,
    library_id: String,
    is_selected: bool,
    is_active: bool,
    renaming_version_name: Option<&str>,
    rename_version_input: Option<&gpui::Entity<gpui_component::input::InputState>>,
    cx: &mut Context<crate::CanDataApp>,
) -> impl IntoElement {
    let stats = version.get_stats();
    let is_renaming = renaming_version_name == Some(version_name.as_str());

    div()
        .id(format!("ver-{}", version_name))
        .px_3()
        .py_1p5()
        .h(px(32.))
        .cursor_pointer()
        .when(is_selected, |el| {
            el.bg(rgb(0x252525))
                .border_l_1()
                .border_color(rgb(0x89b4fa)) // Zed blue
        })
        .hover(|style| style.bg(rgb(0x1a1a1a)))
        .flex()
        .items_center()
        .justify_between()
        .when(!is_renaming, |el| {
            el.on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener({
                    let version_name = version_name.clone();
                    move |this, _event, _window, cx| {
                        cx.stop_propagation();
                        eprintln!("🖱️ Selected version: {}", version_name);
                        this.selected_version_id = Some(version_name.clone());
                        this.status_msg = format!("Selected version: {}", version_name).into();
                        // Ensure add channel input is hidden when determining selection
                        this.hide_add_channel_input(cx);
                        cx.notify();
                    }
                }),
            )
        })
        .when(is_renaming, |el| {
            el.child(
                div()
                    .flex()
                    .flex_1()
                    .items_center()
                    .gap_1()
                    .child(if let Some(input) = rename_version_input {
                        div()
                            .flex_1()
                            .child(Input::new(input).appearance(true))
                            .into_any_element()
                    } else {
                        div().flex_1().into_any_element()
                    })
                    .child(
                        div()
                            .id("ver-rename-ok")
                            .px_1()
                            .text_xs()
                            .text_color(rgb(0xa6e3a1))
                            .cursor_pointer()
                            .hover(|s| s.text_color(rgb(0xcdd6f4)))
                            .on_mouse_down(
                                gpui::MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.commit_rename_version(cx);
                                }),
                            )
                            .child("✓"),
                    )
                    .child(
                        div()
                            .id("ver-rename-cancel")
                            .px_1()
                            .text_xs()
                            .text_color(rgb(0xf38ba8))
                            .cursor_pointer()
                            .hover(|s| s.text_color(rgb(0xcdd6f4)))
                            .on_mouse_down(
                                gpui::MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.cancel_rename_version(cx);
                                }),
                            )
                            .child("✕"),
                    ),
            )
        })
        .when(!is_renaming, |el| {
            el.child(
                div().flex().flex_col().gap_0().child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(0xcdd6f4))
                                .child(version.name.clone()),
                        )
                        .when(is_active, |s| {
                            s.child(
                                div()
                                    .px_1()
                                    .text_xs()
                                    .text_color(rgb(0xa6e3a1))
                                    .child("● active"),
                            )
                        }),
                ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x6c7086))
                            .child(format!("{}", stats.total_channels)),
                    )
                    .child(
                        // Activate / Deactivate button. ▷ → activate; ▶ → deactivate.
                        div()
                            .id(format!("ver-activate-btn-{}", version_name))
                            .px_1()
                            .text_xs()
                            .text_color(if is_active { rgb(0xa6e3a1) } else { rgb(0x45475a) })
                            .cursor_pointer()
                            .hover(|s| s.text_color(rgb(0xa6e3a1)))
                            .on_mouse_down(
                                gpui::MouseButton::Left,
                                cx.listener({
                                    let version_name = version_name.clone();
                                    let library_id = library_id.clone();
                                    move |this, _, _window, cx| {
                                        cx.stop_propagation();
                                        if is_active {
                                            this.deactivate_library_version(cx);
                                        } else {
                                            this.activate_library_version(&library_id, &version_name, cx);
                                        }
                                    }
                                }),
                            )
                            .child(if is_active { "▶" } else { "▷" }),
                    )
                    .child(
                        div()
                            .id(format!("ver-rename-btn-{}", version_name))
                            .px_1()
                            .text_xs()
                            .text_color(rgb(0x45475a))
                            .cursor_pointer()
                            .hover(|s| s.text_color(rgb(0x89b4fa)))
                            .on_mouse_down(
                                gpui::MouseButton::Left,
                                cx.listener({
                                    let version_name = version_name.clone();
                                    move |this, _, window, cx| {
                                        cx.stop_propagation();
                                        this.start_rename_version(
                                            version_name.clone(),
                                            window,
                                            cx,
                                        );
                                    }
                                }),
                            )
                            .child("✎"),
                    )
                    .child(
                        div()
                            .id(format!("ver-delete-btn-{}", version_name))
                            .px_1()
                            .text_xs()
                            .text_color(rgb(0x45475a))
                            .cursor_pointer()
                            .hover(|s| s.text_color(rgb(0xf38ba8)))
                            .on_mouse_down(
                                gpui::MouseButton::Left,
                                cx.listener({
                                    let version_name = version_name.clone();
                                    let library_id = library_id.clone();
                                    move |this, _, _window, cx| {
                                        cx.stop_propagation();
                                        this.delete_library_version(&library_id, &version_name, cx);
                                    }
                                }),
                            )
                            .child("🗑"),
                    ),
            )
        })
}

/// Right column: channel configuration (Zed IDE style)
fn render_right_column(
    libraries: &[SignalLibrary],
    selected_library_id: &Option<String>,
    selected_version_id: &Option<String>, // Add selected version ID parameter
    _mappings: &[ChannelMapping],
    show_add_channel_input: bool,
    channel_id_input: Option<&gpui::Entity<gpui_component::input::InputState>>,
    channel_name_input: Option<&gpui::Entity<gpui_component::input::InputState>>,
    channel_db_path_input: Option<&gpui::Entity<gpui_component::input::InputState>>,
    new_channel_db_path: &str, // Add this parameter to avoid reading entity in render
    new_channel_type: crate::models::ChannelType, // Use the new channel type being added
    editing_channel_index: Option<usize>, // Row being edited, if any
    cx: &mut Context<crate::CanDataApp>,
) -> impl IntoElement {
    // Find the selected library and version
    let selected_library = selected_library_id
        .as_ref()
        .and_then(|id| libraries.iter().find(|lib| &lib.id == id));

    // Use the selected version name, not the latest version
    let selected_version = selected_library.and_then(|lib| {
        selected_version_id
            .as_ref()
            .and_then(|version_name| lib.versions.iter().find(|v| &v.name == version_name))
    });

    let channel_count = selected_version
        .map(|v| v.channel_databases.len())
        .unwrap_or(0);

    // Use the passed new_channel_type parameter directly instead of getting from library
    let channel_type = new_channel_type;

    // Use the passed parameter instead of reading entity - this avoids borrow conflicts
    let (path_text, path_is_empty) = if show_add_channel_input {
        let is_empty = new_channel_db_path.is_empty();
        let text = if is_empty {
            "No file selected".to_string()
        } else {
            new_channel_db_path.to_string()
        };
        (text, is_empty)
    } else {
        (String::new(), true)
    };

    // Clone entity BEFORE entering .when_some() closure for file picker
    let entity_clone = if show_add_channel_input {
        Some(cx.entity().clone())
    } else {
        None
    };

    div()
        .flex_1() // fill the remaining width
        .h_full()
        .flex()
        .flex_col()
        .overflow_hidden()
        .child(
            // Fixed top area: title
            div()
                .flex()
                .flex_col()
                .gap_2()
                .p_4()
                .border_b_1()
                .border_color(rgb(0x1a1a1a))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(0xcdd6f4)) // Zed text color
                                .child("Channel Configuration"),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_3()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(0x646473)) // Zed muted
                                        .child(format!("{} channels", channel_count)),
                                )
                                .when_some(selected_version, |this, version| {
                                    let lib_id = selected_library_id.clone().unwrap_or_default();
                                    let ver_name = version.name.clone();
                                    this.child(
                                        div()
                                            .px_2()
                                            .py_1()
                                            .bg(rgb(0x3b82f6))
                                            .rounded(px(4.0))
                                            .cursor_pointer()
                                            .hover(|s| s.bg(rgb(0x2563eb)))
                                            .id("apply-version-btn")
                                            .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                                cx.stop_propagation();
                                                this.apply_version_to_mappings(&lib_id, &ver_name, cx);
                                            }))
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(FontWeight::BOLD)
                                                    .text_color(rgb(0xffffff))
                                                    .child("Apply to Plot")
                                            )
                                    )
                                }),
                        ),
                ),
        )
        .child(
            // Scrollable channel list
            div()
                .flex_1()
                .overflow_hidden()
                .flex()
                .flex_col()
                .when_some(selected_version, |this, _version| {
                    // Table header
                    this.child(
                        div()
                            .px_3()
                            .py_2()
                            .bg(rgb(0x0c0c0e))
                            .border_b_1()
                            .border_color(rgb(0x1a1a1a))
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                // Type column
                                div().w(px(60.0)).flex_shrink_0().child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(rgb(0x646473))
                                        .child("Type"),
                                ),
                            )
                            .child(
                                // CH column (channel number)
                                div().w(px(50.0)).flex_shrink_0().child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(rgb(0x646473))
                                        .child("CH"),
                                ),
                            )
                            .child(
                                // Name column
                                div().w(px(120.0)).flex_shrink_0().child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(rgb(0x646473))
                                        .child("Name"),
                                ),
                            )
                            .child(
                                // Database Path column
                                div().flex_1().min_w_0().child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(rgb(0x646473))
                                        .child("Database Path"),
                                ),
                            )
                            // Spacer for the edit + delete buttons (16 + 12 gap + 16)
                            .child(div().w(px(44.)).flex_shrink_0()),
                    )
                })
                .child(
                    div()
                        .id("channel-list")
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .px_2()
                        .py_2()
                        .when(selected_version.is_none(), |this| {
                            this.child(
                                div().px_3().py_8().items_center().justify_center().child(
                                    div().flex().flex_col().items_center().gap_3().child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(0x646473))
                                            .child("Select a library and version to view channels"),
                                    ),
                                ),
                            )
                        })
                        .when_some(selected_version, |this, version| {
                            let mut list = this;
                            // Show the existing channels
                            for (index, channel_db) in version.channel_databases.iter().enumerate() {
                                if show_add_channel_input && editing_channel_index == Some(index) {
                                    // Edit in place, using the same form as Add Channel
                                    list = list.child(render_add_channel_input_row_with_path(
                                        channel_id_input,
                                        channel_name_input,
                                        channel_db_path_input,
                                        channel_type,
                                        path_text.clone(),
                                        path_is_empty,
                                        entity_clone.clone(),
                                        cx,
                                    ));
                                } else {
                                    list = list.child(render_channel_item(index, channel_db, cx));
                                }
                            }
                            // Show the input row (if show_add_channel_input is true)
                            if show_add_channel_input && editing_channel_index.is_none() {
                                list = list.child(render_add_channel_input_row_with_path(
                                    channel_id_input,
                                    channel_name_input,
                                    channel_db_path_input,
                                    channel_type,
                                    path_text.clone(),    // Use pre-read value
                                    path_is_empty,        // Use pre-read value
                                    entity_clone.clone(), // Use pre-cloned entity
                                    cx,
                                ));
                            }
                            // "Add Channel" button
                            list.child(render_add_channel_button(cx))
                        }),
                ),
        )
}

/// Render one channel item as a full single-row entry.
fn render_channel_item(
    index: usize,
    channel_db: &ChannelDatabase,
    cx: &mut Context<crate::CanDataApp>,
) -> impl IntoElement {
    let path = channel_db.database_path.clone();
    let channel_name = channel_db.channel_name.clone();
    let channel_type = channel_db.channel_type;
    let edit_name = channel_name.clone();
    let edit_path = path.clone();

    // Copy channel_id to avoid borrow issues in closure
    let channel_id = channel_db.channel_id;
    // Remote (http/https) database paths are not supported; flag them in red.
    let is_remote = crate::library::is_remote_path(&path);

    div()
        .id(format!("ch-item-{}", channel_id))
        .px_3()
        .py_1()
        .mb_1()
        .min_h(px(32.)) // Grows so long channel names wrap inside the row
        .bg(rgb(0x0c0c0e)) // Zed panel bg
        .border_1()
        .border_color(rgb(0x1a1a1a))
        .rounded(px(3.0))
        .flex()
        .items_center()
        .gap_3()
        .child(
            // Channel type - fixed width
            div().w(px(60.0)).flex_shrink_0().child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if channel_type == crate::models::ChannelType::CAN {
                        rgb(0xa6e3a1) // Green for CAN
                    } else {
                        rgb(0xf9e2af) // Yellow for LIN
                    })
                    .child(if channel_type == crate::models::ChannelType::CAN {
                        "CAN"
                    } else {
                        "LIN"
                    }),
            ),
        )
        .child(
            // Channel ID - fixed width, number only
            div().w(px(50.0)).flex_shrink_0().child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgb(0x89b4fa)) // Zed blue for ID
                    .child(format!("{}", channel_db.channel_id)),
            ),
        )
        .child(
            // Channel name - fixed width
            div().w(px(120.0)).flex_shrink_0().child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgb(0xcdd6f4)) // Zed text
                    .child(channel_name),
            ),
        )
        .child(
            div().flex_1().min_w_0().child(
                div()
                    .text_sm()
                    .text_color(if is_remote { rgb(0xf38ba8) } else { rgb(0x646473) })
                    .truncate()
                    .child(if is_remote {
                        format!("⚠ Remote paths are not supported: {}", path)
                    } else {
                        let normalized = path.replace('\\', "/");
                        if let Some(idx) = normalized.find("libraries/") {
                            normalized[idx..].to_string()
                        } else {
                            path.clone()
                        }
                    }),
            ),
        )
        .child(
            // Edit button
            div()
                .id(format!("ch-edit-{}", channel_id))
                .w(px(16.))
                .h(px(16.))
                .cursor_pointer()
                .hover(|style| style.bg(rgb(0x313244)).text_color(rgb(0xcdd6f4)))
                .rounded(px(2.))
                .flex()
                .items_center()
                .justify_center()
                .flex_shrink_0()
                .text_color(rgb(0x646473)) // Zed muted
                .child("✎")
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(move |this, _event, window, cx| {
                        this.editing_channel_index = Some(index);
                        this.new_channel_type = channel_type;
                        this.new_channel_db_path = edit_path.clone();
                        this.new_channel_id = channel_id.to_string();
                        this.new_channel_name = edit_name.clone();
                        open_channel_form(this, &channel_id.to_string(), &edit_name, window, cx);
                    }),
                ),
        )
        .child(
            // Delete button
            div()
                .id(format!("ch-del-{}", channel_id))
                .w(px(16.))
                .h(px(16.))
                .cursor_pointer()
                .hover(|style| style.bg(rgb(0x382828)).text_color(rgb(0xf38ba8)))
                .rounded(px(2.))
                .flex()
                .items_center()
                .justify_center()
                .flex_shrink_0()
                .text_color(rgb(0x646473)) // Zed muted
                .child("🗑")
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(move |this, _event, _window, cx| {
                        this.delete_channel(channel_id, cx);
                    }),
                ),
        )
}

fn render_add_channel_button(cx: &mut Context<crate::CanDataApp>) -> impl IntoElement {
    div()
        .px_3()
        .py_2()
        .mt_1()
        .border_1()
        .border_dashed()
        .border_color(rgb(0x45475a))
        .rounded(px(3.0))
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0x1a1f2e)))
        .flex()
        .items_center()
        .justify_center()
        .gap_2()
        .on_mouse_down(
            gpui::MouseButton::Left,
            cx.listener(|this, _event, window, cx| {
                eprintln!("🖱️ Add Channel button clicked");

                // Clear previous values
                this.new_channel_db_path.clear();
                this.new_channel_id.clear();
                this.new_channel_name.clear();

                this.editing_channel_index = None;
                open_channel_form(this, "", "", window, cx);
            }),
        )
        .child(
            div().flex().items_center().gap_2().child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgb(0x7dcfff))
                    .child("+ Add Channel"),
            ),
        )
}

/// Create the channel form's inputs and show it. Add Channel passes empty values;
/// Edit passes the channel's current values.
fn open_channel_form(
    this: &mut crate::CanDataApp,
    id: &str,
    name: &str,
    window: &mut Window,
    cx: &mut Context<crate::CanDataApp>,
) {
    // Create fresh InputState entities in the event handler (not in render)
    let id_input = cx.new(|cx| {
        InputState::new(window, cx)
            .placeholder("Channel ID")
            .default_value(id.to_string())
    });
    cx.subscribe(&id_input, |this, input, event, cx| {
        match event {
            gpui_component::input::InputEvent::Change => {
                this.new_channel_id = input.read(cx).text().to_string();
            }
            gpui_component::input::InputEvent::PressEnter { .. } => {
                // Defer focus to next render where window is available
                this.pending_add_channel_focus =
                    Some(crate::app::PendingAddChannelFocus::ChannelName);
                cx.notify();
            }
            _ => {}
        }
    })
    .detach();
    this.channel_id_input = Some(id_input);

    let name_input =
        cx.new(|cx| {
        InputState::new(window, cx)
            .placeholder("Channel name")
            .default_value(name.to_string())
    });
    cx.subscribe(&name_input, |this, input, event, cx| {
        match event {
            gpui_component::input::InputEvent::Change => {
                this.new_channel_name = input.read(cx).text().to_string();
            }
            gpui_component::input::InputEvent::PressEnter { .. } => {
                // Have render call window.blur() next frame so the name input's cursor goes away
                this.pending_add_channel_focus =
                    Some(crate::app::PendingAddChannelFocus::ChannelBlur);
                cx.notify();
            }
            _ => {}
        }
    })
    .detach();
    this.channel_name_input = Some(name_input);

    this.show_add_channel_input = true;
    cx.notify();
}

/// Render the inline "add channel" input row, with the path and entity read ahead (so the entity isn't read during render).
fn render_add_channel_input_row_with_path(
    channel_id_input: Option<&gpui::Entity<gpui_component::input::InputState>>,
    channel_name_input: Option<&gpui::Entity<gpui_component::input::InputState>>,
    _channel_db_path_input: Option<&gpui::Entity<gpui_component::input::InputState>>,
    channel_type: crate::models::ChannelType,
    path_text: String,
    path_is_empty: bool,
    entity_clone: Option<gpui::Entity<crate::CanDataApp>>, // Pre-cloned entity
    cx: &mut Context<crate::CanDataApp>,
) -> impl IntoElement {
    div()
        .px_3()
        .py_1()
        .mb_1()
        .h(px(32.))
        .border_1()
        .border_color(rgb(0x252525))
        .flex()
        .items_center()
        .gap_3()
        .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
            if event.keystroke.key == "escape" {
                // Close the input without saving
                this.hide_add_channel_input(cx);
            }
            // Enter key handling moved to per-input PressEnter subscriptions
            // (see render_add_channel_button) and the ✓ Confirm button.
        }))
        .child(
            // Type selector - click to toggle; width matches the header
            div().w(px(60.0)).flex_shrink_0().child(
                div()
                    .id("add-ch-type-toggle")
                    .px_2()
                    .py_1()
                    .bg(rgb(0x1a1a1a))
                    .rounded(px(2.0))
                    .text_color(if channel_type == crate::models::ChannelType::CAN {
                        rgb(0xa6e3a1) // Green for CAN
                    } else {
                        rgb(0xf9e2af) // Yellow for LIN
                    })
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(0x2a2a2a)))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(if channel_type == crate::models::ChannelType::CAN {
                        "CAN"
                    } else {
                        "LIN"
                    })
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|this, _event, _window, cx| {
                            // Toggle the channel type
                            this.new_channel_type =
                                if this.new_channel_type == crate::models::ChannelType::CAN {
                                    crate::models::ChannelType::LIN
                                } else {
                                    crate::models::ChannelType::CAN
                                };
                            cx.notify();
                        }),
                    ),
            ),
        )
        .child(
            // Channel ID input
            div()
                .w(px(50.0))
                .flex_shrink_0()
                .child(if let Some(input) = channel_id_input {
                    div()
                        .flex_1()
                        .child(Input::new(input))
                        .into_any_element()
                } else {
                    div()
                        .text_color(gpui::rgb(0xffffff))
                        .text_sm()
                        .child("ID...")
                        .into_any_element()
                }),
        )
        .child(
            // Channel name input
            div()
                .w(px(120.0))
                .flex_shrink_0()
                .child(if let Some(input) = channel_name_input {
                    div()
                        .flex_1()
                        .child(Input::new(input))
                        .into_any_element()
                } else {
                    div()
                        .text_color(gpui::rgb(0xffffff))
                        .text_sm()
                        .child("Name...")
                        .into_any_element()
                }),
        )
        .child(
            // Database path display - read-only, chosen with the Browse button
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .gap_2()
                .items_center()
                .child(
                    // Show the chosen path or a hint
                    div().flex_1().min_w_0().child(
                        div()
                            .text_sm()
                            .text_color(if path_is_empty {
                                rgb(0x646473) // gray hint
                            } else {
                                rgb(0xcdd6f4) // white text
                            })
                            .truncate()
                            .child(path_text),
                    ),
                )
                .child({
                    // File picker button - saves automatically after choosing
                    // Uses the entity cloned ahead of time, so it isn't read inside .when_some()
                    if let Some(this) = entity_clone {
                        div()
                            .px_3()
                            .py_1()
                            .cursor_pointer()
                            .hover(|style| style.bg(rgb(0x313244)))
                            .rounded(px(2.0))
                            .border_1()
                            .border_color(rgb(0x45475a))
                            .text_color(rgb(0x7dcfff))
                            .text_xs()
                            .child("Select File...")
                            .on_mouse_down(gpui::MouseButton::Left, move |_event, _window, app| {
                                let this = this.clone();
                                app.spawn(async move |cx| {
                                    let dialog = rfd::AsyncFileDialog::new();
                                    
                                    let dialog = match channel_type {
                                        crate::models::ChannelType::CAN => dialog.add_filter("DBC Files", &["dbc"]),
                                        crate::models::ChannelType::LIN => dialog.add_filter("LDF Files", &["ldf"]),
                                    };

                                    if let Some(file) = dialog
                                        .pick_file()
                                        .await
                                    {
                                        let path_str = file.path().to_string_lossy().to_string();
                                        this.update(cx, |view, _cx| {
                                            // Save the file path
                                            view.new_channel_db_path = path_str.clone();
                                            eprintln!("📁 File selected: {}", path_str);

                                            // Auto-fill channel name from filename if empty
                                            if view.new_channel_name.is_empty() {
                                                if let Some(stem) = std::path::Path::new(&path_str).file_stem() {
                                                    view.new_channel_name = stem.to_string_lossy().to_string();
                                                }
                                            }

                                            // Auto-save the channel configuration
                                            // view.save_channel_config(cx); // Removed auto-save to allow user to input ID/Name after file selection
                                        });
                                    }
                                    Ok::<(), anyhow::Error>(())
                                })
                                .detach();
                            })
                    } else {
                        // Fallback if no entity clone (shouldn't happen)
                        div().child("Error: No entity")
                    }
                }),
        )
        .child(
            // Action buttons: confirm and cancel
            div()
                .flex()
                .items_center()
                .gap_1()
                .ml_2()
                .flex_shrink_0()
                .child(
                    // Confirm button
                    // Note: Enter-on-✓ requires a FocusHandle (out of scope per
                    // plan Task 8 design note). User clicks ✓ or tabs to it via
                    // the row container's focus.
                    div()
                        .id("add-ch-confirm")
                        .w(px(20.))
                        .h(px(20.))
                        .cursor_pointer()
                        .hover(|style| style.bg(rgb(0x313244)))
                        .rounded(px(3.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(0xa6e3a1)) // Green
                                .child("✓"),
                        )
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                eprintln!("🖱️ Confirm button clicked");
                                this.save_channel_config(cx);
                            }),
                        ),
                )
                .child(
                    // Cancel button
                    div()
                        .id("add-ch-cancel")
                        .w(px(20.))
                        .h(px(20.))
                        .cursor_pointer()
                        .hover(|style| style.bg(rgb(0x313244)))
                        .rounded(px(3.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(0xf38ba8)) // Red
                                .child("✕"),
                        )
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.hide_add_channel_input(cx);
                            }),
                        ),
                ),
        )
}

/// Render the inline "add version" input row, blended into the list.
fn render_add_version_input_row(
    version_name_input: Option<&gpui::Entity<gpui_component::input::InputState>>,
    _new_version_name: &str,
    cx: &mut Context<crate::CanDataApp>,
) -> impl IntoElement {
    div()
        .px_3()
        .py_1p5()
        .h(px(32.))
        .border_1()
        .border_color(rgb(0x252525))
        .flex()
        .items_center()
        .gap_2()
        .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
            if event.keystroke.key == "escape" {
                // Close the input without saving
                this.show_version_input = false;
                this.version_name_input = None;
                cx.notify();
            }
        }))
        .child(
            div()
                .flex_1()
                .child(if let Some(input) = version_name_input {
                    div()
                        .flex_1()
                        .child(Input::new(input).appearance(true))
                        .into_any_element()
                } else {
                    div()
                        .text_color(gpui::rgb(0xffffff))
                        .text_sm()
                        .child("Version name...")
                        .into_any_element()
                }),
        )
}

/// Render the "add version" button as a compact single row.
fn render_add_version_button(cx: &mut Context<crate::CanDataApp>) -> impl IntoElement {
    div()
        .id("add-version-btn")
        .px_3()
        .py_1() // less padding, more compact
        .mb_1()
        .h(px(32.)) // fixed single-row height
        .border_1()
        .border_dashed()
        .border_color(rgb(0x45475a)) // Zed border
        .rounded(px(3.0))
        .cursor_pointer()
        .hover(|style| {
            style.bg(rgb(0x1a1f2e)) // Zed green hint on hover
        })
        .flex()
        .items_center()
        .gap_2()
        .on_mouse_down(
            gpui::MouseButton::Left,
            cx.listener(|this, _event, window, cx| {
                cx.stop_propagation();
                eprintln!("🖱️ Add version button clicked");
                // Initialize input state when showing input
                if this.version_name_input.is_none() {
                    let input = cx.new(|cx| {
                        InputState::new(window, cx)
                            .placeholder("Enter version name (e.g., v1.0)...")
                    });

                    this.version_name_input = Some(input);
                }
                this.show_version_input = true;
                cx.notify();
            }),
        )
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(0x9399b2)) // Zed muted
                .child("+ Add Version"),
        )
}
