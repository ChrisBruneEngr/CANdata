//! Application module
//!
//! Contains the core application state and implementation.

mod impls;
mod impls_rendering;
mod state;
mod commands;

// Re-export the main types
pub use state::{AppView, CanViewerApp, LoadingProgress, PendingAddChannelFocus};

// Define actions for text input handling (public, so other modules can use them)
// Note: actions! macro defines the types in the current scope, not in a separate module
gpui::actions!(library_input, [Backspace, Delete, Left, Right, Home, End]);

