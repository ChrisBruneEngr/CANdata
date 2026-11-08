//! Application state structures
//!
//! This module contains the core application state structures.

/// 多文件加载进度
#[derive(Clone, Debug, Default)]
pub struct LoadingProgress {
    pub total_files: usize,
    pub completed_files: usize,
    pub current_file_name: Option<String>,
    pub total_messages_so_far: usize,
    pub is_cancelled: bool,
}

use blf::LogObject;
use gpui::{Entity, Pixels, UniformListScrollHandle};
use parser::dbc::DbcDatabase;
use parser::ldf::LdfDatabase;
use std::collections::HashMap;
use std::path::PathBuf;

// Import AppConfig and ChannelMapping from crate root (defined in main.rs)
use crate::{AppConfig, ChannelType};

// Import the real LibraryManager from the library module
pub use crate::library::LibraryManager;


// Import gpui-component input support
use gpui_component::input::InputState;

/// Application view modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppView {
    LogView,
    ConfigView,
    LibraryView,
    PlotView,
}

/// State for tracking scrollbar drag operation
#[derive(Clone)]
pub struct ScrollbarDragState {
    pub start_y: Pixels,
    pub start_scroll_offset: f32,
    pub filtered_count: usize, // Number of filtered messages at drag start
}

/// Main application state
pub struct CanDataApp {
    // View state
    pub current_view: AppView,

    // Data
    pub messages: std::sync::Arc<[LogObject]>,
    pub dbc_channels: HashMap<u16, DbcDatabase>,
    pub ldf_channels: HashMap<u16, LdfDatabase>,
    pub app_config: AppConfig,
    pub selected_signals: Vec<String>,
    pub plot_lanes: crate::plot_lanes::LaneLayout,
    pub start_time: Option<chrono::NaiveDateTime>,
    pub plot_data: std::sync::Arc<[crate::models::Series]>,
    /// Full (unfiltered) decoded series — zoom only re-filters this, never re-decodes messages
    pub plot_full_data: std::sync::Arc<[crate::models::Series]>,

    // Configuration
    pub config_dir: Option<PathBuf>,
    pub config_file_path: Option<PathBuf>,


    // Currently loaded BLF file name (for StatusBar display)
    pub current_file_name: Option<String>,
    // 多文件加载状态
    pub files: Vec<std::sync::Arc<crate::domain::multi_file::FileSegment>>,
    pub merged: crate::domain::multi_file::MergedView,
    pub show_files_popover: bool,
    pub loading_progress: Option<crate::app::state::LoadingProgress>,
    // Library picker UI state (not persisted to config)
    pub library_picker_dismissed: bool,
    pub library_picker_selected_version: std::collections::HashMap<String, String>,
    // BLF file size and parser-consumed bytes (for StatusBar progress)
    pub blf_bytes_total: u64,
    pub blf_bytes_consumed: u64,
    // BLF parse errors (full Display strings, shown in the details popover)
    pub blf_parse_errors: Vec<String>,
    // Controls the BLF errors details popover in the StatusBar
    pub show_blf_errors_popover: bool,
    // Drag-and-drop state
    /// Validated BLF paths waiting for an in-flight load to finish.
    /// Drained by the render tick when `loading_progress` is None.
    pub pending_drop_paths: Vec<std::path::PathBuf>,

    // Scroll state
    pub list_scroll_handle: UniformListScrollHandle,
    pub scrollbar_drag_state: Option<ScrollbarDragState>,
    pub scroll_offset: Pixels,
    pub list_container_height: f32,

    // Display settings
    pub id_display_decimal: bool, // true for decimal, false for hexadecimal

    // ID filter
    pub id_filter: Option<u32>,
    pub id_filter_text: gpui::SharedString,
    pub show_id_filter_input: bool,

    // Filter dropdown state
    pub filter_scroll_offset: Pixels,
    pub filter_scroll_handle: UniformListScrollHandle,
    pub mouse_over_filter_dropdown: bool,

    // Channel filter
    pub channel_filter: Option<u16>,
    pub channel_filter_text: gpui::SharedString,
    pub show_channel_filter_input: bool,
    pub channel_filter_scroll_offset: Pixels,
    pub channel_filter_scroll_handle: UniformListScrollHandle,

    // Signal filter
    pub signal_filter_text: gpui::SharedString,
    pub signal_search_input: Option<Entity<InputState>>,

    // Status message
    pub status_msg: gpui::SharedString,

