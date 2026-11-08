//! Domain layer - Pure business logic without UI dependencies
//!
//! This module contains core business logic that is independent of the UI framework (GPUI).
//! All code here should:
//! - Not depend on gpui types
//! - Be testable without UI context
//! - Handle data processing, validation, transformation

// Pure domain models
pub mod models;

// Business services
pub mod services;

// Legacy modules (to be refactored)
pub mod config_manager;
pub mod log_processor;
pub mod signal_decoder;
pub mod time_handler;

// Multi-file loading and global merged view
pub mod multi_file;
