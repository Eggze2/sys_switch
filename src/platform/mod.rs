//! Platform abstraction layer for boot management.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;

use anyhow::Result;

pub use crate::models::BootEntry;

#[cfg(target_os = "linux")]
pub use linux::LinuxBootManager;

#[cfg(target_os = "windows")]
pub use windows::WindowsBootManager;

/// Trait for platform-specific boot management operations.
pub trait BootManager: Send + Sync {
    /// Check if the boot manager is available on this system.
    fn available(&self) -> bool;

    /// List all available boot entries.
    fn list_entries(&self) -> Result<Vec<BootEntry>>;

    /// Set the next boot entry (one-time).
    fn set_next(&self, entry_id: &str) -> Result<String>;

    /// Reboot the system immediately.
    fn reboot_now(&self) -> Result<()>;
}

/// Get the appropriate boot manager for the current platform.
pub fn get_boot_manager() -> Box<dyn BootManager> {
    #[cfg(target_os = "linux")]
    {
        Box::new(LinuxBootManager::new())
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(WindowsBootManager::new())
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        compile_error!("Unsupported platform")
    }
}