    // Library management
    pub library_manager: LibraryManager,
    pub selected_library_id: Option<String>,
    pub selected_version_id: Option<String>, // Add selected version ID
    /// The currently "activated" library version used for log decoding and plot
    pub active_library_id: Option<String>,
    pub active_version_name: Option<String>,
    pub new_library_name: String,
    pub library_cursor_position: usize,
    pub show_version_input: bool,
    pub new_version_name: String,
    pub new_version_cursor_position: usize,
    pub show_library_dialog: bool,

    // gpui-component input support for library management
    pub library_name_input: Option<Entity<InputState>>,
    pub version_name_input: Option<Entity<InputState>>,

    // Rename inline state
    pub renaming_library_id: Option<String>,
    pub renaming_version_name: Option<String>,
    pub rename_library_input: Option<Entity<InputState>>,
    pub rename_version_input: Option<Entity<InputState>>,
    pub rename_library_text: String,
    pub rename_version_text: String,

    // Channel configuration dialog state
    pub new_channel_id: String,
    pub new_channel_name: String,
    pub new_channel_db_path: String,
    pub editing_channel_index: Option<usize>, // None for adding new, Some(index) for editing
    pub channel_id_input: Option<Entity<InputState>>,
    pub channel_name_input: Option<Entity<InputState>>,
    pub show_add_channel_input: bool, // Controls inline input display in channel list
    pub channel_db_path_input: Option<Entity<InputState>>, // For database path input
    pub new_channel_type: ChannelType, // Store selected channel type (CAN/LIN)
    pub pending_file_path: Option<std::sync::mpsc::Receiver<Option<String>>>, // For file dialog result

    // Plot zoom state
    pub plot_zoom_start: Option<f64>,
    pub plot_zoom_end: Option<f64>,
    pub plot_full_time_min: Option<f64>,
    pub plot_full_time_max: Option<f64>,
    pub is_dragging_zoom: bool,
    /// While right-dragging: the mouse x and zoom window where the pan began.
    pub plot_pan_start: Option<(Pixels, f64, f64)>,
    pub zoom_drag_start_x: Option<Pixels>,
    pub zoom_drag_current_x: Option<Pixels>,

    // Plot display settings

    // Plot interaction state
    /// Time of the cursor set by clicking a chart.
    pub plot_cursor_time: Option<f64>,
    /// Formatted value of every signal at the cursor, keyed by signal ID.
    pub plot_cursor_values: std::collections::HashMap<String, String>,
    /// Scroll handle for the plot canvas (vertical scroll over the chart list).
    pub plot_scroll_handle: gpui::ScrollHandle,

    // File menu dropdown state
    pub show_file_menu: bool,
    // Help menu dropdown state
    pub show_help_menu: bool,
    pub show_settings_menu: bool,
    pub plot_y_min_input: Option<Entity<InputState>>,
    pub plot_y_max_input: Option<Entity<InputState>>,
    /// The Y range the Min and Max boxes currently hold.
    pub plot_y_inputs_loaded: Option<(f64, f64)>,

    // Selected row index (for row-highlight on click-to-copy)
    pub selected_row_index: Option<usize>,

    // Plot sidebar fold state (session-only, not persisted to disk)
    pub expanded_channels: std::collections::HashSet<u16>,
    pub expanded_messages: std::collections::HashSet<(u16, u32)>, // (ch_id, msg_id)

    // Add-channel form Enter-key focus chain: set by PressEnter subscribe,
    // consumed by render() which has window access.
    pub pending_add_channel_focus: Option<PendingAddChannelFocus>,

    // Signal sets — named collections of (ch, msg_id, signal) per library
    pub signal_set_store: crate::library::signal_sets::SignalSetStore,
    /// Currently active set as (library_id, set_name); cleared on library switch / manual edit
    pub active_signal_set: Option<(String, String)>,
    /// Inline input buffer when saving the current selection as a set
    pub pending_signal_set_name: Option<String>,
    /// Whether the inline "save as set" input row is currently shown
    pub show_save_set_input: bool,
    /// Whether the signal-sets dropdown menu is currently open
    pub show_signal_set_dropdown: bool,
    /// Inline-input entity for the "save as signal set" name field.
    /// Lazily created on first show (see render_signal_set_dropdown_row
    /// initialization in impls_rendering.rs render loop).
    pub signal_set_name_input: Option<gpui::Entity<gpui_component::input::InputState>>,

}

/// Which input to focus next when the user presses Enter in the add-channel form.
/// Set by `InputEvent::PressEnter` subscriptions (no window access there),
/// consumed by `render()` which has window access.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PendingAddChannelFocus {
    /// Focus the channel name input.
    ChannelName,
    /// Blur the window (so the channel name input loses focus and the cursor stops blinking).
    ChannelBlur,
}

