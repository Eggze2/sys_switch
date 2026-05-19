//! Linux boot manager implementation using efibootmgr and grub-reboot.

use anyhow::{anyhow, Result};
use regex::Regex;
use std::process::Command;

use super::{BootEntry, BootManager};

/// Linux boot manager that uses efibootmgr or grub-reboot.
pub struct LinuxBootManager {
    efibootmgr_path: Option<String>,
    grub_reboot_path: Option<String>,
}

impl LinuxBootManager {
    /// Create a new Linux boot manager, detecting available tools.
    pub fn new() -> Self {
        Self {
            efibootmgr_path: which("efibootmgr"),
            grub_reboot_path: which("grub-reboot"),
        }
    }

    /// Check if running as root.
    fn is_root() -> bool {
        unsafe { libc::geteuid() == 0 }
    }
}

impl Default for LinuxBootManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BootManager for LinuxBootManager {
    fn available(&self) -> bool {
        self.efibootmgr_path.is_some() || self.grub_reboot_path.is_some()
    }

    fn list_entries(&self) -> Result<Vec<BootEntry>> {
        let mut entries = Vec::new();

        if let Some(ref efibootmgr) = self.efibootmgr_path {
            let output = Command::new(efibootmgr).output()?;
            let text = String::from_utf8_lossy(&output.stdout);

            // Parse BootCurrent and BootNext
            let current_re = Regex::new(r"BootCurrent:\s*(\w+)")?;
            let next_re = Regex::new(r"BootNext:\s*(\w+)")?;
            let entry_re = Regex::new(r"Boot(\w+)\*?\s+(.+)")?;

            let current = current_re
                .captures(&text)
                .map(|c| c.get(1).unwrap().as_str().to_string());
            let next = next_re
                .captures(&text)
                .map(|c| c.get(1).unwrap().as_str().to_string());

            for cap in entry_re.captures_iter(&text) {
                let id = cap.get(1).unwrap().as_str().to_string();
                let desc = cap.get(2).unwrap().as_str().trim().to_string();
                let is_current = current.as_ref().map_or(false, |c| c == &id);
                let is_next = next.as_ref().map_or(false, |n| n == &id);

                entries.push(
                    BootEntry::new(id, desc)
                        .with_current(is_current)
                        .with_next(is_next),
                );
            }
            return Ok(entries);
        }

        // Fallback to grub
        if let Some(ref _grub_reboot) = self.grub_reboot_path {
            if let Some(grub_editenv) = which("grub-editenv") {
                let output = Command::new(grub_editenv).arg("list").output()?;
                let text = String::from_utf8_lossy(&output.stdout);

                let saved_re = Regex::new(r"saved_entry=(.+)")?;
                let saved = saved_re
                    .captures(&text)
                    .map(|c| c.get(1).unwrap().as_str().to_string());

                entries.push(
                    BootEntry::new("0", "GRUB default entry")
                        .with_next(saved.as_ref().map_or(false, |s| s == "0")),
                );
            }
        }

        Ok(entries)
    }

    fn set_next(&self, entry_id: &str) -> Result<String> {
        if !Self::is_root() {
            return Err(anyhow!("Root privileges required"));
        }

        // Prefer efibootmgr
        if let Some(ref efibootmgr) = self.efibootmgr_path {
            let output = Command::new(efibootmgr).args(["-n", entry_id]).output()?;

            if output.status.success() {
                return Ok(format!("Next boot entry set to: {}", entry_id));
            }
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("{}", stderr));
        }

        // Fallback to grub-reboot
        if let Some(ref grub_reboot) = self.grub_reboot_path {
            let output = Command::new(grub_reboot).arg(entry_id).output()?;

            if output.status.success() {
                return Ok(format!("GRUB next boot entry set to: {}", entry_id));
            }
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("{}", stderr));
        }

        Err(anyhow!(
            "No supported boot manager found (efibootmgr/grub-reboot)"
        ))
    }

    fn reboot_now(&self) -> Result<()> {
        if !Self::is_root() {
            return Err(anyhow!("Root privileges required to reboot"));
        }

        let output = Command::new("systemctl").arg("reboot").output()?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("{}", stderr))
        }
    }
}

/// Find an executable in PATH.
fn which(cmd: &str) -> Option<String> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .filter_map(|dir| {
                let full_path = dir.join(cmd);
                if full_path.is_file() {
                    Some(full_path.to_string_lossy().into_owned())
                } else {
                    None
                }
            })
            .next()
    })
}
