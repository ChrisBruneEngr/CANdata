//! Domain models - Pure business logic without UI dependencies
//!
//! This module contains core business models that are independent of the UI framework.
//! All models here:
//! - Do not depend on gpui types
//! - Are testable without UI context
//! - Represent business concepts clearly

pub mod message;
pub mod channel;
pub mod filter;
