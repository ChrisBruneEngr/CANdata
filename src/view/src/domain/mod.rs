//! Domain layer - Pure business logic without UI dependencies
//!
//! This module contains core business logic that is independent of the UI framework (GPUI).
//! All code here should:
//! - Not depend on gpui types
//! - Be testable without UI context
//! - Handle data processing, validation, transformation

// Multi-file loading and global merged view
pub mod multi_file;