impl CanDataApp {
    /// Create a new CanDataApp instance with default state (used by tests)
    #[cfg(test)]
    pub fn new_state() -> Self {
        Self {
            current_view: AppView::LogView, // Force LogView to prevent chart/library crashes
            messages: std::sync::Arc::from([]),
            status_msg: gpui::SharedString::from(""),
            dbc_channels: HashMap::new(),
            ldf_channels: HashMap::new(),
            app_config: AppConfig::default(),
            selected_signals: Vec::new(),
            start_time: None,
            plot_data: std::sync::Arc::from([]),
            plot_full_data: std::sync::Arc::from([]),
            config_dir: None,
            config_file_path: None,
            current_file_name: None,
            files: Vec::new(),
            merged: crate::domain::multi_file::MergedView::empty(),
            show_files_popover: false,
            loading_progress: None,
            library_picker_dismissed: false,
            library_picker_selected_version: std::collections::HashMap::new(),
            blf_bytes_total: 0,
            blf_bytes_consumed: 0,
            blf_parse_errors: Vec::new(),
            show_blf_errors_popover: false,
            pending_drop_paths: Vec::new(),
            list_scroll_handle: UniformListScrollHandle::new(),
            scrollbar_drag_state: None,
            scroll_offset: gpui::px(0.0),
            list_container_height: 850.0,
            id_display_decimal: false,
            id_filter: None,
            id_filter_text: gpui::SharedString::from(""),
            show_id_filter_input: false,
            filter_scroll_offset: gpui::px(0.0),
            filter_scroll_handle: UniformListScrollHandle::new(),
            mouse_over_filter_dropdown: false,
            channel_filter: None,
            channel_filter_text: gpui::SharedString::from(""),
            show_channel_filter_input: false,
            channel_filter_scroll_offset: gpui::px(0.0),
            channel_filter_scroll_handle: UniformListScrollHandle::new(),
            signal_filter_text: gpui::SharedString::from(""),
            library_manager: LibraryManager::new(),
            selected_library_id: None,
            selected_version_id: None, // Initialize selected version ID
            active_library_id: None,
            active_version_name: None,
            new_library_name: String::new(),
            library_cursor_position: 0,
            show_version_input: false,
            new_version_name: String::new(),
            new_version_cursor_position: 0,
            show_library_dialog: false,
            signal_search_input: None,
            // gpui-component input support
            library_name_input: None, // Will be initialized when cx is available
            version_name_input: None, // Will be initialized when cx is available
            // Rename inline state
            renaming_library_id: None,
            renaming_version_name: None,
            rename_library_input: None,
            rename_version_input: None,
            rename_library_text: String::new(),
            rename_version_text: String::new(),
            // Channel configuration dialog
            new_channel_id: String::new(),
            new_channel_name: String::new(),
            new_channel_db_path: String::new(),
            editing_channel_index: None,
            channel_id_input: None, // Will be initialized when cx is available
            channel_name_input: None, // Will be initialized when cx is available
            show_add_channel_input: false,
            channel_db_path_input: None, // Will be initialized when cx is available
            new_channel_type: ChannelType::CAN, // Default to CAN
            pending_file_path: None,     // For file dialog result
            // Deprecated fields for backward compatibility
            // Plot zoom state
            plot_zoom_start: None,
            plot_zoom_end: None,
            plot_full_time_min: None,
            plot_full_time_max: None,
            is_dragging_zoom: false,
            plot_pan_start: None,
            zoom_drag_start_x: None,
            zoom_drag_current_x: None,
            // Plot display settings
            plot_cursor_time: None,
            plot_cursor_values: std::collections::HashMap::new(),
            plot_scroll_handle: gpui::ScrollHandle::new(),
            // File menu dropdown state
            show_file_menu: false,
            // Help menu dropdown state
            show_help_menu: false,
            show_settings_menu: false,
            plot_y_min_input: None,
            plot_y_max_input: None,
            plot_y_inputs_loaded: None,
            plot_lanes: Default::default(),
            selected_row_index: None,
            expanded_channels: std::collections::HashSet::new(),
            expanded_messages: std::collections::HashSet::new(),
            pending_add_channel_focus: None,
            // Signal sets
            signal_set_store: crate::library::signal_sets::load_signal_set_store(None),
            active_signal_set: None,
            pending_signal_set_name: None,
            show_save_set_input: false,
            show_signal_set_dropdown: false,
            signal_set_name_input: None,
        }
    }

}
