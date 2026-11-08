//! UI components
//!
//! Reusable UI components for the application.

// Core working components
pub mod button;
pub mod top_bar;
pub mod status_bar;
pub mod library_picker;
pub use top_bar::render_top_bar;
pub use status_bar::render_status_bar;
pub use status_bar::render_status_bar_popovers;
pub use library_picker::render_library_picker_overlay;

// Re-exports for working components
pub use button::{Button, ButtonSize};
