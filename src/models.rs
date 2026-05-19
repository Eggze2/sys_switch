//! Boot entry data model.

use serde::{Deserialize, Serialize};

/// Represents a boot entry in the system's boot manager.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootEntry {
    /// Entry identifier (Linux: "0000" style; Windows: "{GUID}")
    pub id: String,
    /// Human-readable description of the boot entry
    pub description: String,
    /// Whether this is the currently booted entry
    pub is_current: bool,
    /// Whether this is set as the next boot entry
    pub is_next: bool,
}

impl BootEntry {
    /// Create a new boot entry.
    pub fn new(id: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            is_current: false,
            is_next: false,
        }
    }

    /// Set the current boot flag.
    pub fn with_current(mut self, is_current: bool) -> Self {
        self.is_current = is_current;
        self
    }

    /// Set the next boot flag.
    pub fn with_next(mut self, is_next: bool) -> Self {
        self.is_next = is_next;
        self
    }
}
